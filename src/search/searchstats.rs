use crate::clock::{Clock, DeterministicClock};
use crate::eval::score::Score;
use crate::movelist::MoveList;
use crate::types::MAX_PLY;
use std::fmt;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SearchStats {
    realtime_clock: Clock,
    deterministic_clock: DeterministicClock,

    pub completed: bool,
    pub user_cancelled: bool,
    total: NodeStats,
    plies: Vec<NodeStats>,

    pub pv: MoveList,
    pub alpha: Score,
    pub beta: Score,
    pub score: Score,
}

impl fmt::Display for SearchStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "pv               : {}", self.pv())?;
        writeln!(f, "alpha            : {}", self.alpha)?;
        writeln!(f, "beta             : {}", self.beta)?;
        writeln!(f, "score            : {}", self.score)?;
        writeln!(f, "clock (detmstic) : {}", Clock::format_duration(self.deterministic_clock.elapsed()))?;
        writeln!(f, "clock (realtime) : {}", Clock::format_duration(self.realtime_clock.elapsed()))?;
        writeln!(f, "completed        : {}", self.completed())?;
        writeln!(f, "user cancelled   : {}", self.user_cancelled)?;
        writeln!(f, "depth            : {}", self.depth())?;
        writeln!(f, "selective depth  : {}", self.selective_depth())?;
        writeln!(f, "tot nodes/sec (k): {}", self.total_knps())?;
        writeln!(f, "int nodes/sec (k): {}", self.interior_knps())?;
        writeln!(f, "branching factor : {:.02}", self.branching_factor())?;
        writeln!(f, "q branch factor  : {:.02}", self.q_branching_factor())?;
        writeln!(f)?;

        write!(f, "{:<7}", "ply")?;
        NodeStats::fmt_header(f)?;
        writeln!(f)?;
        write!(f, "{:<7}", "---")?;
        NodeStats::fmt_underline(f)?;
        writeln!(f)?;

        for (i, p) in self.plies().iter().enumerate() {
            write!(f, "{:>3}    ", i)?;
            p.fmt_data(f)?;
            writeln!(f)?;
        }
        write!(f, "{:<7}", "---")?;
        NodeStats::fmt_underline(f)?;
        writeln!(f)?;
        write!(f, "{:<7}", "tot")?;
        self.total().fmt_data(f)?;
        writeln!(f)?;
        Ok(())
    }
}

impl Default for SearchStats {
    fn default() -> Self {
        Self {
            realtime_clock: Clock::default(),
            deterministic_clock: DeterministicClock::default(),

            completed: false,
            user_cancelled: false,
            total: NodeStats::default(),
            plies: std::iter::repeat(NodeStats::new()).take(MAX_PLY).collect(),

            pv: MoveList::default(),
            alpha: Score::default(),
            beta: Score::default(),
            score: Score::default(),
        }
    }
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn pv(&self) -> &MoveList {
        &self.pv
    }

    pub fn restart_clocks(&mut self) {
        self.realtime_clock.restart();
        self.deterministic_clock.restart();
    }

    pub fn elapsed(&self, deterministic: bool) -> Duration {
        if deterministic {
            self.deterministic_clock.elapsed()
        } else {
            self.realtime_clock.elapsed()
        }
    }

    pub fn reset_keeping_pv(&mut self) {
        self.plies_mut().iter_mut().for_each(|s| s.clear_node_stats());
        self.total.clear_node_stats();
        self.alpha = Score::default();
        self.beta = Score::default();
        self.score = Score::default();
        self.completed = false;
        self.user_cancelled = false;
        self.restart_clocks();
    }

    #[inline]
    pub fn total(&self) -> &NodeStats {
        &self.total
    }

    #[inline]
    pub fn plies(&self) -> &[NodeStats] {
        &self.plies[0..self.depth() as usize]
    }

    #[inline]
    pub fn plies_mut(&mut self) -> &mut [NodeStats] {
        let d = self.depth() as usize;
        &mut self.plies[0..d]
    }

