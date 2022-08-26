use anyhow::Result;
use clap::Parser;
use cli_utils::{get_config_file, get_default_config, parse_key_val};
use colored::Colorize;
use jql::groups_walker;
use mime::Mime;
use requester::{RequestConfig, Response};
use serde_json::Value;
use std::path::PathBuf;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

/// Diff API response.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// API profile to use.
    #[clap(short, long, value_parser)]
    profile: String,

    /// Extra parameters to pass to the API.
    #[clap(short, value_parser = parse_key_val, number_of_values = 1)]
    extra_params: Vec<(String, String)>,

    /// JQ query string.
    #[clap(long, value_parser)]
    jq: Option<String>,

    /// Path to the config file.
    #[clap(short, long, value_parser = get_config_file)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config_file = args.config.unwrap_or(get_default_config("xreq.yml")?);

    let request_config = RequestConfig::try_load(&config_file).await?;

    let mut config = request_config.get(&args.profile)?.clone();

    for (key, val) in args.extra_params {
        config.params[&key] = serde_json::Value::String(val.clone());
    }

    let resp = config.send().await?;

    print_status(&resp);
    print_headers(&resp);
    let mime = get_content_type(&resp);
    let mut body = resp.text().await?;

    if let Some(jq) = args.jq {
        if let Ok(v) = serde_json::from_str(&body) {
            let groups = jql::selectors_parser(&jq).map_err(|e| anyhow::anyhow!(e))?;
            let result = groups_walker(&v, &groups).map_err(|e| anyhow::anyhow!(e))?;
            body = serde_json::to_string_pretty(&result)?;
        }
    }
    print_body(mime, &body);

    Ok(())
}

fn print_status(resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

fn print_headers(resp: &Response) {
    for (name, value) in resp.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }

    println!();
}

fn print_body(m: Option<Mime>, body: &str) {
    match m {
        Some(v) if v.essence_str() == mime::APPLICATION_JSON => {
            let json: Value = serde_json::from_str(body).unwrap();
            let body = serde_json::to_string_pretty(&json).unwrap();
            print_syntect(&body, "json");
        }
        Some(v) if v == mime::TEXT_HTML => print_syntect(body, "html"),

        _ => println!("{}", body),
    }
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

fn print_syntect(s: &str, ext: &str) {
    // Load these once at the start of your program
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension(ext).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for line in LinesWithEndings::from(s) {
        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{}", escaped);
    }
}
