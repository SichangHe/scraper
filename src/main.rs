use anyhow::Result;
use clap::Parser;
use regex::Regex;
use reqwest::Url;
use scraper::schedule::Scheduler;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("{args:?}");
    let start_url = Url::parse(&args.start_url)?;
    let mut scheduler = Scheduler::new()?;
    if let Some(blacklist) = args.blacklist {
        let blacklist = Regex::new(&blacklist)?;
        scheduler = scheduler.blacklist(blacklist);
    }
    if let Some(filter) = args.filter {
        let filter = Regex::new(&filter)?;
        scheduler = scheduler.filter(filter);
    }

    scheduler.add_pending(start_url);
    scheduler.recursion().await;
    Ok(())
}

#[derive(Debug, Parser)]
#[clap(author, version)]
struct Args {
    start_url: String,
    #[clap(short, long)]
    blacklist: Option<String>,
    #[clap(short, long)]
    filter: Option<String>,
}
