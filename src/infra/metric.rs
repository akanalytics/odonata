use crate::eval::endgame::EndGame;
use crate::mv::MoveDetail;
use crate::piece::{MoveType, Ply};
use crate::search::node::{Counter, Histograms, Node, Timing};
use crate::utils::Formatting;
use hdrhist::HDRHist;
use itertools::Itertools;
use static_init::dynamic;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::ops::AddAssign;
use std::time::{Duration, Instant};
use std::{fmt, iter};
use strum::{EnumCount, IntoEnumIterator};
use tabled::builder::Builder;
use tabled::object::{Columns, Rows, Segment};
use tabled::style::{Border, BorderText};
use tabled::{Alignment, Modify, Style};

pub use crate::search::node::Event;
use strum::EnumMessage;

#[derive(Clone)]
struct Histogram(HDRHist);

impl Default for Histogram {
    fn default() -> Self {
        Self(HDRHist::new())
    }
}

impl Histogram {
    #[allow(dead_code)]
    pub fn add_value(&mut self, c: u64) {
        self.0.add_value(c);
    }
}

impl fmt::Debug for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Histogram")
            // .field(&self.0.summary_string())
            .finish()
    }
}

impl AddAssign<&Histogram> for Histogram {
    fn add_assign(&mut self, rhs: &Self) {
        self.0 = HDRHist::combined(self.0.clone(), rhs.0.clone()).clone();
    }
}

//
// ArrayOf
//
#[derive(Debug, Clone)]
struct ArrayOf<const N: usize, T>([T; N]);

impl<const N: usize, T: Default> Default for ArrayOf<{ N }, T> {
    fn default() -> Self {
        // Self([T::default(); N])
        Self([(); N].map(|_| T::default())) // no copy needed
    }
}

impl<const N: usize, T> AddAssign<&ArrayOf<{ N }, T>> for ArrayOf<{ N }, T>
where
    for<'a> T: AddAssign<&'a T>,
{
    fn add_assign(&mut self, rhs: &Self) {
        for i in 0..self.0.len() {
            self.0[i] += &rhs.0[i];
        }
    }
}

impl<const N: usize> ArrayOf<N, u64>
where
    [u64; N]: Default,
{
    // fn incr(&mut self, i: usize) {
    //     self.0[min(i, N - 1) as usize] += 1;
    // }
}

//
// Node counter
//
#[derive(Default, Debug, Clone, Copy)]
pub struct NodeCounter([u64; 32], [u64; 32]);

impl NodeCounter {
    pub fn add(&mut self, n: &Node, i: u64) {
        self.0[min(n.ply, 31) as usize] += i;
        self.1[min(max(n.depth, 0), 31) as usize] += i;
    }

    // -1 => total
    fn for_ply(&self, y: isize) -> u64 {
        if y >= 0 {
            self.0[min(y, 31) as usize]
        } else {
            self.0.iter().sum()
        }
    }

    // -1 => total
    fn for_depth(&self, y: isize) -> u64 {
        if y >= 0 {
            self.1[min(y, 31) as usize]
        } else {
            self.1.iter().sum()
        }
    }
}

impl AddAssign<&NodeCounter> for NodeCounter {
    fn add_assign(&mut self, rhs: &Self) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
            self.1[i] += rhs.1[i];
        }
    }
}

//
// DurationCounter
//
#[derive(Default, Debug, Clone, Copy)]
pub struct DurationCounter([Duration; 32]);

impl DurationCounter {
    pub fn set(&mut self, y: Ply, dur: Duration) {
        self.0[min(y, 31) as usize] = dur;
    }

    // fn add(&mut self, y: Ply, dur: Duration) {
    //     self.0[min(y, 31) as usize] += dur;
    // }

    // // -1 => total
    // fn for_ply(&self, y: isize) -> Duration {
    //     if y >= 0 {
    //         self.0[min(y, 31) as usize]
    //     } else {
    //         self.0.iter().sum()
    //     }
    // }
}

