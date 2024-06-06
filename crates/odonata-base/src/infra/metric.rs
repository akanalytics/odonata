use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::ops::{AddAssign, Deref};
use std::sync::{Arc, Mutex};
use std::{
    // borrow::BorrowMut,
    cell::RefCell,
    cmp::{max, min},
    iter,
    rc::Rc,
    // sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use append_only_vec::AppendOnlyVec;
use crossbeam_utils::atomic::AtomicCell;
use crossbeam_utils::CachePadded;
use hdrhist::HDRHist;
use once_cell::sync::Lazy;
use static_init::dynamic;
use strum::{EnumCount, EnumMessage, IntoEnumIterator};
use tabled::builder::Builder;
use tabled::settings::object::{Columns, Rows, Segment};
use tabled::settings::style::BorderText;
use tabled::settings::{Alignment, Border, Modify, Style};

use super::value::Stats;
pub use crate::domain::node::Event;
use crate::domain::node::{Counter, Histograms, Node, Timing};
use crate::eg::EndGame;
use crate::infra::utils::{DecimalFormatter, DurationFormatter as _, Formatting};
use crate::infra::value::Aggregate;
use crate::piece::MoveType;
use crate::prelude::*;

pub trait Metric: fmt::Debug {}
impl<T: Metric> Metric for Rc<T> {}
impl<T: Metric> Metric for Arc<T> {}
impl<T: Metric> Metric for &T {}

thread_local!(
    static METRICS: RefCell<HashMap<String, Rc<dyn Metric>>> = RefCell::new(HashMap::new());
);

pub fn register_metric<T: Metric + 'static>(name: &str, metric: T) {
    let rc = Rc::new(metric);
    METRICS.with(|m| m.borrow_mut().insert(name.to_string(), rc));
}

pub fn metrics_to_string() -> String {
    let mut s = String::new();
    METRICS.with(|m| {
        m.borrow().iter().for_each(|met| {
            s += met.0;
            s += ":";
            s += &format!("{:?}", met.1); // uses fmt::Debug
            s += "\n";
        })
    });
    s
}

#[derive(Clone, Debug, serde_with::SerializeDisplay)]
pub struct CellThroughput {
    start: Cell<Instant>,
    hits:  Cell<u64>,
}

impl Metric for CellThroughput {}

impl Default for CellThroughput {
    fn default() -> Self {
        Self {
            start: Cell::new(Instant::now()),
            hits:  Cell::default(),
        }
    }
}

impl Display for CellThroughput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/sec", self.per_second().sig_fig(2))
    }
}

impl CellThroughput {
    pub fn hit(&self) {
        self.hits.set(self.hits.get() + 1);
    }

    pub fn per_second(&self) -> f64 {
        self.hits.get() as f64 / self.start.get().elapsed().as_secs_f64()
    }
}

#[derive(Debug, serde_with::SerializeDisplay)]
pub struct AtomicThroughput {
    start: Mutex<Option<Instant>>,
    hits:  AtomicCell<u64>,
}

impl Clone for AtomicThroughput {
    fn clone(&self) -> Self {
        Self {
            start: Mutex::new(*self.start.lock().unwrap()),
            hits:  AtomicCell::new(self.hits.load()),
        }
    }
}

impl Metric for AtomicThroughput {}

impl Default for AtomicThroughput {
    fn default() -> Self {
        Self {
            start: Mutex::new(Some(Instant::now())),
            hits:  AtomicCell::default(),
        }
    }
}

impl Display for AtomicThroughput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/sec", self.per_second().sig_fig(2))
    }
}

impl AtomicThroughput {
    pub const fn new() -> Self {
        Self {
            start: Mutex::new(None),
            hits:  AtomicCell::new(0),
        }
    }

    pub fn start(&self) {
        let mut start = self.start.lock().unwrap();
        if start.is_none() {
            *start = Some(Instant::now());
        }
    }

    pub fn hit(&self) {
        self.hits.fetch_add(1);
    }

    pub fn hit_many(&self, i: u64) {
        self.hits.fetch_add(i);
    }

    pub fn per_second(&self) -> f64 {
        self.hits.load() as f64 / self.start.lock().unwrap().expect("not started").elapsed().as_secs_f64()
    }

    pub fn total(&self) -> u64 {
        self.hits.load()
    }
}

