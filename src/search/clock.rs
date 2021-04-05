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

#[derive(Debug, Copy, Clone)]
pub struct Clock {
    start: Instant,
}


impl Default for Clock {
    fn default() -> Self {
        Clock{ start: Instant::now() }
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

    pub fn format_duration(d: Duration) -> String {
        DurationNewType(d).to_string()
    }

}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Self::format_duration(self.start.elapsed()))?;
        Ok(())
    }
}
