use crate::board::Board;
use crate::mv::Move;
use crate::types::{Color, Piece};
use std::cmp;
use std::fmt;

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Material {
    // counts[color][piece] = #
    counts: [[i32; Piece::len()]; Color::len()],
}

impl fmt::Display for Material {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &c in &Color::ALL {
            // write!(f, "{}: ", c)?;
            for &p in Piece::ALL_BAR_NONE.iter().rev() {
                write!(
                    f,
                    "{}",
                    p.to_char(Some(c)).to_string().repeat(self.counts(c, p) as usize)
                )?;
            }
        }
        Ok(())
    }
}

impl cmp::PartialOrd for Material {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        if self == other {
            return Some(cmp::Ordering::Equal);
        }
        if Piece::ALL_BAR_NONE
            .iter()
            .zip(&Color::ALL)
            .all(|(&p, &c)| self.counts(c, p) > other.counts(c, p))
        {
            return Some(cmp::Ordering::Greater);
        }
        if Piece::ALL_BAR_NONE
            .iter()
            .zip(&Color::ALL)
            .all(|(&p, &c)| self.counts(c, p) < other.counts(c, p))
        {
            return Some(cmp::Ordering::Less);
        }
        None
    }
}





impl Material {
    pub fn from_board(board: &Board) -> Material {
        let mut m = Material { ..Self::default() };
        for &p in &Piece::ALL_BAR_NONE {
            m.counts[Color::White][p] = (board.pieces(p) & board.white()).popcount() as i32;
            m.counts[Color::Black][p] = (board.pieces(p) & board.black()).popcount() as i32;
        }
        m
    }

    #[inline]
    pub fn new() -> Material {
        Self::default()
    }

    /// Material.from_piece_str("PPPBNRQKppbbqk")
    #[inline]
    pub fn from_piece_str(s: &str) -> Result<Material, String> {
        let mut m: Material = Material::new();
        for ch in s.chars() {
            let p = Piece::from_char(ch)?;
            let c = Color::from_piece_char(ch)?;
            m.counts[c][p] += 1;
        }
        Ok(m)
    }

    #[inline]
    pub fn total_count(&self) -> i32 {
        Piece::ALL_BAR_NONE
            .iter()
            .map(|&p|
                self.counts(Color::White, p) + self.counts(Color::Black, p)
            )
            .sum::<i32>()
    }


    #[inline]
    pub fn counts(&self, c: Color, p: Piece) -> i32 {
        self.counts[c][p]
    }

    #[inline]
    pub fn counts_mut(&mut self, c: Color, p: Piece) -> &mut i32 {
        &mut self.counts[c][p]
    }

    // #[inline]
    // pub fn counts_mut(&mut self, c: Color, p: Piece) -> &mut i32 {
    //     &mut self.counts[c][p]
    // }

    #[inline]
    pub fn minors_and_majors(&self) -> Material {
        let mut m = *self;
        m.counts[Color::White][Piece::Pawn] = 0;
        m.counts[Color::Black][Piece::Pawn] = 0;
        m.counts[Color::White][Piece::King] = 0;
        m.counts[Color::Black][Piece::King] = 0;
        m
    }

    #[inline]
    pub fn white(&self) -> Material {
        Material {
            counts: [self.counts[Color::White], [0; Piece::len()]],
        }
    }

    #[inline]
    pub fn black(&self) -> Material {
        Material {
            counts: [[0; Piece::len()], self.counts[Color::Black]],
        }
    }

    #[inline]
    pub fn flip(&self) -> Material {
        Material {
            counts: [self.counts[Color::Black], self.counts[Color::White]],
        }
    }


    #[inline]
    pub fn color(&self, c: Color) -> Material {
        c.chooser_wb(self.white(), self.black())
    }

    #[inline]
    pub fn centipawns(&self) -> i32 {
        Piece::ALL_BAR_KING
            .iter()
            .map(|&p|
                p.centipawns() * (self.counts(Color::White, p) - self.counts(Color::Black, p))
            )
            .sum::<i32>()
    }

    /// removes common material leaving only the advantage material
    pub fn advantage(&self) -> Material {
        let mut advantage = *self;
        for &p in &Piece::ALL_BAR_NONE {
            let common = cmp::min(
                advantage.counts[Color::White][p],
                advantage.counts[Color::Black][p],
            );
            advantage.counts[Color::White][p] -= common;
            advantage.counts[Color::Black][p] -= common;
        }
        advantage
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
        let (w, b) = (Color::White, Color::Black);
        let ni = self.counts(w, Piece::Knight) + self.counts(b, Piece::Knight);
        let bi = 2 * (self.counts(w, Piece::Bishop) + self.counts(b, Piece::Bishop));

        let prq = 3
            * (self.counts(w, Piece::Pawn)
                + (self.counts(b, Piece::Pawn)
                    + self.counts(w, Piece::Rook)
                    + self.counts(b, Piece::Rook)
                    + self.counts(w, Piece::Queen)
                    + self.counts(b, Piece::Queen)));
        if ni + bi + prq <= 2 {
            return true;
        }
        if prq == 0 && self.counts(w, Piece::Bishop) == 1 && self.counts(b, Piece::Bishop) == 1 {
            return true; //case 4
        }
        false
    }