impl AddAssign<&DurationCounter> for DurationCounter {
    fn add_assign(&mut self, rhs: &Self) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
        }
    }
}

//
// Profile Counter
//
#[derive(Default, Debug, Clone, Copy)]
pub struct ProfilerCounter(Duration, u64);

impl ProfilerCounter {
    pub fn record(&mut self, dur: Duration) {
        self.0 += dur;
        self.1 += 1;
    }

    fn average(&self) -> Duration {
        self.0 / self.1 as u32
    }

    fn total(&self) -> Duration {
        self.0
    }
}

impl AddAssign<&ProfilerCounter> for ProfilerCounter {
    fn add_assign(&mut self, rhs: &Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    counters: ArrayOf<{ Counter::COUNT }, u64>,
    nodes: Vec<NodeCounter>,
    profilers: ArrayOf<{ Timing::COUNT }, ProfilerCounter>,
    durations: ArrayOf<{ Event::len() }, DurationCounter>,
    endgame: ArrayOf<{ EndGame::COUNT }, u64>,
    histograms: Vec<Histogram>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            nodes: vec![NodeCounter::default(); Event::len()],
            counters: Default::default(),
            profilers: Default::default(),
            durations: Default::default(),
            endgame: Default::default(),
            histograms: vec![Default::default(); 1],
        }
    }
}

