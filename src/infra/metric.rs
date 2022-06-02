use crate::search::node::Node;
use static_init::dynamic;
use std::cell::RefCell;
use std::cmp::min;
use std::fmt;
use std::ops::AddAssign;
use std::time::Duration;
use tabled::builder::Builder;
use tabled::object::{Rows, Segment};
use tabled::style::Border;
use tabled::{Alignment, Modify, Style};

#[derive(Default, Debug)]
pub struct Histogram(u32);

#[derive(Default, Debug)]
pub struct NodeCounter([u64; 32], [u64; 32]);

impl NodeCounter {
    pub fn incr(&mut self, n: &Node) {
        self.0[min(n.ply, 31) as usize] += 1;
        self.1[min(n.depth, 31) as usize] += 1;
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
    pub null_move_prune: NodeCounter,
    // counters: Vec<u64>,
    // node_counters: Vec<(Vec<u64>, Vec<u64>)>, // ply and depth
    // histograms: Vec<Histogram>,
    timings: Vec<Duration>,
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
            NullMovePrune(ref n) => self.null_move_prune.incr(n),
            LegalMoves(i) => self.make_move += i as u64,
        }
    }

    pub fn add(&mut self, other: &Metrics) {
        self.make_move += other.make_move;
        self.null_move_prune += &other.null_move_prune;
    }

    pub fn to_string() -> String {
        THREAD_METRICS.with(|tm| {
            format!("{}", &*tm.borrow())
        })
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
    NullMovePrune(Node),
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let b = Builder::default()
            .set_columns(["Counter", "Value"])
            .add_record(["Make move", &self.make_move.to_string()])
            .add_record(["Move gen", &self.move_gen.to_string()])
            .add_record(["Hash board", &self.hash_board.to_string()])
            .add_record(["Hash move", &self.hash_move.to_string()])
            .add_record(["Eval", &self.eval.to_string()])
            .add_record(["Iter complete", &self.iter_complete.to_string()])
            .add_record(["Iter timeout", &self.iter_timeout.to_string()]);
        let style = Style::github_markdown().bottom('-');
        let table = b
            .build()
            .with(style)
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::left()));
        table.fmt(f)
    }
}

impl Metric {
    pub fn record(&self) {
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
