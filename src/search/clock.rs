use std::fmt;
use std::time::{Duration, Instant};





use crate::types::Color;



/// https://en.wikipedia.org/wiki/Chess_clock
#[derive(Copy, Clone, Debug)]
pub enum TimingMethod {
    Depth(u32),         // uci "depth"
    MoveTime(Duration), // uci "movetime"
    NodeCount(u64),     // uci "nodes"
    Infinite,           // uci "infinite"
    MateIn(u32),        // uci "mate"
    RemainingTime {    
        our_color: Color,
        wtime: Duration,
        btime: Duration,
        winc: Duration,
        binc: Duration,
        movestogo: u16,
    },
}

impl Default for TimingMethod {
    fn default() -> Self {
        TimingMethod::MoveTime(Duration::from_secs(5))
    }
}




struct DurationNewType(Duration);

fn format_duration(d: Duration) -> String {
    DurationNewType(d).to_string()
}

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
        write!(f, "{}h {}m {:.3}s", hours, mins, secs)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Clock {
    start_time: Option<Instant>,
}

impl Clock {
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// will panic if clock not started
    pub fn elapsed(&self) -> Duration {
        self.start_time.unwrap().elapsed()
    }
}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(t) = self.start_time {
            writeln!(f, "since start time :{}", format_duration(t.elapsed()))?;
        } else {
            writeln!(f, "start time       :not set")?;
        }
        Ok(())
    }
}
