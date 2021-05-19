use crate::movelist::{Move, MoveList};
use crate::types::{MAX_PLY, Ply};
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

// matrix[0][0..5]
// matrix[1][0..4]
// matrix[2][0..3]
// ... pvs get copied up, extracted from matrix[0]
// ... set ply sets m[ply][0]
impl PvTable {
    pub fn new(max_ply: usize) -> PvTable {
        let mut pvc = PvTable { matrix: vec![Vec::new(); max_ply], size: 0, };
        for (r, row) in pvc.matrix.iter_mut().enumerate() {
            row.resize_with(MAX_PLY as usize - r as usize, Move::new_null)
            // row.extend( vec![Move::new(); r+1] );
        }
        pvc
    }
    pub fn set(&mut self, ply: Ply, m: &Move, terminal_move: bool) {
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

    pub fn extract_pv_for(&self, ply: Ply) -> MoveList {
        let mut res = MoveList::new();
        if let Some(pv) = self.matrix[ply as usize].get(1..self.size) {
            res.extend(pv.iter().take_while(|m| !m.is_null()));
        }
        res
    }



    pub fn extract_pv(&self) -> MoveList {
        self.extract_pv_for(0)
    }
}

impl fmt::Display for PvTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "size             : {}", self.size)?;
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
        let a1h1 = Move::parse_uci("a1h1").unwrap();
        let b1h1 = Move::parse_uci("b1h1").unwrap();
        let c1h1 = Move::parse_uci("c1h1").unwrap();
        pvc.set(0, &a1h1, false);
        pvc.set(1, &b1h1, false);
        assert_eq!(format!("{}", pvc), "size             : 2\n  0:   a1h1     -\n  1:   b1h1\n");
        pvc.set(2, &c1h1, false);
        println!("{}", pvc);

        pvc.propagate_from(1);
        println!("{}", pvc);
        assert_eq!(pvc.matrix[0][0], a1h1);
        assert_eq!(pvc.matrix[0][1], b1h1);
        assert_eq!(pvc.matrix[0][2], Move::NULL_MOVE);

        assert_eq!(pvc.matrix[1][0], b1h1);
        assert_eq!(pvc.matrix[2][0], c1h1);

        let c1h2 = Move::parse_uci("c1h2").unwrap();
        pvc.propagate_from(2);
        pvc.set(2, &c1h2, false);
        println!("{}", pvc);
        assert_eq!(pvc.matrix[0][0], a1h1);
        assert_eq!(pvc.matrix[0][1], b1h1);
        assert_eq!(pvc.matrix[0][2], Move::NULL_MOVE);

        assert_eq!(pvc.matrix[1][0], b1h1);
        assert_eq!(pvc.matrix[1][1], c1h1);

        assert_eq!(pvc.matrix[2][0], c1h2);
        pvc.propagate_from(1);
        assert_eq!(pvc.extract_pv().len(), 2);

        // set a truncated pv
        pvc.set(1, &c1h2, true);
        assert_eq!(pvc.matrix[1][0], c1h2);
        assert_eq!(pvc.matrix[1][1], Move::NULL_MOVE);
        pvc.propagate_from(1);
        assert_eq!(pvc.extract_pv().len(), 1);

        // assert_eq!( format!("{:?}", pvc), "" );
    }
}
