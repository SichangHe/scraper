use anyhow::{Error, Result};
use bytes::Bytes;
use reqwest::{Client, Response, Url};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Duration,
};
use tokio::time::sleep;

use crate::{
    file::FileContent,
    io::save_file,
    middle::{double_unwrap, Process, Request},
    scrape::Conclusion,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_MULTIPLIER: u32 = 5;
const HTML_DIR: &str = "html/";
const OTHER_DIR: &str = "other/";

#[derive(Debug)]
pub struct Scheduler {
    client: Client,
    urls: BTreeMap<Url, usize>,
    url_ids: BTreeMap<usize, Url>,
    scrapes: BTreeSet<usize>,
    fails: BTreeSet<usize>,
    redirects: BTreeMap<usize, usize>,
    pending: VecDeque<usize>,
    requests: Vec<Request>,
    processes: Vec<Process>,
    conclusions: Vec<(usize, Conclusion)>,
}

impl Scheduler {
    pub fn from_client(client: Client) -> Self {
        Self {
            client,
            urls: BTreeMap::new(),
            url_ids: BTreeMap::new(),
            scrapes: BTreeSet::new(),
            fails: BTreeSet::new(),
            redirects: BTreeMap::new(),
            pending: VecDeque::new(),
            requests: Vec::new(),
            processes: Vec::new(),
            conclusions: Vec::new(),
        }
    }

    pub fn default_client() -> reqwest::Result<Client> {
        Scheduler::client_with_timeout(DEFAULT_TIMEOUT)
    }

    pub fn client_with_timeout(timeout: Duration) -> reqwest::Result<Client> {
        Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout * TIMEOUT_MULTIPLIER)
            .build()
    }

    pub fn add_pending(&mut self, url: Url) {
        if let Ok(index) = self.check_add_url(url) {
            self.pending.push_back(index);
        }
    }

    /// If given URL is recorded, return `Err(url_id)`
    ///
    /// If it is not, record it and return `Ok(url_id)`
    pub fn check_add_url(&mut self, url: Url) -> Result<usize, usize> {
        if let Some(index) = self.urls.get(&url) {
            return Err(*index);
        }
        let index = self.urls.len();
        self.urls.insert(url.clone(), index);
        self.url_ids.insert(index, url);
        Ok(index)
    }

    pub async fn launch_scraper(&mut self) -> Result<()> {
        let url_id = self
            .pending
            .pop_front()
            .ok_or_else(|| Error::msg("No pending URLs."))?;
        let url = self.url_ids.get(&url_id).unwrap().to_owned();
        self.requests
            .push(Request::spawn(url_id, self.client.get(url)).await);
        Ok(())
    }

    pub async fn check_requests(&mut self) {
        let mut index = self.requests.len();
        while index > 0 {
            index -= 1;
            self.check_one_request(&mut index).await;
        }
    }

    async fn check_one_request(&mut self, index: &mut usize) {
        if !self.requests[*index].handle.is_finished() {
            return;
        }
        let Request { url_id, handle } = self.requests.remove(*index);
        *index = index.saturating_sub(1);
        match double_unwrap(handle).await {
            Ok(response) => self.process_response(url_id, response).await,
            Err(err) => {
                println!("{err}");
                self.fail(url_id);
            }
        };
    }

    async fn check_processes(&mut self) {
        let mut index = self.processes.len();
        while index > 0 {
            index -= 1;
            self.check_one_process(&mut index).await;
        }
    }

    async fn check_one_process(&mut self, index: &mut usize) {
        if !self.processes[*index].handle.is_finished() {
            return;
        }
        let Process { url_id, handle } = self.processes.remove(*index);
        *index = index.saturating_sub(1);
        match double_unwrap(handle).await {
            Ok(conclusion) => self.conclusions.push((url_id, conclusion)),
            Err(err) => {
                println!("{err}");
                self.fail(url_id);
            }
        }
    }

    async fn process_response(&mut self, url_id: usize, response: Response) {
        let final_url_id = match self.check_add_url(response.url().to_owned()) {
            Ok(final_url_id) => final_url_id,
            Err(final_url_id) => {
                if url_id != final_url_id && self.scrapes.contains(&final_url_id) {
                    return; // abort because scraped
                }
                final_url_id
            }
        };
        if url_id != final_url_id {
            self.redirects.insert(url_id, final_url_id);
        }
        self.scrapes.insert(final_url_id);
        self.processes
            .push(Process::spawn(final_url_id, response).await)
    }

    pub async fn process_one_conclusion(&mut self) {
        let (url_id, conclusion) = {
            if let Some(conclusion) = self.conclusions.pop() {
                conclusion
            } else {
                // No conclusions pending.
                return;
            }
        };
        let Conclusion {
            final_url,
            extension,
            content,
        } = conclusion;
        match content {
            FileContent::Html(text, hrefs, imgs) => {
                self.process_html(url_id, text, hrefs, imgs).await
            }
            FileContent::Other(bytes) => self.process_other(url_id, &extension, bytes).await,
        }
        .unwrap_or_else(|err| {
            println!("{err}");
            self.fail(url_id);
        });
    }

    async fn process_html(
        &mut self,
        url_id: usize,
        text: String,
        hrefs: Vec<Url>,
        imgs: Vec<Url>,
    ) -> Result<()> {
        for href in hrefs {
            self.add_pending(href);
        }
        for img in imgs {
            self.add_pending(img);
        }
        save_file(&format!("{HTML_DIR}{url_id}.html"), text.as_bytes()).await?;
        Ok(())
    }

    async fn process_other(&mut self, url_id: usize, extension: &str, bytes: Bytes) -> Result<()> {
        save_file(&format!("{OTHER_DIR}{url_id}{extension}"), &bytes).await?;
        Ok(())
    }

    // Strictly for test.
    pub async fn finish(&mut self) -> Result<()> {
        while !self.requests.is_empty()
            || !self.processes.is_empty()
            || !self.conclusions.is_empty()
        {
            self.check_requests().await;
            self.check_processes().await;
            self.process_one_conclusion().await;
            sleep(Duration::from_millis(250)).await;
        }
        Ok(())
    }

    fn fail(&mut self, url_id: usize) {
        if self.fails.contains(&url_id) {
            return;
        }
        self.fails.insert(url_id);
        self.pending.push_back(url_id);
    }
}