    // 236196
    pub const HASH_VALUES: usize =
        (((((((((((1 * 2 + 1) * 3 + 2) * 3 + 2) * 3 + 2) * 3 + 2) * 3) + 2) * 3) + 2) * 9 + 8) * 9) + 8 + 1;

    // hash of no material = 0
    pub fn hash(&self) -> usize {
        let wq = self.counts(Color::White, Piece::Queen) as usize;
        let wr = self.counts(Color::White, Piece::Rook) as usize;
        let wb = self.counts(Color::White, Piece::Bishop) as usize;
        let wn = self.counts(Color::White, Piece::Knight) as usize;
        let wp = self.counts(Color::White, Piece::Pawn) as usize;
        if wq > 1 || wr > 2 || wb > 2 || wn > 2 || wp > 8 {
            return 0;
        }
        let bq = self.counts(Color::Black, Piece::Queen) as usize;
        let br = self.counts(Color::Black, Piece::Rook) as usize;
        let bb = self.counts(Color::Black, Piece::Bishop) as usize;
        let bn = self.counts(Color::Black, Piece::Knight) as usize;
        let bp = self.counts(Color::Black, Piece::Pawn) as usize;
        if bq > 1 || br > 2 || bb > 2 || bn > 2 || bp > 8 {
            return 0;
        }
        // let w_hash = (((wq * 3 + wr) * 3 + wb) * 3 + wn) * 9 + wp;
        // let hash = (((((((w_hash * 2 + bq) * 3) + br) * 3) + bb) * 3 + bn) * 9) + bp;

        let mut hash = 0;
        hash = hash * 9 + wp;
        hash = hash * 9 + bp;

        hash = hash * 3 + wn;
        hash = hash * 3 + bn;

        hash = hash * 3 + wb;
        hash = hash * 3 + bb;

        hash = hash * 3 + wr;
        hash = hash * 3 + br;

        hash = hash * 2 + wq;
        hash = hash * 2 + bq;



        hash
    }

    #[inline]
    pub fn maybe_from_hash(mut hash: usize) -> Material {

        let bq = hash % 2;
        hash = (hash - bq) / 2;

        let wq = hash % 2;
        hash = (hash - wq) / 2;

        let br = hash % 3;
        hash = (hash - br) / 3;

        let wr = hash % 3;
        hash = (hash - wr) / 3;

        let bb = hash % 3;
        hash = (hash - bb) / 3;

        let wb = hash % 3;
        hash = (hash - wb) / 3;

        let bn = hash % 3;
        hash = (hash - bn) / 3;

        let wn = hash % 3;
        hash = (hash - wn) / 3;

        let bp = hash % 9;
        hash = (hash - bp) / 9;

        let wp = hash % 9;
        hash = (hash - wp) / 9;

        debug_assert!(hash == 0);

        let mut m = Material::new();
        m.counts[Color::White][Piece::Pawn] = wp as i32;
        m.counts[Color::White][Piece::Knight] = wn as i32;
        m.counts[Color::White][Piece::Bishop] = wb as i32;
        m.counts[Color::White][Piece::Rook] = wr as i32;
        m.counts[Color::White][Piece::Queen] = wq as i32;
        m.counts[Color::White][Piece::King] = 1;

        m.counts[Color::Black][Piece::Pawn] = bp as i32;
        m.counts[Color::Black][Piece::Knight] = bn as i32;
        m.counts[Color::Black][Piece::Bishop] = bb as i32;
        m.counts[Color::Black][Piece::Rook] = br as i32;
        m.counts[Color::Black][Piece::Queen] = bq as i32;
        m.counts[Color::Black][Piece::King] = 1;
        m
    }

}


impl Material {
    #[inline]
    pub fn niche() -> Material {
        Self::default()
    }

    pub fn make_move(&mut self, c: Color, m: &Move) {
        if m.is_promo() {
            self.counts[c][Piece::Pawn] -= 1;
            self.counts[c][m.promo_piece()] += 1;
        }
        if m.is_capture() {
            self.counts[c.opposite()][m.capture_piece()] -= 1;
        }
    }
}



