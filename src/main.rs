use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use regex::Regex;
use reqwest::Url;
use scraper::schedule::Scheduler;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let start_urls: Vec<_> = args
        .start_urls
        .split(',')
        .map(|str| Url::parse(str).unwrap())
        .collect();
    let mut scheduler = Scheduler::new()?;
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
    scheduler.recursion().await;
    Ok(())
}

#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    about = "Scrapes given urls (separated by commas) recursively. \
    Saves the results to `html/` and `other/`, the log to `log/`, \
    or other directories if specified."
)]
struct Args {
    start_urls: String,
    #[clap(short, long)]
    blacklist: Option<String>,
    #[clap(short, long)]
    delay: Option<u64>,
    #[clap(short, long)]
    filter: Option<String>,
    #[clap(short = 'i', long, action)]
    disregard_html: bool,
    #[clap(short, long)]
    log_dir: Option<String>,
    #[clap(short, long)]
    other_dir: Option<String>,
    #[clap(short = 's', long, action)]
    disregard_other: bool,
    #[clap(short = 't', long)]
    html_dir: Option<String>,
}
