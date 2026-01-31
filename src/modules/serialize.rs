use serde::{Deserialize, Serialize};
use std::fs;
use crate::modules::crawlers::{Crawler, CrawlersConfig, CrawlersConfigs};
use crate::modules::fetchers::{Fetcher, FetchersConfig, FetchersConfigs};
use crate::modules::types::Content;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ContentFile {
    pub content: Vec<Content>,
}

pub fn load_contents(path: &str) -> Result<Vec<Content>, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let file: ContentFile = toml::from_str(&text)?;
    Ok(file.content)
}

pub fn save_contents(path: &str, contents: &Vec<Content>) -> Result<(), Box<dyn std::error::Error>> {
    let cf = ContentFile {content: contents.clone()};
    let toml_str = toml::to_string_pretty(&cf)?;
    fs::write(path, toml_str)?;
    Ok(())
}

pub fn load_crawlers(path: &str) -> Result<Vec<Box<dyn Crawler>>, Box<dyn std::error::Error>> {

    let text = fs::read_to_string(path)?;
    let cfg: CrawlersConfigs = toml::from_str(&text)?;

    let mut crawlers: Vec<Box<dyn Crawler>> = Vec::new();

    for crawler_cfg in cfg.crawlers {
        let crawler: Box<dyn Crawler> = match crawler_cfg {
            CrawlersConfig::TwoStageWeb(r) => Box::new(r),
            // Add other types
        };
        crawlers.push(crawler);
    }

    Ok(crawlers)
}

pub fn load_fetchers(path: &str) -> Result<Vec<Box<dyn Fetcher>>, Box<dyn std::error::Error>> {

    let text = fs::read_to_string(path)?;
    let cfg: FetchersConfigs = toml::from_str(&text)?;

    let mut fetchers: Vec<Box<dyn Fetcher>> = Vec::new();

    for fetcher_cfg in cfg.fetchers {
        let fetcher: Box<dyn Fetcher> = match fetcher_cfg {
            FetchersConfig::QBFetcher(r) => Box::new(r),
            // Add other types
        };
        fetchers.push(fetcher);
    }

    Ok(fetchers)
}

pub fn save_crawlers(path: &str, crawlers: &CrawlersConfigs) -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = toml::to_string_pretty(crawlers)?;
    fs::write(path, toml_str)?;
    Ok(())
}

pub fn save_fetchers(path: &str, fetchers: &FetchersConfigs) -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = toml::to_string_pretty(fetchers)?;
    fs::write(path, toml_str)?;
    Ok(())
}

pub fn load_contents_file(path: &str) -> Result<ContentFile, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let file: ContentFile = toml::from_str(&text)?;
    Ok(file)
}

pub fn load_crawlers_file(path: &str) -> Result<CrawlersConfigs, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let file: CrawlersConfigs = toml::from_str(&text)?;
    Ok(file)
}

pub fn load_fetchers_file(path: &str) -> Result<FetchersConfigs, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let file: FetchersConfigs = toml::from_str(&text)?;
    Ok(file)
}
