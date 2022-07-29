use anyhow::Result;
use bytes::Bytes;
use reqwest::{header::HeaderMap, Url};
use select::{document::Document, predicate::Name};

pub enum FileType {
    Html,
    Other,
}

pub fn process_headers(headers: &HeaderMap) -> Result<FileType> {
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

/// `Html(text: String, hrefs: Vec<Url>, imgs: Vec<Url>)`
///
/// or
///
/// `Other(extension: String, bytes: Bytes)`
#[derive(Debug)]
pub enum FileContent {
    Html(String, Vec<Url>, Vec<Url>),
    Other(String, Bytes),
}

pub fn links_from_html(url: &Url, str: String) -> (String, Vec<Url>, Vec<Url>) {
    let mut hrefs = Vec::new();
    let document = Document::from(str.as_str());
    for href in document.find(Name("a")).filter_map(|n| n.attr("href")) {
        let href_url = url.join(href.split('#').next().unwrap());
        match href_url {
            Ok(href_url) => hrefs.push(href_url),
            Err(err) => {
                println!("{err}.");
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
                println!("{err}.");
            }
        }
    }
    imgs.sort();
    imgs.dedup();
    (str, hrefs, imgs)
}
