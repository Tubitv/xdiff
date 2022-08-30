use crate::req::RequestContext;
use anyhow::Result;
use console::{style, Style};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::{collections::HashMap, fmt, io::Write, path::Path};
use tokio::fs;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DiffConfig {
    #[serde(flatten)]
    ctxs: HashMap<String, DiffContext>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DiffContext {
    pub request1: RequestContext,
    pub request2: RequestContext,
    #[serde(skip_serializing_if = "is_default_response", default)]
    pub response: ResponseContext,
}

fn is_default_response(r: &ResponseContext) -> bool {
    r == &ResponseContext::default()
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct ResponseContext {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skip_headers: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DiffResult {
    Equal,
    Diff(String),
}

impl ResponseContext {
    pub fn new(skip_headers: Vec<String>) -> Self {
        Self { skip_headers }
    }
}

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

impl DiffConfig {
    pub fn new_with_profile(
        profile: String,
        req1: RequestContext,
        req2: RequestContext,
        res: ResponseContext,
    ) -> Self {
        let ctx = DiffContext::new(req1, req2, res);
        let mut ctxs = HashMap::new();
        ctxs.insert(profile, ctx);
        Self { ctxs }
    }

    pub async fn try_load(path: impl AsRef<Path>) -> Result<DiffConfig> {
        let file = fs::read_to_string(path).await?;
        let config: DiffConfig = serde_yaml::from_str(&file)?;
        for (profile, ctx) in config.ctxs.iter() {
            if !ctx.request1.params.is_object() || !ctx.request2.params.is_object() {
                return Err(anyhow::anyhow!(
                    "params in request1 or request2 must be an object in profile: {}",
                    profile
                ));
            }
        }
        Ok(config)
    }

    pub fn get(&self, profile: &str) -> Result<&DiffContext> {
        self.ctxs
            .get(profile)
            .ok_or_else(|| anyhow::anyhow!("profile {} not found", profile))
    }

    pub async fn diff(&self, profile: &str) -> Result<DiffResult> {
        let ctx = self
            .ctxs
            .get(profile)
            .ok_or_else(|| anyhow::anyhow!("profile {} not found", profile))?;

        ctx.diff().await
    }
}

impl DiffContext {
    pub fn new(req1: RequestContext, req2: RequestContext, resp: ResponseContext) -> Self {
        Self {
            request1: req1,
            request2: req2,
            response: resp,
        }
    }

    pub async fn diff(&self) -> Result<DiffResult> {
        let res1 = self.request1.send().await?;
        let res2 = self.request2.send().await?;

        self.diff_response(res1, res2).await
    }

    async fn diff_response(&self, res1: Response, res2: Response) -> Result<DiffResult> {
        let url1 = res1.url().to_string();
        let url2 = res2.url().to_string();

        let text1 = self.request_to_string(res1).await?;
        let text2 = self.request_to_string(res2).await?;

        if text1 != text2 {
            let headers = format!("--- a/{}\n+++ b/{}\n", url1, url2);
            return Ok(DiffResult::Diff(build_diff(headers, text1, text2)?));
        }

        Ok(DiffResult::Equal)
    }

    async fn request_to_string(&self, res: Response) -> Result<String> {
        let mut buf = Vec::new();

        writeln!(&mut buf, "{}", res.status()).unwrap();
        res.headers().iter().for_each(|(k, v)| {
            if self.response.skip_headers.iter().any(|v| v == k.as_str()) {
                return;
            }
            writeln!(&mut buf, "{}: {:?}", k, v).unwrap();
        });
        writeln!(&mut buf).unwrap();

        let mut body = res.text().await?;

        if let Ok(json) = serde_json::from_str::<Value>(&body) {
            body = serde_json::to_string_pretty(&json)?;
        }

        writeln!(&mut buf, "{}", body).unwrap();

        Ok(String::from_utf8(buf)?)
    }
}

fn build_diff(headers: String, old: String, new: String) -> Result<String> {
    let diff = TextDiff::from_lines(&old, &new);
    let mut buf = Vec::with_capacity(4096);
    writeln!(&mut buf, "{}", headers).unwrap();
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            writeln!(&mut buf, "{:-^1$}", "-", 80)?;
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                write!(
                    &mut buf,
                    "{}{} |{}",
                    style(Line(change.old_index())).dim(),
                    style(Line(change.new_index())).dim(),
                    s.apply_to(sign).bold(),
                )?;
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        write!(&mut buf, "{}", s.apply_to(value).underlined().on_black())?;
                    } else {
                        write!(&mut buf, "{}", s.apply_to(value))?;
                    }
                }
                if change.missing_newline() {
                    writeln!(&mut buf)?;
                }
            }
        }
    }
    Ok(String::from_utf8(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn diff_request_should_work() {
        let config = DiffConfig::try_load("fixtures/diff.yml").await.unwrap();
        let result = config.diff("rust").await.unwrap();
        assert_eq!(result, DiffResult::Equal);
    }
}
