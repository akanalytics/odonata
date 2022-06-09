use crate::eval::endgame::EndGame;
use crate::search::node::Node;
use crate::types::Ply;
use crate::utils::Formatting;
use static_init::dynamic;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::ops::AddAssign;
use std::time::{Duration, Instant};
use std::{fmt, iter};
use strum::{EnumCount, IntoEnumIterator};
use tabled::builder::Builder;
use tabled::object::{Columns, Rows, Segment};
use tabled::style::Border;
use tabled::{Alignment, Modify, Rotate, Style};

pub use crate::search::node::Event;

// #[derive()]
// pub struct Histogram(HDRHist);

// #[derive(Default, Debug)]
// pub struct NodeHistogram([Histogram; 32], [Histogram; 32]);

// impl Default for Histogram {
//     fn default() -> Self {
//         Self(HDRHist::new())
//     }
// }

// impl fmt::Debug for Histogram {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_tuple("Histogram")
//             // .field(&self.0.summary_string())
//             .finish()
//     }
// }

// impl Histogram {
//     pub fn add_value(&mut self, v: u64) {
//         self.0.add_value(v);
//     }
// }

// impl NodeHistogram {
//     pub fn add_value(&mut self, n: &Node, v: u64) {
//         self.0[min(n.ply, 31) as usize].add_value(v);
//         self.1[min(max(n.depth, 0), 31) as usize].add_value(v);
//     }
// }

#[derive(Debug, Clone)]
pub struct ArrayCounter<const N: usize, T>([T; N]);

impl Default for ArrayCounter<{ EndGame::COUNT }, u64> {
    fn default() -> Self {
        Self([0; EndGame::COUNT])
    }
}

// impl Default for ArrayCounter<{ Event::COUNT }, u64> {
//     fn default() -> Self {
//         Self([0; Event::COUNT])
//     }
// }

impl<T> Default for ArrayCounter<{ Event::COUNT }, T>
where
    T: Default + Copy,
{
    fn default() -> Self {
        Self([T::default(); Event::COUNT])
    }
}

impl<const N: usize, T> AddAssign<&ArrayCounter<{ N }, T>> for ArrayCounter<{ N }, T>
where
    for<'a> T: AddAssign<&'a T>,
{
    fn add_assign(&mut self, rhs: &ArrayCounter<{ N }, T>) {
        for i in 0..self.0.len() {
            self.0[i] += &rhs.0[i];
        }
    }
}

impl<const N: usize> ArrayCounter<N, u64>
where
    [u64; N]: Default,
{
    pub fn incr(&mut self, i: usize) {
        self.0[min(i, N - 1) as usize] += 1;
    }
}

#[derive(Default, Debug, Clone, Copy)]
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

#[derive(Default, Debug, Clone, Copy)]
pub struct TimePlyCounter([Duration; 32]);

impl TimePlyCounter {
    pub fn set(&mut self, y: Ply, dur: Duration) {
        self.0[min(y, 31) as usize] = dur;
    }

    pub fn add(&mut self, y: Ply, dur: Duration) {
        self.0[min(y, 31) as usize] += dur;
    }

    // -1 => total
    pub fn for_ply(&self, y: isize) -> Duration {
        if y >= 0 {
            self.0[min(y, 31) as usize]
        } else {
            self.0.iter().sum()
        }
    }
}

impl AddAssign<&TimePlyCounter> for TimePlyCounter {
    fn add_assign(&mut self, rhs: &TimePlyCounter) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ProfilerCounter(Duration, u32);

impl ProfilerCounter {
    pub fn record(&mut self, dur: Duration) {
        self.0 += dur;
        self.1 += 1;
    }

    pub fn average(&self) -> Duration {
        self.0 / self.1
    }

