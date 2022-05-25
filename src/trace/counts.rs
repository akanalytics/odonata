use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use tabwriter::TabWriter;

use crate::infra::component::Component;
use crate::search::node::{Event, Node};
use crate::types::{MoveType, Ply, LEN_PLY, MAX_PLY};
use strum::IntoEnumIterator;

// const MAX_PLY: Ply = 6;

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

thread_local! {
    static COUNTS: Counts = Counts::default();
}

impl Event {
    pub fn incr_by_ply(&self, y: Ply) {
        COUNTS.with(|c| {
            c.data[c.iter as usize][y as usize][self.index()]
                .0
                .fetch_add(1, Ordering::Relaxed)
        });
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Counts {
    enabled: bool,

    #[serde(skip)]
    iter: Ply,

    #[serde(skip)]
    data: Vec<Vec<Vec<ClonableAtomicU64>>>,
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            enabled: true,
            iter: 0,
            data: vec![vec![vec![ClonableAtomicU64::default(); Event::len()]; LEN_PLY]; LEN_PLY],
        }
    }
}

impl fmt::Debug for Counts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Counts")
            .field("enabled", &self.enabled)
            .field("iter", &self.iter)
            .finish()
    }
}

// impl Display for Table {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         let write_table = || -> io::Result<Vec<u8>> {
//             let table = Vec::new();
//             let mut tf = TabWriter::new(table).padding(1);

//             for r in self.0.iter() {
//                 let mut ci = r.iter();
//                 if let Some(c0) = ci.next() {
//                     write!(tf, "{}", c0)?;
//                     for c in ci {
//                         write!(tf, "\t{}", c)?;
//                     }
//                 }
//                 writeln!(tf)?;
//             }

//             tf.flush().unwrap();
//             Ok(tf.into_inner().unwrap())
//         };

//         let table = write_table().or(Err(fmt::Error))?;
//         write!(f, "{}", std::str::from_utf8(&table).unwrap())
//     }

impl fmt::Display for Counts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let write_table = || -> std::io::Result<Vec<u8>> {
            use std::io::Write;
            let table = Vec::new();
            let mut f = TabWriter::new(table).padding(1);
            writeln!(f, "enabled: {}", self.enabled)?;

            let from = 0; // std::cmp::max(self.iter - 1, 0);
            let to = self.iter;
            for iter in from..=to {
                writeln!(f, ".\niter: {}", iter)?;
                for cn in Event::iter() {
                    for y in -1..self.len_ply(iter) as i32 {
                        if y < 0 {
                            write!(f, "{}\t", cn.name())?;
                        } else if cn == Event::Unknown {
                            write!(f, "{}\t", y)?;
                        } else {
                            write!(f, "{}\t", self.count(iter, y, cn))?;
                        }
                    }
                    writeln!(f)?;
                }
            }
            f.flush().unwrap();
            Ok(f.into_inner().unwrap())
        };
        let table = write_table().or(Err(fmt::Error))?;
        write!(f, "{}", std::str::from_utf8(&table).unwrap())
    }
}

impl Component for Counts {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_iter(&mut self) {
        self.iter += 1;
    }

    fn new_position(&mut self) {
        *self = Self::default();
    }
}

impl Counts {
    pub fn inc(&mut self, n: &Node, cn: Event) {
        self.inc_by_ply(n.ply, cn);
    }

    pub fn inc_move(&self, n: &Node, mt: MoveType) {
        let event = match mt {
            MoveType::Start => Event::MoveStart,
            MoveType::Hash => Event::MoveHash,
            MoveType::Initialize => Event::MoveInitialize,
            MoveType::GoodCapture => Event::MoveGoodCapture,
            MoveType::GoodCaptureUpfrontSorted => Event::MoveGoodCaptureUpfrontSorted,
            MoveType::Killer => Event::MoveKiller,
            MoveType::Null => Event::MoveNull,
            MoveType::Promo => Event::MovePromo,
            MoveType::QueenPromo => Event::MovePromo,
            MoveType::Evasion => Event::MoveEvasion,
            MoveType::Quiet => Event::MoveQuiet,
            MoveType::QuietUnsorted => Event::MoveQuietUnsorted,
            MoveType::BadCapture => Event::MoveBadCapture,
            MoveType::Remaining => Event::MoveRemaining,
            MoveType::Unsorted => Event::MoveUnsorted,
            MoveType::Capture => Event::MoveCapture,
            MoveType::End => Event::MoveEnd,
        };
        self.inc_by_ply(n.ply, event);
    }
    // pub fn is_empty(&self, iter: Ply, y: Ply) -> bool {
    //     self.counts[iter as usize][y as usize].iter().max() == Some(&0u64)
    // }

