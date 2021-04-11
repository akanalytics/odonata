use crate::config::{Config, Configurable};
use crate::log_debug;
use crate::movelist::MoveList;
use crate::search::algo::{Algo, Node};
use std::fmt;

#[derive(Copy, Clone, Debug)]
pub struct MoveOrderer {
    pub enabled: bool,
    pub pv_from_prior_iteration: bool,
}

impl Configurable for MoveOrderer {
    fn settings(&self, c: &mut Config) {
        c.set("move_orderer.enabled", "type check default true");
        c.set("move_orderer.pv_from_prior_iteration", "type check default true");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("move_orderer.configure with {}", c);
        self.enabled = c.bool("move_orderer.enabled").unwrap_or(self.enabled);
        if let Some(prior) = c.bool("move_orderer.pv_from_prior_iteration") {
            self.pv_from_prior_iteration = prior;
        }
    }
}

impl MoveOrderer {
    // pub const MOVE_SORTS: &'static [&'static str] = &["Natural", "PV from Prior Iteration", "MVV/LVA"];
}

impl Default for MoveOrderer {
    fn default() -> Self {
        MoveOrderer { enabled: true, pv_from_prior_iteration: true }
    }
}

impl fmt::Display for MoveOrderer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "pv prior iter    : {}", self.pv_from_prior_iteration)?;
        Ok(())
    }
}

impl Algo {
    pub fn order_moves(&self, node: &Node, movelist: &mut MoveList) {
        if !self.move_orderer.enabled {
            return;
        }

        if self.move_orderer.pv_from_prior_iteration {
            // if node.ply == 0 {
            //     if let Some(current_best) = self.current_best {
            //         if let Some(i) = movelist.iter().position(|mv| mv == &current_best) {
            //             // println!("Swapped move {} with position {} on depth {}!", current_best, i, self.max_depth);
            //             movelist.swap(0, i);
            //         }
            //     }
            // }
            Self::order_from_prior_pv(movelist, &self.current_variation, &self.pv );
        }
    }


    pub fn order_from_prior_pv(moves: &mut MoveList, var: &MoveList, pv: &MoveList ) {
        // println!( "{:>30} / [{:>30}] - {}",var.to_string(), pv.to_string(), moves  );
        if pv.len() == 0 {
            return
        }
        for (i, m) in var.iter().enumerate() {
            if *m != pv[i] {
                let best = pv[i];
                if let Some(j) = moves.iter().position(|mv| mv == &best) {
                    moves.swap(0, j);
                }
                // println!( "{:>30} / [{:>30}] ==> {}",var.to_string(), pv.to_string(), moves  );
                return;
            }
        }
        if var.len() == 0 {
            if let Some(i) = moves.iter().position(|mv| mv == &pv[0]) {
                moves.swap(0, i);
            }
        }
    }
}

// if let Some(current_best) = self.current_best {
//     if let Some(i) = movelist.iter().position(|mv| mv == &current_best) {
//         // println!("Swapped move {} with position {} on depth {}!", current_best, i, self.max_depth);
//         movelist.swap(0, i);
//     }



#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::globals::constants::*;
    use crate::movelist::*;



    #[test]
    fn test_prior_pv() {
        let a1a2 = Move { from: a1, to: a2, ..Default::default() };
        let a1a3 = Move { from: a1, to: a3, ..Default::default() };
        let a1a4 = Move { from: a1, to: a4, ..Default::default() };
        let b1a2 = Move { from: b1, to: a2, ..Default::default() };
        let b1a3 = Move { from: b1, to: a3, ..Default::default() };
        let b1a4 = Move { from: b1, to: a4, ..Default::default() };
        let c1c2 = Move { from: c1, to: c2, ..Default::default() };

        let mut moves = MoveList::new();
        moves.extend([b1a2, b1a3, b1a4, a1a3, a1a4, a1a2].iter());

        let mut pv = MoveList::new();
        pv.extend([a1a2, a1a3, a1a4].iter());


        // if variation = empty, and pv = empty, no ordering
        let variation = MoveList::new();
        Algo::order_from_prior_pv(&mut moves, &variation, &variation );
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

        // if variation = empty, use pv[0] if it exists
        let mut variation = MoveList::new();
        Algo::order_from_prior_pv(&mut moves, &variation, &pv );
        assert_eq!(moves.to_string(), "a1a2, b1a3, b1a4, a1a3, a1a4, b1a2");


        variation.extend([a1a2, c1c2].iter());
        Algo::order_from_prior_pv(&mut moves, &variation, &pv );
        assert_eq!(moves.to_string(), "a1a3, b1a3, b1a4, a1a2, a1a4, b1a2");

        let mut variation = MoveList::new();
        variation.extend([a1a2, a1a3, c1c2].iter());
        Algo::order_from_prior_pv(&mut moves, &variation, &pv );
        assert_eq!(moves.to_string(), "a1a4, b1a3, b1a4, a1a2, a1a3, b1a2");

        // if variation = pv, no movelist ordering
        let mut moves = MoveList::new();
        moves.extend([b1a2, b1a3, b1a4, a1a3, a1a4, a1a2].iter());
        Algo::order_from_prior_pv(&mut moves, &pv, &pv );
        assert_eq!(moves.to_string(), "b1a2, b1a3, b1a4, a1a3, a1a4, a1a2");

    }
}