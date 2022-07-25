use anyhow::{Error, Result};
use reqwest::{header::HeaderMap, RequestBuilder, Response, Url};
use tokio::{spawn, task::JoinHandle};

#[derive(Debug)]
pub struct Attempt {
    pub request: Option<RequestBuilder>,
    pub response: Option<Response>,
    handle: Option<JoinHandle<Result<(Response, Url, FileType)>>>,
}

impl Attempt {
    pub fn with_request(request: RequestBuilder) -> Self {
        Self {
            request: Some(request),
            response: None,
            handle: None,
        }
    }
    pub async fn run(&mut self) -> Result<()> {
        let request = self.request.take();
        if let Some(request) = request {
            self.handle = Some(spawn(process_request(request)));
        } else {
            return Err(Error::msg("No Request here."));
        }
        Ok(())
    }

    pub async fn finish(&mut self) -> Result<()> {
        if self.handle.is_none() {
            return Err(Error::msg("No handle."));
        }
        let handle = self.handle.take().unwrap();
        let (response, final_url, file_type) = handle.await??;
        self.response = Some(response);
        Ok(())
    }
}

async fn process_request(request: RequestBuilder) -> Result<(Response, Url, FileType)> {
    let response = request.send().await?;
    let final_url = response.url().to_owned();
    let headers = response.headers();
    let file_type = process_headers(headers)?;
    Ok((response, final_url, file_type))
}

#[derive(Debug)]
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
