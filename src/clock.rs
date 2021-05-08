use crate::globals::counts;
use std::fmt;
use std::time::{Duration, Instant};

pub struct DurationNewType(pub Duration);

fn pluralize(n: u64) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

// eg 2 days 15h 4m 3.003s
impl fmt::Display for DurationNewType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let days = self.0.as_secs() / (60 * 60 * 24);
        let hours = self.0.as_secs() / (60 * 60) % 24;
        let mins = self.0.as_secs() / 60;
        let secs = self.0.as_secs_f32() - (60 * mins) as f32;
        let mins = mins % 60;
        if days > 0 {
            write!(f, "{} day{} ", days, pluralize(days))?;
        }
        if hours > 0 {
            write!(f, "{}h ", hours)?;
        }
        if mins > 0 {
            write!(f, "{}m ", mins)?;
        }
        write!(f, "{:.3}s", secs)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Clock {
    start: Instant,
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
        write!(f, "{}", Clock::format(self.elapsed()))
    }
}

impl Default for Clock {
    fn default() -> Self {
        Clock {
            start: Instant::now(),
        }
    }
}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Self::format(self.elapsed()))
    }
}

impl Clock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn restart(&mut self) {
        self.start = Instant::now();
    }

    /// will panic if clock not started
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn format(d: Duration) -> String {
        DurationNewType(d).to_string()
    }
}
