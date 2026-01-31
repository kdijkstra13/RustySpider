use std::error::Error;
use std::time::Duration;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::Deserialize;
use crate::modules::types::{WebFile, WebResponse};

#[derive(Debug, Deserialize)]
pub struct FetchersConfigs {
    pub fetchers: Vec<FetchersConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FetchersConfig {
    QBFetcher(QBFetcher),
}

#[derive(Debug, Deserialize)]
pub struct QBFetcher {
    url: String,
    add_url: String,
    login_url: String,
    username: String,
    password: String,
    save_path: String
}

pub trait Fetcher {
    fn fetch(&self, content: WebFile) -> Result<WebResponse, Box<dyn Error>>;
}

impl Fetcher for QBFetcher {
    fn fetch(&self, content: WebFile) -> Result<WebResponse, Box<dyn Error>> {
        let mut result = WebResponse {
            content: content.clone(),
            response: "".to_string(),
            success: false,
        };

        result.response = add_url_blocking(&self.url,
                                           &self.add_url,
                                           &self.login_url,
                                           &self.username,
                                           &self.password,
                                           &content.link,
                                           &format!("{0}{1}", self.save_path, content.content.title))?;
        result.success = result.response == "Ok.";
        Ok(result)
    }
}


pub fn add_url_blocking(
    url: &str,
    add_url: &str,
    login_url: &str,
    username: &str,
    password: &str,
    link: &str,
    save_path: &str,
) -> Result<String, Box<dyn Error>> {
    let url = url.trim_end_matches('/');

    let client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .build()?;

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("rusty-spider/1.0"));
    headers.insert(
        REFERER,
        HeaderValue::from_str(url)?,
    );
    if username != "" {
        let login_url = format!("{url}{login_url}");
        let login_resp = client
            .post(login_url)
            .headers(headers.clone())
            .form(&[("username", username), ("password", password)])
            .send()?
            .error_for_status()?
            .text()?;

        // qBittorrent typically returns "Ok." on success, "Fails." on failure.
        if !login_resp.to_lowercase().contains("ok") {
            return Err("login failed".into());
        }
    }

    let add_url = format!("{url}{add_url}");
    let add_resp = client
        .post(add_url)
        .headers(headers)
        .form(&[
            ("urls", link),
            ("savepath", save_path),
        ])
        .send()?
        .error_for_status()?
        .text()?;

    Ok(add_resp)
}