    pub fn total(&self) -> Duration {
        self.0
    }
}

impl AddAssign<&ProfilerCounter> for ProfilerCounter {
    fn add_assign(&mut self, rhs: &ProfilerCounter) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

#[derive(Default, Debug, Clone)]
pub struct Metrics {
    events: ArrayCounter<{ Event::len() }, u64>,
    nodes: ArrayCounter<{ Event::len() }, NodeCounter>,
    profilers: ArrayCounter<{ Event::len() }, ProfilerCounter>,
    durations: ArrayCounter<{ Event::len() }, TimePlyCounter>,
    endgame: ArrayCounter<{ EndGame::COUNT }, u64>,

    // make_move: u64,
    // move_gen: u64,
    // hash_board: u64,
    // hash_move: u64,
    // eval: u64,
    // iter_complete: u64,
    // iter_timeout: u64,
    // t_eval: TimePlyCounter,
    // t_move_gen: TimePlyCounter,
    // t_make_move: TimePlyCounter,
    // timing_search_root: TimePlyCounter,
    // t_sort_moves: TimePlyCounter,
    // tt_collision: u64,
    // tt_insert: u64,
    // tt_update: u64,
    // tt_pv_overwrite: u64,
    // tt_illegal_move: u64,


    // tt_hit: NodeCounter,
    // tt_probe: NodeCounter,
    // tt_store: NodeCounter,
    // tt_cut: NodeCounter,
    // tt_all: NodeCounter,
    // tt_pv: NodeCounter,

    // interior: NodeCounter,
    // leaf: NodeCounter,
    // qs_interior: NodeCounter,
    // qs_leaf: NodeCounter,

    // moves: NodeCounter,

    // eval_from_tt: NodeCounter,
    // eval_calc: NodeCounter,
    // eval_eg_draw: NodeCounter,
    // eval_eg_win: NodeCounter,
    // eval_eg_maybe: NodeCounter,
    // eval_see: NodeCounter,

    // node_cut: NodeCounter,
    // node_all: NodeCounter,
    // node_pv: NodeCounter,
    // node_zw: NodeCounter,

    // // cut_move: NodeHistogram,
    // null_move_prune_attempt: NodeCounter,
    // null_move_prune: NodeCounter,
    // razor_prune_d2: NodeCounter,
    // razor_prune_d3: NodeCounter,
    // razor_prune_fail: NodeCounter,
    // standing_pat_prune: NodeCounter,
    // futility_prune: NodeCounter,
    // late_move_prune: NodeCounter,

    // late_move_reduce: NodeCounter,
    // check_extend: NodeCounter,

    // search_fwfd: NodeCounter,
    // search_zwrd: NodeCounter,
    // search_zwfd: NodeCounter,
    // re_search_zwfd: NodeCounter,
    // re_search_fwfd: NodeCounter,
    // aspiration_none: NodeCounter,
    // aspiration_1: NodeCounter,
    // aspiration_2: NodeCounter,
    // aspiration_3: NodeCounter,
    // aspiration_n: NodeCounter,

    // iter_est: TimePlyCounter,
    // iter_act: TimePlyCounter,
    // iter_allotted: TimePlyCounter,
    // counters: Vec<u64>,
    // node_counters: Vec<(Vec<u64>, Vec<u64>)>, // ply and depth
    // histograms: Vec<Histogram>,
    // timings: Vec<Duration>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    fn record_metric(&mut self, m: &Metric) {
        use Metric::*;
        // unwraps safe as None only used if configured off, in which case this logioc not called
        // match *m {
            // MakeMove => self.make_move += 1,
            // MoveGen => self.move_gen += 1,
            // HashBoard => self.hash_board += 1,
            // HashMove => self.hash_move += 1,
            // Eval => self.eval += 1,
            // IterationComplete => self.iter_complete += 1,
            // IterationTimeout => self.iter_timeout += 1,
            // LegalMoves(i) => self.make_move += i as u64,
            // EndGame(eg) => self.endgame.0[eg as usize] += 1,

            // TimingEval(start) => self.t_eval.add(0, start.unwrap().elapsed()),
            // TimingMoveGen(start) => self.t_move_gen.add(0, start.unwrap().elapsed()),
            // TimingMakeMove(start) => self.t_make_move.add(0, start.unwrap().elapsed()),
            // TimingSearchRoot(start) => self.timing_search_root.add(0, start.unwrap().elapsed()),
            // TimingSortMoves(start) => self.t_sort_moves.add(0, start.unwrap().elapsed()),

