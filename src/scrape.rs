use anyhow::Result;
use bytes::Bytes;
use reqwest::{header::HeaderMap, RequestBuilder, Url};
use select::{document::Document, predicate::Name};
use tokio::{spawn, task::JoinHandle};

pub async fn scrape_with_new_thread(request: RequestBuilder) -> JoinHandle<Result<Conclusion>> {
    spawn(process_request(request))
}

async fn process_request(request: RequestBuilder) -> Result<Conclusion> {
    let response = request.send().await?;
    let final_url = response.url().to_owned();
    let url_str = clean_url(&final_url);
    let headers = response.headers();
    let file_type = process_headers(headers)?;
    let extension;
    let content;
    if let FileType::Html = file_type {
        extension = ".html".to_owned();
        let (text, hrefs, imgs) = links_from_html(&final_url, response.text().await?);
        content = FileContent::Html(text, hrefs, imgs);
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
    Html(String, Vec<Url>, Vec<Url>),
    Other(Bytes),
}

fn links_from_html(url: &Url, str: String) -> (String, Vec<Url>, Vec<Url>) {
    let mut hrefs = Vec::new();
    let document = Document::from(str.as_str());
    for href in document.find(Name("a")).filter_map(|n| n.attr("href")) {
        let href_url = url.join(href.split('#').next().unwrap());
        match href_url {
            Ok(href_url) => hrefs.push(href_url),
            Err(err) => {
                println!("{err}");
            }
        }
    }
    hrefs.sort();
    hrefs.dedup();
    let mut imgs = Vec::new();
    for img in document.find(Name("img")).filter_map(|n| n.attr("src")) {
        let img_url = url.join(img);
        match img_url {
            Ok(img_url0) => imgs.push(img_url0),
            Err(err) => {
                println!("{err}");
            }
        }
    }
    imgs.sort();
    imgs.dedup();
    (str, hrefs, imgs)
}

#[derive(Debug)]
pub struct Conclusion {
    pub final_url: Url,
    pub extension: String,
    pub content: FileContent,
}
