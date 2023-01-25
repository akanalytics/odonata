use crate::{
    boards::Board,
    eval::endgame::EndGame,
    infra::utils::{Displayable, Formatting},
    mv::Move,
    piece::{MoveType, Ply},
    search::node::{Counter, Histograms, Node, Timing},
};
use hdrhist::HDRHist;
use itertools::Itertools;
use static_init::dynamic;
use std::{
    cell::RefCell,
    cmp::{max, min},
    fmt, iter,
    time::{Duration, Instant},
};
use strum::{EnumCount, IntoEnumIterator};
use tabled::{
    builder::Builder,
    object::{Columns, Rows, Segment},
    style::{Border, BorderText},
    Alignment, Modify, Style, Table,
};

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
    fn include(&mut self, rhs: &Self) {
        self.0 = HDRHist::combined(self.0.clone(), rhs.0.clone()).clone();
    }
}

impl fmt::Debug for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Histogram")
            // .field(&self.0.summary_string())
            .finish()
    }
}

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
    fn count(&self) -> u64 {
        self.for_ply(-1)
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

    fn include(&mut self, rhs: &Self) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
            self.1[i] += rhs.1[i];
        }
    }
}

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
    fn include(&mut self, rhs: &Self) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
        }
    }
}

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

    fn include(&mut self, new_profile: &Self) {
        self.0 += new_profile.0;
        self.1 += new_profile.1;
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    counters:   Vec<u64>,
    nodes:      Vec<NodeCounter>,
    profilers:  Vec<ProfilerCounter>,
    durations:  Vec<DurationCounter>,
    endgame:    Vec<u64>,
    histograms: Vec<Histogram>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            nodes:      vec![NodeCounter::default(); Event::COUNT],
            counters:   vec![Default::default(); Counter::COUNT],
            profilers:  vec![Default::default(); Timing::COUNT],
            durations:  vec![Default::default(); Event::COUNT],
            endgame:    vec![Default::default(); EndGame::COUNT],
            histograms: vec![Default::default(); 1],
        }
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include(&mut self, o: &Self) {
        for (n1, n2) in self.durations.iter_mut().zip(&o.durations) {
            n1.include(n2);
        }

        for (n1, n2) in self.endgame.iter_mut().zip(&o.endgame) {
            *n1 += n2;
        }
        for (n1, n2) in self.counters.iter_mut().zip(&o.counters) {
            *n1 += n2;
        }

        for (n1, n2) in self.profilers.iter_mut().zip(&o.profilers) {
            n1.include(n2);
        }
        for (n1, n2) in self.nodes.iter_mut().zip(&o.nodes) {
            n1.include(n2);
        }
        for (n1, n2) in self.histograms.iter_mut().zip(&o.histograms) {
            n1.include(n2);
        }
    }

    pub fn summary(&self, pattern: &str) -> String {
        let mut s = String::new();
        for e in Event::iter() {
            let name = e.name();
            if !name.contains(pattern) {
                continue;
            }

            s += &format!(
                "{name:<30} {count}\n",
                count = self.nodes[e.index()].count()
            );
        }
        s.lines().sorted().join("\n")
    }

    pub fn to_string() -> String {
        // let tl = METRICS_THREAD.with(|tm| format!("{}", &*tm.borrow()));
        format!(
            "Global\n{}\n\nLast Iter\n{}",
            &*METRICS_TOTAL.read(),
            &*METRICS_LAST_ITER.read(),
        )
    }

    pub fn write_as_table(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Self::write_profilers_as_table(f, &self.profilers)?;
        Self::write_counters_as_table(f, &self.counters)
    }

    fn write_profilers_as_table(f: &mut fmt::Formatter, profs: &[ProfilerCounter]) -> fmt::Result {
        // Profilers
        //
        fn as_table(profilers: &[ProfilerCounter]) -> Table {
            let mut b = Builder::default();
            b.set_columns(["Counter", "Time %", "Count", "Average", "Total"]);
            for e in Timing::iter() {
                let tot = profilers[Timing::TimingSearchRoot as usize].total();
                if profilers[e.index()].1 != 0 {
                    b.add_record([
                        e.as_ref(),
                        &pd(profilers[e.index()].total(), tot),
                        &i(profilers[e.index()].1),
                        &d(profilers[e.index()].average()),
                        &d(profilers[e.index()].total()),
                    ]);
                }
            }
            let style = Style::markdown().bottom('-');
            b.build()
                .with(style.clone())
                .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
                .with(Modify::new(Segment::all()).with(Alignment::right()))
                .with(Modify::new(Columns::single(0)).with(Alignment::left()))
        }
        let t = as_table(profs);
        writeln!(f, "{t}")
    }

    fn write_counters_as_table(f: &mut fmt::Formatter, counters: &[u64]) -> fmt::Result {
        let mut b = Builder::default();
        b.set_columns(["Counter", "Value"]);

        for e in Counter::iter() {
            b.add_record([e.as_ref(), &match e {
                Counter::EvalCachePercent => perc(
                    counters[Counter::EvalCacheHit.index()],
                    counters[Counter::EvalCacheHit.index()]
                        + counters[Counter::EvalCacheMiss.index()],
                ),
                _ if counters[e.index()] != 0 => i(counters[e.index()]),
                _ => String::new(),
            }]);
        }
        let mut t = b
            .build()
            .with(Style::markdown().bottom('-'))
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t = t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
        }
        writeln!(f, "{t}")
    }

    pub fn get_last_iter() -> Metrics {
        Clone::clone(&METRICS_LAST_ITER.read())
    }

    pub fn flush_thread_local() {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD.with(|tm| {
            METRICS_TOTAL.write().include(&*tm.borrow());
            *METRICS_LAST_ITER.write() = std::mem::take(&mut tm.borrow_mut());
        });
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn add_value(v: u64, h: Histograms) {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD.with(|s| s.borrow_mut().histograms[h as usize].add_value(v));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn inc_endgame(eg: EndGame) {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD.with(|s| s.borrow_mut().endgame[eg as usize] += 1);
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr(e: Counter) {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD.with(|s| s.borrow_mut().counters[e.index()] += 1);
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn classify_move(n: &Node, mv: Move, mt: MoveType, bd: &Board) {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
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
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        {
            use crate::Piece;
            match mv.mover_piece(bd) {
                Piece::Pawn => Self::incr_node(n, Event::MovePawn),
                Piece::Knight => Self::incr_node(n, Event::MoveKnight),
                Piece::Bishop => Self::incr_node(n, Event::MoveBishop),
                Piece::Rook => Self::incr_node(n, Event::MoveRook),
                Piece::Queen => Self::incr_node(n, Event::MoveQueen),
                Piece::King => Self::incr_node(n, Event::MoveKing),
            }
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr_node(n: &Node, e: Event) {
        trace!(target: "metrics", "{n} {e}");
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD.with(|s| s.borrow_mut().nodes[e.index()].add(n, 1));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn add_node(n: &Node, e: Event, i: u64) {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD.with(|s| s.borrow_mut().nodes[e.index()].add(n, i));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn profile(start: Option<Instant>, e: Timing) {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD
            .with(|s| s.borrow_mut().profilers[e as usize].record(start.unwrap().elapsed()));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn elapsed(ply: Ply, dur: Duration, e: Event) {
        #[cfg(all(not(feature = "remove_metrics"), debug_assertions))]
        METRICS_THREAD.with(|s| s.borrow_mut().durations[e.index()].set(ply, dur));
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn timing_start() -> Option<Instant> {
        // with metrics 39,302,656,127
        // no metrics   36,113,825,832
        // no metrics   35,733,319,464 but "instant=" #[dynamic] static EPOCH: Instant = Instant::now();
        // no metrics   35,683,293,565 but with option instant
        if cfg!(all(not(feature = "remove_metrics"), debug_assertions)) {
            Some(Instant::now())
        } else {
            None
        }
    }
}

fn i(i: u64) -> String {
    if i > 0 {
        Formatting::u64(i)
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

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let style = Style::markdown().bottom('-');

        writeln!(
            f,
            "{d}",
            d = Displayable(|f| Self::write_counters_as_table(f, &self.counters))
        )?;

        // Endgame
        //
        let mut b = Builder::default();
        b.set_columns(["Counter", "Value"]);
        for eg in EndGame::iter() {
            b.add_record([&eg.to_string(), &i(self.endgame[eg as usize])]);
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

        // Histograms
        //
        let mut b = Builder::default();
        b.set_columns(["Histogram", "Q1", "Q2", "Q3", "Q4", "Summary"]);
        for x in Histograms::iter() {
            let qs = self.histograms[x as usize]
                .0
                .quantiles([0.25_f64, 0.5, 0.75, 1.0].into_iter())
                .collect_vec();
            b.add_record([
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

        writeln!(
            f,
            "{d}",
            d = Displayable(|f| Self::write_profilers_as_table(f, &self.profilers))
        )?;

        // ply/depth tables
        //
        let generate_table = |corner: &str, by_ply: bool| {
            let mut cols = vec![corner.into()];
            cols.extend((0..32_u32).map(|u| u.to_string()));
            cols.push("Total".into());

            let mut b = Builder::default();
            b.set_columns(cols);
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
                    b.add_record(v);
                }
            }

            let style = Style::markdown().bottom('-');
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
        writeln!(f)?;
        generate_table("Counter \\ Ply", true).fmt(f)?;
        Ok(())
    }
}

thread_local! {
    pub static METRICS_THREAD: RefCell<Metrics>  = RefCell::new(Metrics::new());
}

#[dynamic(lazy)]
pub static mut METRICS_TOTAL: Metrics = Metrics::new();

#[dynamic(lazy)]
static mut METRICS_LAST_ITER: Metrics = Metrics::new();

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_metrics() {
        let t = Metrics::timing_start();
        Metrics::profile(t, Timing::TimingMakeMove);

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