            // TtHit(ref n) => self.tt_hit.incr(n),
            // TtProbe(ref n) => self.tt_probe.incr(n),
            // TtStore(ref n) => self.tt_store.incr(n),
            // TtCut(ref n) => self.tt_cut.incr(n),
            // TtAll(ref n) => self.tt_all.incr(n),
            // TtPv(ref n) => self.tt_pv.incr(n),

            // Interior(ref n) => self.interior.incr(n),
            // Leaf(ref n) => self.leaf.incr(n),
            // QsInterior(ref n) => self.qs_interior.incr(n),
            // QsLeaf(ref n) => self.qs_leaf.incr(n),

            // Moves(ref n) => self.moves.incr(n),

            // EvalFromTt(ref n) => self.eval_from_tt.incr(n),
            // EvalCalc(ref n) => self.eval_calc.incr(n),
            // EvalEgDraw(ref n) => self.eval_eg_draw.incr(n),
            // EvalEgKnown(ref n) => self.eval_eg_win.incr(n),
            // EvalEgMaybe(ref n) => self.eval_eg_maybe.incr(n),
            // EvalSee(ref n) => self.eval_see.incr(n),

            // NodeCut(ref n) => self.node_cut.incr(n),
            // NodeAll(ref n) => self.node_all.incr(n),
            // NodePv(ref n) => self.node_pv.incr(n),
            // NodeZw(ref n) => self.node_zw.incr(n),

            // RazorPruneD2(ref n) => self.razor_prune_d2.incr(n),
            // RazorPruneD3(ref n) => self.razor_prune_d3.incr(n),
            // RazorPruneFail(ref n) => self.razor_prune_fail.incr(n),
            // StandingPatPrune(ref n) => self.standing_pat_prune.incr(n),
            // FutilityPrune(ref n) => self.futility_prune.incr(n),
            // LateMovePrune(ref n) => self.late_move_prune.incr(n),

            // LateMoveReduce(ref n) => self.late_move_reduce.incr(n),
            // CheckExtend(ref n) => self.check_extend.incr(n),

            // SearchFwFd(ref n) => self.search_fwfd.incr(n),
            // SearchZwRd(ref n) => self.search_zwrd.incr(n),
            // SearchZwFd(ref n) => self.search_zwfd.incr(n),
            // ReSearchZwFd(ref n) => self.re_search_zwfd.incr(n),
            // ReSearchFwFd(ref n) => self.re_search_fwfd.incr(n),

            // IterEst(ply, dur) => self.iter_est.set(ply, dur),
            // IterActual(ply, start) => self.iter_act.set(ply, start.unwrap().elapsed()),
            // IterAllotted(ply, dur) => self.iter_allotted.set(ply, dur),
        // }
    }

