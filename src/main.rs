mod modules;
use crate::modules::content::{Searchable, Predictable};
use crate::modules::serialize::{load_contents, load_crawlers, load_fetchers, save_contents};
use std::error::Error;
use crate::modules::crawlers::Crawler;
use simplelog::*;
use std::fs::{File, OpenOptions};
use log::{info, warn, error};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "spider",
    version,
    about = "RustySpider content fetcher",
    long_about = include_str!("../help.txt")
)]
struct Cli {
    #[arg(short = 'l', long = "log-file", default_value = "RustySpider.log")]
    log_file: String,

    #[arg(short = 'c', long = "contents", default_value = "./contents.toml")]
    contents: String,

    #[arg(short = 'r', long = "crawlers", default_value = "./crawlers.toml")]
    crawlers: String,

    #[arg(short = 'f', long = "fetchers", default_value = "./fetchers.toml")]
    fetchers: String,
}

fn init_logger(log_path: &str) -> Result<(), Box<dyn Error>> {
    WriteLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new()
            .set_time_format_rfc3339()
            .build(),
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    init_logger(&cli.log_file)?;

    let crawlers = load_crawlers(&cli.crawlers)?;
    let mut contents = load_contents(&cli.contents)?;
    let fetchers = load_fetchers(&cli.fetchers)?;

    for i in 0..contents.len() {
        let predictions = contents[i].predict_new_content()?;

        for new_content in predictions {
            info!("Trying to find: {new_content}");

            let web_file = match crawlers[0].find(new_content.clone()) {
                Ok(f) => f,
                Err(e) => {
                    error!("Content not found: {e}");
                    continue;
                }
            };
            info!("Now downloading: {new_content}!");
            let web_response = match fetchers[0].fetch(web_file) {
                Ok(r) => r,
                Err(e) => {
                    error!("Cannot start download: {e}");
                    continue;
                }
            };

            info!("Done: {web_response}");

            contents[i] = new_content;
            save_contents(&cli.contents, &contents);
            break;
        }
    }

    Ok(())
}
