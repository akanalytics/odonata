use crate::globals::counts;
use std::fmt;
use std::time::{Duration, Instant};
use crate::utils::Formatting;



#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Clock3 {
    start_search: Instant,
    start_iteration: Instant,
    start_ply: Instant,
}

// a clock that is deterministic based upon the count of various operations
// not at all accurate - but useful for testing the timekeeping logic
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeterministicClock {
    legal_move_count: i64,
    eval_count: i64,
    qeval_count: i64,
}

impl Default for DeterministicClock {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl DeterministicClock {
    #[inline]
    pub fn new() -> Self {
        Self {
            legal_move_count: counts::LEGAL_MOVE_COUNT.get(),
            eval_count: counts::EVAL_COUNT.get(),
            qeval_count: counts::QEVAL_COUNT.get(),
        }
    }

    #[inline]
    pub fn restart(&mut self) {
        *self = Self::new();
    }

    /// will panic if clock not started
    #[inline]
    pub fn elapsed(&self) -> Duration {
        let now = Self::new();
        let evals = now.eval_count - self.eval_count;
        let qevals = now.qeval_count - self.qeval_count;
        let legal_move_gens = now.legal_move_count - self.legal_move_count;
        let nano_secs = 6 * 200 * (evals + qevals) + 900 * legal_move_gens; // initial mutiplier = fudge factor

        Duration::from_nanos(nano_secs as u64)
    }
}

impl fmt::Display for DeterministicClock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Formatting::format_duration(self.elapsed()))
    }
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
        write!(f, "{}", Formatting::format_duration(self.elapsed_search()))
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

    pub fn elapsed_iteration(&self) -> Duration {
        self.start_iteration.elapsed()
    }

    pub fn elapsed_ply(&self) -> Duration {
        self.start_ply.elapsed()
    }

    pub fn elapsed_search_millis(&self) -> u64 {
        self.elapsed_search().as_millis() as u64
    }

}
