use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use log::debug;
use recursive_scraper::{
    config::SchedulerConfig,
    schedule::{client_with_timeout, Scheduler, DEFAULT_TIMEOUT},
};
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
    let client = client_with_timeout(timeout);
    let mut cfg = SchedulerConfig::default();
    if let Some(blacklist) = args.blacklist {
        let blacklist = Regex::new(&blacklist)?;
        cfg = cfg.blacklist(blacklist);
    }
    if let Some(filter) = args.filter {
        let filter = Regex::new(&filter)?;
        cfg = cfg.filter(filter);
    }
    if let Some(delay) = args.delay {
        let delay = Duration::from_millis(delay);
        cfg = cfg.delay(delay);
    }
    if let Some(html_dir) = args.html_dir {
        cfg = cfg.html_dir(html_dir);
    }
    if let Some(other_dir) = args.other_dir {
        cfg = cfg.other_dir(other_dir);
    }
    if let Some(log_dir) = args.log_dir {
        cfg = cfg.log_dir(log_dir)
    }
    if args.disregard_html {
        cfg = cfg.disregard_html();
    }
    if args.disregard_other {
        cfg = cfg.disregard_other();
    }
    if let Some(number_of_rings) = args.number_of_rings {
        cfg = cfg.with_number_of_rings(number_of_rings);
    }
    let mut scheduler = Scheduler::from_client(client, cfg);

    for url in start_urls {
        scheduler.add_pending(url);
    }
    debug!("Starting with {scheduler:#?}.");
    scheduler.recursion().await;
    Ok(())
}

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Scrapes given urls (separated by commas) recursively.\n\
Saves the results to `html/` and `other/`, the log to `log/`,\n\
or other directories if specified.
See <https://github.com/SichangHe/scraper> for more instructions."
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
    #[clap(
        short = 'r',
        long,
        help = "Set the number of rings for the URLs outside the filter."
    )]
    number_of_rings: Option<u8>,
    #[clap(short = 's', long, action, help = "Do not save non-HTMLs.")]
    disregard_other: bool,
    #[clap(short = 't', long, help = "Directory to save HTMLs.")]
    html_dir: Option<String>,
}
