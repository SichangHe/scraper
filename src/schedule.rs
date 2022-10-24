use anyhow::Result;
use bytes::Bytes;
use log::{debug, error, info};
use regex::Regex;
use reqwest::{Client, Response, Url};
use std::{
    collections::{BTreeSet, VecDeque},
    time::Duration,
};
use tokio::time::{sleep, Instant};

use crate::{
    file::FileContent,
    io::{save_file, Writer},
    middle::{double_unwrap, Conclusion, Process, Request},
    ring::Ring,
    urls::Record,
};

pub const DEFAULT_DELAY: Duration = Duration::from_millis(500);
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub const TIMEOUT_MULTIPLIER: u32 = 5;
pub const WRITE_FREQUENCY: usize = 8;
pub const RECORD_DIR: &str = "record.toml";

#[derive(Debug)]
pub struct Scheduler {
    time: Instant,
    delay: Duration,
    client: Client,
    filter: Regex,
    blacklist: Regex,
    rec: Record,
    pending: VecDeque<usize>,
    requests: Vec<Request>,
    processes: Vec<Process>,
    conclusions: VecDeque<Conclusion>,
    disregard_html: bool,
    disregard_other: bool,
    html_dir: String,
    other_dir: String,
    log_dir: String,
    writer: Option<Writer>,
    ring: Option<Ring>,
}

impl Scheduler {
    pub fn from_client(client: Client) -> Self {
        Self {
            time: Instant::now(),
            delay: DEFAULT_DELAY,
            client,
            filter: Regex::new(".*").unwrap(),
            blacklist: Regex::new("#").unwrap(),
            rec: Record::default(),
            pending: VecDeque::new(),
            requests: Vec::new(),
            processes: Vec::new(),
            conclusions: VecDeque::new(),
            disregard_html: false,
            disregard_other: false,
            html_dir: "html".to_owned(),
            other_dir: "other".to_owned(),
            log_dir: "log".to_owned(),
            writer: None,
            ring: None,
        }
    }

    pub fn delay(self, delay: Duration) -> Self {
        Self { delay, ..self }
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

    pub fn filter(self, filter: Regex) -> Self {
        Self { filter, ..self }
    }

    pub fn blacklist(self, blacklist: Regex) -> Self {
        Self { blacklist, ..self }
    }

    pub fn disregard_html(self) -> Self {
        Self {
            disregard_html: true,
            ..self
        }
    }

    pub fn disregard_other(self) -> Self {
        Self {
            disregard_other: true,
            ..self
        }
    }

    pub fn html_dir(self, html_dir: String) -> Self {
        Self { html_dir, ..self }
    }

    pub fn other_dir(self, other_dir: String) -> Self {
        Self { other_dir, ..self }
    }

    pub fn log_dir(self, log_dir: String) -> Self {
        Self { log_dir, ..self }
    }

    pub fn with_number_of_rings(self, number_of_rings: u8) -> Self {
        Self {
            ring: Some(Ring::new(number_of_rings)),
            ..self
        }
    }

    pub fn add_pending(&mut self, url: Url) {
        if let Ok(index) = self.rec.check_add_url(url) {
            self.pending.push_back(index);
        }
    }

    pub fn add_next_pending(&mut self, url: Url) {
        if let Some(ref mut ring) = self.ring {
            if let Ok(index) = self.rec.check_add_url(url) {
                ring.next.push_back(index);
            }
        }
    }

    pub async fn spawn_one_request(&mut self) -> bool {
        let url_id = match self.pending.pop_front() {
            Some(url_id) => url_id,
            None => return false,
        };
        let url = self.rec.url_ids.get(&url_id).unwrap().to_owned();
        info!("Requesting {url_id} | {url}.");
        self.requests
            .push(Request::spawn(url_id, self.client.get(url)).await);
        true
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
                error!("{url_id}: {err}.");
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
            Ok(conclusion) => self.conclusions.push_back(conclusion),
            Err(err) => {
                error!("{url_id}: {err}.");
                self.fail(url_id);
            }
        }
    }

    async fn process_response(&mut self, url_id: usize, response: Response) {
        let final_url_id = match self.rec.check_final_url(url_id, &response).await {
            Some(id) => id,
            None => return,
        };
        debug!("Processing {final_url_id}.");
        self.processes
            .push(Process::spawn(final_url_id, response).await);
    }

    pub async fn process_one_conclusion(&mut self) {
        let conclusion = {
            if let Some(conclusion) = self.conclusions.pop_front() {
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
            error!("{url_id}: {err}.");
            self.fail(url_id);
        });
    }

