use std::path::{Path, PathBuf};

use anyhow::Result;
use xreq_lib::{KeyVal, KeyValType};

/// Parse a single key-value pair
/// - if key has no any prefix, it is for query
/// - if key starts with '%', it is for header
/// - if key starts with '@', it is for body
pub fn parse_key_val(s: &str) -> Result<KeyVal> {
    let (kv_type, input) = match s.chars().next() {
        Some(c) => match c {
            '%' => (KeyValType::Header, &s[1..]),
            '@' => (KeyValType::Body, &s[1..]),
            'A'..='Z' | 'a'..='z' => (KeyValType::Query, s),
            _ => return Err(anyhow::anyhow!("invalid key val pair: {}", s)),
        },
        None => return Err(anyhow::anyhow!("empty key-value pair is invalid")),
    };

    let mut parts = input.splitn(2, '=');
    let key = parts.next().ok_or_else(|| anyhow::anyhow!("missing key"))?;
    let val = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing value"))?;
    Ok(KeyVal::new(kv_type, key, val))
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
