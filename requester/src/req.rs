use std::{collections::HashMap, path::Path, str::FromStr};

use anyhow::Result;
use http::{header::HeaderName, HeaderMap, HeaderValue, Method};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;
use url::Url;

use crate::{KeyVal, KeyValType};

const USER_AGENT: &str = "Requester/0.1.0";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RequestConfig {
    #[serde(flatten)]
    ctxs: HashMap<String, RequestContext>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    #[serde(
        with = "http_serde::method",
        skip_serializing_if = "is_default",
        default
    )]
    pub method: Method,
    pub url: Url,
    #[serde(skip_serializing_if = "is_empty_value", default = "default_params")]
    pub params: Value,
    #[serde(skip_serializing_if = "HeaderMap::is_empty", default)]
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub body: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub user_agent: Option<String>,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

fn is_empty_value(v: &Value) -> bool {
    v.is_null() || (v.is_object() && v.as_object().unwrap().is_empty())
}

fn default_params() -> Value {
    serde_json::json!({})
}

impl RequestConfig {
    pub async fn try_load(path: impl AsRef<Path>) -> Result<Self> {
        let file = fs::read_to_string(path).await?;
        let config: Self = serde_yaml::from_str(&file)?;
        for (profile, ctx) in config.ctxs.iter() {
            if !ctx.params.is_object() {
                return Err(anyhow::anyhow!(
                    "params must be an object in profile: {}",
                    profile
                ));
            }
        }
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
    pub fn update(&mut self, values: &[KeyVal]) -> Result<()> {
        for v in values {
            match v.kv_type {
                KeyValType::Query => {
                    self.params[&v.key] = serde_json::Value::String(v.val.to_owned());
                }
                KeyValType::Header => {
                    self.headers.insert(
                        HeaderName::from_str(&v.key)?,
                        HeaderValue::from_str(&v.val)?,
                    );
                }
                KeyValType::Body => {
                    if let Some(body) = self.body.as_mut() {
                        body[&v.key] = serde_json::Value::String(v.val.to_owned())
                    }
                }
            }
        }

        Ok(())
    }

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

                let mut builder = client
                    .request(self.method.clone(), url)
                    .headers(self.headers.clone());

                if let Some(body) = &self.body {
                    match self.headers.get(http::header::CONTENT_TYPE) {
                        Some(content_type) => {
                            if content_type.to_str().unwrap().contains("application/json") {
                                builder = builder.json(body);
                            } else {
                                return Err(anyhow::anyhow!(
                                    "unsupported content-type: {:?}",
                                    content_type
                                ));
                            }
                        }
                        None => {
                            // TODO (tchen): here we just assume the content-type is json
                            builder = builder.json(body)
                        }
                    }
                    builder = builder.body(serde_json::to_string(body)?);
                }

                let res = builder.send().await?;

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
