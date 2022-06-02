use crate::utils::Formatting;
use std::fmt;
use std::time::{Duration, Instant};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Clock3 {
    start_search: Instant,
    start_iteration: Instant,
    start_ply: Instant,
}


impl Default for Clock3 {
    fn default() -> Self {
        let now = Instant::now();
        Clock3 {
            start_search: now,
            start_iteration: now,
            start_ply: now,
        }
    }
}

impl fmt::Display for Clock3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Formatting::duration(self.elapsed_search()))
    }
}

impl Clock3 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_iteration(&mut self) {
        self.start_iteration = Instant::now();
    }

    pub fn start_ply(&mut self) {
        self.start_ply = Instant::now();
    }

    /// will panic if clock not started
    pub fn elapsed_search(&self) -> Duration {
        self.start_search.elapsed()
    }

    // pub fn elapsed_iteration(&self) -> Duration {
    //     self.start_iteration.elapsed()
    // }

    pub fn elapsed_ply(&self) -> Duration {
        self.start_ply.elapsed()
    }

    pub fn elapsed_search_millis(&self) -> u64 {
        self.elapsed_search().as_millis() as u64
    }
}
