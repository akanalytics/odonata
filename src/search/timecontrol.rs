use crate::clock::DurationNewType;
use crate::types::{Color, Ply};
use std::fmt;
use std::time::Duration;

/// https://en.wikipedia.org/wiki/Time_control
///
#[derive(Copy, Clone, Debug)]
pub enum TimeControl {
    Depth(Ply),         // uci "depth"
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

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimeControl::Depth(max_ply) => write!(f, "Depth({})", max_ply)?,
            TimeControl::MoveTime(duration) => write!(f, "MoveTime({})", DurationNewType(*duration))?,
            TimeControl::NodeCount(max_nodes) => write!(f, "NodeCount({})", max_nodes)?,
            TimeControl::Infinite => write!(f, "Infinite")?,
            TimeControl::MateIn(depth) => write!(f, "MateIn({})", depth)?,
            TimeControl::RemainingTime { our_color, wtime, btime, winc: _, binc: _, movestogo: _ } => {
                let duration = our_color.chooser_wb(wtime, btime);
                write!(f, "RemainingTime({})", DurationNewType(*duration))?;
            }
        }
        Ok(())
    }
}

impl Default for TimeControl {
    fn default() -> Self {
        TimeControl::MoveTime(Duration::from_secs(5))
    }

}

impl TimeControl {
    pub fn from_remaining_time(d: Duration) -> Self {
        let zero = Duration::default();
        TimeControl::RemainingTime{ 
            our_color: Color::White, 
            wtime: d, 
            btime: d, 
            winc: zero, 
            binc: zero, 
            movestogo: 0
        }
    }
}


