use std::error::Error;
use std::io;
use reqwest::header::{HeaderMap, USER_AGENT};
use crate::modules::content::Searchable;
use serde::{Deserialize, Serialize};
use url::Url;
use scraper::{Html, Selector};
use crate::modules::types::{Content, WebFile};

#[derive(Debug, Deserialize, Serialize)]
pub struct CrawlersConfigs {
    pub crawlers: Vec<CrawlersConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CrawlersConfig {
    TwoStageWeb(TwoStageWeb),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TwoStageWeb {
    url: String,
    search_page: String,
    search_get_name: String,
    categories: Vec<String>,
    categories_get_name: String,
    user_agent: String,
    limit: u32,
    first_stage_match: String,
    second_stage_match: String
}

pub trait Crawler {
    fn find(&self, content: Content) -> Result<WebFile, Box<dyn Error>>;
}

fn filter_by_keywords(items: &[String], keywords: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let words: Vec<String> = keywords
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();

    let filtered = items
        .iter()
        .filter(|s| {
            words.iter().all(|w| s.contains(w))
        })
        .cloned()
        .collect();
    Ok(filtered)
}

impl Crawler for TwoStageWeb {
    fn find(&self, content: Content) -> Result<WebFile, Box<dyn Error>> {
        // Create URL with parameters
        let mut url = Url::parse(&self.url)?.join(&self.search_page)?;
        let query = content.to_query()?;
        url.query_pairs_mut().append_pair(&self.search_get_name, &query);
        for category in &self.categories {
            url.query_pairs_mut().append_pair(&self.categories_get_name, &category);
        }

        // Create header
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, self.user_agent.parse()?);

        // Get result
        let html = reqwest::blocking::Client::new()
            .get(url.as_str())
            .headers(headers)
            .send()?
            .error_for_status()?
            .text()?;

        // Parse links for search results
        let parsed_html = Html::parse_document(&html);
        let links_sel = Selector::parse(self.first_stage_match.as_str())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        let mut url_strings: Vec<String> = Vec::new();
        for a in parsed_html.select(&links_sel) {
            if let Some(href) = a.value().attr("href") {
                if let Ok(resolved) = url.join(href) {
                    url_strings.push(resolved.to_string());
                }
            }
        }

        // Double check with keywords
        let url_strings = filter_by_keywords(&url_strings, &query)?;

        // Return no magnet link if there were no results
        if url_strings.is_empty() {
            return Err("Nothing found in first stage.".into());
        };
        let url_string = url_strings[0].clone();

        // Create header
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, self.user_agent.parse()?);

        // Get the magnet link
        let html = reqwest::blocking::Client::new()
            .get(url_string)
            .headers(headers)
            .send()?
            .error_for_status()?
            .text()?;

        // Parse links for search results
        let parsed_html = Html::parse_document(&html);
        let links_sel = Selector::parse(self.second_stage_match.as_str())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        let mut link = String::new();

        for a in parsed_html.select(&links_sel) {
            if let Some(href) = a.value().attr("href") {
                if let Ok(resolved) = url.join(href) {
                    link = resolved.to_string();
                    break;
                }
            }
        }
        if link == "" {
            return Err("Search string not found".into())
        }
        Ok(WebFile {content: content.clone(), link: link})
    }
}
