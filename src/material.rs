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
            m.counts[Color::White][p] = (board.pieces(p) & board.white()).popcount() as i32;
            m.counts[Color::Black][p] = (board.pieces(p) & board.black()).popcount() as i32;
        }
        m
    }

    /// Material.from_str("PPPBNRQKppbbqk")
    pub fn from_str(s: &str) -> Result<Material, String> {
        let mut m: Material = Default::default();
        for ch in s.chars() {
            let p = Piece::from_char(ch)?;
            let c = Color::from_piece_char(ch)?;
            m.counts[c][p] += 1;
        }
        Ok(m)
    }

    #[inline]
    pub fn counts(&self, c: Color, p: Piece) -> i32 {
        self.counts[c][p]
    }

    pub fn is_insufficient2(bd: &Board) -> bool {
        // If both sides have any one of the following, and there are no pawns on the board:
        // 1. A lone king
        // 2. a king and bishop
        // 3. a king and knight
        // 4. K+B v K+B (same color Bs)
        //
        // queens, rooks or pawns => can still checkmate
        if !(bd.pawns() | bd.rooks() | bd.queens()).is_empty() {
            return false;
        }
        // can assume just bishops, knights and kings now
        let bishops_w = (bd.bishops() & bd.white()).popcount();
        let bishops_b = (bd.bishops() & bd.black()).popcount();
        let knights = bd.bishops().popcount();
        if bishops_w + bishops_b + knights <= 1 {
            return true; // cases 1, 2 & 3
        }
        if knights == 0 && bishops_w == 1 && bishops_b == 1 {
            return true; // FIXME: color of bishop  case 4
        }
        false
    }

    pub fn is_insufficient(&self) -> bool {
        // If both sides have any one of the following, and there are no pawns on the board:
        // 1. A lone king
        // 2. a king and bishop
        // 3. a king and knight
        // 4. K+B v K+B (same color Bs)
        //
        // k=0, n=1, b=2, p=r=q=3. Then every total <= 2 is draw covers 1-3
        // no attempt to check for dead fortress like positions
        let n = self.counts[0][Piece::Knight] + self.counts[1][Piece::Knight];
        let b = 2 * (self.counts[0][Piece::Bishop] + self.counts[1][Piece::Bishop]);
        let prq = 3
            * (self.counts[0][Piece::Pawn]
                + self.counts[1][Piece::Pawn]
                + self.counts[0][Piece::Rook]
                + self.counts[1][Piece::Rook]
                + self.counts[0][Piece::Queen]
                + self.counts[1][Piece::Queen]);
        if n + b + prq <= 2 {
            return true;
        }
        if prq == 0 && self.counts[0][Piece::Bishop] == 1 && self.counts[1][Piece::Bishop] == 1 {
            return true; //case 4
        }
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

        assert!(mat3.is_insufficient());
        assert!(!mat2.is_insufficient());
    }
}
