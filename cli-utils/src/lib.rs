use std::path::{Path, PathBuf};

use anyhow::Result;

/// Parse a single key-value pair
pub fn parse_key_val(s: &str) -> Result<(String, String)> {
    let mut parts = s.splitn(2, '=');
    let key = parts.next().ok_or_else(|| anyhow::anyhow!("missing key"))?;
    let val = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing value"))?;
    Ok((key.to_string(), val.to_string()))
}

pub fn get_config_file(s: &str) -> Result<PathBuf> {
    let path = Path::new(s);
    if path.exists() {
        Ok(path.to_path_buf())
    } else {
        Err(anyhow::anyhow!("config file not found"))
    }
}

pub fn get_default_config(name: &str) -> Result<PathBuf> {
    let paths = [
        format!("{}/.config/{}", std::env::var("HOME").unwrap(), name),
        format!("./{}", name),
        format!("/etc/{}", name),
    ];

    for path in paths.iter() {
        if Path::new(path).exists() {
            return Ok(Path::new(path).to_path_buf());
        }
    }

    Err(anyhow::anyhow!("Config file not found. You can either specify it with the --config option or put it in one of the following locations: {}", paths.join(", ")))
}
