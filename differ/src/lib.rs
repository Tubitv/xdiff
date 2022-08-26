use anyhow::Result;
use http::{HeaderMap, Method};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::TextDiff;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use tokio::fs;
use url::Url;

const USER_AGENT: &str = "API Diff/0.1.0";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiffConfig {
    pub apis: HashMap<String, ApiConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiConfig {
    pub request: ApiRequestConfig,
    pub response: ApiResponseConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiRequestConfig {
    #[serde(with = "http_serde::method")]
    pub method: Method,
    pub url1: Url,
    pub url2: Url,
    pub params: Value,
    #[serde(skip_serializing_if = "HeaderMap::is_empty", default)]
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponseConfig {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skip_headers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DiffResult {
    Equal,
    Diff(String),
}

impl DiffConfig {
    pub async fn try_load(path: impl AsRef<Path>) -> Result<DiffConfig> {
        let file = fs::read_to_string(path).await?;
        let config: DiffConfig = serde_yaml::from_str(&file)?;
        Ok(config)
    }

    pub async fn diff(&self, api_profile: &str) -> Result<DiffResult> {
        let config = self
            .apis
            .get(api_profile)
            .ok_or(anyhow::anyhow!("api profile {} not found", api_profile))?;

        config.diff().await
    }
}

impl ApiConfig {
    pub async fn diff(&self) -> Result<DiffResult> {
        let res1 = self.get_api(self.request.url1.clone()).await?;
        let res2 = self.get_api(self.request.url2.clone()).await?;

        self.diff_response(res1, res2).await
    }

    async fn get_api(&self, mut url: Url) -> Result<Response> {
        match url.scheme() {
            "http" | "https" => {
                let qs = serde_qs::to_string(&self.request.params)?;
                url.set_query(Some(&qs));
                let client = Client::builder().user_agent(USER_AGENT).build()?;

                let res = client
                    .request(self.request.method.clone(), url)
                    .headers(self.request.headers.clone())
                    .send()
                    .await?;

                Ok(res)
            }
            _ => Err(anyhow::anyhow!("unsupported scheme")),
        }
    }

    async fn diff_response(&self, res1: Response, res2: Response) -> Result<DiffResult> {
        if res1.status() != res2.status() {
            return Ok(DiffResult::Diff(format!(
                "status code mismatch: {} != {}",
                res1.status(),
                res2.status()
            )));
        }

        if res1.headers() != res2.headers() {
            let mut buf = Vec::with_capacity(4096);
            res1.headers().iter().for_each(|(k, v)| {
                if self.response.skip_headers.iter().any(|v| v == k.as_str()) {
                    return;
                }
                let v2 = res2.headers().get(k);
                match v2 {
                    None => write!(&mut buf, "header {} mismatch: '{:?}' / None", k, v).unwrap(),
                    Some(v2) if v != v2 => {
                        writeln!(&mut buf, "header {} mismatch: '{:?}' / '{:?}'", k, v, v2).unwrap()
                    }
                    _ => (),
                }
            });
            if !buf.is_empty() {
                return Ok(DiffResult::Diff(String::from_utf8(buf)?));
            }
        }

        let text1 = res1.text().await?;
        let text2 = res2.text().await?;
        if text1 != text2 {
            let mut buf = Vec::with_capacity(4096);
            let text_diff = TextDiff::from_lines(&text1, &text2);
            writeln!(
                &mut buf,
                "{}",
                text_diff.unified_diff().header("Response 1", "Response 2")
            )
            .unwrap();
            return Ok(DiffResult::Diff(String::from_utf8(buf)?));
        }

        Ok(DiffResult::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn diff_google_should_work() {
        let config = DiffConfig::try_load("../fixtures/test.yaml").await.unwrap();
        let result = config.diff("rust").await.unwrap();
        assert_eq!(result, DiffResult::Equal);
    }
}
