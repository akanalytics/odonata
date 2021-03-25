use std::fmt;
use std::time::Duration;
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