#[cfg(test)]
mod test_throughput {
    use serde::Serialize;
    use test_log::test;

    use super::*;

    #[test]
    fn test_core_metrics() {
        // helper struct
        #[derive(Default, Clone, Debug, Serialize)]
        struct Metrics {
            atomic: AtomicThroughput,
            cell:   CellThroughput,
            unused: CellThroughput,
        }

        impl Metric for Metrics {}

        let m = Metrics::default();
        assert_eq!(m.atomic.to_string(), "0.0/sec");
        assert_eq!(m.cell.to_string(), "0.0/sec");
        let s = serde_json::to_string(&m.atomic).unwrap().trim_matches('"').to_string();
        assert_eq!(s, "0.0/sec");
        for _ in 0..100 {
            m.atomic.hit();
            m.cell.hit();
            std::thread::sleep(Duration::from_millis(1));
        }
        let atomic_per_sec = m.atomic.per_second();
        let cell_per_sec = m.cell.per_second();
        println!("atomic: {a}; cell: {c}", a = m.atomic, c = m.cell);
        assert_eq!(200.0 < atomic_per_sec && atomic_per_sec < 1000.0, true);
        assert_eq!(200.0 < cell_per_sec && cell_per_sec < 1000.0, true);
        println!("{toml}", toml = toml::to_string_pretty(&m).unwrap());
        // use json_to_table::json_to_table;
        // println!(
        //     "{t}",
        //     t = json_into_markup(serde_json::to_value(&m).unwrap())
        // );

        static ATOMIC1: Lazy<AtomicThroughput> = Lazy::new(AtomicThroughput::default);
        let atomic2 = Rc::new(AtomicThroughput::default());
        let atomic3 = Arc::new(AtomicThroughput::default());
        register_metric("atomic1", ATOMIC1.deref());
        register_metric("atomic2", Rc::clone(&atomic2));
        register_metric("atomic3", Arc::clone(&atomic3));
        atomic3.hit();
        atomic2.hit();
        atomic2.hit();
        ATOMIC1.hit();
        println!("{}", metrics_to_string());
    }
}

#[derive(Debug, Default, serde_with::SerializeDisplay)]
pub struct GeneralMetrics {
    counters: Mutex<HashMap<String, i64>>,
}

impl Clone for GeneralMetrics {
    fn clone(&self) -> Self {
        Self {
            counters: Mutex::new(self.counters.lock().unwrap().clone()),
        }
    }
}

impl Metric for GeneralMetrics {}

//
impl Display for GeneralMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (k, v) in self.counters.lock().unwrap().iter() {
            write!(f, "{k} => {v},")?
        }
        Ok(())
    }
}

impl GeneralMetrics {
    pub fn inc(&self, key: &str) {
        self.counters
            .lock()
            .unwrap()
            .entry(key.to_string())
            .or_default()
            .add_assign(1);
    }
    pub fn add(&self, key: &str, i: i64) {
        self.counters
            .lock()
            .unwrap()
            .entry(key.to_string())
            .or_default()
            .add_assign(i);
    }
}

// static GLOBAL_REGISTRY: Lazy<Mutex<MetricsRegistry>> =
//     Lazy::new(|| Mutex::new(MetricsRegistry::default()));

thread_local!(
    pub static THREAD_LOCAL_REGISTRY: RefCell<MetricsRegistry> =
    RefCell::new(MetricsRegistry::default())
);

type SnapshotFunc = dyn Fn() -> Stats;

#[derive(Default)]
pub struct MetricsRegistry {
    observers: Vec<Box<SnapshotFunc>>,
}

impl MetricsRegistry {
    // pub fn register_all_threads<F: Fn() -> Table + Send + Sync + 'static>(f: F) {
    //     let dyn_f: Box<SnapshotFunc> = Box::new(f);
    //     GLOBAL_REGISTRY.lock().unwrap().observers.push(dyn_f)
    // }

