use crate::ring::Ring;
use regex::Regex;
use std::time::Duration;

pub const DEFAULT_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub struct SchedulerConfig {
    pub delay: Duration,
    pub filter: Regex,
    pub blacklist: Regex,
    pub disregard_html: bool,
    pub disregard_other: bool,
    pub html_dir: String,
    pub other_dir: String,
    pub log_dir: String,
    pub ring: Option<Ring>,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            delay: DEFAULT_DELAY,
            filter: Regex::new(".*").unwrap(),
            blacklist: Regex::new("#").unwrap(),
            disregard_html: false,
            disregard_other: false,
            html_dir: "html".to_owned(),
            other_dir: "other".to_owned(),
            log_dir: "log".to_owned(),
            ring: None,
        }
    }
}

impl SchedulerConfig {
    pub fn delay(self, delay: Duration) -> Self {
        Self { delay, ..self }
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
}
