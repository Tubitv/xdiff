use std::path::{Path, PathBuf};

use anyhow::Result;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
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

pub fn print_syntect(output: &mut Vec<String>, s: String, ext: &str) {
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