#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn test_material() {
        let board = Catalog::starting_board();
        let mat_full1 = Material::from_board(&board);
        assert_eq!(mat_full1.counts(Color::White, Piece::King), 1);
        assert_eq!(mat_full1.counts(Color::White, Piece::Pawn), 8);

        let mat_full2 = Material::from_piece_str("PPPPPPPPNNBBRRQKppppppppnnbbrrqk").unwrap();
        assert_eq!(mat_full1, mat_full2);
        assert_eq!(mat_full2.total_count(), 32);

        #[allow(non_snake_case)]
        let mat_KBk = Material::from_piece_str("KBk").unwrap();
        assert_ne!(mat_full2, mat_KBk);
        assert_eq!(mat_KBk.flip().to_string(), "Kkb");

        assert!(mat_KBk.is_insufficient());
        assert!(!mat_full2.is_insufficient());

        let mat0 = Material::new();
        // counts and comparisons
        assert_eq!(mat0.counts(Color::White, Piece::Pawn), 0);
        assert_ne!(mat0, mat_full1);
        assert_eq!(mat0, mat0);
        assert!(mat_full1 > mat0);
        assert!(mat0 < mat_full1);
        assert!(mat0 <= mat0);
        assert!(mat_full2 != mat_KBk);

        // mat3 is a subset of mat2
        assert_eq!(mat_full2 < mat_KBk, false);
        assert_eq!(mat_KBk < mat_full2, true);
        assert_eq!(mat_full2 >= mat_KBk, true);
        assert_eq!(mat_full2 <= mat_KBk, false);

        // mat3 (KBk) and mat4 (Kkn) are not comparable (ordering is partial)
        #[allow(non_snake_case)]
        let mat_Kkn = Material::from_piece_str("Kkn").unwrap();
        assert_eq!(mat_KBk < mat_Kkn, false);
        assert_eq!(mat_KBk > mat_Kkn, false);
        assert_eq!(mat_KBk <= mat_Kkn, false);
        assert_eq!(mat_KBk >= mat_Kkn, false);

        // to_string
        assert_eq!(mat_KBk.to_string(), "KBk".to_string());
        assert_eq!(mat_KBk.minors_and_majors().to_string(), "B".to_string());
        assert_eq!(mat_Kkn.to_string(), "Kkn".to_string());
        assert_eq!(
            mat_full1.to_string(),
            "KQRRBBNNPPPPPPPPkqrrbbnnpppppppp".to_string()
        );

        assert_eq!(mat_Kkn.black().to_string(), "kn".to_string());
        assert_eq!(mat_Kkn.white().to_string(), "K".to_string());
        assert_eq!(mat_full1.white().black().to_string(), "".to_string()); // nothing is both colors!
        assert_eq!(mat_Kkn.color(Color::White).to_string(), "K".to_string());
        assert_eq!(mat_Kkn.color(Color::Black).to_string(), "kn".to_string());

        // advantage
        let mat_some = Material::from_piece_str("PPPPPNNBRKpppppppbbqk").unwrap();
        assert_eq!(mat_Kkn.advantage().to_string(), "n".to_string());
        assert_eq!(mat_full1.advantage().to_string(), "".to_string()); // evenly matched
        assert_eq!(mat_some.advantage().to_string(), "RNNqbpp".to_string());

        // centipawns
        #[allow(non_snake_case)]
        let mat_PPP = Material::from_piece_str("PPP").unwrap();
        let mat_p = Material::from_piece_str("p").unwrap();
        assert_eq!(mat_p.centipawns(), -100);
        assert_eq!(mat_PPP.centipawns(), 300);
        assert_eq!(mat_some.advantage().centipawns(), -300); // R+N-Q = -75, N-b=-25, 2x-P=-200
        let board = Catalog::starting_board();
        assert_eq!(board.material().black().minors_and_majors().centipawns(), -3250);
        assert_eq!(board.material().white().minors_and_majors().centipawns(), 3250);
        assert_eq!(
            Material::from_piece_str("KkPPPPPppppp")
                .unwrap()
                .white()
                .minors_and_majors()
                .centipawns(),
            0
        );
        assert_eq!(
            Material::from_piece_str("KkPPPPPppppp")
                .unwrap()
                .black()
                .minors_and_majors()
                .centipawns(),
            0
        );
    }


    #[test]
    fn test_material_hash() {
        let board = Catalog::starting_board();
        let mat_full = Material::from_board(&board);
        assert_eq!(Material::maybe_from_hash(mat_full.hash()), mat_full);
        assert_eq!(mat_full.hash(), Material::HASH_VALUES - 1);

        let mat_part = Material::from_piece_str("KQRBPPPPPkqrrnppppppp").unwrap();
        assert_eq!(Material::maybe_from_hash(mat_part.hash()), mat_part);

    }
}
