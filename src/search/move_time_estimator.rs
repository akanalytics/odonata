use crate::board::Board;
use crate::clock::Clock;
use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::search::timecontrol::TimeControl;
use crate::piece::Ply;
use crate::utils::Formatting;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::time::Duration;

use super::node::Event;

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
    check_every: u64,
    use_last_2_iters: bool,

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

    #[serde(skip)]
    clock_checks: u64,

    #[serde(skip)]
    prior_elapsed_iter: Duration,
}

impl Component for MoveTimeEstimator {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_iter(&mut self) {
        self.clock_checks = 0;
    }

    fn new_position(&mut self) {
        *self = MoveTimeEstimator {
            move_overhead_ms: self.move_overhead_ms,
            min_ply_for_estimation: self.min_ply_for_estimation,
            branching_factor: self.branching_factor,
            perc_of_time_adv: self.perc_of_time_adv,
            moves_rem: self.moves_rem,
            deterministic: self.deterministic,
            nodestime: self.nodestime,
            use_last_2_iters: self.use_last_2_iters,
            time_control: self.time_control,
            ..Default::default()
        };
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
            use_last_2_iters: true,

            estimate_move_time: Duration::default(),
            elapsed_search: Duration::default(),
            elapsed_iter: Duration::default(),

            time_control: TimeControl::default(),
            pondering: Arc::new(AtomicBool::from(false)),
            board: Board::default(),
            check_every: 128,
            clock_checks: 0,
            prior_elapsed_iter: Duration::default(),
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
        writeln!(
            f,
            "allotted for mv  : {}",
            Formatting::duration(self.allotted())
        )?;
        writeln!(
            f,
            "time estimate    : {}",
            Formatting::duration(self.estimate_move_time)
        )?;
        writeln!(f, "deterministic    : {}", self.deterministic)?;
        writeln!(f, "nodestime        : {}", self.nodestime)?;
        writeln!(f, "check every      : {}", self.check_every)?;
        writeln!(f, "clock checks     : {}", self.clock_checks)?;
        writeln!(
            f,
            "elapsed search   : {}",
            Formatting::duration(self.elapsed_search)
        )?;
        writeln!(
            f,
            "elapsed iter     : {}",
            Formatting::duration(self.elapsed_iter)
        )?;
        writeln!(
            f,
            "prior elap iter  : {}",
            Formatting::duration(self.prior_elapsed_iter)
        )?;
        Ok(())
    }
}

impl MoveTimeEstimator {
    #[inline]
    pub fn is_time_up(&mut self, _ply: Ply, clock: &Clock, force_check: bool) -> bool {
        self.clock_checks += 1;

        // only do this every 128th call to avoid expensive time computation
        if self.time_sensitive() && !force_check && self.clock_checks % self.check_every != 0 {
            return false;
        }

        let mut elapsed = clock.elapsed_search().0;
        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            elapsed = Duration::from_millis(clock.elapsed_search().1 / self.nodestime);
        }

        match self.time_control {
            TimeControl::DefaultTime => false,
            TimeControl::Depth(_max_ply) => false, // ply > max_ply,  // dont cause an abort on last iteration
            TimeControl::SearchTime(duration) => 10 * elapsed > duration * 9 && !self.pondering(),
            TimeControl::NodeCount(max_nodes) => clock.elapsed_search().1 >= max_nodes,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::RemainingTime { .. } => elapsed > self.allotted() && !self.pondering(),
        }
    }

    // turning pondering off will kick in the existing time controls
    pub fn set_shared_ponder(&mut self, pondering: bool) {
        self.pondering.store(pondering, atomic::Ordering::SeqCst);
    }

    pub fn pondering(&self) -> bool {
        self.pondering.load(atomic::Ordering::SeqCst)
    }

    /// For some time controls we aren't worried about node counts or search times, so we
    /// can avoid optimizations elsewhere
    pub fn time_sensitive(&self) -> bool {
        match self.time_control {
            TimeControl::DefaultTime => true,
            TimeControl::Depth(_max_ply) => false,
            TimeControl::SearchTime(_duration) => true,
            TimeControl::NodeCount(_max_nodes) => false,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::RemainingTime { .. } => true,
        }
    }

    pub fn estimate_iteration(&mut self, ply: Ply, clock: &Clock) {
        // debug_assert!(search_stats.depth() >= ply-1, "ensure we have enough stats");
        self.prior_elapsed_iter = self.elapsed_iter;
        self.elapsed_iter = clock.elapsed_iter().0;
        self.elapsed_search = clock.elapsed_search().0;

        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            let nodes = clock.elapsed_iter().1;
            self.elapsed_search = Duration::from_millis(nodes / self.nodestime);
        }

        if self.use_last_2_iters {
            self.estimate_move_time = Duration::from_millis(self.move_overhead_ms)
                + self.elapsed_search
                + self.elapsed_iter.mul_f32(self.branching_factor) / 2
                + self
                    .prior_elapsed_iter
                    .mul_f32(self.branching_factor)
                    .mul_f32(self.branching_factor)
                    / 2;
        } else {
            self.estimate_move_time = Duration::from_millis(self.move_overhead_ms)
                + self.elapsed_search
                + self.elapsed_iter.mul_f32(self.branching_factor);
        }
        Metrics::elapsed(ply, self.estimate_move_time, Event::DurationIterEst);
        Metrics::elapsed(ply, self.allotted(), Event::DurationIterAllotted);
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
                self.estimate_move_time > self.allotted()
                    && !self.pondering.load(atomic::Ordering::SeqCst)
                    && ply >= self.min_ply_for_estimation
            }
            _ => false,
        }
    }

    fn allotted(&self) -> Duration {
        let zero = Duration::ZERO;
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
                let (time_them, _inc) = our_color
                    .opposite()
                    .chooser_wb((wtime, winc), (btime, binc));
                let time_adv = if time_us > time_them {
                    time_us - time_them
                } else {
                    Duration::default()
                };
                (time_us + time_adv * self.perc_of_time_adv / 100) / self.moves_rem as u32 + inc
                    - Duration::from_millis(self.move_overhead_ms)
            }
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