    pub fn add(&mut self, o: &Metrics) {
        self.events += &o.events;
        self.nodes += &o.nodes;
        self.profilers += &o.profilers;
        self.durations += &o.durations;
        self.endgame += &o.endgame;

        // self.make_move += o.make_move;
        // self.move_gen += o.move_gen;
        // self.hash_board += o.hash_board;
        // self.hash_move += o.hash_move;
        // self.eval += o.eval;
        // self.iter_complete += o.iter_complete;
        // self.iter_timeout += o.iter_timeout;
        // self.make_move += o.make_move;
        // self.t_eval += &o.t_eval;
        // self.t_move_gen += &o.t_move_gen;
        // self.t_make_move += &o.t_make_move;
        // self.timing_search_root += &o.timing_search_root;
        // self.t_sort_moves += &o.t_sort_moves;
        // self.tt_collision += &o.tt_collision;
        // self.tt_insert += &o.tt_insert;
        // self.tt_update += &o.tt_update;
        // self.tt_pv_overwrite += &o.tt_pv_overwrite;
        // self.tt_illegal_move += &o.tt_illegal_move;


        // self.tt_hit += &o.tt_hit;
        // self.tt_probe += &o.tt_probe;
        // self.tt_store += &o.tt_store;
        // self.tt_cut += &o.tt_cut;
        // self.tt_all += &o.tt_all;
        // self.tt_pv += &o.tt_pv;

        // self.interior += &o.interior;
        // self.leaf += &o.leaf;
        // self.qs_interior += &o.qs_interior;
        // self.qs_leaf += &o.qs_leaf;

        // self.moves += &o.moves;

        // self.eval_from_tt += &o.eval_from_tt;
        // self.eval_calc += &o.eval_calc;
        // self.eval_eg_draw += &o.eval_eg_draw;
        // self.eval_eg_win += &o.eval_eg_win;
        // self.eval_eg_maybe += &o.eval_eg_maybe;
        // self.eval_see += &o.eval_see;

        // self.node_cut += &o.node_cut;
        // self.node_all += &o.node_all;
        // self.node_pv += &o.node_pv;
        // self.node_zw += &o.node_zw;

        // self.null_move_prune_attempt += &o.null_move_prune_attempt;
        // self.null_move_prune += &o.null_move_prune;
        // self.razor_prune_d2 += &o.razor_prune_d2;
        // self.razor_prune_d3 += &o.razor_prune_d3;
        // self.razor_prune_fail += &o.razor_prune_fail;
        // self.standing_pat_prune += &o.standing_pat_prune;
        // self.futility_prune += &o.futility_prune;
        // self.late_move_prune += &o.late_move_prune;

        // self.late_move_reduce += &o.late_move_reduce;
        // self.check_extend += &o.check_extend;

        // self.search_fwfd += &o.search_fwfd;
        // self.search_zwrd += &o.search_zwrd;
        // self.search_zwfd += &o.search_zwfd;
        // self.re_search_zwfd += &o.re_search_zwfd;
        // self.re_search_fwfd += &o.re_search_fwfd;
        // self.aspiration_none += &o.aspiration_none;
        // self.aspiration_1 += &o.aspiration_1;
        // self.aspiration_2 += &o.aspiration_2;
        // self.aspiration_3 += &o.aspiration_3;
        // self.aspiration_n += &o.aspiration_n;

        // self.iter_est += &o.iter_est;
        // self.iter_act += &o.iter_act;
        // self.iter_allotted += &o.iter_allotted;
    }

    pub fn to_string() -> String {
        // let tl = METRICS_THREAD.with(|tm| format!("{}", &*tm.borrow()));
        format!(
            "Global\n{}\n\nLast Iter\n{}",
            &*METRICS_TOTAL.read(),
            &*METRICS_LAST_ITER.read(),
        )
    }

