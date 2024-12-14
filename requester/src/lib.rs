mod diff;
mod req;

pub use diff::{DiffConfig, DiffContext, DiffResult, ResponseContext};
pub use req::{RequestConfig, RequestContext};

// re-exports
pub use reqwest::Response;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyValType {
    /// if key has no any prefix, it is for query
    #[default]
    Query,
    /// if key starts with '#', it is for header
    Header,
    /// if key starts with '@', it is for body
    Body,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyVal {
    pub kv_type: KeyValType,
    pub key: String,
    pub val: String,
}

impl KeyVal {
    pub fn new(kv_type: KeyValType, key: impl Into<String>, val: impl Into<String>) -> Self {
        Self {
            kv_type,
            key: key.into(),
            val: val.into(),
        }
    }
}
