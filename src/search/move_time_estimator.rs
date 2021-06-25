use crate::board::Board;
use crate::clock::Clock;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::search::searchstats::SearchStats;
use crate::search::timecontrol::TimeControl;
use crate::types::Ply;
use std::fmt;
use std::time::Duration;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;


#[derive(Clone, Debug)]
pub struct MoveTimeEstimator {
    pub time_control: TimeControl,
    pondering: Arc<AtomicBool>,
    board: Board,
    pub branching_factor: u16,
    perc_of_time_adv: u32,
    moves_rem: u16,
    pub time_estimate: Duration,
    pub elapsed_used: Duration,
    pub deterministic: bool,
    pub nodestime: i64,
}

impl Component for MoveTimeEstimator {
    fn settings(&self, c: &mut Config) {
        c.set("mte.branching_factor", "type spin default 10 min 1 max 100");
        c.set("mte.moves_rem", "type spin default 20 min 1 max 100");
        c.set("mte.perc_of_time_adv", "type spin default 100 min 0 max 1000");
        c.set("nodestime", "type spin default 0 min 0 max 50000");
        c.set("mte.deterministic", "type check default false");
    }
    fn configure(&mut self, c: &Config) {
        debug!("mte.configure with {}", c);
        self.branching_factor = c
            .int("mte.branching_factor")
            .unwrap_or(self.branching_factor as i64) as u16;
        self.moves_rem = c.int("mte.moves_rem").unwrap_or(self.moves_rem as i64) as u16;
        self.perc_of_time_adv = c
            .int("mte.perc_of_time_adv")
            .unwrap_or(self.perc_of_time_adv as i64) as u32;
        self.deterministic = c.bool("mte.deterministic").unwrap_or(self.deterministic);
        self.nodestime = c.int("nodestime").unwrap_or(self.nodestime);
    }

    fn new_game(&mut self) {
    }

    fn new_search(&mut self) {
    }
}

impl Default for MoveTimeEstimator {
    fn default() -> Self {
        MoveTimeEstimator {
            branching_factor: 9,
            perc_of_time_adv: 100,
            moves_rem: 20,
            board: Board::default(),
            time_control: TimeControl::default(),
            pondering: Arc::new(AtomicBool::from(false)),
            time_estimate: Duration::default(),
            elapsed_used: Duration::default(),
            deterministic: false,
            nodestime: 0,
        }
    }
}

impl fmt::Display for MoveTimeEstimator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "time_control     : {}", self.time_control)?;
        writeln!(f, "pondering        : {}", self.pondering())?;
        // writeln!(f, "board            : {}", self.board.to_fen())?;
        writeln!(f, "branching factor : {}", self.branching_factor)?;
        writeln!(f, "const moves rem. : {}", self.moves_rem)?;
        writeln!(f, "% of time adv    : {}", self.perc_of_time_adv)?;
        writeln!(f, "allotted for mv  : {}", Clock::format(self.allotted()))?;
        writeln!(f, "time estimate    : {}", Clock::format(self.time_estimate))?;
        writeln!(f, "deterministic    : {}", self.deterministic)?;
        writeln!(f, "nodestime        : {}", self.nodestime)?;
        writeln!(f, "elapsed used     : {}", Clock::format(self.elapsed_used))?;
        Ok(())
    }
}

impl MoveTimeEstimator {
    pub fn is_time_up(&self, _ply: Ply, search_stats: &SearchStats) -> bool {
        let elapsed = search_stats.elapsed(self.deterministic);

        let time_up = match self.time_control {
            TimeControl::Depth(_max_ply) => false, // ply > max_ply,  // dont cause an abort on last iteration
            TimeControl::MoveTime(duration) => 10 * elapsed > duration * 9 && !self.pondering(),
            TimeControl::NodeCount(max_nodes) => search_stats.total().nodes() > max_nodes,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::RemainingTime { .. } => elapsed > self.allotted() && !self.pondering(),
        };
        time_up
    }

    // turning pondering off will kick in the existing time controls
    pub fn set_shared_ponder(&mut self, pondering: bool) {
        self.pondering.store(pondering, atomic::Ordering::SeqCst);
    }

    pub fn pondering(&self) -> bool {
        self.pondering.load(atomic::Ordering::SeqCst)
    }

    pub fn estimate_ply(&mut self, _ply: Ply, search_stats: &SearchStats) {
        // debug_assert!(search_stats.depth() >= ply-1, "ensure we have enough stats");
        let _forecast_depth = search_stats.depth();
        self.elapsed_used = search_stats.elapsed(self.deterministic);
        self.time_estimate = self.elapsed_used * self.branching_factor as u32;
    }

    pub fn probable_timeout(&self, _search_stats: &SearchStats) -> bool {
        match self.time_control {
            TimeControl::RemainingTime {
                our_color,
                wtime,
                btime,
                winc,
                binc,
                movestogo: _,
            } => {
                let (_time, _inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                self.time_estimate > self.allotted() && !self.pondering.load(atomic::Ordering::SeqCst)
            }
            _ => false,
        }
    }

    fn allotted(&self) -> Duration {
        let zero = Duration::from_secs(0);
        match self.time_control {
            TimeControl::Depth(_) => zero,
            TimeControl::MoveTime(duration) => duration,
            TimeControl::NodeCount(_) => zero,
            TimeControl::Infinite => zero,
            TimeControl::MateIn(_) => zero,
            TimeControl::RemainingTime {
                our_color,
                wtime,
                btime,
                winc,
                binc,
                movestogo: _,
            } => {
                let (time_us, inc) = our_color.chooser_wb((wtime, winc), (btime, binc));
                let (time_them, _inc) = our_color.opposite().chooser_wb((wtime, winc), (btime, binc));
                let time_adv = if time_us > time_them {
                    time_us - time_them
                } else {
                    Duration::default()
                };
                (time_us + time_adv * self.perc_of_time_adv / 100) / self.moves_rem as u32 + inc
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::*;
    use crate::eval::eval::*;
    use crate::search::algo::*;

    #[test]
    fn test_display_mte() {
        let mte = MoveTimeEstimator::default();
        println!("{}", mte);
        println!("{:?}", mte);
        println!("{:#?}", mte);
    }

    #[test]
    fn test_mate_with_tc() {
        let position = Catalog::mate_in_2()[0].clone();
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new()
            .set_timing_method(TimeControl::from_remaining_time(Duration::from_secs(20)))
            .set_eval(eval)
            .set_callback(Uci::uci_info)
            .clone();
        search.mte.deterministic = true;
        search.search(position.board());
        println!("{}", search);
        assert!(search.search_stats().total().nodes() < 5200, "nodes {}", search.search_stats().total().nodes());
        assert!(search.search_stats().total().nodes() > 4300, "nodes {}", search.search_stats().total().nodes());
        assert_eq!(search.score().mate_in(), Some(2));
    }
}