    pub fn flush_thread_local() {
        METRICS_THREAD.with(|tm| {
            METRICS_TOTAL.write().add(&*tm.borrow());
            **METRICS_LAST_ITER.write() = std::mem::take(&mut tm.borrow_mut());
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
    EndGame(EndGame),
    TimingEval(Option<Instant>),
    TimingMoveGen(Option<Instant>),
    TimingMakeMove(Option<Instant>),
    TimingSearchRoot(Option<Instant>),
    TimingSortMoves(Option<Instant>),

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

    RazorPruneD2(Node),
    RazorPruneD3(Node),
    RazorPruneFail(Node),
    StandingPatPrune(Node),
    FutilityPrune(Node),
    LateMovePrune(Node),

    LateMoveReduce(Node),
    CheckExtend(Node),

    SearchFwFd(Node),
    SearchZwRd(Node),
    SearchZwFd(Node),
    ReSearchZwFd(Node),
    ReSearchFwFd(Node),

    IterEst(Ply, Duration),
    IterActual(Ply, Option<Instant>),
    IterAllotted(Ply, Duration),
}

impl Metrics {
    fn rows(&self, events: &[Event]) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        for e in events {
            let mut v = vec![];
            v.push(e.name().to_string());
            let total = iter::once(-1);
            let iters = 32_isize;

            for ply in (0..iters).chain(total) {
                v.push(self.nodes.0[e.index()].for_ply(ply).to_string())
            }
            rows.push(v);
        }
        rows
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn i(i: u64) -> String {
            if i > 0 {
                Formatting::u128(i as u128)
            } else {
                String::new()
            }
        }
        fn d(dur: Duration) -> String {
            if dur > Duration::ZERO {
                Formatting::duration(dur)
            } else {
                String::new()
            }
        }

        fn pd(dur: Duration, total: Duration) -> String {
            if dur > Duration::ZERO && !total.is_zero() {
                format!(
                    "{}%",
                    Formatting::decimal(1, dur.as_secs_f32() * 100.0 / total.as_secs_f32())
                )
            } else {
                String::new()
            }
        }

        let style = Style::github_markdown().bottom('-');
        // let tot = self.timing_search_root.for_ply(0);
        let mut b = Builder::default()
            .set_columns(["Counter", "Value"]);
            // .add_record(["Eval", &i(self.eval)])
            // .add_record(["Time SearchRoot", &d(tot)])
            // .add_record(["Timing SearchRoot", &pd(tot, tot)])
            // .add_record(["Timing Eval", &pd(self.t_eval.for_ply(0), tot)])
            // .add_record(["Timing MoveGen", &pd(self.t_move_gen.for_ply(0), tot)])
            // .add_record(["Timing MakeMove", &pd(self.t_make_move.for_ply(0), tot)])
            // .add_record([
            //     "Timing SortMoves-MM",
            //     &pd(
            //         self.t_sort_moves
            //             .for_ply(0)
            //             .saturating_sub(self.t_make_move.for_ply(0)),
            //         tot,
            //     ),
            // ]);

        for e in Event::iter() {
            if self.events.0[e.index()] != 0 {
                b = b.add_record([e.name(), &i(self.events.0[e.index()])]);
            }
        }
        for e in Event::iter() {
            let tot = self.profilers.0[Event::TimingSearchRoot.index()].total();
            if self.profilers.0[e.index()].1 != 0 {
                b = b.add_record([e.name(), &pd(self.profilers.0[e.index()].total(), tot)]);
            }
        }
        for eg in EndGame::iter() {
            b = b.add_record([&eg.to_string(), &i(self.endgame.0[eg as usize])]);
        }
        let mut t = b
            .build()
            .with(style)
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t = t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
        }
        t.fmt(f)?;
        writeln!(f)?;

        let mut b = Builder::default();
        for e in Event::iter() {
            if self.nodes.0[e.index()].for_ply(-1) == 0 {
                continue;
            }
            let mut v = vec![];
            v.push(e.name().to_string());
            let total = iter::once(-1);
            let iters = 32_isize;

            for ply in (0..iters).chain(total) {
                v.push(self.nodes.0[e.index()].for_ply(ply).to_string())
            }
            b = b.add_record(v);

        }

        // for r in self
        //     .rows(&[Event::EvalStatic, Event::NmpConsider])
        //     .iter()
        // {
        //     b = b.add_record(r);
        // }

        // .set_columns([
        //     "Ply",
        //     // node 1
        //     "Interior",
        //     Event::NodeTotal.name(),
        //     "Leaf",
        //     "QS Int",
        //     "QS Leaf",
        //     // node 2
        //     "Cut",
        //     "All",
        //     "Pv",
        //     "ZW",
        //     // TT
        //     Event::TtProbeNode.name(),
        //     Event::TtHitNode.name(),
        //     Event::TtStoreNode.name(),
        //     "TT cut",
        //     "TT all",
        //     "TT pv",
        //     "Moves",
        //     // evals
        //     "Eval tt",
        //     "Eval calc",
        //     "Eg draw",
        //     "Eg w/l",
        //     "Eg maybe",
        //     "Eval see",
        //     // pruning/etx
        //     Event::NmpConsider.name(),
        //     Event::NmpDeclineDepth.name(),
        //     Event::NmpDeclineBetaNumeric.name(),
        //     Event::NmpDeclineEvalNumeric.name(),
        //     Event::NmpDeclineEvalMargin.name(),
        //     Event::NmpDeclineMaterial.name(),
        //     Event::NmpDeclineInCheck.name(),
        //     Event::NmpDeclineSuccessive.name(),
        //     Event::NmpDeclineRecursive.name(),
        //     Event::NmpAttempt.name(),
        //     Event::NmpSuccess.name(),
        //     Event::NmpFail.name(),
        //     "Razor D2",
        //     "Razor D3",
        //     "Razor Fail",
        //     "Pat",
        //     "Fut",
        //     Event::LmpSuccess.name(),
        //     "LMR",
        //     "Chk Ext",
        //     // search and re-search
        //     "Search FwFd",
        //     "Search ZwRd",
        //     "Search ZwFd",
        //     "Re-search ZwFd",
        //     "Re-search FwFd",
        //     // Per iter
        //     "Iter Est",
        //     "Iter Act",
        //     "Iter Alloc",
        //     Event::AspirationNone.name(),
        //     Event::Aspiration1.name(),
        //     Event::Aspiration2.name(),
        //     Event::Aspiration3.name(),
        //     Event::AspirationN.name(),
        // ]);

        // let total = iter::once(-1);
        // for y in (0..iters).chain(total) {
        //     let _d = iters - 1 - y;
        //     b = b.add_record([
        //         if y >= 0 {
        //             y.to_string()
        //         } else {
        //             "Total".to_string()
        //         },
        //         // node 1
        //         i(self.interior.for_ply(y)),
        //         i(self.nodes.0[Event::NodeTotal.index()].for_ply(y)),
        //         i(self.leaf.for_ply(y)),
        //         i(self.qs_interior.for_ply(y)),
        //         i(self.qs_leaf.for_ply(y)),
        //         // node 2
        //         i(self.node_cut.for_ply(y)),
        //         i(self.node_all.for_ply(y)),
        //         i(self.node_pv.for_ply(y)),
        //         i(self.node_zw.for_ply(y)),
        //         // TT
        //         i(self.nodes.0[Event::TtProbeNode.index()].for_ply(y)),
        //         i(self.nodes.0[Event::TtHitNode.index()].for_ply(y)),
        //         i(self.nodes.0[Event::TtStoreNode.index()].for_ply(y)),
        //         i(self.tt_cut.for_ply(y)),
        //         i(self.tt_all.for_ply(y)),
        //         i(self.tt_pv.for_ply(y)),
        //         i(self.moves.for_ply(y)),
        //         // eval
        //         i(self.eval_from_tt.for_ply(y)),
        //         i(self.eval_calc.for_ply(y)),
        //         i(self.eval_eg_draw.for_ply(y)),
        //         i(self.eval_eg_win.for_ply(y)),
        //         i(self.eval_eg_maybe.for_ply(y)),
        //         i(self.eval_see.for_ply(y)),
        //         // prune+extend
        //         i(self.nodes.0[Event::NmpConsider.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineDepth.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineBetaNumeric.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineEvalNumeric.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineEvalMargin.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineMaterial.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineInCheck.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineSuccessive.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpDeclineRecursive.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpAttempt.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpSuccess.index()].for_ply(y)),
        //         i(self.nodes.0[Event::NmpFail.index()].for_ply(y)),
        //         i(self.razor_prune_d2.for_ply(y)),
        //         i(self.razor_prune_d3.for_ply(y)),
        //         i(self.razor_prune_fail.for_ply(y)),
        //         i(self.standing_pat_prune.for_ply(y)),
        //         i(self.futility_prune.for_ply(y)),
        //         i(self.nodes.0[Event::LmpSuccess.index()].for_ply(y)),
        //         i(self.late_move_reduce.for_ply(y)),
        //         i(self.check_extend.for_ply(y)),
        //         // search and re-search
        //         i(self.search_fwfd.for_ply(y)),
        //         i(self.search_zwrd.for_ply(y)),
        //         i(self.search_zwfd.for_ply(y)),
        //         i(self.re_search_zwfd.for_ply(y)),
        //         i(self.re_search_fwfd.for_ply(y)),
        //         // per iter
        //         d(self.iter_est.for_ply(y)),
        //         d(self.iter_act.for_ply(y)),
        //         d(self.iter_allotted.for_ply(y)),
        //         i(self.nodes.0[Event::AspirationNone.index()].for_ply(y)),
        //         i(self.nodes.0[Event::Aspiration1.index()].for_ply(y)),
        //         i(self.nodes.0[Event::Aspiration2.index()].for_ply(y)),
        //         i(self.nodes.0[Event::Aspiration3.index()].for_ply(y)),
        //         i(self.nodes.0[Event::AspirationN.index()].for_ply(y)),
        //         // d as u64,
        //         // self.interior.1[d],
        //         // self.leaf.1[d],
        //         // self.qs_interior.1[d],
        //         // self.qs_leaf.1[d],
        //     ]);
        // }
        let style = Style::github_markdown().bottom('-');
        b.build()
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            // .with(Modify::new(Rows::single(0)).with(MaxWidth::wrapping(5).keep_words()))
            // .with(Rotate::Left)
            // .with(Rotate::Top)
            .with(style)
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()))
            // nodes
            .with(Modify::new(Rows::single(9)).with(Border::default().top('-')))
            // TT
            .with(Modify::new(Rows::single(15)).with(Border::default().top('-')))
            .with(Modify::new(Rows::single(22)).with(Border::default().top('-')))
            .with(Modify::new(Rows::single(32)).with(Border::default().top('-')))
            .with(Modify::new(Rows::single(37)).with(Border::default().top('-')))
            .fmt(f)?;
        Ok(())
    }
}

