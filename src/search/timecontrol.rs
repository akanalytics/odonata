use crate::board::Board;
use crate::search::clock::DurationNewType;
use crate::search::stats::SearchStats;
use crate::types::Color;
use std::fmt;
use std::time::Duration;
use crate::search::clock::Clock;
use crate::config::{Config, Configurable};
use crate::log_debug;

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
}

#[derive(Copy, Clone, Debug)]
pub struct MoveTimeEstimator {
    pub time_control: TimeControl,
    board: Board,
    branching_factor: u16,
    moves_rem: u16,
    pub allotted: Duration,
    pub estimate_for_ply: u32,
    pub time_estimate: Duration,
}


impl Configurable for MoveTimeEstimator {
    fn define(&self, c: &mut Config) {
        c.set("mte.branching_factor", "type spin default 15 min 1 max 100");
        c.set("mte.moves_rem", "type spin default 20 min 1 max 100");
    }
    
    fn configure(&mut self, c: &Config) {
        log_debug!("mte.configure with {}", c);
        self.branching_factor = c.int("mte.branching_factor").unwrap_or(self.branching_factor as i64) as u16;
        self.moves_rem = c.int("mte.moves_rem").unwrap_or(self.moves_rem as i64) as u16;
    }
}

impl Default for MoveTimeEstimator {
    fn default() -> Self {
        MoveTimeEstimator {
            branching_factor: 15,
            moves_rem: 20,
            board: Board::default(),
            time_control: TimeControl::default(),
            allotted: Duration::default(),
            estimate_for_ply: 0,
            time_estimate: Duration::default(),
        }
    }
}

impl fmt::Display for MoveTimeEstimator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "time_control     : {}", self.time_control)?;
        writeln!(f, "board            : {}", self.board.to_fen())?;
        writeln!(f, "branching factor : {}", self.branching_factor)?;
        writeln!(f, "const moves rem. : {}", self.moves_rem)?;
        writeln!(f, "alloted for move : {}", Clock::format_duration(self.allotted))?;
        writeln!(f, "estimate for ply : {}", self.estimate_for_ply)?;
        writeln!(f, "time estimate    : {}", Clock::format_duration(self.time_estimate))?;
        Ok(())
    }
}

impl MoveTimeEstimator {
    pub fn is_time_up(&self, ply: u32, nodes: u64, elapsed: &Duration) -> bool {
        let time_up = match self.time_control {
            TimeControl::Depth(max_ply) => ply > max_ply,
            TimeControl::MoveTime(duration) => *elapsed > duration,
            TimeControl::NodeCount(max_nodes) => nodes > max_nodes,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                *elapsed > time / self.moves_rem as u32
            }
        };
        time_up
    }

    pub fn calculate_etimates_for_ply(&mut self, _ply: u32, search_stats: &SearchStats) {
        // debug_assert!(search_stats.depth() >= ply-1, "ensure we have enough stats");
        let _forecast_depth = search_stats.depth();
        self.time_estimate = search_stats.clock.elapsed() * self.branching_factor as u32;
        if let TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } = self.time_control {
            let (time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
            self.allotted = time / self.moves_rem as u32;
        }
    }

    pub fn probable_timeout(&self, _search_stats: &SearchStats) -> bool {
        match self.time_control {
            TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (_time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                self.time_estimate > self.allotted
            },
            _ => false,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_mte() {
        let mte = MoveTimeEstimator::default();
        println!("{}", mte);
        println!("{:?}", mte);
        println!("{:#?}", mte);
    }
}