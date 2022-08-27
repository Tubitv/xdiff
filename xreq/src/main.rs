use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use mime::Mime;
use serde_json::Value;
use std::{io::Write, path::PathBuf};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
use xreq_cli_utils::{get_config_file, get_default_config, parse_key_val};
use xreq_lib::{KeyVal, RequestConfig, Response};

/// HTTP request tool just as curl/httpie, but easier to use.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
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

    let config_file = args.config.unwrap_or(get_default_config("xreq.yml")?);

    let request_config = RequestConfig::try_load(&config_file).await?;

    let mut config = request_config.get(&args.profile)?.clone();

    config.update(&args.extra_params)?;

    let resp = config.send().await?;
    let mut output: Vec<String> = Vec::new();

    if atty::is(atty::Stream::Stdout) {
        print_status(&mut output, &resp);
        print_headers(&mut output, &resp);
    }

    let mime = get_content_type(&resp);
    let body = resp.text().await?;

    print_body(&mut output, mime, body);

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    for line in output {
        write!(stdout, "{}", line)?;
    }

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

fn print_body(output: &mut Vec<String>, m: Option<Mime>, body: String) {
    match m {
        Some(v) if v.essence_str() == mime::APPLICATION_JSON => {
            let json: Value = serde_json::from_str(&body).unwrap();
            let body = serde_json::to_string_pretty(&json).unwrap();
            print_syntect(output, body, "json");
        }
        Some(v) if v == mime::TEXT_HTML => print_syntect(output, body, "html"),

        _ => output.push(format!("{}\n", body)),
    }
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

fn print_syntect(output: &mut Vec<String>, s: String, ext: &str) {
    if atty::isnt(atty::Stream::Stdout) {
        output.push(s);
        return;
    }

    // Load these once at the start of your program
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension(ext).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for line in LinesWithEndings::from(&s) {
        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        output.push(escaped);
    }
}
