use anyhow::Result;
use reqwest::{RequestBuilder, Url};
use tokio::{spawn, task::JoinHandle};

use crate::{
    file::{links_from_html, process_headers, FileContent, FileType},
    scrape::Conclusion,
};

#[derive(Debug)]
pub struct Attempt {
    pub url_id: usize,
    pub handle: JoinHandle<Result<Conclusion>>,
}

impl Attempt {
    pub async fn spawn(url_id: usize, request: RequestBuilder) -> Self {
        Self {
            url_id,
            handle: spawn(process_request(request)),
        }
    }
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
