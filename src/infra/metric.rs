use crate::search::node::Node;
use crate::utils::Formatting;
use hdrhist::HDRHist;
use static_init::dynamic;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::ops::AddAssign;
use std::{fmt, iter};
use tabled::builder::Builder;
use tabled::object::{Rows, Segment};
use tabled::style::Border;
use tabled::{Alignment, MaxWidth, Modify, Style};

#[derive()]
pub struct Histogram(HDRHist);

#[derive(Default, Debug)]
pub struct NodeHistogram([Histogram; 32], [Histogram; 32]);

impl Default for Histogram {
    fn default() -> Self {
        Self(HDRHist::new())
    }
}

impl fmt::Debug for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Histogram")
            // .field(&self.0.summary_string())
            .finish()
    }
}

impl Histogram {
    pub fn add_value(&mut self, v: u64) {
        self.0.add_value(v);
    }
}

impl NodeHistogram {
    pub fn add_value(&mut self, n: &Node, v: u64) {
        self.0[min(n.ply, 31) as usize].add_value(v);
        self.1[min(max(n.depth, 0), 31) as usize].add_value(v);
    }
}

#[derive(Default, Debug)]
pub struct NodeCounter([u64; 32], [u64; 32]);

impl NodeCounter {
    pub fn incr(&mut self, n: &Node) {
        self.0[min(n.ply, 31) as usize] += 1;
        self.1[min(max(n.depth, 0), 31) as usize] += 1;
    }

    // -1 => total
    pub fn for_ply(&self, y: isize) -> u64 {
        if y >= 0 {
            self.0[min(y, 31) as usize]
        } else {
            self.0.iter().sum()
        }
    }
}

impl AddAssign<&NodeCounter> for NodeCounter {
    fn add_assign(&mut self, rhs: &NodeCounter) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
            self.1[i] += rhs.1[i];
        }
    }
}

#[derive(Default, Debug)]
pub struct Metrics {
    pub make_move: u64,
    pub move_gen: u64,
    pub hash_board: u64,
    pub hash_move: u64,
    pub eval: u64,
    pub iter_complete: u64,
    pub iter_timeout: u64,

    pub tt_hit: NodeCounter,
    pub tt_probe: NodeCounter,
    pub tt_store: NodeCounter,
    pub tt_cut: NodeCounter,
    pub tt_all: NodeCounter,
    pub tt_pv: NodeCounter,

    pub interior: NodeCounter,
    pub leaf: NodeCounter,
    pub qs_interior: NodeCounter,
    pub qs_leaf: NodeCounter,

    pub moves: NodeCounter,

    pub eval_from_tt: NodeCounter,
    pub eval_calc: NodeCounter,
    pub eval_eg_draw: NodeCounter,
    pub eval_eg_win: NodeCounter,
    pub eval_eg_maybe: NodeCounter,
    pub eval_see: NodeCounter,

    pub node_cut: NodeCounter,
    pub node_all: NodeCounter,
    pub node_pv: NodeCounter,
    pub node_zw: NodeCounter,

    // pub cut_move: NodeHistogram,
    pub null_move_prune_attempt: NodeCounter,
    pub null_move_prune: NodeCounter,
    pub razor_prune: NodeCounter,
    pub standing_pat_prune: NodeCounter,
    pub futility_prune: NodeCounter,
    pub late_move_prune: NodeCounter,

    pub late_move_reduce: NodeCounter,
    pub check_extend: NodeCounter,

    pub re_search: NodeCounter,
    // counters: Vec<u64>,
    // node_counters: Vec<(Vec<u64>, Vec<u64>)>, // ply and depth
    // histograms: Vec<Histogram>,
    // timings: Vec<Duration>,
}

