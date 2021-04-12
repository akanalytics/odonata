use crate::config::{Config, Configurable};
use crate::log_debug;
use crate::movelist::MoveList;
use crate::search::algo::{Algo, Node};
use crate::stat::{PlyStat, ArrayPlyStat};
use std::fmt;

#[derive(Clone, Debug)]
pub struct MoveOrderer {
    enabled: bool,
    prior_pv: bool,
    prior_bm: bool,
    count_pv: PlyStat,
    count_bm: PlyStat,
}

impl Configurable for MoveOrderer {
    fn settings(&self, c: &mut Config) {
        c.set("move_orderer.enabled", "type check default true");
        c.set("move_orderer.prior_pv", "type check default true");
        c.set("move_orderer.prior_bm", "type check default true");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("move_orderer.configure with {}", c);
        self.enabled = c.bool("move_orderer.enabled").unwrap_or(self.enabled);
        self.prior_bm = c.bool("move_orderer.prior_bm").unwrap_or(self.prior_bm);
        self.prior_pv = c.bool("move_orderer.prior_pv").unwrap_or(self.prior_pv);
    }
}

impl MoveOrderer {
    // pub const MOVE_SORTS: &'static [&'static str] = &["Natural", "PV from Prior Iteration", "MVV/LVA"];
}

impl Default for MoveOrderer {
    fn default() -> Self {
        MoveOrderer { 
            enabled: true, 
            prior_pv: true, 
            prior_bm: false,
            count_pv: PlyStat::new("order pv"),
            count_bm: PlyStat::new("order bm"),
        }
    }
}

impl fmt::Display for MoveOrderer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "prior pv         : {}", self.prior_pv)?;
        writeln!(f, "prior bm         : {}", self.prior_bm)?;
        writeln!(f, "{}", ArrayPlyStat(&[&self.count_pv, &self.count_bm]))?;
        Ok(())
    }
}

impl Algo {
    pub fn order_moves(&self, node: &Node, movelist: &mut MoveList) -> bool{
        if !self.move_orderer.enabled {
            return false;
        }

        if self.move_orderer.prior_pv {
            if Self::order_from_prior_pv(movelist, &self.current_variation, &self.pv) {
                self.move_orderer.count_pv.add(node.ply, 1);
            }
        }
        if self.move_orderer.prior_bm {
            if node.is_root() {
                if let Some(current_best) = self.current_best {
                    if let Some(i) = movelist.iter().position(|mv| mv == &current_best) {
                        // println!("Swapped move {} with position {} on depth {}!", current_best, i, self.max_depth);
                        movelist.swap(0, i);
                        self.move_orderer.count_bm.add(node.ply, 1);
                        return true;
                    }
                }
            }
        }
        return false;
    }

    pub fn order_from_prior_pv(moves: &mut MoveList, var: &MoveList, pv: &MoveList) -> bool {
        // println!( "{:>30} / [{:>30}] - {}",var.to_string(), pv.to_string(), moves  );
        if pv.len() == 0 {
            return false;
        }
        if var.len() >= pv.len() { 
            return false;  // we're already exploring beyond what we have pv for
        }

        if pv.starts_with(var) {
            let best = pv[var.len()];
            if let Some(j) = moves.iter().position(|mv| mv == &best) {
                moves.swap(0, j);
                return true;
            }
            //println!("{:>30} / [{:>30}] ==> {}", var.to_string(), pv.to_string(), moves);
            return false;
        }
        return false;
    }


    pub fn order_by_mvv_lva(moves: &mut MoveList) {
        

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::constants::*;
    use crate::movelist::*;
    use crate::catalog::Catalog;
    use crate::search::timecontrol::*;

    #[test]
    fn test_prior_pv() {
        let a1a2 = Move { from: a1, to: a2, ..Default::default() };
        let a1a3 = Move { from: a1, to: a3, ..Default::default() };
        let a1a4 = Move { from: a1, to: a4, ..Default::default() };
        let b1a2 = Move { from: b1, to: a2, ..Default::default() };
        let b1a3 = Move { from: b1, to: a3, ..Default::default() };
        let b1a4 = Move { from: b1, to: a4, ..Default::default() };
        let c1c2 = Move { from: c1, to: c2, ..Default::default() };

        let mut moves_orig = MoveList::new();
        moves_orig.extend([b1a2, b1a3, b1a4, a1a3, a1a4, a1a2].iter());
        assert_eq!(moves_orig.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        let mut pv = MoveList::new();
        pv.extend([a1a2, a1a3, a1a4].iter());

        // if variation = empty, and pv = empty, no ordering
        let variation = MoveList::new();
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &variation);
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // if variation = empty, use pv[0] if it exists
        let mut variation = MoveList::new();
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a2, b1a3, b1a4, a1a3, a1a4, b1a2");

        // variation strays from PV - no reordering
        // pv = a1a2 a1a3 a1a4   var = a1a2 c1c2
        let mut moves = moves_orig.clone();
        variation.extend([a1a2, c1c2].iter());
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves_orig.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // variation strays from PV - no reordering
        // pv = a1a2 a1a3 a1a4   var = a1a2 a1a3 c1c2
        let mut variation = MoveList::new();
        variation.extend([a1a2, a1a3, c1c2].iter());
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // if variation = pv, no movelist ordering
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &pv, &pv);

        // variation is PV[0] - use PV[1]
        // pv = a1a2 a1a3 a1a4   var = a1a2
        let mut variation = MoveList::new();
        variation.extend([a1a2].iter());
        let mut moves = moves_orig.clone();
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a3, b1a3, b1a4, b1a2, a1a4, a1a2");

        // variation is PV[0] and [1] - use PV[2]
        // pv = a1a2 a1a3 a1a4   var = a1a2 a1a3
        let mut moves = moves_orig.clone();
        let mut variation = MoveList::new();
        variation.extend([a1a2, a1a3].iter());
        Algo::order_from_prior_pv(&mut moves, &variation, &pv);
        assert_eq!(moves.to_string(), "a1a4, b1a3, b1a4, a1a3, b1a2, a1a2");


    }

    #[test]
    fn test_compare() {
        let position = &Catalog::mate_in_2()[0];
        let mut algo = Algo::new().set_timing_method(TimeControl::Depth(3));
        algo.move_orderer.enabled = false;
        algo.move_orderer.prior_bm = false;
        algo.move_orderer.prior_pv = false;
        algo.search(position.board().clone());
        println!("{}", algo);

        algo.move_orderer.enabled = true;
        algo.move_orderer.prior_bm = true;
        algo.move_orderer.prior_pv = false;
        algo.search(position.board().clone());
        println!("{}", algo);

        algo.move_orderer.prior_bm = false;
        algo.move_orderer.prior_pv = true;
        algo.search(position.board().clone());
        println!("{}", algo);

    }
}
