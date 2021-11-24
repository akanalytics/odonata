use crate::board::Board;
use crate::infra::component::{Component};
use crate::utils::Formatting;
use crate::search::searchstats::SearchStats;
use crate::search::timecontrol::TimeControl;
use crate::types::Ply;
use std::fmt;
use std::time::Duration;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use serde::{Deserialize, Serialize};



#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MoveTimeEstimator {
    pub branching_factor: f32,
    perc_of_time_adv: u32,
    moves_rem: u16,
    pub deterministic: bool,
    pub nodestime: u64,

    #[serde(skip)]
    pub time_estimate: Duration,

    #[serde(skip)]
    pub elapsed_used: Duration,

    #[serde(skip)]
    pub time_control: TimeControl,

    #[serde(skip)]
    pondering: Arc<AtomicBool>,

    #[serde(skip)]
    board: Board,
}

impl Component for MoveTimeEstimator {

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
        self.time_estimate = Duration::default();
        self.elapsed_used = Duration::default();
        self.time_control = TimeControl::default();
        self.pondering = Arc::new(AtomicBool::from(false));
        self.board = Board::default();
    }
}

impl Default for MoveTimeEstimator {
    fn default() -> Self {
        MoveTimeEstimator {
            branching_factor: 11.3,
            perc_of_time_adv: 67,
            moves_rem: 11,
            deterministic: false,
            nodestime: 0,

            time_estimate: Duration::default(),
            elapsed_used: Duration::default(),
            time_control: TimeControl::default(),
            pondering: Arc::new(AtomicBool::from(false)),
            board: Board::default(),
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
        writeln!(f, "allotted for mv  : {}", Formatting::duration(self.allotted()))?;
        writeln!(f, "time estimate    : {}", Formatting::duration(self.time_estimate))?;
        writeln!(f, "deterministic    : {}", self.deterministic)?;
        writeln!(f, "nodestime        : {}", self.nodestime)?;
        writeln!(f, "elapsed used     : {}", Formatting::duration(self.elapsed_used))?;
        Ok(())
    }
}

impl MoveTimeEstimator {
    pub fn is_time_up(&self, _ply: Ply, search_stats: &SearchStats, _force: bool) -> bool {
        let mut elapsed = search_stats.elapsed_search();
        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            let nodes = search_stats.cumulative_nodes();
            elapsed = Duration::from_millis(nodes / self.nodestime);
        }

        let time_up = match self.time_control {
            TimeControl::DefaultTime => false, 
            TimeControl::Depth(_max_ply) => false, // ply > max_ply,  // dont cause an abort on last iteration
            TimeControl::SearchTime(duration) => 10 * elapsed > duration * 9 && !self.pondering(),
            TimeControl::NodeCount(max_nodes) => search_stats.cumulative_nodes() > max_nodes,
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

    pub fn estimate_iteration(&mut self, _ply: Ply, search_stats: &SearchStats) {
        // debug_assert!(search_stats.depth() >= ply-1, "ensure we have enough stats");
        let _forecast_depth = search_stats.depth();
        self.elapsed_used = search_stats.clock.elapsed_iteration();

        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            let nodes = search_stats.iteration().all_nodes();
            self.elapsed_used = Duration::from_millis(nodes / self.nodestime);
        }

        // self.time_estimate = self.elapsed_used * self.branching_factor as u32;
        self.time_estimate = self.elapsed_used.mul_f32(self.branching_factor);
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
            TimeControl::DefaultTime => zero,
            TimeControl::Depth(_) => zero,
            TimeControl::SearchTime(duration) => duration,
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
        let position = &Catalog::mate_in_2()[0];
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new()
            .set_timing_method(TimeControl::Depth(3))
            .set_eval(eval)
            .set_callback(Uci::uci_info)
            .clone();
        search.mte.deterministic = true;
        search.set_position(position.clone()).search();
        println!("{}", search);
        assert!(search.search_stats().iteration().all_nodes() < 117500, "nodes {}", search.search_stats().iteration().all_nodes());
        assert!(search.search_stats().iteration().all_nodes() > 300, "nodes {}", search.search_stats().iteration().all_nodes());
        assert_eq!(search.score().mate_in(), Some(2));
    }
}
