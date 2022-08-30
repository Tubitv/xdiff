use anyhow::Result;
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect};
use std::{io::Write, path::PathBuf};
use xreq_cli_utils::{get_config_file, get_default_config, parse_key_val, print_syntect};
use xreq_lib::{DiffConfig, DiffResult, KeyVal, RequestContext, ResponseContext};

/// Diff API response.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand, Debug, Clone)]
enum Action {
    /// parse a URL and print the generated request config.
    Parse,
    Run(RunArgs),
}

#[derive(Parser, Debug, Clone)]
struct RunArgs {
    /// API profile to use.
    #[clap(short, long, value_parser)]
    profile: String,

    /// Extra parameters to pass to the API.
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
        Action::Parse => parse(&mut output).await?,
        Action::Run(args) => run(&mut output, args).await?,
    }

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    for line in output {
        write!(stdout, "{}", line)?;
    }

    Ok(())
}

async fn parse(output: &mut Vec<String>) -> Result<()> {
    let url1: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Url1")
        .interact()?;
    let url2: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Url2")
        .interact()?;
    let profile = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Give this a profile name")
        .default("default".into())
        .interact()?;

    let ctx1: RequestContext = url1.parse()?;
    let ctx2: RequestContext = url2.parse()?;

    let response = ctx1.send().await?;
    let headers = response
        .headers()
        .iter()
        .map(|(k, _)| k.as_str().to_string())
        .collect::<Vec<_>>();

    let chosen: Vec<usize> = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select response headers to skip")
        .items(&headers)
        .interact()?;

    let skip_headers = chosen
        .into_iter()
        .map(|i| headers[i].clone())
        .collect::<Vec<_>>();

    let res = ResponseContext::new(skip_headers);
    let config = DiffConfig::new_with_profile(profile, ctx1, ctx2, res);

    let result = serde_yaml::to_string(&config)?;

    output.push("---\n".to_string());
    print_syntect(output, result, "yaml");
    Ok(())
}

async fn run(output: &mut Vec<String>, args: RunArgs) -> Result<()> {
    let config_file = args.config.unwrap_or(get_default_config("xdiff.yml")?);
    let diff_config = DiffConfig::try_load(&config_file).await?;

    let mut config = diff_config.get(&args.profile)?.clone();

    config.request1.update(&args.extra_params)?;
    config.request2.update(&args.extra_params)?;

    let result = config.diff().await?;

    match result {
        DiffResult::Equal => {
            output.push("API responses are equal".into());
        }
        DiffResult::Diff(diff) => {
            output.push(diff);
        }
    }

    Ok(())
}
