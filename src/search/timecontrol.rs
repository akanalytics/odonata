use crate::board::Board;
use crate::search::clock::DurationNewType;
use crate::search::stats::SearchStats;
use crate::types::Color;
use std::fmt;
use std::time::Duration;
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
    pub fn is_time_up(&self, ply: u32, nodes: u64, elapsed: &Duration) -> bool {
        let time_up = match self {
            TimeControl::Depth(max_ply) => ply > *max_ply,
            TimeControl::MoveTime(duration) => elapsed > duration,
            TimeControl::NodeCount(max_nodes) => nodes > *max_nodes,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::RemainingTime { our_color, wtime, btime, winc, binc, movestogo: _ } => {
                let (time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                *elapsed > *time / 20
            }
        };
        time_up
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MoveTimeEstimator {
    pub time_control: TimeControl,
    board: Board,
    percentage: u16, // 80% means estimate 80% of the time (optimistic)
    branching_factor: u16,
    moves_rem: u16,
}


impl Configurable for MoveTimeEstimator {
    fn define(&self, c: &mut Config) {
        c.set("mte.est_percentage_of_actual", "type spin default 80 min 10 max 300");
        c.set("mte.branching_factor", "type spin default 15 min 1 max 100");
        c.set("mte.moves_rem", "type spin default 20 min 1 max 100");
    }
    
    fn configure(&mut self, c: &Config) {
        log_debug!("mte.configure with {}", c);
        self.percentage = c.int("mte.est_percentage_of_actual").unwrap_or(self.percentage as i64) as u16;
        self.branching_factor = c.int("mte.branching_factor").unwrap_or(self.branching_factor as i64) as u16;
        self.moves_rem = c.int("mte.moves_rem").unwrap_or(self.moves_rem as i64) as u16;
    }
}

impl Default for MoveTimeEstimator {
    fn default() -> Self {
        MoveTimeEstimator {
            branching_factor: 15,
            percentage: 120,
            moves_rem: 20,
            board: Board::default(),
            time_control: TimeControl::default(),
        }
    }
}

impl fmt::Display for MoveTimeEstimator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "time_control     : {}", self.time_control)?;
        writeln!(f, "board            : {}", self.board.to_fen())?;
        writeln!(f, "percentage       : {}", self.percentage)?;
        writeln!(f, "branching factor : {}", self.branching_factor)?;
        writeln!(f, "const moves rem. : {}", self.moves_rem)?;
        Ok(())
    }
}

impl MoveTimeEstimator {

    pub fn estimate_time_up_next_ply(&self, search_stats: &SearchStats) -> bool {
        // we guess what the situation will look like at the end of next play
        //FIXME  depth is wrong!!!!!
        let forecast_depth = search_stats.depth();
        let forecast_nodes =
        search_stats.total().nodes() * self.branching_factor as u64 * self.percentage as u64 / 100;
        let forecast_elapsed = search_stats.clock.elapsed() * self.branching_factor as u32 * self.percentage as u32 / 100;
        self.time_control.is_time_up(forecast_depth, forecast_nodes, &forecast_elapsed)
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