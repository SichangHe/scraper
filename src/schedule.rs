use anyhow::Result;
use bytes::Bytes;
use regex::Regex;
use reqwest::{Client, Response, Url};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Duration,
};
use tokio::time::{sleep, Instant};

use crate::{
    file::FileContent,
    io::save_file,
    middle::{double_unwrap, Conclusion, Process, Request},
};

const DEFAULT_DELAY: Duration = Duration::from_millis(500);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const TIMEOUT_MULTIPLIER: u32 = 5;
const HTML_DIR: &str = "html/";
const OTHER_DIR: &str = "other/";

#[derive(Debug)]
pub struct Scheduler {
    time: Instant,
    delay: Duration,
    client: Client,
    filter: Regex,
    blacklist: Regex,
    urls: BTreeMap<Url, usize>,
    url_ids: BTreeMap<usize, Url>,
    scrapes: BTreeSet<usize>,
    fails: BTreeSet<usize>,
    redirects: BTreeMap<usize, usize>,
    pending: VecDeque<usize>,
    requests: Vec<Request>,
    processes: Vec<Process>,
    conclusions: Vec<Conclusion>,
}

impl Scheduler {
    pub fn from_client(client: Client) -> Self {
        Self {
            time: Instant::now(),
            delay: DEFAULT_DELAY,
            client,
            filter: Self::default_filter(),
            blacklist: Self::default_blacklist(),
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
        Self::client_with_timeout(DEFAULT_TIMEOUT)
    }

    pub fn client_with_timeout(timeout: Duration) -> reqwest::Result<Client> {
        Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout * TIMEOUT_MULTIPLIER)
            .build()
    }

    pub fn new() -> Result<Self> {
        let scheduler = Self::from_client(Self::default_client()?);
        Ok(scheduler)
    }

    fn default_filter() -> Regex {
        Regex::new(".*").unwrap()
    }

    fn default_blacklist() -> Regex {
        Regex::new("#").unwrap()
    }

    pub fn filter(self, filter: Regex) -> Self {
        Self { filter, ..self }
    }

    pub fn blacklist(self, blacklist: Regex) -> Self {
        Self { blacklist, ..self }
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

    pub async fn spawn_one_request(&mut self) {
        let url_id = match self.pending.pop_front() {
            Some(url_id) => url_id,
            None => return,
        };
        let url = self.url_ids.get(&url_id).unwrap().to_owned();
        println!("\tRequesting {url_id} | {url}.");
        self.requests
            .push(Request::spawn(url_id, self.client.get(url)).await);
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
                println!("{url_id}: {err}.");
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
            Ok(conclusion) => self.conclusions.push(conclusion),
            Err(err) => {
                println!("{url_id}: {err}.");
                self.fail(url_id);
            }
        }
    }

    async fn process_response(&mut self, url_id: usize, response: Response) {
        let final_url_id = match self.check_add_url(response.url().to_owned()) {
            Ok(final_url_id) => final_url_id,
            Err(final_url_id) => {
                if url_id != final_url_id && self.scrapes.contains(&final_url_id) {
                    println!("\t{url_id}: already scraped as {final_url_id}.");
                    return; // abort because scraped
                }
                final_url_id
            }
        };
        if url_id != final_url_id {
            println!("\t{url_id} redirected to {url_id}.");
            self.redirects.insert(url_id, final_url_id);
        }
        self.scrapes.insert(final_url_id);
        println!("\tProcessing {final_url_id}.");
        self.processes
            .push(Process::spawn(final_url_id, response).await);
    }

    pub async fn process_one_conclusion(&mut self) {
        let conclusion = {
            if let Some(conclusion) = self.conclusions.pop() {
                conclusion
            } else {
                // No conclusions pending.
                return;
            }
        };
        let Conclusion { url_id, content } = conclusion;
        match content {
            FileContent::Html(text, hrefs, imgs) => {
                self.process_html(url_id, text, hrefs, imgs).await
            }
            FileContent::Other(extension, bytes) => {
                self.process_other(url_id, &extension, bytes).await
            }
        }
        .unwrap_or_else(|err| {
            println!("{url_id}: {err}.");
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
            let href_str = href.as_str();
            if self.filter.is_match(href_str) && !self.blacklist.is_match(href_str) {
                self.add_pending(href);
            }
        }
        for img in imgs {
            // Not filtering images.
            self.add_pending(img);
        }
        save_file(&format!("{HTML_DIR}{url_id}.html"), text.as_bytes()).await?;
        Ok(())
    }

    async fn process_other(&mut self, url_id: usize, extension: &str, bytes: Bytes) -> Result<()> {
        save_file(&format!("{OTHER_DIR}{url_id}{extension}"), &bytes).await?;
        Ok(())
    }

    pub async fn recursion(&mut self) {
        self.time = Instant::now();
        let (mut pending_len, mut requests_len, mut processes_len, mut conclusions_len) = (
            self.pending.len(),
            self.requests.len(),
            self.processes.len(),
            self.conclusions.len(),
        );
        while !self.pending.is_empty()
            || !self.requests.is_empty()
            || !self.processes.is_empty()
            || !self.conclusions.is_empty()
        {
            self.one_cycle().await;
            let changed = pending_len != self.pending.len()
                || requests_len != self.requests.len()
                || processes_len != self.processes.len()
                || conclusions_len != self.conclusions.len();
            (pending_len, requests_len, processes_len, conclusions_len) = (
                self.pending.len(),
                self.requests.len(),
                self.processes.len(),
                self.conclusions.len(),
            );
            if changed {
                println!(
                    "\t{} pending, {} requests, {} processes, {} conclusions.",
                    pending_len, requests_len, processes_len, conclusions_len
                );
            }
        }
    }

    async fn one_cycle(&mut self) {
        self.check_spawn_request().await;
        self.check_requests().await;
        self.check_spawn_request().await;
        self.check_processes().await;
        self.check_spawn_request().await;
        self.process_one_conclusion().await;
        sleep(self.delay.saturating_sub(self.time.elapsed())).await;
    }

    async fn check_spawn_request(&mut self) {
        if self.time.elapsed() >= self.delay && !self.pending.is_empty() {
            self.spawn_one_request().await;
            self.time += self.delay;
        }
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
