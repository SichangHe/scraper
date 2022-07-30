use anyhow::Result;
use bytes::Bytes;
use reqwest::{header::HeaderMap, Url};
use select::{document::Document, predicate::Name};
use std::collections::BTreeSet;

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
    Html(String, BTreeSet<Url>, BTreeSet<Url>),
    Other(String, Bytes),
}

pub fn links_from_html(url: &Url, str: String) -> (String, BTreeSet<Url>, BTreeSet<Url>) {
    let mut hrefs = BTreeSet::new();
    let document = Document::from(str.as_str());
    let a_hrefs: BTreeSet<_> = document
        .find(Name("a"))
        .filter_map(|n| n.attr("href"))
        .collect();
    for href in a_hrefs {
        let href_url = url.join(href.split('#').next().unwrap());
        match href_url {
            Ok(href_url) => {
                hrefs.insert(href_url);
            }
            Err(err) => {
                println!("{err}.");
            }
        }
    }
    let mut imgs = BTreeSet::new();
    let imgs_src: BTreeSet<_> = document
        .find(Name("img"))
        .filter_map(|n| n.attr("src"))
        .collect();
    for img in imgs_src {
        let img_url = url.join(img);
        match img_url {
            Ok(img_url0) => {
                imgs.insert(img_url0);
            }
            Err(err) => {
                println!("{err}.");
            }
        }
    }
    (str, hrefs, imgs)
}
