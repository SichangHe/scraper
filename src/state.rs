use std::collections::VecDeque;

use futures::stream::FuturesUnordered;
use tokio::time::Instant;

use crate::{
    io::Writer,
    middle::{Conclusion, Process, Request},
};

#[derive(Debug)]
pub struct SchedulerState {
    pub time: Instant,
    pub pending: VecDeque<usize>,
    pub requests: FuturesUnordered<Request>,
    pub processes: FuturesUnordered<Process>,
    pub conclusions: VecDeque<Conclusion>,
    pub writer: Option<Writer>,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self {
            time: Instant::now(),
            pending: VecDeque::new(),
            requests: FuturesUnordered::new(),
            processes: FuturesUnordered::new(),
            conclusions: VecDeque::new(),
            writer: None,
        }
    }
}

impl SchedulerState {
    pub fn has_processing(&self) -> bool {
        !self.requests.is_empty() || !self.processes.is_empty() || !self.conclusions.is_empty()
    }

    pub fn has_more_tasks(&self) -> bool {
        !self.pending.is_empty() || self.has_processing()
    }

    pub fn lens(&self) -> (usize, usize, usize, usize) {
        (
            self.pending.len(),
            self.requests.len(),
            self.processes.len(),
            self.conclusions.len(),
        )
    }

}
