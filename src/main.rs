use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use log::debug;
use recursive_scraper::schedule::{Scheduler, DEFAULT_TIMEOUT};
use regex::Regex;
use reqwest::Url;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let start_urls: Vec<_> = args
        .start_urls
        .split(',')
        .map(|str| Url::parse(str).unwrap())
        .collect();
    let timeout = match args.connection_timeout {
        Some(timeout) => Duration::from_millis(timeout),
        None => DEFAULT_TIMEOUT,
    };
    let client = Scheduler::client_with_timeout(timeout)?;
    let mut scheduler = Scheduler::from_client(client);
    if let Some(blacklist) = args.blacklist {
        let blacklist = Regex::new(&blacklist)?;
        scheduler = scheduler.blacklist(blacklist);
    }
    if let Some(filter) = args.filter {
        let filter = Regex::new(&filter)?;
        scheduler = scheduler.filter(filter);
    }
    if let Some(delay) = args.delay {
        let delay = Duration::from_millis(delay);
        scheduler = scheduler.delay(delay);
    }
    if let Some(html_dir) = args.html_dir {
        scheduler = scheduler.html_dir(html_dir);
    }
    if let Some(other_dir) = args.other_dir {
        scheduler = scheduler.other_dir(other_dir);
    }
    if let Some(log_dir) = args.log_dir {
        scheduler = scheduler.log_dir(log_dir)
    }
    if args.disregard_html {
        scheduler = scheduler.disregard_html();
    }
    if args.disregard_other {
        scheduler = scheduler.disregard_other();
    }

    for url in start_urls {
        scheduler.add_pending(url);
    }
    debug!("Starting with {scheduler:#?}.");
    scheduler.recursion().await;
    Ok(())
}

#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    about = "<https://github.com/SichangHe/scraper>

Scrapes given urls (separated by commas) recursively.\n\
Saves the results to `html/` and `other/`, the log to `log/`,\n\
or other directories if specified."
)]
struct Args {
    #[clap(help = "The URLs to start scraping from, separated by commas.")]
    start_urls: String,
    #[clap(short, long, help = "Regex to match URLs that should be excluded.")]
    blacklist: Option<String>,
    #[clap(
        short,
        long,
        help = "Connection timeout for each request in integer milliseconds."
    )]
    connection_timeout: Option<u64>,
    #[clap(
        short,
        long,
        help = "Delay between each request in integer milliseconds"
    )]
    delay: Option<u64>,
    #[clap(short, long, help = "Regex to match URLs that should be included.")]
    filter: Option<String>,
    #[clap(short = 'i', long, action, help = "Do not save HTMLs.")]
    disregard_html: bool,
    #[clap(short, long, help = "Directory to output the log.")]
    log_dir: Option<String>,
    #[clap(short, long, help = "Directory to save non-HTMLs.")]
    other_dir: Option<String>,
    #[clap(short = 's', long, action, help = "Do not save non-HTMLs.")]
    disregard_other: bool,
    #[clap(short = 't', long, help = "Directory to save HTMLs.")]
    html_dir: Option<String>,
}