    async fn process_html(
        &mut self,
        url_id: usize,
        text: String,
        hrefs: BTreeSet<Url>,
        imgs: BTreeSet<Url>,
    ) -> Result<()> {
        for href in hrefs {
            let href_str = href.as_str();
            if !self.blacklist.is_match(href_str) {
                if self.filter.is_match(href_str) {
                    self.add_pending(href);
                } else {
                    self.add_next_pending(href);
                }
            } else {
                _ = self.rec.check_add_url(href)
            }
        }
        for img in imgs {
            // Not filtering images.
            self.add_pending(img);
        }
        if !self.disregard_html {
            save_file(&format!("{}/{url_id}.html", self.html_dir), text.as_bytes()).await?;
        }
        Ok(())
    }

    async fn process_other(&mut self, url_id: usize, extension: &str, bytes: Bytes) -> Result<()> {
        if self.disregard_other {
            return Ok(());
        }
        save_file(&format!("{}/{url_id}{extension}", self.other_dir), &bytes).await?;
        Ok(())
    }

    fn vec_lens(&self) -> (usize, usize, usize, usize) {
        (
            self.pending.len(),
            self.requests.len(),
            self.processes.len(),
            self.conclusions.len(),
        )
    }

    fn rec_lens(&self) -> (usize, usize, usize, usize) {
        (
            self.rec.urls.len(),
            self.rec.scrapes.len(),
            self.rec.fails.len(),
            self.rec.redirects.len(),
        )
    }

    /// Recursively scrape until there are no more pending URLs.
    pub async fn recursion(&mut self) {
        self.time = Instant::now();
        let (mut pending_len, mut requests_len, mut processes_len, mut conclusions_len) =
            self.vec_lens();
        let (mut urls_len, mut scrapes_len, mut fails_len, mut redirects_len) = self.rec_lens();
        let mut changes: usize = 0;
        while !self.pending.is_empty()
            || !self.requests.is_empty()
            || !self.processes.is_empty()
            || !self.conclusions.is_empty()
            || self.increment_ring()
        {
            self.one_cycle().await;
            if pending_len != self.pending.len()
                || requests_len != self.requests.len()
                || processes_len != self.processes.len()
                || conclusions_len != self.conclusions.len()
            {
                (pending_len, requests_len, processes_len, conclusions_len) = self.vec_lens();
                debug!(
                    "{} pending, {} requests, {} processes, {} conclusions.",
                    pending_len, requests_len, processes_len, conclusions_len
                );
            }
            if urls_len != self.rec.urls.len()
                || scrapes_len != self.rec.scrapes.len()
                || fails_len != self.rec.fails.len()
                || redirects_len != self.rec.redirects.len()
            {
                changes += 1;
                (urls_len, scrapes_len, fails_len, redirects_len) = self.rec_lens();
                if changes % WRITE_FREQUENCY == 0 {
                    self.write().await;
                }
            }
        }

        self.write_all().await;
    }

    fn increment_ring(&mut self) -> bool {
        if let Some(ref mut ring) = self.ring {
            if let Some(pending) = ring.increment() {
                self.pending = pending;
                return true;
            }
        }
        false
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
        if self.time.elapsed() >= self.delay && self.spawn_one_request().await {
            self.time += self.delay;
        }
    }

    /// Tell the scheduler to finish whatever is already started
    /// and do not initiate any more requests.
    pub async fn finish(&mut self) {
        self.time = Instant::now();
        while !self.requests.is_empty()
            || !self.processes.is_empty()
            || !self.conclusions.is_empty()
        {
            self.check_requests().await;
            self.check_processes().await;
            self.process_one_conclusion().await;
            sleep(self.delay.saturating_sub(self.time.elapsed())).await;
            self.time += self.delay;
        }
        self.write_all().await;
    }

    fn fail(&mut self, url_id: usize) {
        if self.rec.fails.contains(&url_id) {
            return;
        }
        self.rec.fails.insert(url_id);
        self.pending.push_back(url_id);
    }

    async fn write(&mut self) {
        {
            let _ = self.writer.take();
        }
        self.writer = Some(
            Writer::spawn(
                format!("{}/{RECORD_DIR}", self.log_dir),
                toml::to_string_pretty(&self.rec).unwrap(),
            )
            .await,
        );
    }

    async fn write_all(&mut self) {
        for _ in 0..8 {
            self.write().await;
            let writer = self.writer.take().unwrap();
            if let Err(err) = writer.wait().await {
                error!("Write all: {err}.");
                sleep(Duration::from_secs(1)).await;
            } else {
                return;
            }
        }
        error!("Fatal! Write all: all eight attempts failed!");
    }
}
