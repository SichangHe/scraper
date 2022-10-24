use std::collections::VecDeque;
use std::mem::take;

#[derive(Debug, Default)]
pub struct Ring {
    pub max: u8,
    pub current: u8,
    pub next: VecDeque<usize>,
}

impl Ring {
    pub fn new(number_of_rings: u8) -> Self {
        Self {
            max: number_of_rings,
            current: 0,
            next: VecDeque::new(),
        }
    }

    pub fn increment(&mut self) -> Option<VecDeque<usize>> {
        if self.current == self.max {
            return None;
        }
        self.current += 1;
        Some(take(&mut self.next))
    }
}
