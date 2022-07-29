use anyhow::Result;
use reqwest::{RequestBuilder, Response, Url};
use tokio::{spawn, task::JoinHandle};

use crate::file::{links_from_html, process_headers, FileContent, FileType};

#[derive(Debug)]
pub struct Request {
    pub url_id: usize,
    pub handle: JoinHandle<Result<Response>>,
}

impl Request {
    pub async fn spawn(url_id: usize, request: RequestBuilder) -> Self {
        Self {
            url_id,
            handle: spawn(async {
                let response = request.send().await?;
                Ok(response)
            }),
        }
    }
}

pub async fn double_unwrap<T>(handle: JoinHandle<Result<T>>) -> Result<T> {
    handle.await?
}

async fn process_response(url_id: usize, response: Response) -> Result<Conclusion> {
    let final_url = response.url().to_owned();
    let url_str = clean_url(&final_url);
    let headers = response.headers();
    let file_type = process_headers(headers)?;
    let content;
    if let FileType::Html = file_type {
        let (text, hrefs, imgs) = links_from_html(&final_url, response.text().await?);
        content = FileContent::Html(text, hrefs, imgs);
    } else {
        let extension = ".".to_owned()
            + url_str
                .split('.')
                .last()
                .unwrap()
                .split('/')
                .last()
                .unwrap();
        content = FileContent::Other(extension, response.bytes().await?);
    }
    Ok(Conclusion { url_id, content })
}

fn clean_url(url: &Url) -> String {
    url.to_string().split('#').next().unwrap().to_owned()
}

#[derive(Debug)]
pub struct Process {
    pub url_id: usize,
    pub handle: JoinHandle<Result<Conclusion>>,
}

impl Process {
    pub async fn spawn(url_id: usize, response: Response) -> Self {
        Self {
            url_id,
            handle: spawn(process_response(url_id, response)),
        }
    }
}

#[derive(Debug)]
pub struct Conclusion {
    pub url_id: usize,
    pub content: FileContent,
}
