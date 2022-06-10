use crate::eval::endgame::EndGame;
use crate::search::node::{Node, Timing};
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
use tabled::style::{Border, BorderText};
use tabled::{Alignment, Modify, Style};

pub use crate::search::node::Event;
use strum::EnumMessage;

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

impl<T: Default + Copy> Default for ArrayCounter<{ Timing::COUNT }, T> {
    fn default() -> Self {
        Self([T::default(); Timing::COUNT])
    }
}

impl<T: Default + Copy> Default for ArrayCounter<{ Event::COUNT }, T> {
    fn default() -> Self {
        Self([T::default(); Event::COUNT])
    }
}

impl<const N: usize, T> AddAssign<&ArrayCounter<{ N }, T>> for ArrayCounter<{ N }, T>
where
    for<'a> T: AddAssign<&'a T>,
{
    fn add_assign(&mut self, rhs: &Self) {
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
    fn add_assign(&mut self, rhs: &Self) {
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
    fn add_assign(&mut self, rhs: &Self) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i];
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ProfilerCounter(Duration, u64);

impl ProfilerCounter {
    pub fn record(&mut self, dur: Duration) {
        self.0 += dur;
        self.1 += 1;
    }

    pub fn average(&self) -> Duration {
        self.0 / self.1 as u32
    }

    pub fn total(&self) -> Duration {
        self.0
    }
}

impl AddAssign<&ProfilerCounter> for ProfilerCounter {
    fn add_assign(&mut self, rhs: &Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

#[derive(Default, Debug, Clone)]
pub struct Metrics {
    events: ArrayCounter<{ Event::len() }, u64>,
    nodes: ArrayCounter<{ Event::len() }, NodeCounter>,
    profilers: ArrayCounter<{ Timing::COUNT }, ProfilerCounter>,
    durations: ArrayCounter<{ Event::len() }, TimePlyCounter>,
    endgame: ArrayCounter<{ EndGame::COUNT }, u64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, o: &Self) {
        self.events += &o.events;
        self.nodes += &o.nodes;
        self.profilers += &o.profilers;
        self.durations += &o.durations;
        self.endgame += &o.endgame;
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
pub enum Metric {}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn i(i: u64) -> String {
            if i > 0 {
                Formatting::u128(i as u128)
            } else {
                String::new()
            }
        }
        fn _d(dur: Duration) -> String {
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
        let mut b = Builder::default().set_columns(["Counter", "Value"]);

        for e in Event::iter() {
            if self.events.0[e.index()] != 0 {
                b = b.add_record([e.name(), &i(self.events.0[e.index()])]);
            }
        }
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

        let mut b = Builder::default().set_columns(["Counter", "Time %", "Count"]);
        for e in Timing::iter() {
            let tot = self.profilers.0[Timing::TimingSearchRoot as usize].total();
            if self.profilers.0[e.index()].1 != 0 {
                b = b.add_record([
                    e.as_ref(),
                    &pd(self.profilers.0[e.index()].total(), tot),
                    &i(self.profilers.0[e.index()].1),
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

        let mut cols = vec!["Counter \\ Ply".into()];
        cols.extend((0..32_u32).map(|u| u.to_string()));
        cols.push("Total".into());

        let mut b = Builder::default().set_columns(cols);
        for e in Event::iter() {
            if self.nodes.0[e.index()].for_ply(-1) == 0 {
                continue;
            }
            let mut v = vec![];
            v.push(e.name().to_string());
            let total = iter::once(-1);
            let iters = 32_isize;

            for ply in (0..iters).chain(total) {
                v.push(i(self.nodes.0[e.index()].for_ply(ply)))
            }
            b = b.add_record(v);
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

        for (i, e) in Event::iter()
            .filter(|e| self.nodes.0[e.index()].for_ply(-1) != 0)
            .enumerate()
        {
            if let Some(msg) = e.get_message() {
                tab = tab
                    .with(Modify::new(Rows::single(i + 1)).with(Border::default().top('-')))
                    .with(BorderText::new(i + 1, "-----".to_string() + msg));
            }
        }
        tab.fmt(f)?;
        Ok(())
    }
}

#[dynamic]
static EPOCH: Instant = Instant::now();

impl Metric {
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
