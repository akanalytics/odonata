use std::fmt;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::time::Duration;

use odonata_base::clock::Clock;
use odonata_base::domain::timecontrol::RemainingTime;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::infra::utils::Formatting;
use odonata_base::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveTimeEstimator {
    branching_factor:       f32,
    move_overhead_ms:       u64,
    min_ply_for_estimation: Ply,
    perc_of_time_adv:       u32,
    moves_rem:              u16,
    deterministic:          bool,
    nodestime:              u64,
    check_every:            u64,
    use_last_2_iters:       bool,
    use_moves_to_go:        bool,

    #[serde(skip)]
    time_control: TimeControl,

    #[serde(skip)]
    fischer_increment: Option<Duration>,

    #[serde(skip)]
    estimate_move_time: Duration,

    #[serde(skip)]
    elapsed_search: Duration,

    #[serde(skip)]
    elapsed_iter: Duration,

    #[serde(skip)]
    pondering: Arc<AtomicBool>,

    #[serde(skip)]
    clock_checks: u64,

    #[serde(skip)]
    prior_elapsed_iter: Duration,
}

impl Default for MoveTimeEstimator {
    fn default() -> Self {
        MoveTimeEstimator {
            branching_factor:       5.01,
            check_every:            128,
            deterministic:          false,
            min_ply_for_estimation: 3,
            move_overhead_ms:       1,
            moves_rem:              12,
            nodestime:              0,
            perc_of_time_adv:       62,
            use_last_2_iters:       false,
            use_moves_to_go:        true,

            estimate_move_time: Duration::default(),
            elapsed_search:     Duration::default(),
            elapsed_iter:       Duration::default(),

            time_control:       TimeControl::default(),
            fischer_increment:  None,
            pondering:          Arc::new(AtomicBool::from(false)),
            clock_checks:       0,
            prior_elapsed_iter: Duration::default(),
        }
    }
}

impl Configurable for MoveTimeEstimator {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.branching_factor.set(p.get("branching_factor"))?;
        self.check_every.set(p.get("check_every"))?;
        self.deterministic.set(p.get("deterministic"))?;
        self.min_ply_for_estimation.set(p.get("min_ply_for_estimation"))?;
        self.move_overhead_ms.set(p.get("move_overhead_ms"))?;
        self.moves_rem.set(p.get("moves_rem"))?;
        self.nodestime.set(p.get("nodestime"))?;
        self.perc_of_time_adv.set(p.get("perc_of_time_adv"))?;
        self.use_last_2_iters.set(p.get("use_last_2_iters"))?;
        self.use_moves_to_go.set(p.get("use_moves_to_go"))?;
        Ok(p.is_modified())
    }
}

impl Component for MoveTimeEstimator {
    fn new_game(&mut self) {
        self.new_position();
        self.time_control = TimeControl::default();
        self.estimate_move_time = Duration::default();
        self.elapsed_search = Duration::default();
        self.elapsed_iter = Duration::default();
    }

    fn new_iter(&mut self) {
        self.clock_checks = 0;
    }

    fn new_position(&mut self) {
        self.clock_checks = 0;
        self.prior_elapsed_iter = Duration::ZERO;
    }
}

impl fmt::Display for MoveTimeEstimator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "time_control     : {}", self.time_control)?;
        writeln!(
            f,
            "fischer increment: {}",
            Formatting::duration(self.fischer_increment.unwrap_or_default())
        )?;
        writeln!(f, "pondering        : {}", self.pondering())?;
        // writeln!(f, "board            : {}", self.board.to_fen())?;
        writeln!(f, "move overhead ms : {}", self.move_overhead_ms)?;
        writeln!(f, "min ply for est  : {}", self.min_ply_for_estimation)?;
        writeln!(f, "branching factor : {}", self.branching_factor)?;
        writeln!(f, "const moves rem. : {}", self.moves_rem)?;
        writeln!(f, "% of time adv    : {}", self.perc_of_time_adv)?;
        writeln!(f, "allotted for mv  : {}", Formatting::duration(self.allotted()))?;
        writeln!(
            f,
            "est move time    : {}",
            Formatting::duration(self.estimate_move_time)
        )?;
        writeln!(f, "deterministic    : {}", self.deterministic)?;
        writeln!(f, "nodestime        : {}", self.nodestime)?;
        writeln!(f, "check every      : {}", self.check_every)?;
        writeln!(f, "clock checks     : {}", self.clock_checks)?;
        writeln!(f, "elapsed search   : {}", Formatting::duration(self.elapsed_search))?;
        writeln!(f, "elapsed iter     : {}", Formatting::duration(self.elapsed_iter))?;
        writeln!(
            f,
            "prior elap iter  : {}",
            Formatting::duration(self.prior_elapsed_iter)
        )?;
        Ok(())
    }
}

impl MoveTimeEstimator {
    pub fn set_time_control(&mut self, tc: TimeControl) {
        self.time_control = tc;
        self.fischer_increment = None;
    }

    pub fn set_nodestime(&mut self, nodestime: u64) {
        self.nodestime = nodestime;
    }

    pub fn time_control(&self) -> &TimeControl {
        &self.time_control
    }

