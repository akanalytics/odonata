use crate::board::Board;
use crate::types::{Piece, Color};


#[derive(Copy, Clone, Default, Debug )]
pub struct Material {
    counts: [[u8; Piece::ALL.len()]; 2],
}


impl Material {
    pub fn new(board: &Board) -> Material {
        let mut m = Material { ..Default::default() };
        for &c in &Color::ALL {
            for &p in &Piece::ALL {
                m.counts[c.index()][p.index()] = (board.pieces(p) & board.color(c)).len() as u8;
            } 
        }
        m
    }

    #[inline]
    pub fn counts( &self, c: Color, p: Piece ) -> u8 {
        self.counts[c.index()][p.index()]
    }

    pub fn is_insufficient_material() -> bool {
        panic!("Not implmented");
        // If both sides have any one of the following, and there are no pawns on the board: 
        // A lone king 
        // a king and bishop
        // a king and knight
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;


    #[test]
    fn counts() {
        let board = Catalog::starting_position();
        let mat = Material::new(&board);
        assert_eq!(mat.counts(Color::White, Piece::King), 1);
        assert_eq!(mat.counts(Color::White, Piece::Pawn), 8);
    }

}