impl Metrics {
    pub fn record_metric(&mut self, m: &Metric) {
        use Metric::*;
        match *m {
            MakeMove => self.make_move += 1,
            MoveGen => self.move_gen += 1,
            HashBoard => self.hash_board += 1,
            HashMove => self.hash_move += 1,
            Eval => self.eval += 1,
            IterationComplete => self.iter_complete += 1,
            IterationTimeout => self.iter_timeout += 1,
            LegalMoves(i) => self.make_move += i as u64,

            TtHit(ref n) => self.tt_hit.incr(n),
            TtProbe(ref n) => self.tt_probe.incr(n),
            TtStore(ref n) => self.tt_store.incr(n),
            TtCut(ref n) => self.tt_cut.incr(n),
            TtAll(ref n) => self.tt_all.incr(n),
            TtPv(ref n) => self.tt_pv.incr(n),

            Interior(ref n) => self.interior.incr(n),
            Leaf(ref n) => self.leaf.incr(n),
            QsInterior(ref n) => self.qs_interior.incr(n),
            QsLeaf(ref n) => self.qs_leaf.incr(n),

            Moves(ref n) => self.moves.incr(n),

            EvalFromTt(ref n) => self.eval_from_tt.incr(n),
            EvalCalc(ref n) => self.eval_calc.incr(n),
            EvalEgDraw(ref n) => self.eval_eg_draw.incr(n),
            EvalEgKnown(ref n) => self.eval_eg_win.incr(n),
            EvalEgMaybe(ref n) => self.eval_eg_maybe.incr(n),
            EvalSee(ref n) => self.eval_see.incr(n),

            NodeCut(ref n) => self.node_cut.incr(n),
            NodeAll(ref n) => self.node_all.incr(n),
            NodePv(ref n) => self.node_pv.incr(n),
            NodeZw(ref n) => self.node_zw.incr(n),

            NullMovePruneAttempt(ref n) => self.null_move_prune_attempt.incr(n),
            NullMovePrune(ref n) => self.null_move_prune.incr(n),
            RazorPrune(ref n) => self.razor_prune.incr(n),
            StandingPatPrune(ref n) => self.standing_pat_prune.incr(n),
            FutilityPrune(ref n) => self.futility_prune.incr(n),
            LateMovePrune(ref n) => self.late_move_prune.incr(n),

            LateMoveReduce(ref n) => self.late_move_reduce.incr(n),
            CheckExtend(ref n) => self.check_extend.incr(n),

            ReSearch(ref n) => self.re_search.incr(n),
        }
    }

    pub fn add(&mut self, o: &Metrics) {
        self.make_move += o.make_move;
        self.move_gen += o.move_gen;
        self.hash_board += o.hash_board;
        self.hash_move += o.hash_move;
        self.eval += o.eval;
        self.iter_complete += o.iter_complete;
        self.iter_timeout += o.iter_timeout;
        self.make_move += o.make_move;

        self.tt_hit += &o.tt_hit;
        self.tt_probe += &o.tt_probe;
        self.tt_store += &o.tt_store;
        self.tt_cut += &o.tt_cut;
        self.tt_all += &o.tt_all;
        self.tt_pv += &o.tt_pv;

        self.interior += &o.interior;
        self.leaf += &o.leaf;
        self.qs_interior += &o.qs_interior;
        self.qs_leaf += &o.qs_leaf;

        self.moves += &o.moves;

        self.eval_from_tt += &o.eval_from_tt;
        self.eval_calc += &o.eval_calc;
        self.eval_eg_draw += &o.eval_eg_draw;
        self.eval_eg_win += &o.eval_eg_win;
        self.eval_eg_maybe += &o.eval_eg_maybe;
        self.eval_see += &o.eval_see;

        self.node_cut += &o.node_cut;
        self.node_all += &o.node_all;
        self.node_pv += &o.node_pv;
        self.node_zw += &o.node_zw;

        self.null_move_prune_attempt += &o.null_move_prune_attempt;
        self.null_move_prune += &o.null_move_prune;
        self.razor_prune += &o.razor_prune;
        self.standing_pat_prune += &o.standing_pat_prune;
        self.futility_prune += &o.futility_prune;
        self.late_move_prune += &o.late_move_prune;

        self.late_move_reduce += &o.late_move_reduce;
        self.check_extend += &o.check_extend;

        self.re_search += &o.re_search;
    }

    pub fn to_string() -> String {
        let tl = THREAD_METRICS.with(|tm| format!("{}", &*tm.borrow()));
        format!("{}\n\n{}", tl, &*GLOBAL_METRICS.read())
    }

    pub fn add_thread_local_to_global() {
        THREAD_METRICS.with(|tm| {
            GLOBAL_METRICS.write().add(&*tm.borrow());
            *tm.borrow_mut() = Metrics::default();
        });
    }
}

#[must_use]
pub enum Metric {
    MakeMove,
    MoveGen,
    HashBoard,
    HashMove,
    Eval,
    IterationTimeout,
    IterationComplete,
    LegalMoves(u32),

    Interior(Node),
    Leaf(Node),
    QsInterior(Node),
    QsLeaf(Node),

    NodeCut(Node),
    NodeAll(Node),
    NodePv(Node),
    NodeZw(Node),

    Moves(Node),

    TtHit(Node),
    TtProbe(Node),
    TtStore(Node),
    TtCut(Node),
    TtAll(Node),
    TtPv(Node),

    EvalFromTt(Node),
    EvalCalc(Node),
    EvalEgDraw(Node),
    EvalEgKnown(Node),
    EvalEgMaybe(Node),
    EvalSee(Node),

    NullMovePruneAttempt(Node),
    NullMovePrune(Node),
    RazorPrune(Node),
    StandingPatPrune(Node),
    FutilityPrune(Node),
    LateMovePrune(Node),

    LateMoveReduce(Node),
    CheckExtend(Node),

