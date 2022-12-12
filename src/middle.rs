use anyhow::{Error, Result};
use reqwest::{RequestBuilder, Response, Url};
use tokio::{spawn, task::JoinHandle};

use crate::file::{links_from_html, process_headers, FileContent, FileType};

pub type Request = JoinHandle<(usize, Result<Response>)>;

pub async fn spawn_request(url_id: usize, request: RequestBuilder) -> Request {
    spawn(async move {
        (
            url_id,
            match request.send().await {
                Ok(response) => Ok(response),
                Err(err) => Err(Error::from(err)),
            },
        )
    })
}

async fn process_response(url_id: usize, response: Response) -> Result<Conclusion> {
    let status = response.status();
    if !status.is_success() {
        return Err(Error::msg(format!("status code error: {status}")));
    }
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

pub type Process = JoinHandle<(usize, Result<Conclusion>)>;

pub async fn spawn_process(url_id: usize, response: Response) -> Process {
    spawn(async move { (url_id, process_response(url_id, response).await) })
}

#[derive(Debug)]
pub struct Conclusion {
    pub url_id: usize,
    pub content: FileContent,
}
