use crate::movelist::{Move, MoveList};
use std::fmt;

const MAX_PLY: usize = 128;

#[derive(Debug, Clone)]
pub struct PvTable {
    matrix: Vec<Vec<Move>>,
    size: usize,
}

impl Default for PvTable {
    fn default() -> Self {
        PvTable::new(MAX_PLY)
    }
}

impl PvTable {
    pub fn new(max_ply: usize) -> PvTable {
        let mut pvc = PvTable { matrix: vec![Vec::new(); max_ply], size: 0 };
        for (r, row) in pvc.matrix.iter_mut().enumerate() {
            row.resize_with(MAX_PLY - r, Move::new_null)
            // row.extend( vec![Move::new(); r+1] );
        }
        pvc
    }
    pub fn set(&mut self, ply: u32, m: &Move) {
        let ply = ply as usize;
        self.matrix[ply][0] = *m;
        if self.size <= ply {
            self.size = ply + 1;
        }
    }

    pub fn propagate_from(&mut self, from_ply: u32) {
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

    pub fn extract_pv(&self) -> MoveList {
        let mut pv = MoveList::new();
        pv.extend(&self.matrix[0][1..self.size]);
        pv
    }
}

impl fmt::Display for PvTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.size {
            write!(f, "{:>3}: ", i)?;
            for j in 0..self.size - i {
                write!(f, "{:>6}", self.matrix[i][j].uci())?;
            }
            writeln!(f)?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pv_table() {
        let mut pvc = PvTable::default();
        pvc.set(0, &Move::parse_uci("a1h1").unwrap());
        pvc.set(1, &Move::parse_uci("b1h1").unwrap());
        assert_eq!(format!("{}", pvc), "  0:   a1h1     -\n  1:   b1h1\n");
        pvc.set(2, &Move::parse_uci("c1h1").unwrap());
        println!("{}", pvc);

        pvc.propagate_from(1);
        println!("{}", pvc);

        pvc.propagate_from(2);
        pvc.set(2, &Move::parse_uci("c1h2").unwrap());
        println!("{}", pvc);

        // assert_eq!( format!("{:?}", pvc), "" );
    }
}
