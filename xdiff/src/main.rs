use anyhow::Result;
use clap::Parser;
use cli_utils::{get_config_file, get_default_config, parse_key_val};
use requester::{DiffConfig, DiffResult};
use std::path::PathBuf;

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

    for (key, val) in args.extra_params {
        config.request1.params[&key] = serde_json::Value::String(val.clone());
        config.request2.params[&key] = serde_json::Value::String(val);
    }

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
