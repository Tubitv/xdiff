use crate::req::RequestContext;
use anyhow::Result;
use console::{style, Style};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use std::{collections::HashMap, fmt, io::Write, path::Path};
use tokio::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiffConfig {
    #[serde(flatten)]
    ctxs: HashMap<String, DiffContext>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiffContext {
    pub request1: RequestContext,
    pub request2: RequestContext,
    pub response: ResponseContext,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseContext {
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
    pub async fn try_load(path: impl AsRef<Path>) -> Result<DiffConfig> {
        let file = fs::read_to_string(path).await?;
        let config: DiffConfig = serde_yaml::from_str(&file)?;
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
    pub async fn diff(&self) -> Result<DiffResult> {
        let res1 = self.request1.send().await?;
        let res2 = self.request2.send().await?;

        self.diff_response(res1, res2).await
    }

    async fn diff_response(&self, res1: Response, res2: Response) -> Result<DiffResult> {
        // let url1 = res1.url().to_string();
        // let url2 = res2.url().to_string();
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

        let mut text1 = res1.text().await?;
        let mut text2 = res2.text().await?;

        if let Ok(json) = serde_json::from_str::<Value>(&text1) {
            text1 = serde_json::to_string_pretty(&json)?;
            let json2: Value = serde_json::from_str(&text2)?;
            text2 = serde_json::to_string_pretty(&json2)?;
        }

        if text1 != text2 {
            return Ok(DiffResult::Diff(build_diff(text1, text2)?));
        }

        Ok(DiffResult::Equal)
    }
}

fn build_diff(old: String, new: String) -> Result<String> {
    let diff = TextDiff::from_lines(&old, &new);
    let mut buf = Vec::with_capacity(4096);
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
