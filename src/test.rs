use std::time::Duration;

use anyhow::Result;
use reqwest::{Client, Url};
use xxhash_rust::xxh3::xxh3_64;

use crate::{schedule::Scheduler, scrape::Attempt};

#[test]
fn hash_test() {
    let url_str = "www.google.com";
    let hash = xxh3_64(url_str.as_bytes());
    println!("{hash}.");
    println!("{hash:X}.");
}

#[tokio::test]
async fn reqwest_test() -> Result<()> {
    let timeout = Duration::from_secs(5);
    let builder = Client::builder()
        .connect_timeout(timeout)
        .timeout(timeout * 5);
    let client = builder.build()?;
    // up to here all are reusable
    let url = Url::parse("https://www.rust-lang.org")?;
    let request = client.get(url);
    println!("Request: {request:?}");
    // actual place where work is done
    let response = request.send().await?;
    println!("Response: {response:?}");
    let status = response.status();
    println!("Status: {status}");
    let headers = response.headers();
    println!("Headers: {headers:?}");
    let final_url = response.url();
    println!("Final URL: {final_url}");
    Ok(())
}

#[tokio::test]
async fn attempt_send_test() -> Result<()> {
    let request = Client::new().get("https://www.rust-lang.org");
    let mut attempt = Attempt::with_request(request);
    attempt.run().await?;
    attempt.finish().await?;
    println!("{attempt:#?}");
    Ok(())
}

#[tokio::test]
async fn scheduler_test() -> Result<()> {
    let mut scheduler = Scheduler::from_client(Scheduler::default_client()?);
    scheduler.add_url(Url::parse("https://www.rust-lang.org")?);
    scheduler.launch_scraper().await?;
    scheduler.finish().await?;
    println!("{scheduler:#?}");
    Ok(())
}

#[tokio::test]
async fn process_header_test() -> Result<()> {
    let response = Scheduler::default_client()?
        .get("https://www.rust-lang.org")
        .send()
        .await?;
    let headers = response.headers();
    let content_type = headers
        .get("content-type")
        .unwrap()
        .to_str()?
        .split("; ")
        .collect::<Vec<_>>();
    println!("{headers:#?}\n{content_type:#?}");
    Ok(())
}
