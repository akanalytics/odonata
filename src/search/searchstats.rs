use crate::clock3::{Clock3};
use crate::eval::score::Score;
use crate::variation::Variation;
use crate::types::{Ply, MAX_PLY, MoveType};
use crate::utils::Formatting;
use std::fmt;
use std::cmp;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use strum::EnumCount;
use std::time::Duration;
use format_num::*;

use super::node::{Category, Node};






#[derive(Clone, Debug)]
pub struct SearchStats {

    all_threads_node_count: Arc<AtomicU64>,
    this_thread_node_count: u64,

    pub depth: Ply,
    clock: Clock3,

    pub completed: bool,
    pub user_cancelled: bool,
    cumulative: NodeStats,
    total: NodeStats,
    plies: Vec<NodeStats>,

    pub pv: Variation,
    score: Score,
    category: Category,
}

#[rustfmt::skip]
impl fmt::Display for SearchStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "depth            : {}", self.depth)?;
        writeln!(f, "pv               : {}", self.pv())?;
        writeln!(f, "score            : {}", self.score)?;
        writeln!(f, "category         : {:?}", self.category)?;
        writeln!(f, "clock (elapsed)  : {}", Formatting::format_duration(self.clock.elapsed_search()))?;
        writeln!(f, "interrupted      : {}", self.interrupted())?;
        writeln!(f, "user cancelled   : {}", self.user_cancelled)?;
        writeln!(f, "calc depth       : {}", self.depth())?;
        writeln!(f, "selective depth  : {}", self.selective_depth())?;
        writeln!(f, "all threads nodes: {}", self.all_threads_node_count.load(Ordering::Relaxed))?;
        writeln!(f, "tot nodes/sec (k): {}", self.all_threads_cumulative_knps())?;
        writeln!(f, "cuts on 1st move : {:.01}%", self.cuts_on_first_move() * 100.0)?;
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
        self.iteration().fmt_data(f)?;
        writeln!(f)?;
        Ok(())
    }
}

impl Default for SearchStats {
    fn default() -> Self {
        Self {
            all_threads_node_count: Arc::new(AtomicU64::new(0)),
            this_thread_node_count: 0, 
            depth: 0,
            clock: Clock3::default(),
            completed: false,
            user_cancelled: false,
            total: NodeStats::default(),
            cumulative: NodeStats::default(),
            plies: std::iter::repeat(NodeStats::new())
                .take(MAX_PLY as usize)
                .collect(),
            pv: Variation::default(),
            score: Score::default(),
            category: Category::Unknown,
        }
    }
}

impl SearchStats {
    pub fn new() -> Self {
        Self::default()
    }


    #[inline]
    pub fn score(&self) -> Score {
        self.score
    }

    #[inline]
    pub fn set_score(&mut self, s: Score, cat: Category) {
        self.score = s;
        self.category = cat;
    }

    #[inline]
    fn inc_all_nodes(&mut self) {
        self.this_thread_node_count += 1;
        if self.this_thread_node_count % 1024 == 0 {
            self.all_threads_node_count.fetch_add(1024, Ordering::Relaxed);
        }
    }

    pub fn interrupted(&self) -> bool {
        !self.completed
    }

    pub fn pv(&self) -> &Variation {
        &self.pv
    }

    pub fn restart_clocks(&mut self) {
        self.clock = Clock3::new();
    }

    pub fn elapsed_search(&self) -> Duration {
        self.clock.elapsed_search()
    }

    pub fn new_iteration(&mut self) {
        self.plies_mut().iter_mut().for_each(|s| s.clear_node_stats());
        self.total.clear_node_stats();
        self.score = Score::default();
        self.completed = false;
        self.user_cancelled = false;
        self.clock.start_iteration();
    }

