mod diff;
mod req;

pub use diff::{DiffConfig, DiffContext, DiffResult};
pub use req::{RequestConfig, RequestContext};

// re-exports
pub use reqwest::Response;