    #[inline]
    pub fn depth(&self) -> u32 {
        if let Some(d) = self.plies.iter().rposition(|stats| stats.leaf_nodes() + stats.interior_nodes() != 0) {
            return 1 + d as u32; // 1 off the end for all "size" types
        }
        0
    }

    #[inline]
    pub fn selective_depth(&self) -> u32 {
        if let Some(d) = self.plies.iter().rposition(|stats| stats.nodes() != 0) {
            return 1 + d as u32; // 1 off the end for all "size" types
        }
        0
    }

    #[inline]
    pub fn record_time_estimate(&mut self, ply: u32, estimate: &Duration) {
        self.plies[ply as usize].est_time = *estimate;
    }

    pub fn record_time_actual_and_completion_status(&mut self, ply: u32, completed: bool, pv: MoveList) {
        let ply = ply as usize;
        self.plies[ply].real_time = self.realtime_clock.elapsed();
        self.plies[ply].deterministic_time = self.deterministic_clock.elapsed();
        self.completed = completed;
        if completed {
            self.pv = pv;
        }
    }

    #[inline]
    pub fn inc_custom_stat(&mut self, ply: u32) {
        self.total.custom += 1;
        self.plies[ply as usize].custom += 1;
    }

    #[inline]
    pub fn inc_leaf_nodes(&mut self, ply: u32) {
        self.total.leaf_nodes += 1;
        self.plies[ply as usize].leaf_nodes += 1;
    }

    #[inline]
    pub fn inc_interior_nodes(&mut self, ply: u32) {
        self.total.interior_nodes += 1;
        self.plies[ply as usize].interior_nodes += 1;
    }

    #[inline]
    pub fn inc_q_leaf_nodes(&mut self, sel_ply: u32) {
        self.total.q_leaf_nodes += 1;
        self.plies[sel_ply as usize].q_leaf_nodes += 1;
    }

    #[inline]
    pub fn inc_q_interior_nodes(&mut self, sel_ply: u32) {
        self.total.q_interior_nodes += 1;
        self.plies[sel_ply as usize].q_interior_nodes += 1;
    }

    // #[inline]
    // pub fn inc_nodes(&mut self, ply: u32) {
    //     self.total.interior_nodes += 1;
    //     self.plies[ply as usize].interior_nodes +=1;
    // }

    #[inline]
    pub fn inc_cuts(&mut self, ply: u32) {
        self.total.cuts += 1;
        self.plies[ply as usize].cuts += 1;
    }

    #[inline]
    pub fn inc_improvements(&mut self, ply: u32) {
        self.total.improvements += 1;
        self.plies[ply as usize].improvements += 1;
    }

    #[inline]
    pub fn stats_mut(&mut self, ply: u32) -> &mut NodeStats {
        &mut self.plies[ply as usize]
    }

    #[inline]
    pub fn stats(&self, ply: u32) -> &NodeStats {
        &self.plies[ply as usize]
    }

    #[inline]
    pub fn total_knps(&self) -> u128 {
        self.total.nodes() as u128 / (1 + self.realtime_clock.elapsed().as_millis())
    }

    #[inline]
    pub fn interior_knps(&self) -> u128 {
        self.total.interior_nodes() as u128 / (1 + self.realtime_clock.elapsed().as_millis())
    }

    #[inline]
    pub fn branching_factor(&self) -> f64 {
        let t = self.total();
        (t.leaf_nodes() + t.q_leaf_nodes()) as f64 / (t.interior_nodes() + t.q_interior_nodes() + 1) as f64
    }

