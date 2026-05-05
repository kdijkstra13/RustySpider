use std::error::Error;
use std::io;
use std::thread::sleep;
use std::time::Duration;
use log::info;
use reqwest::header::{HeaderMap, USER_AGENT};
use crate::modules::content::Searchable;
use serde::{Deserialize, Serialize};
use url::Url;
use scraper::{Html, Selector};
use crate::modules::types::{Content, WebFile};
use regex::Regex;

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
    second_stage_match: String,
    wait: u64,
}

pub trait Crawler {
    fn find(&self, content: Content) -> Result<WebFile, Box<dyn Error>>;
}

fn filter_by_keywords(
    items: &[(String, String)],
    keywords: &str,
    keywords_neg: &str,
    regexp: &str,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let words: Vec<String> = keywords
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();

    let neg_words: Vec<String> = keywords_neg
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();

    let regex = if regexp.trim().is_empty() {
        None
    } else {
        Some(Regex::new(regexp)?)
    };

    let filtered = items
        .iter()
        .filter(|(link, title)| {
            // Prefer matching against the visible title; fall back to the URL if
            // the selector doesn't include text.
            let haystack = if title.trim().is_empty() { link } else { title };
            let haystack_lower = haystack.to_lowercase();

            let has_all_keywords = words.iter().all(|w| haystack_lower.contains(w));
            let has_no_neg_keywords = neg_words.iter().all(|w| !haystack_lower.contains(w));
            let matches_regex = regex
                .as_ref()
                .map_or(true, |re| re.is_match(haystack));

            has_all_keywords && has_no_neg_keywords && matches_regex
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
        if !&self.categories_get_name.is_empty() {
            for category in &self.categories {
                url.query_pairs_mut().append_pair(&self.categories_get_name, &category);
            }
        }

        // Create header
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, self.user_agent.parse()?);

        sleep(Duration::from_secs(self.wait));
        info!("Crawler fetches first stage url: {}", &url);

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
        let mut links: Vec<(String, String)> = Vec::new();

        for a in parsed_html.select(&links_sel) {
            if let Some(href) = a.value().attr("href") {
                if let Ok(resolved) = url.join(href) {
                    let title = a.value().attr("title").unwrap_or("").trim().to_string();
                    links.push((resolved.to_string(), title));
                }
            }
        }
        // Double check with keywords (also filter with negative keywords)
        let negative = content.to_negative()?;
        let regexp = content.to_regexp()?;
        let before_links = links.clone();
        let links = filter_by_keywords(&links, &query, &negative, &regexp)?;
        info!(
            "Before filtering: {}, after filtering: {}, with -'{}' and +'{}'",
            &before_links.len(),
            &links.len(),
            &negative,
            &query
        );

        // Return no magnet link if there were no results
        if links.is_empty() {
            return Err("Nothing found in first stage.".into());
        };
        let (url_string, _title) = links[0].clone();
        sleep(Duration::from_secs(self.wait));
        info!("Crawler fetches second stage url: {}", &url_string);

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
            return Err("Search string not found (or filtered)".into())
        }

        info!("Crawler found link: {:.35}...", &link);
        Ok(WebFile {content: content.clone(), link: link})
    }
}
