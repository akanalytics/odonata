use crate::mv::Move;
use crate::variation::Variation;

use crate::piece::{Ply, MAX_PLY};
use std::fmt;

#[derive(Debug, Clone)]
pub struct PvTable {
    matrix: Vec<Vec<Move>>,
    size: usize,
}

impl Default for PvTable {
    fn default() -> Self {
        PvTable::new(MAX_PLY as usize)
    }
}

//
// Depth reductions mean that we can have terminal 0000's
//
//     changing mv gets put here at right ply level
//        |
//        V
//
//  0:   0000  e5f7  e8f7  a1d1  d4c2  e1e2  d6e5  e7d8  f8e7  0000  0000
//  1:   e5f7  e8f7  a1d1  d4c2  e1e2  d6e5  e7d8  f8e7  0000  0000
//  2:   e8f7  f7d8  d4c2  e1e2  d6e5  e7d8  f8e7  0000  0000
//  3:   f7d8  d4c2  e1e2  d6e5  e7d8  f8e7  0000  0000
//  4:   d4c2  d5e7  d6e5  e7d8  f8e7  0000  0000
//  5:   d5e7  d6e5  e7d8  f8e7  0000  0000
//  6:   d6e5  g5d8  f8e7  0000  0000
//  7:   g5d8  f8e7  0000  0000
//  8:   f8e7  0000  0000
//  9: t 0000  d8e7
//     --------------  propagate_from(ply=10)
// 10: b d8e7
//
// matrix[0][0..5]
// matrix[1][0..4]
// matrix[2][0..3]
// ... pvs get copied up, extracted from matrix[0]
// ... set ply sets m[ply][0]
impl PvTable {
    pub fn new(max_ply: usize) -> PvTable {
        let mut pvc = PvTable {
            matrix: vec![Vec::new(); max_ply],
            size: 0,
        };
        for (r, row) in pvc.matrix.iter_mut().enumerate() {
            row.resize_with(MAX_PLY as usize - r as usize, Move::new_null)
            // row.extend( vec![Move::new(); r+1] );
        }
        pvc
    }
    pub fn set(&mut self, ply: Ply, m: &Move, terminal_move: bool) {
        // debug_assert!(
        //     {
        //         if !m.is_null() {
        //             let pv = self.extract_pv_for(ply);
        //             self.board.make_moves(&pv.to_inner()).is_legal_move(m)
        //         } else {
        //             true
        //         }
        //     },
        //     "PvTable.set({ply},{m},{terminal_move}) invalid on var {pv}, board {board}",
        //     pv = self.extract_pv_for(ply), board = self.board
        // );
        let p = ply as usize;
        self.matrix[p][0] = *m;
        if terminal_move {
            self.matrix[p][1..].fill(Move::NULL_MOVE);
        }
        if self.size <= p {
            self.size = p + 1;
        }
        // debug_assert!(b.is_none() || b.unwrap().is_legal_variation(&self.extract_pv_for(ply)), "Board: {} pv: {} mv:{} ply:{}\n{}", b.unwrap().to_fen(), &self.extract_pv_for(ply), m, ply, self );
    }

    pub fn propagate_from(&mut self, from_ply: Ply) {
        // copy up one ply and accross one
        debug_assert!(from_ply > 0, "PV propagation from ply=0");
        let from_ply = from_ply as usize;
        let (top, bottom) = self.matrix.split_at_mut(from_ply);
        let len = self.size - from_ply;
        let dst = &mut top[from_ply - 1][1..=len];
        let src = &mut bottom[0][0..len];
        dst.clone_from_slice(src);

        // if from_ply == 1 {
        //     println!("{}", self);
        // }
    }

    pub fn selective_depth(&self) -> Ply {
        (self.size - 1) as i32
    }

    pub fn extract_pv_for(&self, ply: Ply) -> Variation {
        let mut res = Variation::new();
        if let Some(pv) = self.matrix[ply as usize].get(0..self.size) {
            // res.extend(pv.iter().take_while(|m| !m.is_null()));
            for mv in pv.iter().take_while(|m| !m.is_null()) {
                res.push(*mv)
            }
        }
        res
    }

    pub fn extract_pv(&self) -> Variation {
        self.extract_pv_for(0)
    }
}

impl fmt::Display for PvTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "size             : {}", self.size)?;
        for i in 0..self.size {
            write!(f, "{:>3}: ", i)?;
            for j in 0..self.size - i {
                write!(f, "{:>6}", self.matrix[i][j].to_uci())?;
            }
            writeln!(f)?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn test_pv_table() {
        let mut pvc = PvTable::default();
        let board = Catalog::starting_position().board().clone();
        let a2a3 = board.parse_uci_move("a2a3").unwrap();
        let b2b3 = board.parse_uci_move("b2b3").unwrap();
        let c2c3 = board.parse_uci_move("c2c3").unwrap();
        pvc.set(0, &a2a3, false);
        pvc.set(1, &b2b3, false);
        assert_eq!(
            format!("{}", pvc),
            "size             : 2\n  0:   a2a3  0000\n  1:   b2b3\n"
        );
        pvc.set(2, &c2c3, false);
        println!("{}", pvc);

        pvc.propagate_from(1);
        println!("{}", pvc);
        assert_eq!(pvc.matrix[0][0], a2a3);
        assert_eq!(pvc.matrix[0][1], b2b3);
        assert_eq!(pvc.matrix[0][2], Move::NULL_MOVE);

        assert_eq!(pvc.matrix[1][0], b2b3);
        assert_eq!(pvc.matrix[2][0], c2c3);

        let d2d4 = board.parse_uci_move("d2d4").unwrap();
        pvc.propagate_from(2);
        pvc.set(2, &d2d4, false);
        println!("{}", pvc);
        assert_eq!(pvc.matrix[0][0], a2a3);
        assert_eq!(pvc.matrix[0][1], b2b3);
        assert_eq!(pvc.matrix[0][2], Move::NULL_MOVE);

        assert_eq!(pvc.matrix[1][0], b2b3);
        assert_eq!(pvc.matrix[1][1], c2c3);

        assert_eq!(pvc.matrix[2][0], d2d4);
        pvc.propagate_from(1);
        assert_eq!(pvc.extract_pv().len(), 3);

        // set a truncated pv
        pvc.set(1, &d2d4, true);
        assert_eq!(pvc.matrix[1][0], d2d4);
        assert_eq!(pvc.matrix[1][1], Move::NULL_MOVE);
        pvc.propagate_from(1);
        pvc.set(0, &a2a3, false);
        assert_eq!(pvc.extract_pv().len(), 2);
        assert_eq!(pvc.extract_pv().to_string(), String::from("a2a3, d2d4"));

        // assert_eq!( format!("{:?}", pvc), "" );
    }
}
