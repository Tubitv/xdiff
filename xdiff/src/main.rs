use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use xreq_cli_utils::{get_config_file, get_default_config, parse_key_val};
use xreq_lib::{DiffConfig, DiffResult, KeyVal};

/// Diff API response.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
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

    let config_file = args.config.unwrap_or(get_default_config("xdiff.yml")?);
    let diff_config = DiffConfig::try_load(&config_file).await?;

    let mut config = diff_config.get(&args.profile)?.clone();

    config.request1.update(&args.extra_params)?;
    config.request2.update(&args.extra_params)?;

    let result = config.diff().await?;

    match result {
        DiffResult::Equal => {
            println!("API responses are equal");
        }
        DiffResult::Diff(diff) => {
            println!("{}", diff);
        }
    }
    Ok(())
}
