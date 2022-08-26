use std::{collections::HashMap, path::Path};

use anyhow::Result;
use http::{HeaderMap, Method};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;
use url::Url;

const USER_AGENT: &str = "Requester/0.1.0";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestConfig {
    #[serde(flatten)]
    ctxs: HashMap<String, RequestContext>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestContext {
    #[serde(with = "http_serde::method")]
    pub method: Method,
    pub url: Url,
    pub params: Value,
    #[serde(skip_serializing_if = "HeaderMap::is_empty", default)]
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub body: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub user_agent: Option<String>,
}

impl RequestConfig {
    pub async fn try_load(path: impl AsRef<Path>) -> Result<Self> {
        let file = fs::read_to_string(path).await?;
        let config: Self = serde_yaml::from_str(&file)?;
        Ok(config)
    }

    pub fn get(&self, profile: &str) -> Result<&RequestContext> {
        self.ctxs
            .get(profile)
            .ok_or_else(|| anyhow::anyhow!("profile {} not found", profile))
    }

    pub async fn send(&self, profile: &str) -> Result<Response> {
        let ctx = self
            .ctxs
            .get(profile)
            .ok_or_else(|| anyhow::anyhow!("profile {} not found", profile))?;

        ctx.send().await
    }
}

impl RequestContext {
    pub async fn send(&self) -> Result<Response> {
        let mut url = self.url.clone();
        let user_agent = self
            .user_agent
            .clone()
            .unwrap_or_else(|| USER_AGENT.to_string());
        match url.scheme() {
            "http" | "https" => {
                let qs = serde_qs::to_string(&self.params)?;
                url.set_query(Some(&qs));
                let client = Client::builder().user_agent(user_agent).build()?;

                let res = client
                    .request(self.method.clone(), url)
                    .headers(self.headers.clone())
                    .send()
                    .await?;

                Ok(res)
            }
            _ => Err(anyhow::anyhow!("unsupported scheme")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn send_request_should_work() {
        let config = RequestConfig::try_load("fixtures/req.yml").await.unwrap();
        let result = config.send("rust").await.unwrap();
        assert_eq!(result.status(), 200);
    }
}
