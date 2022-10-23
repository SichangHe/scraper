use std::collections::{BTreeMap, BTreeSet};

use log::{debug, info};
use reqwest::{Response, Url};
use serde::{ser::SerializeStruct, Serialize};

#[derive(Debug, Default)]
pub struct Record {
    pub urls: BTreeMap<Url, usize>,
    pub url_ids: BTreeMap<usize, Url>,
    pub scrapes: BTreeSet<usize>,
    pub fails: BTreeSet<usize>,
    pub redirects: BTreeMap<usize, usize>,
}

impl Record {
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

    /// # Return
    /// `None` if the URL is already scraped.
    ///
    /// `Some(final_url_id)` otherwise.
    pub async fn check_final_url(&mut self, url_id: usize, response: &Response) -> Option<usize> {
        let final_url_id = match self.check_add_url(response.url().to_owned()) {
            Ok(id) => id,
            Err(id) => {
                if url_id != id && self.scrapes.contains(&id) {
                    debug!("{url_id}: already scraped as {id}.");
                    return None;
                }
                id
            }
        };
        if url_id != final_url_id {
            info!("{url_id} redirected to {url_id}.");
            self.redirects.insert(url_id, final_url_id);
        }
        self.scrapes.insert(final_url_id);
        Some(final_url_id)
    }
}

impl Serialize for Record {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_struct("record", 4)?;
        seq.serialize_field("scrapes", &self.scrapes)?;
        seq.serialize_field("fails", &self.fails)?;
        let urls: BTreeMap<_, _> = self
            .urls
            .iter()
            .map(|(url, id)| (url.to_string(), id))
            .collect();
        seq.serialize_field("urls", &urls)?;
        let redirects: BTreeMap<_, _> = self
            .redirects
            .iter()
            .map(|(before, after)| (before.to_string(), after))
            .collect();
        seq.serialize_field("redirects", &redirects)?;
        seq.end()
    }
}
