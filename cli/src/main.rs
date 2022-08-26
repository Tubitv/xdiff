use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use differ::{DiffConfig, DiffResult};

/// Diff API response.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// API profile to use.
    #[clap(short, long, value_parser)]
    api: String,

    /// Extra parameters to pass to the API.
    #[clap(short, value_parser = parse_key_val, number_of_values = 1)]
    extra_params: Vec<(String, String)>,

    /// Path to the config file.
    #[clap(short, long, value_parser = get_config_file)]
    config: Option<PathBuf>,
}

/// Parse a single key-value pair
fn parse_key_val(s: &str) -> Result<(String, String)> {
    let mut parts = s.splitn(2, '=');
    let key = parts.next().ok_or(anyhow::anyhow!("missing key"))?;
    let val = parts.next().ok_or(anyhow::anyhow!("missing value"))?;
    Ok((key.to_string(), val.to_string()))
}

fn get_config_file(s: &str) -> Result<PathBuf> {
    let path = Path::new(s);
    if path.exists() {
        Ok(path.to_path_buf())
    } else {
        Err(anyhow::anyhow!("config file not found"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config_file = args
        .config
        .unwrap_or(Path::new("~/.config/differ.yaml").to_path_buf());

    let diff_config = DiffConfig::try_load(&config_file).await?;

    let mut config = diff_config
        .apis
        .get(&args.api)
        .ok_or(anyhow::anyhow!("api profile {} not found", args.api))?
        .clone();

    for (key, val) in args.extra_params {
        config.request.params[key] = serde_json::Value::String(val);
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
