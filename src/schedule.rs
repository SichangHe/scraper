use anyhow::{Error, Result};
use bytes::Bytes;
use reqwest::{Client, Url};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Duration,
};
use tokio::task::JoinHandle;

use crate::scrape::{scrape_with_new_thread, Conclusion, FileContent};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_MULTIPLIER: u32 = 5;
const HTML_DIR: &str = "html/";
const OTHER_DIR: &str = "other/";

#[derive(Debug)]
pub struct Scheduler {
    client: Client,
    handles: Vec<(usize, JoinHandle<Result<Conclusion>>)>,
    urls: BTreeMap<Url, usize>,
    url_ids: BTreeMap<usize, Url>,
    scrapes: BTreeSet<usize>,
    fails: BTreeSet<usize>,
    redirects: BTreeMap<usize, usize>,
    pending: VecDeque<usize>,
    conclusions: Vec<(usize, Conclusion)>,
}

impl Scheduler {
    pub fn from_client(client: Client) -> Self {
        Self {
            client,
            handles: Vec::new(),
            urls: BTreeMap::new(),
            url_ids: BTreeMap::new(),
            scrapes: BTreeSet::new(),
            fails: BTreeSet::new(),
            redirects: BTreeMap::new(),
            pending: VecDeque::new(),
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
        self.handles
            .push((url_id, scrape_with_new_thread(self.client.get(url)).await));
        Ok(())
    }

    pub async fn check_handles(&mut self) -> Result<()> {
        let mut index = self.handles.len();
        while index > 0 {
            index -= 1;
            if self.handles[index].1.is_finished() {
                let handle = self.handles.remove(index);
                index -= 1;
                match handle.1.await {
                    Ok(conclusion_or_err) => match conclusion_or_err {
                        Ok(conclusion) => self.conclusions.push((handle.0, conclusion)),
                        Err(err) => {
                            println!("{err}");
                            self.fail(handle.0)
                        }
                    },
                    Err(err) => {
                        println!("{err}");
                        self.fail(handle.0)
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn process_conclusion(&mut self) -> Result<()> {
        let (mut url_id, conclusion) = {
            if let Some(conclusion) = self.conclusions.pop() {
                conclusion
            } else {
                // No conclusions pending.
                return Ok(());
            }
        };
        let Conclusion {
            final_url,
            extension,
            content,
        } = conclusion;
        match self.check_add_url(final_url) {
            Ok(id) => {
                self.redirects.insert(url_id, id);
                url_id = id;
            }
            Err(id) => {
                if url_id == id {
                    self.redirects.insert(url_id, id);
                    if self.scrapes.contains(&id) {
                        // Recorded, skipping.
                        return Ok(());
                    }
                }
            }
        }
        match content {
            FileContent::Html(text, hrefs, imgs) => {
                self.process_html(url_id, text, hrefs, imgs).await
            }
            FileContent::Other(bytes) => self.process_other(url_id, &extension, bytes).await,
        }?;
        Ok(())
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
        while let Some(handle) = self.handles.pop() {
            self.conclusions.push((handle.0, handle.1.await??));
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

async fn save_file(name: &str, bytes: &[u8]) -> Result<()> {
    todo!()
}