impl Metrics {
    fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, o: &Self) {
        self.counters += &o.counters;
        self.profilers += &o.profilers;
        self.durations += &o.durations;
        self.endgame += &o.endgame;

        for (n1, n2) in self.nodes.iter_mut().zip(&o.nodes) {
            *n1 += &n2;
        }
        for (n1, n2) in self.histograms.iter_mut().zip(&o.histograms) {
            *n1 += &n2;
        }
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

    #[allow(unused_variables)]
    #[inline]
    pub fn add_value(v: u64, h: Histograms) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().histograms[h as usize].add_value(v));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn inc_endgame(eg: EndGame) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().endgame.0[eg as usize] += 1);
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr(e: Counter) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().counters.0[e.index()] += 1);
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn classify_move(n: &Node, mv: MoveDetail, mt: MoveType) {
        #[cfg(not(feature = "remove_metrics"))]
        {
            let ev = match mt {
                MoveType::GoodCapture => Event::MoveGoodCapture,
                MoveType::GoodCaptureUpfrontSorted => Event::MoveGoodCapture,
                MoveType::Hash => Event::MoveHash,
                MoveType::Killer => Event::MoveKiller,
                MoveType::BadCapture => Event::MoveBadCapture,
                MoveType::Quiet => Event::MoveQuiet,
                _ => Event::MoveOther,
            };

            Self::incr_node(n, ev);

            if mv.is_null() {
                Self::incr_node(n, Event::MoveNull);
                return;
            }
            if mv.is_promo() {
                Self::incr_node(n, Event::MovePromo)
            }
            if mv.is_capture() {
                Self::incr_node(n, Event::MoveCapture)
            }
        }
        #[cfg(not(feature = "remove_metrics"))]
        {
            use crate::Piece;
            match mv.mover_piece() {
                Piece::Pawn => Self::incr_node(n, Event::MovePawn),
                Piece::Knight => Self::incr_node(n, Event::MoveKnight),
                Piece::Bishop => Self::incr_node(n, Event::MoveBishop),
                Piece::Rook => Self::incr_node(n, Event::MoveRook),
                Piece::Queen => Self::incr_node(n, Event::MoveQueen),
                Piece::King => Self::incr_node(n, Event::MoveKing),
                _ => unreachable!(),
            }
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr_node(n: &Node, e: Event) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().nodes[e.index()].add(n, 1));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn add_node(n: &Node, e: Event, i: u64) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD.with(|s| s.borrow_mut().nodes[e.index()].add(n, i));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn profile(start: Option<Instant>, e: Timing) {
        #[cfg(not(feature = "remove_metrics"))]
        METRICS_THREAD
            .with(|s| s.borrow_mut().profilers.0[e as usize].record(start.unwrap().elapsed()));
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

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn i(i: u64) -> String {
            if i > 0 {
                Formatting::u128(i as u128)
            } else {
                String::new()
            }
        }
        fn perc(i: u64, total: u64) -> String {
            if total > 0 {
                format!(
                    "{}%",
                    Formatting::decimal(1, i as f32 * 100.0 / total as f32)
                )
            } else {
                String::new()
            }
        }
        fn dec(x: u64, y: u64) -> String {
            if y > 0 {
                format!("{}", Formatting::decimal(2, x as f32 / y as f32))
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

        //
        // Counter
        //
        let mut b = Builder::default().set_columns(["Counter", "Value"]);

        for e in Counter::iter() {
            b = b.add_record([
                e.as_ref(),
                &match e {
                    Counter::EvalCachePercent => perc(
                        self.counters.0[Counter::EvalCacheHit.index()],
                        self.counters.0[Counter::EvalCacheHit.index()]
                            + self.counters.0[Counter::EvalCacheMiss.index()],
                    ),
                    _ if self.counters.0[e.index()] != 0 => i(self.counters.0[e.index()]),
                    _ => String::new(),
                },
            ]);
        }
        let mut t = b
            .build()
            .with(style.clone())
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t = t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
        }
        t.fmt(f)?;
        writeln!(f)?;

        //
        // Endgame
        //
        let mut b = Builder::default().set_columns(["Counter", "Value"]);
        for eg in EndGame::iter() {
            b = b.add_record([&eg.to_string(), &i(self.endgame.0[eg as usize])]);
        }
        let mut t = b
            .build()
            .with(style.clone())
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t = t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
        }
        t.fmt(f)?;
        writeln!(f)?;

        //
        // Histograms
        //
        let mut b =
            Builder::default().set_columns(["Histogram", "Q1", "Q2", "Q3", "Q4", "Summary"]);
        for x in Histograms::iter() {
            let qs = self.histograms[x as usize]
                .0
                .quantiles([0.25_f64, 0.5, 0.75, 1.0].into_iter())
                .collect_vec();
            b = b.add_record([
                &x.to_string(),
                &i(qs[0].1),
                &i(qs[1].1),
                &i(qs[2].1),
                &i(qs[3].1),
                // &self.histograms[x as usize]
                //     .0
                //     .ccdf()
                //     .map(|(a, b, c)| format!("{a:>6} {} {c}", Formatting::decimal(2, b)))
                //     .join("\n"),
            ]);
        }
        let mut t = b
            .build()
            .with(style.clone())
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t = t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
        }
        t.fmt(f)?;
        writeln!(f)?;

        //
        //Profilers
        //
        let mut b =
            Builder::default().set_columns(["Counter", "Time %", "Count", "Average", "Total"]);
        for e in Timing::iter() {
            let tot = self.profilers.0[Timing::TimingSearchRoot as usize].total();
            if self.profilers.0[e.index()].1 != 0 {
                b = b.add_record([
                    e.as_ref(),
                    &pd(self.profilers.0[e.index()].total(), tot),
                    &i(self.profilers.0[e.index()].1),
                    &d(self.profilers.0[e.index()].average()),
                    &d(self.profilers.0[e.index()].total()),
                ]);
            }
        }
        let t = b
            .build()
            .with(style.clone())
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        t.fmt(f)?;
        writeln!(f)?;

        //
        // ply/depth tables
        //
        let generate_table = |corner: &str, by_ply: bool| {
            let mut cols = vec![corner.into()];
            cols.extend((0..32_u32).map(|u| u.to_string()));
            cols.push("Total".into());

            let mut b = Builder::default().set_columns(cols);
            for e in Event::iter() {
                let mut v = vec![];
                v.push(e.name().to_string());
                let total = iter::once(-1);
                let iters = 32_isize;

                for ply in (0..iters).chain(total) {
                    let num = if by_ply {
                        self.nodes[e.index()].for_ply(ply)
                    } else {
                        self.nodes[e.index()].for_depth(ply)
                    };

                    let s = match e {
                        Event::TtHitRate => perc(
                            self.nodes[Event::TtHitNode.index()].for_ply(ply),
                            self.nodes[Event::TtProbeNode.index()].for_ply(ply),
                        ),
                        Event::NodeNmpPerc => perc(
                            self.nodes[Event::NmpSuccess.index()].for_ply(ply),
                            self.nodes[Event::NodeInterior.index()].for_ply(ply),
                        ),
                        Event::NodeRazorPerc => perc(
                            self.nodes[Event::RazorSuccess.index()].for_ply(ply),
                            self.nodes[Event::NodeInterior.index()].for_ply(ply),
                        ),
                        Event::NodeRevFutPerc => perc(
                            self.nodes[Event::RevFutSuccess.index()].for_ply(ply),
                            self.nodes[Event::NodeInterior.index()].for_ply(ply),
                        ),
                        Event::MeanBranchingFactor => {
                            if self.nodes[Event::NodeTotal.index()].for_ply(ply) > 0 {
                                dec(
                                    (0..=ply)
                                        .map(|y| self.nodes[Event::NodeTotal.index()].for_ply(y))
                                        .sum(),
                                    (0..=ply)
                                        .map(|y| self.nodes[Event::NodeInterior.index()].for_ply(y))
                                        .sum(),
                                )
                            } else {
                                String::new()
                            }
                        }
                        Event::EffectiveBranchingFactor => dec(
                            self.nodes[Event::NodeTotal.index()].for_ply(ply),
                            if ply >= 1 {
                                self.nodes[Event::NodeTotal.index()].for_ply(ply - 1)
                            } else {
                                0
                            },
                        ),
                        _ => i(num),
                    };
                    v.push(s);
                }
                // only add row if non-empty
                if v.iter().any(|s| !s.is_empty()) {
                    b = b.add_record(v);
                }
            }

            let style = Style::github_markdown().bottom('-');
            let mut tab = b
                .build()
                .with(Modify::new(Segment::all()).with(Alignment::right()))
                // .with(Modify::new(Rows::single(0)).with(MaxWidth::wrapping(5).keep_words()))
                // .with(Rotate::Left)
                // .with(Rotate::Top)
                .with(style)
                .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
                .with(Modify::new(Columns::single(0)).with(Alignment::left()));
            // nodes

            // loop through again adding some titles
            for (i, e) in Event::iter()
                // .filter(|e| {
                //     if by_ply {
                //         self.nodes[e.index()].for_ply(-1) != 0
                //     } else {
                //         self.nodes[e.index()].for_depth(-1) != 0
                //     }
                // })
                .enumerate()
            {
                if let Some(msg) = e.get_message() {
                    tab = tab
                        .with(Modify::new(Rows::single(i + 1)).with(Border::default().top('-')))
                        .with(BorderText::new(i + 1, "-".repeat(20) + msg));
                }
            }
            tab
        };

        generate_table("Counter \\ Depth", false).fmt(f)?;
        generate_table("Counter \\ Ply", true).fmt(f)?;
        Ok(())
    }
}

thread_local! {
    pub static METRICS_THREAD: RefCell<Metrics>  = RefCell::new(Metrics::new());
}

#[dynamic(lazy)]
pub static mut METRICS_TOTAL: Box<Metrics> = Box::new(Metrics::new());

#[dynamic(lazy)]
static mut METRICS_LAST_ITER: Box<Metrics> = Box::new(Metrics::new());

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_metrics() {
        Metrics::incr(Counter::MakeMove);
        Metrics::incr(Counter::MakeMove);
        Metrics::incr_node(
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