#[dynamic]
static EPOCH: Instant = Instant::now();

impl Metric {
    #[allow(unused_variables)]
    #[inline]
    pub fn record(&self) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().record_metric(self));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn inc_endgame(eg: EndGame) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().endgame.0[eg as usize] += 1);
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr(e: Event) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().events.0[e.index()] += 1);
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr_node(n: &Node, e: Event) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().nodes.0[e.index()].incr(n));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn profile(start: Option<Instant>, e: Event) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD
            .with(|s| s.borrow_mut().profilers.0[e.index()].record(start.unwrap().elapsed()));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn elapsed(ply: Ply, dur: Duration, e: Event) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().durations.0[e.index()].set(ply, dur));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn timing_start() -> Option<Instant> {
        // with metrics 39,302,656,127
        // no metrics   36,113,825,832
        // no metrics   35,733,319,464 but "instant=" #[dynamic] static EPOCH: Instant = Instant::now();
        // no metrics   35,683,293,565 but with option instant
        if cfg!(feature = "remove_metrics") {
            None
        } else {
            Some(Instant::now())
        }
    }
}

thread_local! {
    pub static METRICS_THREAD: RefCell<Metrics>  = RefCell::new(Metrics::new());
}

#[dynamic(lazy)]
static mut METRICS_TOTAL: Box<Metrics> = Box::new(Metrics::new());

#[dynamic(lazy)]
static mut METRICS_LAST_ITER: Box<Metrics> = Box::new(Metrics::new());

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_metrics() {
        Metric::incr(Event::MakeMove);
        Metric::incr(Event::MakeMove);
        Metric::incr_node(
            &Node {
                ply: 1,
                depth: 5,
                ..Node::default()
            },
            Event::NmpSuccess,
        );

        METRICS_THREAD.with(|tm| {
            println!("1. global    \n{}", *METRICS_TOTAL.read());
            println!("thread local \n{}", tm.borrow());
        });

        Metrics::flush_thread_local();

        METRICS_THREAD.with(|tm| {
            println!("2. global    \n{}", *METRICS_TOTAL.read());
            println!("thread local \n{}", tm.borrow());
        });
    }
}