    #[inline]
    pub fn q_branching_factor(&self) -> f64 {
        let t = self.total();
        (t.q_leaf_nodes as f64) / (t.q_interior_nodes() + 1) as f64
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct NodeStats {
    // nodes
    pub interior_nodes: u64,
    pub leaf_nodes: u64, // FIXME and terminal
    pub improvements: u64,
    pub cuts: u64,

    pub q_interior_nodes: u64,
    pub q_leaf_nodes: u64, // FIXME and terminal

    pub est_time: Duration,
    pub real_time: Duration,
    pub deterministic_time: Duration,

    pub custom: u64,
}

impl NodeStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_node_stats(&mut self) {
        self.interior_nodes = 0;
        self.leaf_nodes = 0;
        self.improvements = 0;
        self.cuts = 0;
        self.q_interior_nodes = 0;
        self.q_leaf_nodes = 0;
    }

    #[inline]
    pub fn interior_nodes(&self) -> u64 {
        self.interior_nodes
    }

    #[inline]
    pub fn nodes(&self) -> u64 {
        self.interior_nodes() + self.leaf_nodes() + self.q_interior_nodes() + self.q_leaf_nodes()
        // root
    }

    #[inline]
    pub fn leaf_nodes(&self) -> u64 {
        self.leaf_nodes
    }

    #[inline]
    pub fn custom(&self) -> u64 {
        self.custom
    }

    #[inline]
    pub fn q_interior_nodes(&self) -> u64 {
        self.q_interior_nodes
    }

    #[inline]
    pub fn q_nodes(&self) -> u64 {
        self.q_interior_nodes() + self.q_leaf_nodes() // root
    }

    #[inline]
    pub fn q_leaf_nodes(&self) -> u64 {
        self.q_leaf_nodes
    }

    pub fn cut_percentage(&self) -> u64 {
        self.cuts * 100 / (1 + self.nodes())
    }
}

macro_rules! header_format {
    () => {
        concat!(
            "{node:>11} {interior:>11} {leaf:>11} ",
            "{cut:>11} {improv:>11} {cut_perc:>6} ",
            "{qnode:>11} {qinterior:>11} {qleaf:>11} ",
            "{est_time:>11} {real_time:>11} {deterministic_time:>11}"
        )
    };
}

impl NodeStats {
    pub fn fmt_header(f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            header_format!(),
            cut = "cuts",
            improv = "improv",
            node = "total nodes",
            interior = "interior",
            leaf = "leaf nodes",
            qnode = "q total",
            qinterior = "q interior",
            qleaf = "q leaf",
            cut_perc = "cuts %",
            est_time = "est_time",
            real_time = "real_time",
            deterministic_time = "determstic",
        )
    }

    pub fn fmt_underline(f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            header_format!(),
            cut = "-----------",
            improv = "-----------",
            node = "-----------",
            interior = "-----------",
            leaf = "-----------",
            cut_perc = "------",
            qnode = "-----------",
            qinterior = "-----------",
            qleaf = "-----------",
            est_time = "-----------",
            real_time = "-----------",
            deterministic_time = "-----------",
        )
    }

    pub fn fmt_data(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            header_format!(),
            cut = self.cuts,
            improv = self.improvements,
            node = self.nodes(),
            interior = self.interior_nodes,
            leaf = self.leaf_nodes(),
            cut_perc = self.cut_percentage(),
            qnode = self.q_nodes(),
            qinterior = self.q_interior_nodes,
            qleaf = self.q_leaf_nodes(),
            est_time = Clock::format_duration(self.est_time),
            real_time = Clock::format_duration(self.real_time),
            deterministic_time = Clock::format_duration(self.deterministic_time),
        )
    }
}

impl fmt::Display for NodeStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Self::fmt_header(f)?;
        writeln!(f)?;
        self.fmt_data(f)?;
        writeln!(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_stats() {
        let ply_stats = NodeStats::new();
        println!("{}", ply_stats);
        println!("{:?}", ply_stats);
        println!("{:#?}", ply_stats);

        let mut search = SearchStats::default();
        search.inc_leaf_nodes(2);
        search.inc_leaf_nodes(2);
        search.inc_cuts(2);
        search.inc_interior_nodes(0);
        println!("{}", search);
    }
}
