use crate::clock::{Clock, DeterministicClock};
use crate::eval::score::Score;
use crate::variation::Variation;
use crate::types::{Ply, MAX_PLY, MoveType};
use std::fmt;
use std::cmp;
 use strum::EnumCount;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SearchStats {

    pub depth: Ply,
    realtime: Clock,
    deterministic: DeterministicClock,

    pub completed: bool,
    pub user_cancelled: bool,
    cumulative: NodeStats,
    total: NodeStats,
    plies: Vec<NodeStats>,

    pub pv: Variation,
    pub score: Score,
}

#[rustfmt::skip]
impl fmt::Display for SearchStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "depth            : {}", self.depth)?;
        writeln!(f, "pv               : {}", self.pv())?;
        writeln!(f, "score            : {}", self.score)?;
        writeln!(f, "clock (detmstic) : {}", Clock::format(self.deterministic.elapsed()))?;
        writeln!(f, "clock (realtime) : {}", Clock::format(self.realtime.elapsed()))?;
        writeln!(f, "completed        : {}", self.completed())?;
        writeln!(f, "user cancelled   : {}", self.user_cancelled)?;
        writeln!(f, "calc depth       : {}", self.depth())?;
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
            depth: 0,
            realtime: Clock::default(),
            deterministic: DeterministicClock::default(),
            completed: false,
            user_cancelled: false,
            total: NodeStats::default(),
            cumulative: NodeStats::default(),
            plies: std::iter::repeat(NodeStats::new())
                .take(MAX_PLY as usize)
                .collect(),
            pv: Variation::default(),
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

    pub fn pv(&self) -> &Variation {
        &self.pv
    }

    pub fn restart_clocks(&mut self) {
        self.realtime.restart();
        self.deterministic.restart();
    }

    pub fn elapsed(&self, deterministic: bool) -> Duration {
        if deterministic {
            self.deterministic.elapsed()
        } else {
            self.realtime.elapsed()
        }
    }

    pub fn reset_keeping_pv(&mut self) {
        self.plies_mut().iter_mut().for_each(|s| s.clear_node_stats());
        self.total.clear_node_stats();
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
    pub fn cumulative(&self) -> &NodeStats {
        &self.cumulative
    }

    #[inline]
    pub fn plies(&self) -> &[NodeStats] {
        &self.plies[0..self.len()]
    }

    #[inline]
    pub fn plies_mut(&mut self) -> &mut [NodeStats] {
        let len = self.len();
        &mut self.plies[0..len]
    }

    #[inline]
    pub fn depth(&self) -> Ply {
        self.depth
    }

    pub fn len(&self) -> usize {
        if let Some(d) = self
            .plies
            .iter()
            .rposition(|stats| stats.nodes() + stats.q_tt_nodes + stats.tt_nodes != 0)
        {
            return 1 + d; // a usize is one-off-the-end
        }
        0
    }

    #[inline]
    pub fn selective_depth(&self) -> Ply {
        cmp::max(self.len(), 1) as Ply - 1
    }

    #[inline]
    pub fn record_time_estimate(&mut self, ply: Ply, estimate: &Duration) {
        self.plies[ply as usize].est_time = *estimate;
    }

    pub fn record_iteration(&mut self, ply: Ply, completed: bool, pv: Variation) {
        let ply = ply as usize;
        self.plies[ply].real_time = self.realtime.elapsed();
        self.plies[ply].deterministic_time = self.deterministic.elapsed();
        self.total.real_time += self.realtime.elapsed();
        self.total.deterministic_time += self.deterministic.elapsed();
        self.completed = completed;
        if completed {
            self.pv = pv;
        }
        self.cumulative.accumulate(&self.total);
    }


    #[inline]
    pub fn inc_leaf_nodes(&mut self, ply: Ply) {
        self.total.leaf_nodes += 1;
        self.plies[ply as usize].leaf_nodes += 1;
    }

    #[inline]
    pub fn inc_interior_nodes(&mut self, ply: Ply) {
        self.total.interior_nodes += 1;
        self.plies[ply as usize].interior_nodes += 1;
    }

    #[inline]
    pub fn inc_q_leaf_nodes(&mut self, sel_ply: Ply) {
        self.total.q_leaf_nodes += 1;
        self.plies[sel_ply as usize].q_leaf_nodes += 1;
    }

    #[inline]
    pub fn inc_q_interior_nodes(&mut self, sel_ply: Ply) {
        self.total.q_interior_nodes += 1;
        self.plies[sel_ply as usize].q_interior_nodes += 1;
    }

    #[inline]
    pub fn inc_tt_nodes(&mut self, sel_ply: Ply) {
        self.total.tt_nodes += 1;
        self.plies[sel_ply as usize].tt_nodes += 1;
    }

    #[inline]
    pub fn inc_q_tt_nodes(&mut self, sel_ply: Ply) {
        self.total.q_tt_nodes += 1;
        self.plies[sel_ply as usize].q_tt_nodes += 1;
    }

    // #[inline]
    // pub fn inc_nodes(&mut self, ply: u32) {
    //     self.total.interior_nodes += 1;
    //     self.plies[ply as usize].interior_nodes +=1;
    // }

    #[inline]
    pub fn inc_node_cut(&mut self, ply: Ply, move_type: MoveType) {
        self.plies[ply as usize].node_cut += 1;
        self.plies[ply as usize].cut_on_move[move_type as usize] += 1;

    }

    #[inline]
    pub fn inc_node_pv(&mut self, ply: Ply) {
        self.plies[ply as usize].node_pv += 1;
    }

    #[inline]
    pub fn inc_node_all(&mut self, ply: Ply) {
        self.plies[ply as usize].node_all += 1;
    }

    #[inline]
    pub fn inc_nmp(&mut self, ply: Ply) {
        self.plies[ply as usize].nmp += 1;
    }

    #[inline]
    pub fn inc_fp(&mut self, ply: Ply) {
        self.plies[ply as usize].fp += 1;
    }

    #[inline]
    pub fn inc_pvs(&mut self, ply: Ply) {
        self.plies[ply as usize].pvs += 1;
    }

    #[inline]
    pub fn inc_pvs_research(&mut self, ply: Ply) {
        self.plies[ply as usize].pvs_research += 1;
    }

    #[inline]
    pub fn stats_mut(&mut self, ply: Ply) -> &mut NodeStats {
        &mut self.plies[ply as usize]
    }

    #[inline]
    pub fn stats(&self, ply: u32) -> &NodeStats {
        &self.plies[ply as usize]
    }

    #[inline]
    pub fn total_knps(&self) -> u128 {
        self.total.nodes() as u128 / (1 + self.realtime.elapsed().as_millis())
    }

    #[inline]
    pub fn cumulative_knps(&self) -> u128 {
        self.cumulative.nodes() as u128 / (1 + self.cumulative.real_time.as_millis())
    }

    #[inline]
    pub fn total_time(&self) -> Duration {
        self.total.real_time
    }

    #[inline]
    pub fn interior_knps(&self) -> u128 {
        self.total.interior_nodes() as u128 / (1 + self.realtime.elapsed().as_millis())
    }

    #[inline]
    pub fn branching_factor(&self) -> f64 {
        let t = self.total();
        (t.leaf_nodes + t.q_leaf_nodes) as f64 / (t.interior_nodes + t.q_interior_nodes + 1) as f64
    }

    #[inline]
    pub fn q_branching_factor(&self) -> f64 {
        let t = self.total();
        (t.q_leaf_nodes as f64) / (t.q_interior_nodes + 1) as f64
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct NodeStats {
    // nodes
    pub interior_nodes: u64,
    pub leaf_nodes: u64, // FIXME and terminal

    pub q_interior_nodes: u64,
    pub q_leaf_nodes: u64, // FIXME and terminal

    pub tt_nodes: u64,
    pub tt_hints: u64,
    pub q_tt_nodes: u64,

    pub node_all: u64,
    pub node_pv: u64,
    pub node_cut: u64,

    pub pvs: u64,
    pub pvs_research: u64,
    pub nmp: u64,
    pub fp: u64,
    pub cut_on_move: [u64; MoveType::COUNT],

    pub est_time: Duration,
    pub real_time: Duration,
    pub deterministic_time: Duration,

}




impl NodeStats {
    pub fn new() -> Self {
        Self::default()
    }

    // FIXME! assigne from default
    pub fn clear_node_stats(&mut self) {
        self.interior_nodes = 0;
        self.leaf_nodes = 0;
        
        self.node_pv = 0;
        self.node_all = 0;
        self.node_cut = 0;

        self.pvs = 0;
        self.pvs_research = 0;
        self.nmp = 0;
        self.fp = 0;
        self.cut_on_move = [0; MoveType::COUNT];
        
        self.q_interior_nodes = 0;
        self.q_leaf_nodes = 0;
        
        self.tt_nodes = 0;
        self.tt_hints = 0;
        self.q_tt_nodes = 0;
        
        self.real_time = Duration::default();
        self.deterministic_time = Duration::default();


    }

    fn cut_move_perc(&self, mt: MoveType) -> u64 {
        self.cut_on_move[mt as usize] * 100 / (1+self.node_cut)
    } 

    pub fn accumulate(&mut self, other: &NodeStats) {
        self.interior_nodes += other.interior_nodes;
        self.leaf_nodes += other.leaf_nodes;

        self.node_all += other.node_all;
        self.node_pv += other.node_pv;
        self.node_cut += other.node_cut;

        self.pvs += other.pvs;
        self.pvs_research += other.pvs_research;
        self.nmp += other.nmp;
        self.fp += other.fp;
        for i in 0..6 {
            self.cut_on_move[i] += other.cut_on_move[i];
        }

        self.q_interior_nodes += other.q_interior_nodes;
        self.q_leaf_nodes += other.q_leaf_nodes;

        self.tt_nodes += other.tt_nodes;
        self.tt_hints += other.tt_hints;
        self.q_tt_nodes += other.q_tt_nodes;

        self.est_time += other.est_time;
        self.real_time += other.real_time;
        self.deterministic_time += other.deterministic_time;
    }

    #[inline]
    pub fn interior_nodes(&self) -> u64 {
        self.interior_nodes
    }

    #[inline]
    pub fn nodes(&self) -> u64 {
        self.interior_nodes + self.leaf_nodes + self.q_interior_nodes + self.q_leaf_nodes + self.tt_nodes
        // root
    }
}

macro_rules! header_format {
    () => {
        concat!(
            "{node:>11} ",
            "{interior:>11} ",
            "{leaf:>11} ",
            "{ttnode:>11}  ",

            "{pv_perc:>3} ",
            "{cut_perc:>3} ",
            "{all_perc:>3}   ",

            "{cut_move_hash:>3} ",
            "{cut_move_null:>3} ",
            "{cut_move_cap:>3} ",
            "{cut_move_killer:>3} ",
            "{cut_move_other:>3}  ",

            "{pvs:>7} ",
            "{pvs_research:>7} ",
            "{nmp:>7} ",
            "{fp:>7} ",

            "{qnode:>11} ",
            "{qinterior:>11} ",
            "{qleaf:>11} ",
            "{qttnode:>11} ",

            "{est_time:>11} ",
            "{real_time:>11} ",
            // "{deterministic_time:>11}",
        )
    };
}

impl NodeStats {
    pub fn fmt_header(f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            header_format!(),
            node = "total =",
            interior = "[interior",
            leaf = "leaf nodes",
            ttnode = "tt nodes]",

            pv_perc = "P%",
            cut_perc = "C%",
            all_perc = "A%",

            cut_move_hash = "HA",
            cut_move_null = "NU",
            cut_move_cap = "CA",
            cut_move_killer = "KI",
            cut_move_other = "OT",

            pvs = "pvs",
            pvs_research = "pvs/r",
            nmp = "nmp",
            fp = "fp",

            qnode = "q total",
            qinterior = "q interior",
            qleaf = "q leaf",
            qttnode = "q tt nodes",

            est_time = "est_time",
            real_time = "real_time",
            // deterministic_time = "determstic",
        )
    }

    pub fn fmt_underline(f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            header_format!(),
            node = "-----------",
            interior = "-----------",
            leaf = "-----------",
            ttnode = "-----------",

            cut_perc = "--",
            pv_perc = "--",
            all_perc = "--",

            cut_move_hash = "--",
            cut_move_null = "--",
            cut_move_cap = "--",
            cut_move_killer = "--",
            cut_move_other = "--",

            pvs = "-------",
            pvs_research = "-------",
            nmp = "-------",
            fp = "-------",

            qnode = "-----------",
            qinterior = "-----------",
            qleaf = "-----------",
            qttnode = "-----------",

            est_time = "-----------",
            real_time = "-----------",
            // deterministic_time = "-----------",
        )
    }

    pub fn fmt_data(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            header_format!(),
            node = self.nodes(),
            interior = self.interior_nodes,
            leaf = self.leaf_nodes,
            ttnode = self.tt_nodes,

            pv_perc = self.node_pv * 100 / (1+self.nodes()) as u64,
            cut_perc = self.node_cut * 100 / (1+self.nodes()) as u64,
            all_perc = self.node_all * 100 / (1+self.nodes()) as u64,

            cut_move_hash = self.cut_move_perc(MoveType::Hash),
            cut_move_null = self.cut_move_perc(MoveType::Null),
            cut_move_cap = self.cut_move_perc(MoveType::GoodCapture) + self.cut_move_perc(MoveType::GoodCaptureUpfrontSorted),
            cut_move_killer = self.cut_move_perc(MoveType::Killer),
            cut_move_other = self.cut_move_perc(MoveType::Promo) + 
                self.cut_move_perc(MoveType::Quiet) +
                self.cut_move_perc(MoveType::QuietUnsorted) +
                self.cut_move_perc(MoveType::BadCapture) +
                self.cut_move_perc(MoveType::Unsorted) +
                self.cut_move_perc(MoveType::Capture),

            pvs = self.pvs,
            pvs_research = self.pvs_research,
            nmp = self.nmp,
            fp = self.fp,

            qnode = self.q_interior_nodes + self.q_leaf_nodes,
            qinterior = self.q_interior_nodes,
            qleaf = self.q_leaf_nodes,
            qttnode = self.q_tt_nodes,

            est_time = Clock::format(self.est_time),
            real_time = Clock::format(self.real_time),
            // deterministic_time = Clock::format(self.deterministic_time),
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
    use crate::types::MoveType;

    #[test]
    fn test_display_stats() {
        let ply_stats = NodeStats::new();
        println!("{}", ply_stats);
        println!("{:?}", ply_stats);
        println!("{:#?}", ply_stats);

        let mut search = SearchStats::default();
        search.inc_leaf_nodes(2);
        search.inc_leaf_nodes(2);
        search.inc_tt_nodes(2);
        search.inc_node_cut(2, MoveType::GoodCapture);
        search.inc_interior_nodes(0);
        println!("{}", search);
    }
}
