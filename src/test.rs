use std::time::Duration;

use anyhow::{Ok, Result};
use reqwest::{Client, Url};
use tokio::time::sleep;
use xxhash_rust::xxh3::xxh3_64;

use crate::{io::save_file, middle::Request, schedule::Scheduler};

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
async fn request_test() -> Result<()> {
    let request = Request::spawn(
        0,
        Scheduler::default_client()?.get("https://www.rust-lang.org"),
    )
    .await;
    println!("{request:#?}");
    while !request.handle.is_finished() {
        println!("Request hasn't finished.");
        sleep(Duration::from_millis(250)).await;
    }
    let response = request.handle.await??;
    println!("{response:#?}");
    Ok(())
}

#[tokio::test]
async fn scheduler_test() -> Result<()> {
    let mut scheduler = Scheduler::from_client(Scheduler::default_client()?);
    scheduler.add_pending(Url::parse("https://www.rust-lang.org")?);
    scheduler.spawn_one_request().await;
    scheduler.finish().await?;
    println!("{scheduler:#?}");
    Ok(())
}

#[tokio::test]
async fn scheduler_recursion_test() -> Result<()> {
    let mut scheduler = Scheduler::from_client(Scheduler::default_client()?);
    scheduler.add_pending(Url::parse("https://sites.duke.edu/intersections/")?);
    scheduler.recursion().await;
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

#[tokio::test]
async fn save_file_test() -> Result<()> {
    save_file("dne/0.txt", b"hey").await?;
    Ok(())
}