    ReSearch(Node),
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let style = Style::github_markdown().bottom('-');
        let tab = Builder::default()
            .set_columns(["Counter", "Value"])
            .add_record(["Make move", &self.make_move.to_string()])
            .add_record(["Move gen", &self.move_gen.to_string()])
            .add_record(["Hash board", &self.hash_board.to_string()])
            .add_record(["Hash move", &self.hash_move.to_string()])
            .add_record(["Eval", &self.eval.to_string()])
            .add_record(["Iter complete", &self.iter_complete.to_string()])
            .add_record(["Iter timeout", &self.iter_timeout.to_string()])
            .build()
            .with(style)
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()));
        tab.fmt(f)?;

        let iters = 32_isize;
        let mut b = Builder::default().set_columns([
            "Ply",
            // node 1
            "Interior",
            "Leaf",
            "QS Int",
            "QS Leaf",
            // node 2
            "Cut",
            "All",
            "Pv",
            "ZW",
            // TT
            "TT hit",
            "TT probe",
            "TT store",
            "TT cut",
            "TT all",
            "TT pv",
            "Moves",
            // evals
            "Eval tt",
            "Eval calc",
            "Eg draw",
            "Eg w/l",
            "Eg maybe",
            "Eval see",
            // pruning
            "NMP",
            "Razor",
            "Pat",
            "Fut",
            "LMP",
            "LMR",
            "Chk Ext",
            "Re- Srch",
            // "Depth", "Interior", "Leaf", "QS Int", "QS Leaf",
        ]);

        fn s(i: u64) -> String {
            Formatting::u128(i as u128)
        }
        let total = iter::once(-1);
        for y in (0..iters).chain(total) {
            let _d = iters - 1 - y;
            b = b.add_record([
                if y > 0 {
                    s(y as u64)
                } else {
                    "Total".to_string()
                },
                // node 1
                s(self.interior.for_ply(y)),
                s(self.leaf.for_ply(y)),
                s(self.qs_interior.for_ply(y)),
                s(self.qs_leaf.for_ply(y)),
                // node 2
                s(self.node_cut.for_ply(y)),
                s(self.node_all.for_ply(y)),
                s(self.node_pv.for_ply(y)),
                s(self.node_zw.for_ply(y)),
                // TT
                s(self.tt_hit.for_ply(y)),
                s(self.tt_probe.for_ply(y)),
                s(self.tt_store.for_ply(y)),
                s(self.tt_cut.for_ply(y)),
                s(self.tt_all.for_ply(y)),
                s(self.tt_pv.for_ply(y)),
                s(self.moves.for_ply(y)),
                // eval
                s(self.eval_from_tt.for_ply(y)),
                s(self.eval_calc.for_ply(y)),
                s(self.eval_eg_draw.for_ply(y)),
                s(self.eval_eg_win.for_ply(y)),
                s(self.eval_eg_maybe.for_ply(y)),
                s(self.eval_see.for_ply(y)),
                // prune
                s(self.null_move_prune.for_ply(y)),
                s(self.razor_prune.for_ply(y)),
                s(self.standing_pat_prune.for_ply(y)),
                s(self.futility_prune.for_ply(y)),
                s(self.late_move_prune.for_ply(y)),
                s(self.late_move_reduce.for_ply(y)),
                s(self.check_extend.for_ply(y)),
                s(self.re_search.for_ply(y)),
                // d as u64,
                // self.interior.1[d],
                // self.leaf.1[d],
                // self.qs_interior.1[d],
                // self.qs_leaf.1[d],
            ]);
        }
        let style = Style::github_markdown().bottom('-');
        let table = b
            .build()
            .with(style)
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Rows::single(0)).with(MaxWidth::wrapping(5).keep_words()));
        table.fmt(f)?;
        Ok(())
    }
}

impl Metric {
    #[allow(unused_variables)]
    #[inline]
    pub fn record(&self) {
        // #[cfg(not(feature="remove_metrics"))]
        THREAD_METRICS.with(|s| s.borrow_mut().record_metric(self));
    }
}

thread_local! {
    pub static THREAD_METRICS: RefCell<Metrics>  = RefCell::new(Metrics::default());
}

#[dynamic(lazy)]
static mut GLOBAL_METRICS: Metrics = Metrics::default();

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_metrics() {
        Metric::MakeMove.record();
        Metric::MakeMove.record();
        Metric::NullMovePrune(Node {
            ply: 1,
            depth: 5,
            ..Node::default()
        })
        .record();
        THREAD_METRICS.with(|tm| {
            println!("1. global    \n{}", *GLOBAL_METRICS.read());
            println!("thread local \n{}", tm.borrow());
        });

        Metrics::add_thread_local_to_global();

        THREAD_METRICS.with(|tm| {
            println!("2. global    \n{}", *GLOBAL_METRICS.read());
            println!("thread local \n{}", tm.borrow());
        });
    }
}
