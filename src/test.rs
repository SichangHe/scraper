use std::time::Duration;

use anyhow::{Ok, Result};
use regex::Regex;
use reqwest::{Client, Url};
use tokio::time::sleep;

use crate::{io::save_file, middle::spawn_request, schedule::Scheduler, urls::Record};

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

#[test]
#[should_panic]
fn url_slash_test() {
    let url0 = Url::parse("https://sites.duke.edu/intersections/vocabulary-lessons/").unwrap();
    let url1 = Url::parse("https://sites.duke.edu/intersections/vocabulary-lessons").unwrap();
    assert_eq!(url0, url1);
}

#[tokio::test]
async fn request_test() -> Result<()> {
    let request = spawn_request(
        0,
        Scheduler::default_client()?.get("https://www.rust-lang.org"),
    )
    .await;
    dbg!(&request);
    while !request.is_finished() {
        println!("Request hasn't finished.");
        sleep(Duration::from_millis(250)).await;
    }
    let (_, response_result) = request.await?;
    let response = response_result?;
    dbg!(response);
    Ok(())
}

#[tokio::test]
async fn scheduler_test() -> Result<()> {
    let mut scheduler = Scheduler::new()?;
    scheduler.add_pending(Url::parse("https://www.rust-lang.org")?);
    scheduler.spawn_one_request().await;
    scheduler.finish().await;
    println!("{scheduler:#?}");
    Ok(())
}

#[tokio::test]
async fn scheduler_recursion_test() -> Result<()> {
    let mut scheduler =
        Scheduler::new()?.filter(Regex::new(r"https://sites.duke.edu/intersections/.*").unwrap());
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

#[test]
fn regex_test() -> Result<()> {
    assert!(!Regex::new("#")?.is_match("https://sites.duke.edu/intersections/.*"));
    Ok(())
}

#[test]
fn record_serialize_test() -> Result<()> {
    let mut record = Record::default();
    record
        .check_add_url(Url::parse("https://www.rust-lang.org")?)
        .unwrap();
    record
        .check_add_url(Url::parse("https://sites.duke.edu/intersections/")?)
        .unwrap();
    record.scrapes.insert(0);
    record.fails.insert(2);
    record.redirects.insert(3, 4);
    let toml = toml::to_string_pretty(&record)?;
    println!("{toml}");
    Ok(())
}
