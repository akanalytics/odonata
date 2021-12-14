use crate::search::history_heuristic::HistoryHeuristic;

use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    cell::Cell,
    fmt,
    sync::atomic::{AtomicU32, AtomicU64},
};

use strum::{EnumCount, IntoEnumIterator};

// use serde::{Deserialize, Serialize};
// use std::fmt;
// use tabwriter::TabWriter;

// use crate::infra::component::Component;
use crate::search::node::{Event, Node};
use crate::types::{Ply, LEN_PLY}; //, MoveType};
                                  // use strum::IntoEnumIterator;
use std::cmp;

// // const MAX_PLY: Ply = 6;

// #[derive(Clone, Serialize, Deserialize)]
// #[serde(default, deny_unknown_fields)]

use static_init::dynamic;
use tabwriter::TabWriter;

#[derive(Debug, Default)]
struct ClonableAtomicU64(AtomicU64);

impl Clone for ClonableAtomicU64 {
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.0.load(Ordering::SeqCst)))
    }
}

impl ClonableAtomicU64 {
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

// impl cmp::PartialOrd for ClonableAtomicU64 {
//     fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
//         let s = self.0.load(Ordering::Relaxed);
//         let o = other.0.load(Ordering::Relaxed);
//         u64::partial_cmp(&s, &o)
//     }
// }

// impl cmp::Ord for ClonableAtomicU64 {
//     fn cmp(&self, other: &Self) -> cmp::Ordering {
//         let s = self.0.load(Ordering::Relaxed);
//         let o = other.0.load(Ordering::Relaxed);
//         u64::cmp(&s, &o)
//     }
// }

// impl cmp::Eq for ClonableAtomicU64 {}

// impl cmp::PartialEq for ClonableAtomicU64 {
//     fn eq(&self, other: &Self) -> bool {
//         self.0.load(Ordering::Relaxed) == other.0.load(Ordering::Relaxed)
//     }
// }

#[dynamic]
static CURRENT_METRICS: EventMetrics = EventMetrics::default();

thread_local! {
    static COUNTERS: EventMetrics = EventMetrics::default();
}

#[derive(Debug)]
pub struct EventMetrics {
    counters: Vec<Vec<Vec<ClonableAtomicU64>>>, // iter, ply, event
    iter: Ply,
    enabled: AtomicBool,
}

impl Default for EventMetrics {
    fn default() -> Self {
        Self {
            counters: vec![vec![vec![ClonableAtomicU64::default(); Event::len()]; 4]; 5],
            iter: 0,
            enabled: AtomicBool::default(),
        }
    }
}

impl EventMetrics {
    pub fn len_ply(&self, iter: usize) -> usize {
        self.counters[iter]
            .iter()
            .rposition(|x| x.into_iter().map(ClonableAtomicU64::get).max() != Some(0))
            .unwrap_or_default()
            + 1
    }

    pub fn max_ply(&self, iter: Ply) -> i32 {
        self.len_ply(iter as usize) as i32
    }

    fn report(&self, events: &[Event]) -> String {
        let write_table = || -> std::io::Result<Vec<u8>> {
            use std::io::Write;
            let table = Vec::new();
            let mut f = TabWriter::new(table).padding(1);
            writeln!(f, "enabled: {}", self.enabled.load(Ordering::SeqCst))?;

            let from = std::cmp::max(self.iter - 1, 0);
            let to = self.iter;
            for iter in from..=to {
                writeln!(f, ".\niter: {}", iter)?;
                for cn in Event::iter() {
                    for y in -1..self.max_ply(iter) {
                        if y < 0 {
                            write!(f, "{}\t", cn.name())?;
                        } else if cn == Event::Unknown {
                            write!(f, "{}\t", y)?;
                        } else {
                            write!(f, "{}\t", self.counters[iter as usize][y as usize][cn.index()].get())?;
                        }
                    }
                    writeln!(f)?;
                }
            }
            f.flush().unwrap();
            Ok(f.into_inner().unwrap())
        };
        let table = write_table().or(Err(fmt::Error)).unwrap();
        format!("{}", std::str::from_utf8(&table).unwrap())
        // "".into()
    }
}

impl Event {
    pub fn inc(&self, y: Ply) {
        COUNTERS.with(|sm| {
            sm.counters[sm.iter as usize][y as usize][self.index()]
                .0
                .fetch_add(1, Ordering::Relaxed)
        });
    }
}

// #[derive(Debug, Clone)]
// pub enum Counter {
//     Nodes,
//     Futility,
// }

#[derive(Debug, Clone)]
pub struct Histogram([u32; 3], [u32; 3], String);

#[derive(Debug, Clone)]
pub struct Derived(u32, String);

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_metrics() {
        Event::HashProbe.inc(0);
        Event::HashProbe.inc(1);
        Event::MovePromo.inc(2);
        Event::MovePromo.inc(2);
        Event::MovePromo.inc(2);

        let s = COUNTERS.with(|cm| {
            assert_eq!(cm.max_ply(0), 3);
            assert_eq!(cm.iter, 0);
            // println!("{:?}", cm);
            EventMetrics::report(cm, &[Event::HashProbe, Event::HashHit])
        });
        println!("Counters\n{}", s);
    }
}

// #[derive(Clone, Debug)]
// pub struct PlyMetrics {
//     nodes: Counter,
//     futility: Counter,
//     cut_move: Histogram,
//     branching_factor: Derived,
// }

// impl EventMetrics {
//     pub fn ply(&self, y: Ply) -> &PlyMetrics {
//         &self.metrics[self.iter as usize][y as usize]
//     }

//     pub fn len_ply(&self) -> usize {
//         self.metrics[self.iter as usize]
//             .iter()
//             .rposition(|x| !x.is_empty())
//             .unwrap_or_default()
//             + 1
//     }

//     pub fn len_iters(&self) -> usize {
//         self.metrics
//             .iter()
//             .rposition(|x| x.iter().all(PlyMetrics::is_empty))
//             .unwrap_or_default()
//             + 1
//     }
// }

// impl Default for PlyMetrics {
//     fn default() -> Self {
//         Self {
//             nodes: Counter(0, "nodes".into()),
//             futility: Counter(0, "futility".into()),
//             cut_move: Histogram([1, 2, 3], [0, 0, 0], "cut move".into()),
//             branching_factor: Derived(0, "branching fac".into()),
//         }
//     }
// }

// impl PlyMetrics {
//     fn calculate(&mut self) {
//         self.branching_factor.0 = self.nodes.0 + self.futility.0;
//     }

//     fn is_empty(&self) -> bool {
//         self.nodes.0 == 0
//     }

//     fn report(&mut self, fields: &[&dyn fmt::Debug]) {
//         self.calculate();
//         for f in fields {
//             println!("{:?}", f);
//         }
//     }
// }

// GLOBAL::MOVE_COUNTER.inc();

// metrics.current().nodes.inc()

// self.stats.current().INTERIOR_NODES.inc();

// self.stats.INTERIOR_NODES.inc(n);

// self.BRANCHING_FACTOR = self.INTERIOR_NODES.value() * self.ALL_NODES.value()

// self.BRANCHING_FACTOR = computed(|| self.INTERIOR_NODES.value() * self.ALL_NODES.value() );