    pub fn register_snapshot<F: Fn() -> Stats + 'static>(f: F) {
        // let dyn_f: Mutex<Box<SnapshotFunc>> = Mutex::new(Box::new(f));
        let dyn_f: Box<SnapshotFunc> = Box::new(f);
        THREAD_LOCAL_REGISTRY.with(|reg| reg.borrow_mut().observers.push(dyn_f))
    }

    pub fn snapshot_metrics() -> Stats {
        let mut all_metrics = Stats::new();
        // for closure in GLOBAL_REGISTRY.lock().unwrap().observers.iter() {
        //     let table = closure();
        //     trace!("table = \n{table}");
        //     all_metrics.extend_rows(table.into_rows());
        // }
        THREAD_LOCAL_REGISTRY.with(|reg| {
            for closure in reg.borrow().observers.iter() {
                let stats = closure();
                trace!("threadlocal registry stats = \n{stats}");
                all_metrics.aggregate(stats);
            }
        });
        all_metrics
    }
}

static GLOBAL_COUNTS: AppendOnlyVec<CountMetric> = AppendOnlyVec::new();

#[derive(Default, serde_with::SerializeDisplay)]
// #[serde(from = "(String,i64)", into = "(String, i64)")]
pub struct CountMetric2 {
    count: CachePadded<AtomicCell<i64>>,
}

impl fmt::Display for CountMetric2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.count.load())?;
        Ok(())
    }
}
impl fmt::Debug for CountMetric2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.count.load())
    }
}

impl Clone for CountMetric2 {
    fn clone(&self) -> Self {
        Self {
            count: CachePadded::new(self.count.load().into()),
        }
    }
}

impl CountMetric2 {
    pub const fn new() -> Self {
        Self {
            count: CachePadded::new(AtomicCell::new(0)),
        }
    }

    pub fn get(&self) -> i64 {
        self.count.load()
    }

    pub fn set(&self, val: i64) {
        self.count.store(val)
    }

    pub fn inc(&self) {
        self.count.fetch_add(1);
    }

    pub fn add(&self, x: i64) {
        self.count.fetch_add(x);
    }
}

#[derive(Default, serde_with::SerializeDisplay)]
// #[serde(from = "(String,i64)", into = "(String, i64)")]
pub struct CountMetric {
    name:  String,
    count: CachePadded<AtomicCell<i64>>,
}

impl fmt::Display for CountMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.count.load())?;
        Ok(())
    }
}

impl fmt::Debug for CountMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.count.load())
    }
}

impl Clone for CountMetric {
    fn clone(&self) -> Self {
        Self {
            name:  self.name.clone(),
            count: CachePadded::new(self.count.load().into()),
        }
    }
}

impl From<(String, i64)> for CountMetric {
    fn from(value: (String, i64)) -> Self {
        CountMetric {
            name:  value.0,
            count: CachePadded::new(value.1.into()),
        }
    }
}

impl From<CountMetric> for (String, i64) {
    fn from(val: CountMetric) -> Self {
        (val.name, val.count.load())
    }
}

impl CountMetric {
    fn new(name: &str) -> Self {
        Self {
            name:  name.to_string(),
            count: CachePadded::new(AtomicCell::new(0)),
        }
    }

    fn to_stats(&self) -> Stats {
        let mut s = Stats::new();
        // t[&self.name]["name"] = self.name.to_string().into();
        let count = self.count.load();
        if count > 0 {
            s[&self.name] = count.into();
        }
        s
    }

    fn register(name: &str) -> &'static Self {
        match GLOBAL_COUNTS.iter().find(|cm| cm.name == name) {
            Some(metric) => metric,
            None => {
                let index = GLOBAL_COUNTS.push(Self::new(name));
                let me = &GLOBAL_COUNTS[index];
                MetricsRegistry::register_snapshot(|| me.to_stats());
                me
            }
        }
    }

    pub fn set(&self, value: i64) {
        self.count.store(value);
    }

    pub fn inc(&self) {
        self.count.fetch_add(1);
    }
}

static M3: Lazy<&'static CountMetric> = Lazy::new(|| CountMetric::register("counts.rooks"));

#[cfg(test)]
mod tests2 {
    use test_log::test;

    use super::*;

    #[test]
    fn test_metrics2() {
        M3.inc();
        let m1 = CountMetric::register("counts.pawns");
        let m2 = CountMetric::register("counts.kings");
        m1.inc();
        m1.inc();
        m1.inc();
        m2.inc();

        m2.inc();
        MetricsRegistry::snapshot_metrics();
        let metrics1 = MetricsRegistry::snapshot_metrics();
        println!("{metrics1}");
        let m2 = CountMetric::register("counts.kings");
        m2.inc();

        // older style metrics
        Metrics::incr(Counter::MakeMove);
        let n = Node::root(4);
        Metrics::incr_node(&n, Event::FutilityDeclineInCheck);

        let metrics2 = MetricsRegistry::snapshot_metrics();
        println!("{metrics2}");
        // let mut metrics3 = metrics2.clone() - metrics1;
        // println!("{metrics3}");
        // metrics3 += metrics2;
        // println!("{metrics3}");
        println!("m2 = {}", serde_json::to_string(m2).unwrap());
        // println!("metrics2 = {}", serde_json::to_string(&metrics2).unwrap());
    }
}

