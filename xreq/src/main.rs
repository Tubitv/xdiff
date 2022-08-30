use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input};
use mime::Mime;
use serde_json::Value;
use std::{io::Write, path::PathBuf};

use xreq_cli_utils::{get_config_file, get_default_config, parse_key_val, print_syntect};
use xreq_lib::{KeyVal, RequestConfig, RequestContext, Response};

/// HTTP request tool just as curl/httpie, but easier to use.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand, Debug, Clone)]
enum Action {
    /// parse a URL and print the generated request config.
    Parse(ParseArgs),
    /// Send API request based on a given profile.
    Run(RunArgs),
}

#[derive(Parser, Debug, Clone)]
struct ParseArgs {
    /// Profile name. Defaults to "default".
    #[clap(short, long, value_parser, default_value = "default")]
    profile: String,
    /// URL to parse.
    #[clap(value_parser)]
    url: Option<String>,
}

#[derive(Parser, Debug, Clone)]
struct RunArgs {
    /// API profile to use.
    #[clap(short, long, value_parser)]
    profile: String,

    /// Extra parameters to pass to the API.
    /// If no prefix, it will be used for querystring;
    /// If prefix is '@', it will be used for body;
    /// If prefix is '%', it will be used for header.
    #[clap(short, value_parser = parse_key_val, number_of_values = 1)]
    extra_params: Vec<KeyVal>,

    /// Path to the config file.
    #[clap(short, long, value_parser = get_config_file)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut output: Vec<String> = Vec::new();

    match args.action {
        Action::Parse(args) => parse(&mut output, args)?,
        Action::Run(args) => run(&mut output, args).await?,
    }

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    for line in output {
        write!(stdout, "{}", line)?;
    }

    Ok(())
}

fn parse(output: &mut Vec<String>, ParseArgs { profile, url }: ParseArgs) -> Result<()> {
    let (profile, url) = match url {
        Some(url) => (profile, url),
        None => {
            let url = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Url to parse")
                .interact()?;
            let profile = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Give this url a profile name")
                .default("default".into())
                .interact()?;
            (profile, url)
        }
    };

    let ctx: RequestContext = url.parse()?;
    let config = RequestConfig::new_with_profile(profile, ctx);

    let result = serde_yaml::to_string(&config)?;

    output.push("---\n".to_string());
    print_syntect(output, result, "yaml")?;
    Ok(())
}

async fn run(output: &mut Vec<String>, args: RunArgs) -> Result<()> {
    let config_file = args.config.unwrap_or(get_default_config("xreq.yml")?);

    let request_config = RequestConfig::try_load(&config_file).await?;

    let mut config = request_config.get(&args.profile)?.clone();

    config.update(&args.extra_params)?;

    let resp = config.send().await?;

    if atty::is(atty::Stream::Stdout) {
        print_status(output, &resp);
        print_headers(output, &resp);
    }

    let mime = get_content_type(&resp);
    let body = resp.text().await?;

    print_body(output, mime, body)?;

    Ok(())
}

fn print_status(output: &mut Vec<String>, resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    output.push(format!("{}\n", status));
}

fn print_headers(output: &mut Vec<String>, resp: &Response) {
    for (name, value) in resp.headers() {
        output.push(format!("{}: {:?}\n", name.to_string().green(), value));
    }

    output.push("\n".into());
}

fn print_body(output: &mut Vec<String>, m: Option<Mime>, body: String) -> Result<()> {
    match m {
        Some(v) if v.essence_str() == mime::APPLICATION_JSON => {
            let json: Value = serde_json::from_str(&body).unwrap();
            let body = serde_json::to_string_pretty(&json).unwrap();
            print_syntect(output, body, "json")
        }
        Some(v) if v == mime::TEXT_HTML => print_syntect(output, body, "html"),

        _ => {
            output.push(format!("{}\n", body));
            Ok(())
        }
    }
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().parse().unwrap())
}
