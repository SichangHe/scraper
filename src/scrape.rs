use anyhow::Result;
use bytes::Bytes;
use reqwest::{header::HeaderMap, RequestBuilder, Url};
use tokio::{spawn, task::JoinHandle};

pub async fn scrape_with_new_thread(
    url: Url,
    request: RequestBuilder,
) -> JoinHandle<Result<Conclusion>> {
    spawn(process_request(url, request))
}

async fn process_request(url: Url, request: RequestBuilder) -> Result<Conclusion> {
    let response = request.send().await?;
    let final_url = response.url().to_owned();
    let url_str = clean_url(&final_url);
    let headers = response.headers();
    let file_type = process_headers(headers)?;
    let extension;
    let content;
    if let FileType::Html = file_type {
        extension = ".html".to_owned();
        content = FileContent::Html(response.text().await?);
    } else {
        extension = ".".to_owned()
            + url_str
                .split('.')
                .last()
                .unwrap()
                .split('/')
                .last()
                .unwrap();
        content = FileContent::Other(response.bytes().await?);
    }
    Ok(Conclusion {
        url,
        final_url,
        extension,
        content,
    })
}

fn clean_url(url: &Url) -> String {
    url.to_string().split('#').next().unwrap().to_owned()
}

enum FileType {
    Html,
    Other,
}

fn process_headers(headers: &HeaderMap) -> Result<FileType> {
    let content = match headers.get("content-type") {
        Some(value) => value.to_str()?,
        None => return Ok(FileType::Other),
    };
    Ok(if content.contains(&"text/html") {
        FileType::Html
    } else {
        FileType::Other
    })
}

#[derive(Debug)]
pub enum FileContent {
    Html(String),
    Other(Bytes),
}

#[derive(Debug)]
pub struct Conclusion {
    pub url: Url,
    pub final_url: Url,
    pub extension: String,
    pub content: FileContent,
}