#[derive(Clone)]
pub struct Histogram(HDRHist);

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
        self.0 = HDRHist::combined(self.0.clone(), rhs.0.clone());
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
    fn total(&self) -> u64 {
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

impl Metric for ProfilerCounter {}

impl Display for ProfilerCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "average {}", self.average().human())
    }
}

impl ProfilerCounter {
    pub fn record(&mut self, dur: Duration) {
        self.0 += dur;
        self.1 += 1;
    }

    pub fn stop(&mut self, start: Option<Instant>) {
        if let Some(start) = start {
            self.0 += start.elapsed();
            self.1 += 1;
        }
    }

    pub fn average(&self) -> Duration {
        self.0 / self.1 as u32
    }

    pub fn per_second(&self) -> f64 {
        self.1 as f64 / self.0.as_secs_f64()
    }

    pub fn total(&self) -> Duration {
        self.0
    }

    fn include(&mut self, new_profile: &Self) {
        self.0 += new_profile.0;
        self.1 += new_profile.1;
    }
}

impl Metrics {
    pub fn to_uci(&self, filter: &str) -> String {
        let mut uci = vec![];
        for c in Counter::iter() {
            if filter == "*" || filter.contains(&c.to_string()) {
                uci.push(format!("Counter: {c:<25} = {n}", n = self.counters[c.index()]));
                uci.push(format!(
                    "Counter:   {c:<25} = {tot}",
                    tot = self.nodes[c.index()].total()
                ));
            }
        }
        uci.join("\n")
    }

    pub fn parse_uci(_s: &str) -> anyhow::Result<Metrics> {
        Ok(Metrics::default())
    }
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

// MetricsRegistry::register(|| snapshot());

// #[dynamic]
// static METRICS_ENABLED: bool = std::env::var("METRICS").unwrap().eq("TRUE");

// static METRICS_ENABLED: bool = false;

// thread_local!{
//     pub static METRICS_ENABLED: std::cell::Cell<bool>  = std::env::var("METRICS").unwrap().eq("TRUE").into();
// }

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub const fn metrics_enabled() -> bool {
        // if metrics feature enabled or we are a debug build
        // cfg!(any(feature = "metrics", debug_assertions))
        // Self::METRICS_ENABLED.get()
        cfg!(all(feature = "metrics", debug_assertions))
    }

    // pub fn register_arc() -> Arc<Mutex<Self>> {
    //     trace!("Registering metrics...");
    //     let me = Self::new();
    //     let arc = Arc::new(Mutex::new(me));
    //     let another = Arc::clone(&arc);
    //     MetricsRegistry::register_all_threads(move || another.lock().unwrap().snapshot());
    //     arc
    // }

