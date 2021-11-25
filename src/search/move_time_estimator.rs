use crate::board::Board;
use crate::clock::Clock;
use crate::infra::component::Component;
use crate::utils::Formatting;
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
    move_overhead_ms: u64,
    min_ply_for_estimation: Ply,
    perc_of_time_adv: u32,
    moves_rem: u16,
    pub deterministic: bool,
    pub nodestime: u64,

    #[serde(skip)]
    pub estimate_move_time: Duration,

    #[serde(skip)]
    pub elapsed_search: Duration,

    #[serde(skip)]
    pub elapsed_iter: Duration,

    #[serde(skip)]
    pub time_control: TimeControl,

    #[serde(skip)]
    pondering: Arc<AtomicBool>,

    #[serde(skip)]
    board: Board,

    check_every: u64,

    #[serde(skip)]
    clock_checks: u64,
}

impl Component for MoveTimeEstimator {

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_iter(&mut self) {
        self.clock_checks = 0;
    }


    fn new_position(&mut self) {
        self.estimate_move_time = Duration::default();
        self.elapsed_search = Duration::default();
        self.elapsed_iter = Duration::default();
        self.time_control = TimeControl::default();
        self.pondering = Arc::new(AtomicBool::from(false));
        self.board = Board::default();
        self.clock_checks = 0;
        self.estimate_move_time = Duration::default();
    }
}

impl Default for MoveTimeEstimator {
    fn default() -> Self {
        MoveTimeEstimator {
            move_overhead_ms: 20,
            min_ply_for_estimation: 3,
            branching_factor: 12.625,
            perc_of_time_adv: 62,
            moves_rem: 8,
            deterministic: false,
            nodestime: 0,

            estimate_move_time: Duration::default(),
            elapsed_search: Duration::default(),
            elapsed_iter: Duration::default(),

            time_control: TimeControl::default(),
            pondering: Arc::new(AtomicBool::from(false)),
            board: Board::default(),
            check_every: 128,
            clock_checks: 0,
        }
    }
}

impl fmt::Display for MoveTimeEstimator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "time_control     : {}", self.time_control)?;
        writeln!(f, "pondering        : {}", self.pondering())?;
        // writeln!(f, "board            : {}", self.board.to_fen())?;
        writeln!(f, "move overhead ms : {}", self.move_overhead_ms)?;
        writeln!(f, "minj ply for est : {}", self.min_ply_for_estimation)?;
        writeln!(f, "branching factor : {}", self.branching_factor)?;
        writeln!(f, "const moves rem. : {}", self.moves_rem)?;
        writeln!(f, "% of time adv    : {}", self.perc_of_time_adv)?;
        writeln!(f, "allotted for mv  : {}", Formatting::duration(self.allotted()))?;
        writeln!(f, "time estimate    : {}", Formatting::duration(self.estimate_move_time))?;
        writeln!(f, "deterministic    : {}", self.deterministic)?;
        writeln!(f, "nodestime        : {}", self.nodestime)?;
        writeln!(f, "check every      : {}", self.check_every)?;
        writeln!(f, "clock checks     : {}", self.clock_checks)?;
        writeln!(f, "elapsed search   : {}", Formatting::duration(self.elapsed_search))?;
        writeln!(f, "elapsed iter     : {}", Formatting::duration(self.elapsed_iter))?;
        Ok(())
    }
}

impl MoveTimeEstimator {
    #[inline]
    pub fn is_time_up(&mut self, _ply: Ply, clock: &Clock, force_check: bool) -> bool {
        self.clock_checks += 1;

        // only do this every 128th call to avoid expensive time computation
        if !force_check && self.clock_checks % self.check_every != 0 {
            return false;
        }


        let mut elapsed = clock.elapsed_search().0;
        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            elapsed = Duration::from_millis(clock.elapsed_search().1 / self.nodestime);
        }

        let time_up = match self.time_control {
            TimeControl::DefaultTime => false, 
            TimeControl::Depth(_max_ply) => false, // ply > max_ply,  // dont cause an abort on last iteration
            TimeControl::SearchTime(duration) => 10 * elapsed > duration * 9 && !self.pondering(),
            TimeControl::NodeCount(max_nodes) => clock.elapsed_search().1 > max_nodes - self.check_every,
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

    pub fn estimate_iteration(&mut self, _ply: Ply, clock: &Clock) {
        // debug_assert!(search_stats.depth() >= ply-1, "ensure we have enough stats");
        self.elapsed_iter = clock.elapsed_iter().0;
        self.elapsed_search = clock.elapsed_search().0;

        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            let nodes = clock.elapsed_iter().1;
            self.elapsed_search = Duration::from_millis(nodes / self.nodestime);
        }

        self.estimate_move_time =  Duration::from_millis(self.move_overhead_ms) + self.elapsed_search + self.elapsed_iter.mul_f32(self.branching_factor);
    }





    
    pub fn probable_timeout(&self, ply: Ply) -> bool {
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
                self.estimate_move_time > self.allotted() && !self.pondering.load(atomic::Ordering::SeqCst) && ply >= self.min_ply_for_estimation
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
                (time_us + time_adv * self.perc_of_time_adv / 100) / self.moves_rem as u32 + inc - Duration::from_millis(self.move_overhead_ms)
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
        assert!(search.search_stats().iteration().all_nodes() >= 146, "nodes {}", search.search_stats().iteration().all_nodes());
        assert_eq!(search.score().mate_in(), Some(2));
    }
}
