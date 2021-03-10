use crate::board::Board;
use crate::types::{Color, Piece};

#[derive(Copy, Clone, Default, Debug)]
pub struct Material {
    counts: [[i32; Piece::ALL.len()]; 2],
}

impl Material {
    pub fn count_from(board: &Board) -> Material {
        let mut m = Material { ..Default::default() };
        for &c in &Color::ALL {
            for &p in &Piece::ALL {
                m.counts[c.index()][p.index()] = (board.pieces(p) & board.color(c)).len() as i32;
            }
        }
        m
    }

    #[inline]
    pub fn counts(&self, c: Color, p: Piece) -> i32 {
        self.counts[c.index()][p.index()]
    }

    pub fn is_insufficient_material() -> bool {
        panic!("Not implmented");
        // If both sides have any one of the following, and there are no pawns on the board:
        // A lone king
        // a king and bishop
        // a king and knight
        // N=1, B=2, P=R=Q=3. Then every total <= 2 is draw.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn counts() {
        let board = Catalog::starting_position();
        let mat = Material::count_from(&board);
        assert_eq!(mat.counts(Color::White, Piece::King), 1);
        assert_eq!(mat.counts(Color::White, Piece::Pawn), 8);
    }
}
