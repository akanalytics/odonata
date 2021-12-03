use serde::{Deserialize, Serialize};
use std::fmt;
use tabwriter::TabWriter;

use crate::infra::component::Component;
use crate::search::node::{Event, Node};
use crate::types::{Ply, MAX_PLY};
use strum::IntoEnumIterator;

// const MAX_PLY: Ply = 6;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Counts {
    enabled: bool,

    #[serde(skip)]
    iter: Ply,

    #[serde(skip)]
    counts: Box<[[[u64; Event::len()]; MAX_PLY as usize]; MAX_PLY as usize]>,
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            enabled: true,
            iter: 0,
            counts: Box::new([[[0; Event::len()]; MAX_PLY as usize]; MAX_PLY as usize]),
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
            writeln!(f, "{}", self.enabled)?;
            writeln!(f, "{}", self.iter)?;
            let iter = self.iter;
            for cn in Event::iter() {
                for y in -1..self.len_ply(iter) as i32 {
                    if y < 0 {
                        write!(f, "{}\t", cn.name())?;
                    } else if cn == Event::Unknown {
                        write!(f, "{}\t", y)?;
                    } else {
                        write!(f, "{}\t", self.count(self.iter, y, cn))?;
                    }
                }
                writeln!(f)?;
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

    // pub fn is_empty(&self, iter: Ply, y: Ply) -> bool {
    //     self.counts[iter as usize][y as usize].iter().max() == Some(&0u64)
    // }

    #[inline]
    pub fn len_ply(&self, iter: Ply) -> usize {
        self.counts[iter as usize]
            .iter()
            .rposition(|x| x.iter().max() != Some(&0))
            .unwrap_or_default()
            + 1
    }

    pub fn total(&self, i: Ply, cn: Event) -> u64 {
        (0..MAX_PLY).map(|y| self.count(i, y, cn)).sum()
    }

    pub fn cumul(&self, cn: Event) -> u64 {
        use crate::search::node::Event::*;
        match cn {
            PercentBranchingFactor => self.cumul(DerivedLeaf) * 100 / std::cmp::max(1, self.cumul(NodeInterior)),
            _ => (0..MAX_PLY).map(|i| self.total(i, cn)).sum(),
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
            DerivedPrunedInterior => return self.count(i, y, PruneRazor) + self.count(i, y, PruneNullMovePrune),
            DerivedRecog => {
                return self.count(i, y, RecogImmediateDraw) + self.count(i, y, RecogMaybeWin) + self.count(i, y, RecogHelpmateOrDraw)
            }
            PercentPvsReSearch => return (self.count(i, y, PvsReSearch) as f32 / self.count(i, y, Pvs) as f32 * 100.0) as u64,
            PercentLmrReSearch => return (self.count(i, y, LmrReSearch) as f32 / self.count(i, y, Lmr) as f32 * 100.0) as u64,
            PercentPrunedInterior => {
                return (self.count(i, y, DerivedPrunedInterior) as f32 / self.count(i, y, NodeInterior) as f32 * 100.0) as u64
            }
            PercentHashHit => return (self.count(i, y, HashHit) as f32 / self.count(i, y, HashProbe) as f32 * 100.0) as u64,
            DerivedAspiration => {
                return self.count(i, y, Aspiration1)
                    + self.count(i, y, Aspiration2)
                    + self.count(i, y, Aspiration3)
                    + self.count(i, y, AspirationN)
            }
            PercentAspiration1 => {
                return (self.count(i, y, Aspiration1) as f32 / self.count(i, y, DerivedAspiration) as f32 * 100.0) as u64
            }
            _ => {}
        }
        self.counts[i as usize][y as usize][cn.index()]
    }

    #[inline]
    #[allow(unused_variables)]
    pub fn inc_by_ply(&mut self, y: Ply, cn: Event) {
        #[cfg(not(feature = "remove_metrics"))]
        {
            self.counts[self.iter as usize][y as usize][cn as usize] += 1;
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
}
