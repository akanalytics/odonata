use crate::types::Color;
use std::fmt;
use std::time::Duration;

/// https://en.wikipedia.org/wiki/Time_control
///
#[derive(Copy, Clone, Debug)]
pub enum TimeControl {
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

impl Default for TimeControl {
    fn default() -> Self {
        TimeControl::MoveTime(Duration::from_secs(5))
    }
}

impl TimeControl {
    pub fn is_time_up(&self, ply: u32, nodes: u64, elapsed: &Duration) -> bool {
        let time_up = match self {
            TimeControl::Depth(max_ply) => ply > *max_ply,
            TimeControl::MoveTime(duration) => elapsed > duration,
            TimeControl::NodeCount(max_nodes) => nodes > *max_nodes,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                *elapsed > *time / 30
            }
        };
        time_up
    }
}
