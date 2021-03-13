use crate::board::Board;
use crate::types::{Color, Piece};

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Material {
    counts: [[i32; Piece::ALL.len()]; 2],
}

impl Material {
    pub fn from_board(board: &Board) -> Material {
        let mut m = Material { ..Default::default() };
        for &p in &Piece::ALL {
            m.counts[Color::White.index()][p.index()] = (board.pieces(p) & board.white()).len() as i32;
            m.counts[Color::Black.index()][p.index()] = (board.pieces(p) & board.black()).len() as i32;
        }
        m
    }

    /// Material.from_str("PPPBNRQKppbbqk")
    pub fn from_str(s: &str) -> Result<Material, String> {
        let mut m: Material = Default::default();
        for ch in s.chars() {
            let p = Piece::from_char(ch)?;
            let c = Color::from_piece_char(ch)?;
            m.counts[c.index()][p.index()] += 1;
        }
        Ok(m)
    }


    #[inline]
    pub fn counts(&self, c: Color, p: Piece) -> i32 {
        self.counts[c.index()][p.index()]
    }




    pub fn is_insufficient_material(&self) -> bool {
        // If both sides have any one of the following, and there are no pawns on the board:
        // 1. A lone king
        // 2. a king and bishop
        // 3. a king and knight
        // 4. K+B v K+B (same color Bs)
        //
        // k=0, n=1, b=2, p=r=q=3. Then every total <= 2 is draw covers 1-3
        // no attempt to check for dead fortress like positions
        let n = self.counts[0][Piece::Knight.index()] + self.counts[1][Piece::Knight.index()];
        let b = 2 * (self.counts[0][Piece::Bishop.index()] + self.counts[1][Piece::Bishop.index()]);
        let prq = 3
            * (self.counts[0][Piece::Pawn.index()]
                + self.counts[1][Piece::Pawn.index()]
                + self.counts[0][Piece::Rook.index()]
                + self.counts[1][Piece::Rook.index()]
                + self.counts[0][Piece::Queen.index()]
                + self.counts[1][Piece::Queen.index()]);
        if n + b + prq <= 2 {
            return true;
        }
        if prq == 0
            && self.counts[0][Piece::Bishop.index()] == 1
            && self.counts[1][Piece::Bishop.index()] == 1
        {
            return true; //case 4
        }

        // // queens, rooks or pawns => can still checkmate
        // if !(self.pawns() | self.rooks() | self.queens()).is_empty() {
        //     return false;
        // }
        // // can assume just bishops, knights and kinds now
        // let bishop_count_w = (self.bishops() & self.white()).len();
        // let bishop_count_b = (self.bishops() & self.black()).len();
        // let knight_count = self.bishops().len();
        // if bishop_count_w + bishop_count_b + knight_count <= 1 {
        //     return true; // cases 1, 2 & 3
        // }
        // if knight_count == 0 && bishop_count_w == 1 && bishop_count_b == 1 {
        //     return true; // case 4
        // }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn counts() {
        let board = Catalog::starting_position();
        let mat1 = Material::from_board(&board);
        assert_eq!(mat1.counts(Color::White, Piece::King), 1);
        assert_eq!(mat1.counts(Color::White, Piece::Pawn), 8);

        let mat2 = Material::from_str("PPPPPPPPNNBBRRQKppppppppnnbbrrqk").unwrap();
        assert_eq!(mat1, mat2);

        let mat3 = Material::from_str("KBk").unwrap();
        assert_ne!(mat2, mat3);

        assert!(mat3.is_insufficient_material());
        assert!(!mat2.is_insufficient_material());
    }
}