    pub fn register() -> Rc<RefCell<Self>> {
        trace!("Registering metrics...");
        let me = Self::new();
        let arc = Rc::new(RefCell::new(me));
        let another = Rc::clone(&arc);
        MetricsRegistry::register_snapshot(move || another.borrow().to_stats());
        arc
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

    pub fn to_stats(&self) -> Stats {
        let mut stats = Stats::new();
        for e in Counter::iter() {
            let name = e.to_string();
            match e {
                Counter::EvalCachePercent => {
                    let hits = self.counters[Counter::EvalCacheHit.index()];
                    let misses = self.counters[Counter::EvalCacheMiss.index()];
                    if hits + misses > 0 {
                        // stats[&name] = Value::Percent(hits as f64, hits as f64 + misses as f64)
                    }
                }
                _ if self.counters[e.index()] != 0 => stats[&name] = (self.counters[e.index()] as i32).into(),
                _ => {}
            };
        }
        for e in Event::iter() {
            let total = self.nodes[e.index()].for_ply(-1);
            if total != 0 {
                let name = e.to_string();
                stats[&name] = (total as i32).into()
            }
        }
        stats
    }

    // pub fn summary(&self, pattern: &str) -> String {
    //     let mut s = String::new();
    //     for e in Event::iter() {
    //         let name = e.name();
    //         if !name.contains(pattern) {
    //             continue;
    //         }

    //         s += &format!(
    //             "{name:<30} {count}\n",
    //             count = self.nodes[e.index()].total()
    //         );
    //     }
    //     s.lines().sorted().join("\n")
    // }

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
        fn as_table(profilers: &[ProfilerCounter]) -> tabled::Table {
            let mut b = Builder::default();
            b.set_header(["Counter", "Time %", "Count", "Average", "Total"]);
            for e in Timing::iter() {
                let tot = profilers[Timing::TimingSearchRoot as usize].total();
                if profilers[e.index()].1 != 0 {
                    b.push_record([
                        e.as_ref(),
                        &pd(profilers[e.index()].total(), tot),
                        &i(profilers[e.index()].1),
                        &d(profilers[e.index()].average()),
                        &d(profilers[e.index()].total()),
                    ]);
                }
            }
            let style = Style::markdown().bottom('-');
            let mut t = b.build();
            t.with(style)
                .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
                .with(Modify::new(Segment::all()).with(Alignment::right()))
                .with(Modify::new(Columns::single(0)).with(Alignment::left()));
            t
        }
        let t = as_table(profs);
        writeln!(f, "{t}")
    }

