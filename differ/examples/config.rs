use std::collections::HashMap;

use anyhow::Result;
use differ::{ApiConfig, ApiRequestConfig, ApiResponseConfig, DiffConfig};
use http::{HeaderMap, Method};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let mut apis = HashMap::new();
    let url = Url::parse("https://www.rust-lang.org/")?;
    let api_config = ApiConfig {
        request: ApiRequestConfig {
            method: Method::GET,
            url1: url.clone(),
            url2: url,
            params: serde_json::json!({}),
            headers: HeaderMap::new(),
        },
        response: ApiResponseConfig {
            skip_headers: vec!["Set-Cookie".to_string()],
            jq: None,
        },
    };
    apis.insert("google".into(), api_config);
    let config = DiffConfig { apis };
    println!("{}", serde_yaml::to_string(&config)?);
    Ok(())
}