    #[inline]
    pub fn iteration(&self) -> &NodeStats {
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
            .rposition(|stats| stats.all_nodes() != 0)
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

    pub fn record_iteration(&mut self, ply: Ply, category: Category, pv: Variation) {
        let ply = ply as usize;
        self.plies[ply].elapsed = self.clock.elapsed_ply();
        self.completed = category != Category::Cancelled;
        if self.completed {
            self.pv = pv;
        }
        self.cumulative.accumulate(&self.total);
    }


    #[inline]
    pub fn inc_leaf_nodes(&mut self, n: &Node) {
        self.inc_all_nodes();
        if n.depth >= 0 {
            self.total.leaf_nodes += 1;
            self.plies[n.ply as usize].leaf_nodes += 1;
        }
        if n.depth <= 0 {
            self.total.q_leaf_nodes += 1;
            self.plies[n.ply as usize].q_leaf_nodes += 1;
        }
    }



    #[inline]
    pub fn inc_leaf_tt_nodes(&mut self, sel_ply: Ply) {
        self.inc_all_nodes();
        self.total.leaf_tt_nodes += 1;
        self.plies[sel_ply as usize].leaf_tt_nodes += 1;
    }

    #[inline]
    pub fn inc_leaf_qsearch_nodes(&mut self, ply: Ply) {
        self.total.leaf_qsearch_nodes += 1;
        self.plies[ply as usize].leaf_qsearch_nodes += 1;
    }


    #[inline]
    pub fn inc_zw_nodes(&mut self, ply: Ply) {
        self.plies[ply as usize].node_zw += 1;
    }

    #[inline]
    pub fn inc_interior_nodes(&mut self, n: &Node) {
        self.inc_all_nodes();
        if n.depth >= 0 {
            self.total.interior_nodes += 1;
            self.plies[n.ply as usize].interior_nodes += 1;
        } else {
            self.total.q_interior_nodes += 1;
            self.plies[n.ply as usize].q_interior_nodes += 1;
        }
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
    pub fn inc_node_cut(&mut self, ply: Ply, move_type: MoveType, move_number: i32) {
        self.plies[ply as usize].node_cut += 1;
        self.plies[ply as usize].cut_on_move[move_type.index()] += 1;
        if move_number == 0 {
            self.plies[ply as usize].cut_on_first_move += 1;
        }
        if move_number >= 0 {
            self.plies[ply as usize].cut_on_move_number += move_number as u64;
            self.plies[ply as usize].cut_in_movegen += 1;
        }
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
    pub fn inc_fp_move(&mut self, ply: Ply) {
        self.plies[ply as usize].fp += 1;
    }

    #[inline]
    pub fn inc_pvs_move(&mut self, ply: Ply) {
        self.plies[ply as usize].pvs += 1;
    }

    #[inline]
    pub fn inc_move(&mut self, ply: Ply) {
        self.plies[ply as usize].mv += 1;
    }

    #[inline]
    pub fn inc_ext_check(&mut self, ply: Ply) {
        self.plies[ply as usize].ext_check += 1;
    }

    #[inline]
    pub fn inc_red_lmr(&mut self, ply: Ply) {
        self.plies[ply as usize].red_lmr += 1;
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

    // #[inline]
    // pub fn total_knps(&self) -> u64 {
    //     self.total.all_nodes() / (1 + self.realtime.elapsed_millis())
    // }

    pub fn all_threads_cumulative_total_nodes(&self) -> u64 {
        self.all_threads_node_count.load(Ordering::Relaxed)
    }

    pub fn all_threads_cumulative_knps(&self) -> u64 {
        self.all_threads_node_count.load(Ordering::Relaxed) / (1 + self.cumulative_time_as_millis())
    }

    #[inline]
    pub fn cumulative_knps(&self) -> u64 {
        self.cumulative_nodes()
    }

    #[inline]
    pub fn cumulative_nodes(&self) -> u64 {
        self.cumulative.all_nodes() + self.total.all_nodes()
    }

    pub fn cumulative_time_as_millis(&self) -> u64 {
        // whats accumulated plus whats elapsed on this iteration
        self.clock.elapsed_search().as_millis() as u64
    }

    #[inline]
    pub fn total_time(&self) -> Duration {
        self.total.elapsed
    }

    // BF = total / interior
    #[inline]
    pub fn branching_factor(&self) -> f32 {
        let t = self.iteration();
        (t.all_leaf_nodes() + t.interior_nodes()) as f32 / (t.interior_nodes() + 1) as f32
    }

    #[inline]
    pub fn q_branching_factor(&self) -> f32 {
        let t = self.iteration();
        (t.q_leaf_nodes as f32) / (t.q_interior_nodes + 1) as f32
    }

    pub fn cuts_on_first_move(&self) -> f32 {
        let cuts_in_movegen: u64 = self
        .plies
        .iter()
        .map(|stats| stats.cut_in_movegen).sum();

        let cuts_total_first_move: u64 = self
        .plies
        .iter()
        .map(|stats| stats.cut_on_first_move).sum();
        (cuts_total_first_move as f32) / (cuts_in_movegen as f32)
    }

}

#[derive(Copy, Clone, Debug, Default)]
pub struct NodeStats {
    // nodes
    pub interior_nodes: u64,
    pub leaf_nodes: u64, // FIXME and terminal
    pub leaf_tt_nodes: u64,
    pub leaf_qsearch_nodes: u64,

    pub q_interior_nodes: u64,
    pub q_leaf_nodes: u64, // FIXME and terminal

    pub tt_hints: u64,
    pub q_tt_nodes: u64,

    pub node_zw: u64, 
    pub nmp: u64,
    pub node_pv: u64,
    pub node_all: u64,
    pub node_cut: u64,
    
    pub cut_on_move: [u64; MoveType::COUNT],
    pub cut_on_move_number: u64,
    pub cut_on_first_move: u64,
    pub cut_in_movegen: u64,

    pub pvs: u64,   
    pub pvs_research: u64,
    pub fp: u64,   // futility
    pub ext_check: u64,  // check extensions
    pub red_lmr: u64,  // late move reductions
    pub mv: u64,   // total moves

    pub est_time: Duration,
    pub elapsed: Duration,

}




impl NodeStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn elapsed_millis(&self) -> u64 {
        self.elapsed.as_millis() as u64
    }

    pub fn clear_node_stats(&mut self) {
        *self = Self::default();
    }

    fn cut_move_perc(&self, mt: MoveType) -> u64 {
        self.cut_on_move[mt.index()] * 100 / cmp::max(1, self.node_cut)
    } 

    pub fn accumulate(&mut self, other: &NodeStats) {
        self.interior_nodes += other.interior_nodes;
        self.leaf_nodes += other.leaf_nodes;
        self.leaf_qsearch_nodes += other.leaf_qsearch_nodes;
        self.node_zw += other.node_zw;
        self.leaf_tt_nodes += other.leaf_tt_nodes;

        self.node_all += other.node_all;
        self.node_pv += other.node_pv;
        self.node_cut += other.node_cut;
        self.cut_on_move_number += other.cut_on_move_number;
        self.cut_on_first_move += other.cut_on_first_move;
        self.cut_in_movegen += other.cut_in_movegen;

        self.pvs += other.pvs;
        self.pvs_research += other.pvs_research;
        self.nmp += other.nmp;
        self.fp += other.fp;
        self.ext_check += other.ext_check;
        self.red_lmr += other.red_lmr;
        self.mv += other.mv;
        for i in 0..6 {
            self.cut_on_move[i] += other.cut_on_move[i];
        }

        self.q_interior_nodes += other.q_interior_nodes;
        self.q_leaf_nodes += other.q_leaf_nodes;

        self.tt_hints += other.tt_hints;
        self.q_tt_nodes += other.q_tt_nodes;

        self.est_time += other.est_time;
        self.elapsed += other.elapsed;
    }

    #[inline]
    pub fn interior_nodes(&self) -> u64 {
        self.interior_nodes
    }

    #[inline]
    pub fn all_leaf_nodes(&self) -> u64 {
        self.leaf_nodes + self.leaf_tt_nodes + self.leaf_qsearch_nodes
    }

    #[inline]
    pub fn regular_nodes(&self) -> u64 {
        // at horizon every leaf_qsearch_node is also either a q_interior or q_leaf node,
        // so we could count 
        //    leaf_qsearch_node
        // OR
        //    q_interior_nodes + q_leaf_nodes
        //
        // but not both (minmax is different!)
        //
        self.interior_nodes + self.leaf_nodes + self.leaf_tt_nodes + self.leaf_qsearch_nodes
    }

    pub fn all_nodes(&self) -> u64 {
        self.interior_nodes + self.leaf_nodes + self.leaf_tt_nodes + self.q_interior_nodes + self.q_leaf_nodes
    }
}

macro_rules! header_format {
    () => {
        concat!(
            "{node:>11} ",
            "{interior:>11} ",
            "{leaf:>11} ",

            "{zw_perc:>4} ",
            "{nmp_perc:>3} ",
            "{pv_perc:>3} ",
            "{all_perc:>3}  ",
            "{cut_perc:>3}  ",

            "{cut_move_hash:>3} ",
            "{cut_move_null:>3} ",
            "{cut_move_cap:>3} ",
            "{cut_move_killer:>3} ",
            "{cut_move_other:>3}  ",

            "{pvs:>4} ",
            "{res:>5} ",
            "{ec:>4}  ",
            "{lmr:>4}  ",
            "{fp:>4}  ",

            "{ttnode:>11}  ",

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
            leaf = "all leaf]",
            ttnode = "leaf tt",

            zw_perc = "ZW%",
            nmp_perc = "NU%",
            pv_perc = "PV%",
            all_perc = "AL%",
            cut_perc = "CU%",

            cut_move_hash = "HA",
            cut_move_null = "NU",
            cut_move_cap = "CA",
            cut_move_killer = "KI",
            cut_move_other = "OT",

            pvs = "pvs%",
            res = "res%",
            fp = "fut%",
            ec = "chk%",
            lmr = "lmr%",

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

            zw_perc = "---",
            nmp_perc = "---",
            pv_perc = "---",
            all_perc = "---",
            cut_perc = "---",

            cut_move_hash = "--",
            cut_move_null = "--",
            cut_move_cap = "--",
            cut_move_killer = "--",
            cut_move_other = "--",

            pvs = "----",
            res = "----",
            fp = "----",
            ec = "----",
            lmr = "----",

            qnode = "-----------",
            qinterior = "-----------",
            qleaf = "-----------",
            qttnode = "-----------",

            est_time = "-----------",
            real_time = "-----------",
        )
    }

    pub fn fmt_data(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            header_format!(),
            node = self.regular_nodes(),
            interior = self.interior_nodes(),
            leaf = self.all_leaf_nodes(),
            ttnode = self.leaf_tt_nodes,

            pv_perc = self.node_pv * 100 / cmp::max(1, self.regular_nodes()) as u64,
            all_perc = self.node_all * 100 / cmp::max(1, self.regular_nodes()) as u64,
            zw_perc = self.node_zw * 100 / cmp::max(1, self.regular_nodes()) as u64,
            cut_perc = self.node_cut * 100 / cmp::max(1, self.regular_nodes()) as u64,
            nmp_perc = self.nmp * 100 / cmp::max(1,self.regular_nodes()) as u64,

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

            pvs = self.pvs * 100 / cmp::max(1, self.mv) as u64,
            res = format_num!(".2f", self.pvs_research as f64 * 100.0 / cmp::max(1, self.mv) as f64),
            fp = self.fp * 100 / cmp::max(1, self.mv) as u64,
            ec = self.ext_check * 100 / cmp::max(1, self.mv) as u64,
            lmr = self.red_lmr * 100 / cmp::max(1, self.mv) as u64,

            qnode = self.q_interior_nodes + self.q_leaf_nodes,
            qinterior = self.q_interior_nodes,
            qleaf = self.q_leaf_nodes,
            qttnode = self.q_tt_nodes,

            est_time = Formatting::format_duration(self.est_time),
            real_time = Formatting::format_duration(self.elapsed),
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

        let mut node = Node::default();
        node.ply = 2;

        let mut search = SearchStats::default();
        search.inc_leaf_nodes(&node);
        search.inc_leaf_nodes(&node);
        search.inc_leaf_tt_nodes(1);
        search.inc_node_cut(2, MoveType::GoodCapture, 0);
        search.inc_interior_nodes(&node);
        println!("{}", search);
    }
}
