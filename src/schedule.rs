use anyhow::Result;
use bytes::Bytes;
use futures::StreamExt;
use log::{debug, error, info};

use reqwest::{Client, Response, Url};
use std::{collections::BTreeSet, time::Duration};
use tokio::time::{sleep, timeout, Instant};

use crate::{
    config::SchedulerConfig,
    file::FileContent,
    io::{save_file, Writer},
    middle::{spawn_process, spawn_request, Conclusion},
    state::SchedulerState,
    urls::Record,
};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub const TIMEOUT_MULTIPLIER: u32 = 5;
pub const WRITE_FREQUENCY: usize = 8;
pub const RECORD_DIR: &str = "record.toml";

pub fn client_with_timeout(timeout: Duration) -> Client {
    Client::builder()
        .connect_timeout(timeout)
        .timeout(timeout * TIMEOUT_MULTIPLIER)
        .build()
        .expect("Failed to build the client.")
}

pub fn default_client() -> Client {
    client_with_timeout(DEFAULT_TIMEOUT)
}

#[derive(Debug)]
pub struct Scheduler {
    cfg: SchedulerConfig,
    client: Client,
    rec: Record,
    s: SchedulerState,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }
}

impl Scheduler {
    pub fn from_client(client: Client, cfg: SchedulerConfig) -> Self {
        Self {
            cfg,
            client,
            rec: Record::default(),
            s: SchedulerState::default(),
        }
    }

    pub fn new(cfg: SchedulerConfig) -> Self {
        Self::from_client(default_client(), cfg)
    }

    pub fn delaying_requests(&self) -> bool {
        self.s.time.elapsed() < self.cfg.delay
    }

    pub fn add_pending(&mut self, url: Url) {
        if let Ok(index) = self.rec.check_add_url(url) {
            self.s.pending.push_back(index);
        }
    }

    pub fn add_next_pending(&mut self, url: Url) {
        if let Some(ref mut ring) = self.cfg.ring {
            if let Ok(index) = self.rec.check_add_url(url) {
                ring.next.push_back(index);
            }
        }
    }

    pub async fn spawn_one_request(&mut self) -> bool {
        let url_id = match self.s.pending.pop_front() {
            Some(url_id) => url_id,
            None => return false,
        };
        let url = self.rec.url_ids.get(&url_id).unwrap().to_owned();
        info!("Requesting {url_id} | {url}.");
        self.s
            .requests
            .push(spawn_request(url_id, self.client.get(url)).await);
        true
    }

    pub async fn check_requests(&mut self) {
        while self.check_one_request().await && self.delaying_requests() {}
    }

    pub async fn check_one_request(&mut self) -> bool {
        let result = match timeout(Duration::ZERO, self.s.requests.next()).await {
            Ok(r) => r,
            Err(_) => return false,
        };
        let result = match result {
            Some(r) => r,
            None => return false,
        };
        match result {
            Ok((url_id, response_result)) => match response_result {
                Ok(response) => self.process_response(url_id, response).await,
                Err(err) => {
                    error!("{url_id}: {err}.");
                    self.fail(url_id)
                }
            },
            Err(err) => error!("Request: {}", err),
        }
        true
    }

    pub async fn check_processes(&mut self) {
        while self.check_one_process().await && self.delaying_requests() {}
    }

    pub async fn check_one_process(&mut self) -> bool {
        let result = match timeout(Duration::ZERO, self.s.processes.next()).await {
            Ok(r) => r,
            Err(_) => return false,
        };
        let result = match result {
            Some(r) => r,
            None => return false,
        };
        match result {
            Ok((url_id, process_result)) => match process_result {
                Ok(conclusion) => self.s.conclusions.push_back(conclusion),
                Err(err) => {
                    error!("{url_id}: {err}.");
                    self.fail(url_id)
                }
            },
            Err(err) => error!("Request: {}", err),
        }
        true
    }