    fn elapsed_with_margin(&self, clock: &mut Clock) -> Duration {
        let mut elapsed = clock.elapsed_search().time + Duration::from_millis(self.move_overhead_ms);
        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            elapsed = Duration::from_millis(clock.elapsed_search().nodes / self.nodestime);
        }
        elapsed
    }

    #[inline]
    pub fn is_time_up(&mut self, _ply: Ply, c: &mut Clock, force_check: bool) -> bool {
        self.clock_checks += 1;

        // if its not time sensive then always check (=> exact node counts for instance)
        // only do this every 128th call to avoid expensive time computation
        if self.time_control.is_time_sensitive() && !force_check && self.clock_checks % self.check_every != 0 {
            return false;
        }

        match self.time_control {
            TimeControl::DefaultTime => false,
            TimeControl::Depth(_max_ply) => false, // ply > max_ply,  // dont cause an abort on last iteration
            TimeControl::SearchTime(duration) => 10 * self.elapsed_with_margin(c) > duration * 9 && !self.pondering(),
            TimeControl::NodeCount(n) => c.elapsed_search().nodes >= n,
            TimeControl::DepthNodeCount(_, n) => c.elapsed_search().nodes >= n,
            TimeControl::Instructions(n) => c.elapsed_search().instructions >= n,
            TimeControl::Cycles(n) => c.elapsed_search().cycles >= n,
            TimeControl::Infinite => false,
            TimeControl::MateIn(_) => false,
            TimeControl::UciFischer { .. } => self.elapsed_with_margin(c) > self.allotted() && !self.pondering(),
            TimeControl::FischerMulti { .. } => panic!("FischerMulti"),
        }
    }

    // turning pondering off will kick in the existing time controls
    pub fn set_shared_ponder(&mut self, pondering: bool) {
        self.pondering.store(pondering, atomic::Ordering::SeqCst);
    }

    fn pondering(&self) -> bool {
        self.pondering.load(atomic::Ordering::SeqCst)
    }

    pub fn estimate_iteration(&mut self, ply: Ply, clock: &mut Clock) {
        // debug_assert!(search_stats.depth() >= ply-1, "ensure we have enough stats");
        self.prior_elapsed_iter = self.elapsed_iter;
        self.elapsed_iter = clock.elapsed_iter_this_thread().time;
        self.elapsed_search = clock.elapsed_search().time;

        if let TimeControl::UciFischer(rt) = &self.time_control {
            // on initial call, capture the fischer increment
            if self.fischer_increment.is_none() {
                self.fischer_increment = Some(Duration::max(rt.wtime, rt.btime))
            }
        }

        // if in nodestime then convert nodes to time. nodestime is nodes per millisecond
        if self.nodestime > 0 {
            let nodes = clock.elapsed_iter_this_thread().nodes;
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
        match &self.time_control {
            TimeControl::UciFischer(rt) => {
                let (_time, _inc) = rt.our_color.chooser_wb((rt.wtime, rt.winc), (rt.btime, rt.binc));
                self.estimate_move_time > self.allotted()
                    && !self.pondering.load(atomic::Ordering::SeqCst)
                    && ply >= self.min_ply_for_estimation
            }
            _ => false,
        }
    }

    fn calc_from_remaining(&self, rt: &RemainingTime) -> Duration {
        let (time_us, inc) = rt.our_time_and_inc();
        let (time_them, _inc) = rt.their_time_and_inc();
        let time_adv = if time_us > time_them {
            time_us - time_them
        } else {
            Duration::ZERO
        };
        if self.use_moves_to_go {
            let remaining = time_us + time_adv * self.perc_of_time_adv / 100;
            // let per_move_a = remaining / u32::max(u32::min(rt.moves_to_go as u32 / 2, 1), self.moves_rem as u32);
            let per_move = if rt.moves_to_go > 0 {
                remaining / u32::min(u32::max(rt.moves_to_go as u32, 2), self.moves_rem as u32)
            } else {
                remaining / self.moves_rem as u32
            };
            // info!("MTE--> rt={rt:?}, remaining = {remaining:?} per_move = {pm} moves_rem = {moves_rem}", moves_rem = self.moves_rem, pm = Formatting::duration(per_move));
            // error!(
            //     "MTE {} {} {} {} mtg {moves_to_go} pma {} pmb {}",
            //     Formatting::duration(wtime),
            //     Formatting::duration(btime),
            //     Formatting::duration(winc),
            //     Formatting::duration(binc),
            //     Formatting::duration(per_move_a),
            //     Formatting::duration(per_move_b)
            // );
            let rem = per_move + inc;
            rem.saturating_sub(Duration::from_millis(self.move_overhead_ms))
        } else {
            // warn!(
            //     "MTE {} {} {} {} mtg {moves_to_go}",
            //     Formatting::duration(wtime),
            //     Formatting::duration(btime),
            //     Formatting::duration(winc),
            //     Formatting::duration(binc),
            // );
            let rem = (time_us + time_adv * self.perc_of_time_adv / 100) / self.moves_rem as u32 + inc;
            rem.saturating_sub(Duration::from_millis(self.move_overhead_ms))
        }
    }

    fn allotted(&self) -> Duration {
        let zero = Duration::ZERO;
        match &self.time_control {
            TimeControl::DefaultTime => zero,
            TimeControl::Depth(_) => zero,
            TimeControl::SearchTime(duration) => *duration,
            TimeControl::NodeCount(_) => zero,
            TimeControl::DepthNodeCount(..) => zero,
            TimeControl::Cycles(_) => zero,
            TimeControl::Instructions(_) => zero,
            TimeControl::Infinite => zero,
            TimeControl::MateIn(_) => zero,
            TimeControl::UciFischer(rt) => self.calc_from_remaining(rt),
            TimeControl::FischerMulti { .. } => panic!("FischerMulti"),
        }
    }

    pub fn depth(&self) -> Option<Ply> {
        match &self.time_control {
            TimeControl::DepthNodeCount(d, _) => Some(*d),
            TimeControl::Depth(d) => Some(*d),
            _ => None,
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
