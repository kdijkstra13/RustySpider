use derive_more::with_trait::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Display, Serialize)]
#[display(
    "{prefix}{title} {first_prefix}{first:0digits$} {second_prefix}{second:0digits$}{postfix}"
)]
pub struct Content {
    pub(crate) prefix: String,
    pub(crate) title: String,
    pub(crate) first_prefix: String,
    pub(crate) first: u32,
    pub(crate) second_prefix: String,
    pub(crate) second: u32,
    pub(crate) digits: usize,
    pub(crate) postfix: String,
}

#[derive(Debug, Deserialize, Clone, Display, Serialize)]
#[display("{content} -> {link:.15}...")]
pub struct WebFile {
    pub(crate) content: Content,
    pub(crate) link: String,
}

#[derive(Debug, Deserialize, Clone, Display, Serialize)]
#[display("success={success} content=[{content}] response={response}")]
pub struct WebResponse {
    pub(crate) content: WebFile,
    pub(crate) response: String,
    pub(crate) success: bool,
}