    async fn process_response(&mut self, url_id: usize, response: Response) {
        let final_url_id = match self.rec.check_final_url(url_id, &response).await {
            Some(id) => id,
            None => return,
        };
        debug!("Processing {final_url_id}.");
        self.s
            .processes
            .push(spawn_process(final_url_id, response).await);
    }

    pub async fn process_conclusions(&mut self) {
        while self.process_one_conclusion().await && self.delaying_requests() {}
    }

    pub async fn process_one_conclusion(&mut self) -> bool {
        let Conclusion { url_id, content } = match self.s.conclusions.pop_front() {
            Some(conclusion) => conclusion,
            None => return false, // No conclusions pending.
        };
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
        true
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
            if !self.cfg.blacklist.is_match(href_str) {
                if self.cfg.filter.is_match(href_str) {
                    self.add_pending(href);
                } else {
                    self.add_next_pending(href);
                }
            } else {
                _ = self.rec.check_add_url(href)
            }
        }
        if !self.cfg.disregard_other {
            for img in imgs {
                // Not filtering images.
                self.add_pending(img);
            }
        }
        if !self.cfg.disregard_html {
            save_file(
                &format!("{}/{url_id}.html", self.cfg.html_dir),
                text.as_bytes(),
            )
            .await?;
        }
        Ok(())
    }

    async fn process_other(&mut self, url_id: usize, extension: &str, bytes: Bytes) -> Result<()> {
        if self.cfg.disregard_other {
            return Ok(());
        }
        save_file(
            &format!("{}/{url_id}{extension}", self.cfg.other_dir),
            &bytes,
        )
        .await?;
        Ok(())
    }

    /// Recursively scrape until there are no more pending URLs.
    pub async fn recursion(&mut self) {
        self.s.time = Instant::now();
        let mut state_lens = self.s.lens();
        let mut record_lens = self.rec.lens();
        let mut changes: usize = 0;
        while self.s.has_more_tasks() || self.increment_ring() {
            self.one_cycle().await;
            if state_lens != self.s.lens() {
                state_lens = self.s.lens();
                debug!(
                    "{} pending, {} requests, {} processes, {} conclusions.",
                    state_lens.0, state_lens.1, state_lens.2, state_lens.3
                );
            }
            if record_lens != self.rec.lens() {
                changes += 1;
                record_lens = self.rec.lens();
                if changes % WRITE_FREQUENCY == 0 {
                    self.write().await;
                }
            }
        }

        self.write_all().await;
    }

    fn increment_ring(&mut self) -> bool {
        if let Some(ref mut ring) = self.cfg.ring {
            if let Some(pending) = ring.increment() {
                self.s.pending = pending;
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
        self.process_conclusions().await;
        self.check_spawn_request().await;
        sleep(self.cfg.delay.saturating_sub(self.s.time.elapsed())).await;
    }

    async fn check_spawn_request(&mut self) {
        if !self.delaying_requests() && self.spawn_one_request().await {
            self.s.time += self.cfg.delay;
        }
    }

    /// Tell the scheduler to finish whatever is already started
    /// and do not initiate any more requests.
    pub async fn finish(&mut self) {
        self.s.time = Instant::now();
        while self.s.has_processing() {
            self.check_requests().await;
            self.check_processes().await;
            self.process_one_conclusion().await;
            sleep(self.cfg.delay.saturating_sub(self.s.time.elapsed())).await;
            self.s.time += self.cfg.delay;
        }
        self.write_all().await;
    }

    fn fail(&mut self, url_id: usize) {
        if self.rec.fails.contains(&url_id) {
            return;
        }
        self.rec.fails.insert(url_id);
        self.s.pending.push_back(url_id);
    }

    async fn write(&mut self) {
        {
            let _ = self.s.writer.take();
        }
        self.s.writer = Some(
            Writer::spawn(
                format!("{}/{RECORD_DIR}", self.cfg.log_dir),
                toml::to_string_pretty(&self.rec).unwrap(),
            )
            .await,
        );
    }

    async fn write_all(&mut self) {
        for _ in 0..8 {
            self.write().await;
            let writer = self.s.writer.take().unwrap();
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
