use anyhow::{Error, Result};
use reqwest::{Client, Url};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::Duration,
};

use crate::scrape::Attempt;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_MULTIPLIER: u32 = 5;

#[derive(Debug)]
pub struct Scheduler {
    client: Client,
    scrapers: Vec<Attempt>,
    urls: BTreeMap<Url, usize>,
    url_ids: BTreeMap<usize, Url>,
    scrapes: BTreeSet<usize>,
    fails: BTreeSet<usize>,
    redirects: BTreeMap<usize, usize>,
    pending: VecDeque<usize>,
}

impl Scheduler {
    pub fn from_client(client: Client) -> Self {
        Self {
            client,
            scrapers: Vec::new(),
            urls: BTreeMap::new(),
            url_ids: BTreeMap::new(),
            scrapes: BTreeSet::new(),
            fails: BTreeSet::new(),
            redirects: BTreeMap::new(),
            pending: VecDeque::new(),
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

    pub fn add_url(&mut self, url: Url) {
        if let Some(_index) = self.urls.get(&url) {
            return;
        }
        let index = self.urls.len();
        self.urls.insert(url.clone(), index);
        self.url_ids.insert(index, url);
        self.pending.push_back(index);
    }

    pub async fn launch_scraper(&mut self) -> Result<()> {
        let url_id = self
            .pending
            .pop_front()
            .ok_or_else(|| Error::msg("No pending URLs."))?;
        let url = self.url_ids.get(&url_id).unwrap().to_owned();
        let mut attempt = Attempt::with_request(self.client.get(url));
        attempt.run().await.map_err(|e| {
            self.fail(url_id);
            e
        })?;
        self.scrapers.push(attempt);
        Ok(())
    }

    pub async fn finish(&mut self) -> Result<()> {
        for attempt in &mut self.scrapers {
            attempt.finish().await?;
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