    #[inline]
    pub fn len_ply(&self, iter: Ply) -> usize {
        self.data[iter as usize]
            .iter()
            .rposition(|x| x.iter().map(ClonableAtomicU64::get).max() != Some(0))
            .unwrap_or_default()
            + 1
    }

    pub fn total(&self, i: Ply, cn: Event) -> u64 {
        (0..MAX_PLY).map(|y| self.count(i, y, cn)).sum()
    }

    pub fn cumul(&self, cn: Event) -> u64 {
        use crate::search::node::Event::*;
        match cn {
            PercentBranchingFactor => {
                self.cumul(DerivedLeaf) * 100 / std::cmp::max(1, self.cumul(NodeInterior))
            }
            _ => (0..MAX_PLY).map(|i| self.total(i, cn)).sum(),
        }
    }

    fn percent(top: u64, bottom: u64) -> u64 {
        if top == 0 {
            0
        } else {
            (top as f32 / bottom as f32 * 100.0) as u64
        }
    }

    pub fn count(&self, i: Ply, y: Ply, cn: Event) -> u64 {
        use crate::search::node::Event::*;
        match cn {
            DerivedLeaf => {
                return self.count(i, y, NodeLeafDraw)
                    + self.count(i, y, NodeLeafWinLoss)
                    + self.count(i, y, NodeLeafStalemate)
                    + self.count(i, y, RecogImmediateDraw)
                    + self.count(i, y, NodeLeafQuietEval)
                    + self.count(i, y, DerivedPrunedInterior)
                    + self.count(i, y, PruneStandingPat)
            }

            // just node pruning
            DerivedPrunedInterior => {
                return self.count(i, y, PruneRazor) + self.count(i, y, PruneNullMovePrune)
            }
            DerivedRecog => {
                return self.count(i, y, RecogImmediateDraw)
                    + self.count(i, y, RecogMaybeWin)
                    + self.count(i, y, RecogHelpmateOrDraw)
            }
            PercentPvsReSearch => {
                return Self::percent(self.count(i, y, PvsReSearch), self.count(i, y, Pvs))
            }
            PercentLmrReSearch => {
                return Self::percent(self.count(i, y, LmrReSearch), self.count(i, y, Lmr))
            }
            PercentPrunedInterior => {
                return Self::percent(
                    self.count(i, y, DerivedPrunedInterior),
                    self.count(i, y, NodeInterior),
                )
            }
            PercentHashHit => {
                return Self::percent(self.count(i, y, HashHit), self.count(i, y, HashProbe))
            }
            PercentAspiration1 => {
                return Self::percent(
                    self.count(i, y, Aspiration1),
                    self.count(i, y, DerivedAspiration),
                )
            }
            DerivedAspiration => {
                return self.count(i, y, Aspiration1)
                    + self.count(i, y, Aspiration2)
                    + self.count(i, y, Aspiration3)
                    + self.count(i, y, AspirationN)
            }
            _ => {}
        }
        self.data[i as usize][y as usize][cn.index()].get()
    }

    #[inline]
    #[allow(unused_variables)]
    pub fn inc_by_ply(&self, y: Ply, cn: Event) {
        #[cfg(not(feature = "remove_metrics"))]
        {
            self.data[self.iter as usize][y as usize][cn as usize]
                .0
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counts() {
        let mut counters = Counts::default();

        counters.inc_by_ply(4, Event::PruneRazor);
        counters.new_iter();
        counters.new_iter();
        let n = &mut Node::root(5);
        n.ply = 6;
        counters.inc(n, Event::NodeInterior);
        counters.inc(n, Event::NodeLeafDraw);
        counters.inc(n, Event::NodeLeafStalemate);
        counters.inc(n, Event::NodeLeafStalemate);
        println!("{:#?}", counters);
        println!("{}", counters);
        // Count::Razor.inc(4);
        assert_eq!(counters.len_ply(0), 5);
        assert_eq!(counters.len_ply(2), 7);
        assert_eq!(counters.count(2, 6, Event::NodeLeafStalemate), 2);
        assert_eq!(counters.total(2, Event::NodeLeafStalemate), 2);
        assert_eq!(counters.total(2, Event::DerivedLeaf), 3);
        assert_eq!(counters.cumul(Event::DerivedLeaf), 4);
        assert_eq!(counters.cumul(Event::PercentBranchingFactor), 400);
    }

    #[test]
    fn test_thread_local_counts() {
        Event::HashProbe.incr_by_ply(0);
        Event::HashProbe.incr_by_ply(1);
        Event::MovePromo.incr_by_ply(2);
        Event::MovePromo.incr_by_ply(2);
        Event::MovePromo.incr_by_ply(2);

        COUNTS.with(|c| {
            println!("{}", c);
            assert_eq!(c.count(0, 2, Event::MovePromo), 3);
        });
    }
}