    fn write_counters_as_table(f: &mut fmt::Formatter, counters: &[u64]) -> fmt::Result {
        let mut b = Builder::default();
        b.set_header(["Counter", "Value"]);

        for e in Counter::iter() {
            b.push_record([e.as_ref(), &match e {
                Counter::EvalCachePercent => perc(
                    counters[Counter::EvalCacheHit.index()],
                    counters[Counter::EvalCacheHit.index()] + counters[Counter::EvalCacheMiss.index()],
                ),
                _ if counters[e.index()] != 0 => i(counters[e.index()]),
                _ => String::new(),
            }]);
        }
        let mut t = b.build();
        t.with(Style::markdown().bottom('-'))
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
        }
        writeln!(f, "{t}")
    }

    pub fn take_last_iter() -> Metrics {
        if Self::metrics_enabled() {
            Metrics::flush_thread_local();
            Clone::clone(&METRICS_LAST_ITER.read())
        } else {
            Metrics::new()
        }
    }

    pub fn flush_thread_local() {
        if Self::metrics_enabled() {
            METRICS_THREAD.with(|tm| {
                METRICS_TOTAL.write().include(&tm.borrow());
                *METRICS_LAST_ITER.write() = std::mem::take(&mut tm.deref().borrow_mut());
            });
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn add_value(v: u64, h: Histograms) {
        if Self::metrics_enabled() {
            METRICS_THREAD.with(|s| s.deref().borrow_mut().histograms[h as usize].add_value(v));
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn inc_endgame(eg: EndGame) {
        if Self::metrics_enabled() {
            METRICS_THREAD.with(|s| s.deref().borrow_mut().endgame[eg as usize] += 1);
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr(e: Counter) {
        if Self::metrics_enabled() {
            METRICS_THREAD.with(|s| s.deref().borrow_mut().counters[e.index()] += 1);
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn classify_move(n: &Node, mv: Move, mt: MoveType, bd: &Board) {
        if Self::metrics_enabled() {
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
            match mv.mover_piece(bd) {
                Piece::Pawn => Self::incr_node(n, Event::MovePiecePawn),
                Piece::Knight => Self::incr_node(n, Event::MovePieceKnight),
                Piece::Bishop => Self::incr_node(n, Event::MovePieceBishop),
                Piece::Rook => Self::incr_node(n, Event::MovePieceRook),
                Piece::Queen => Self::incr_node(n, Event::MovePieceQueen),
                Piece::King => Self::incr_node(n, Event::MovePieceKing),
            }
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn incr_node(n: &Node, e: Event) {
        if Self::metrics_enabled() {
            trace!(target: "metrics", "{n} {e}");
            METRICS_THREAD.with(|s| s.deref().borrow_mut().nodes[e.index()].add(n, 1));
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn add_node(n: &Node, e: Event, i: u64) {
        if Self::metrics_enabled() {
            METRICS_THREAD.with(|s| s.deref().borrow_mut().nodes[e.index()].add(n, i));
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn profile(start: Option<Instant>, e: Timing) {
        if Self::metrics_enabled() {
            METRICS_THREAD.with(|s| s.deref().borrow_mut().profilers[e as usize].record(start.unwrap().elapsed()));
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn elapsed(ply: Ply, dur: Duration, e: Event) {
        if Self::metrics_enabled() {
            METRICS_THREAD.with(|s| s.deref().borrow_mut().durations[e.index()].set(ply, dur));
        }
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn timing_start() -> Option<Instant> {
        // with metrics 39,302,656,127
        // no metrics   36,113,825,832
        // no metrics   35,733,319,464 but "instant=" #[dynamic] static EPOCH: Instant = Instant::now();
        // no metrics   35,683,293,565 but with option instant
        if Self::metrics_enabled() {
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
        format!("{}%", Formatting::decimal(1, i as f32 * 100.0 / total as f32))
    } else {
        String::new()
    }
}
fn dec(x: u64, y: u64) -> String {
    if y > 0 {
        Formatting::decimal(2, x as f32 / y as f32)
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
        if !Self::metrics_enabled() {
            return Ok(());
        }
        let style = Style::markdown().bottom('-');

        writeln!(
            f,
            "{d}",
            d = Displayable(|f| Self::write_counters_as_table(f, &self.counters))
        )?;

        // Endgame
        //
        let mut b = Builder::default();
        b.set_header(["Counter", "Value"]);
        // for eg in EndGame::iter() {
        //     b.push_record([&eg.to_string(), &i(self.endgame[eg as usize])]);
        // }
        let mut t = b.build();
        t.with(style.clone())
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
        }
        t.fmt(f)?;
        writeln!(f)?;

        // Histograms
        //
        let mut b = Builder::default();
        b.set_header(["Histogram", "Q1", "Q2", "Q3", "Q4", "Summary"]);
        for x in Histograms::iter() {
            let qs = self.histograms[x as usize]
                .0
                .quantiles([0.25_f64, 0.5, 0.75, 1.0].into_iter())
                .collect_vec();
            b.push_record([
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
        let mut t = b.build();
        t.with(style)
            .with(Modify::new(Rows::single(0)).with(Border::default().top('-')))
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()));
        for i in (0..t.shape().0).step_by(5) {
            t.with(Modify::new(Rows::single(i)).with(Border::default().top('-')));
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
            b.set_header(cols);
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
                            self.nodes[Event::TtNodeHit.index()].for_ply(ply),
                            self.nodes[Event::TtNodeMiss.index()].for_ply(ply),
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
                                    (0..=ply).map(|y| self.nodes[Event::NodeTotal.index()].for_ply(y)).sum(),
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
                    b.push_record(v);
                }
            }

            let style = Style::markdown().bottom('-');
            let mut tab = b.build();

            tab.with(Modify::new(Segment::all()).with(Alignment::right()))
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
                    let text = "-".repeat(20) + msg;
                    tab.with(Modify::new(Rows::single(i + 1)).with(Border::default().top('-')))
                        .with(BorderText::new(text).horizontal(1));
                    // i + 1, "-".repeat(20) + msg));
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

#[derive(Debug, Clone)]
pub struct Metrics {
    counters:   Vec<u64>,
    nodes:      Vec<NodeCounter>,
    profilers:  Vec<ProfilerCounter>,
    durations:  Vec<DurationCounter>,
    endgame:    Vec<u64>,
    histograms: Vec<Histogram>,
}

thread_local! {
    pub static METRICS_THREAD: Rc<RefCell<Metrics>>  = Metrics::register();
}

#[allow(clippy::all)]
#[dynamic(lazy)]
pub static mut METRICS_TOTAL: Metrics = Metrics::new();

#[allow(clippy::all)]
#[dynamic(lazy)]
static mut METRICS_LAST_ITER: Metrics = Metrics::new();

#[cfg(test)]
mod tests {
    use test_log::test;

    use super::*;

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
            println!("Metrics:\n{}", tm.borrow().to_uci("*"));
        });

        Metrics::flush_thread_local();

        METRICS_THREAD.with(|tm| {
            println!("2. global    \n{}", *METRICS_TOTAL.read());
            println!("thread local \n{}", tm.borrow());
        });
    }
}
