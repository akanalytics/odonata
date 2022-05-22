use crate::board::Board;
use crate::mv::Move;
use crate::types::{Color, Piece};
use anyhow::Result;
use itertools::Itertools;
use std::cmp;
use std::fmt;
use std::ops;

type MaterialCount = u16;

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Material {
    counts: [[MaterialCount; Piece::len()]; Color::len()],
}

impl fmt::Display for Material {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &c in &Color::ALL {
            // write!(f, "{}: ", c)?;
            for &p in Piece::ALL_BAR_NONE.iter().rev() {
                write!(
                    f,
                    "{}",
                    p.to_char(Some(c))
                        .to_string()
                        .repeat(self.count(c, p) as usize)
                )?;
            }
        }
        Ok(())
    }
}

impl ops::Neg for Material {
    type Output = Material;

    fn neg(self) -> Self::Output {
        (&self).neg()
    }
}

impl ops::Neg for &Material {
    type Output = Material;

    fn neg(self) -> Self::Output {
        let mut m = Material::new();

        for &c in &Color::ALL {
            for &p in &Piece::ALL_BAR_NONE {
                m.set_count(c, p, -self.count(c, p));
            }
        }
        m
    }
}

impl<'a, 'b> ops::Sub<&'b Material> for &'a Material {
    type Output = Material;

    fn sub(self, other: &'b Material) -> Self::Output {
        let mut m = Material::new();

        for &c in &Color::ALL {
            for &p in &Piece::ALL_BAR_NONE {
                m.set_count(c, p, self.count(c, p) - other.count(c, p));
            }
        }
        m
    }
}

impl cmp::PartialOrd for Material {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        if self == other {
            return Some(cmp::Ordering::Equal);
        }
        if Piece::ALL_BAR_NONE
            .iter()
            .cartesian_product(&Color::ALL)
            .all(|(&p, &c)| self.count(c, p) >= other.count(c, p))
        {
            return Some(cmp::Ordering::Greater);
        }
        if Piece::ALL_BAR_NONE
            .iter()
            .cartesian_product(&Color::ALL)
            .all(|(&p, &c)| self.count(c, p) <= other.count(c, p))
        {
            return Some(cmp::Ordering::Less);
        }
        None
    }
}

impl Material {
    /// const constructor (which can panic)
    /// eg const m: Material = Material::from_piece_bytes(b"PPPKk");
    ///
    pub const fn from_piece_bytes(bytes: &[u8]) -> Material {
        let mut m: Material = Material::new();
        let mut i = 0;
        while i < bytes.len() {
            let ch = bytes[i] as char;
            let p = match ch.to_ascii_uppercase() {
                'P' => Piece::Pawn,
                'N' => Piece::Knight,
                'B' => Piece::Bishop,
                'R' => Piece::Rook,
                'Q' => Piece::Queen,
                'K' => Piece::King,
                _ => panic!("Invalid pieces in from_piece_bytes"),
            };
            let c = if ch.is_ascii_lowercase() {
                Color::Black
            } else {
                Color::White
            };
            m.counts[c.index()][p.index()] += 1;
            i += 1;
        }
        m
    }

    pub fn from_board(board: &Board) -> Material {
        let mut m = Material { ..Self::default() };
        for &p in &Piece::ALL_BAR_NONE {
            m.set_count(
                Color::White,
                p,
                (board.pieces(p) & board.white()).popcount(),
            );
            m.set_count(
                Color::Black,
                p,
                (board.pieces(p) & board.black()).popcount(),
            );
        }
        m
    }

    #[inline]
    pub const fn new() -> Material {
        Material {
            counts: [[0; Piece::len()]; Color::len()],
        }
    }

    /// Material.from_piece_str("PPPBNRQKppbbqk")
    #[inline]
    pub fn from_piece_str(s: &str) -> Result<Material> {
        let mut m: Material = Material::new();

        for ch in s.chars() {
            let p = Piece::from_char(ch)?;
            let c = Color::from_piece_char(ch)?;
            m.set_count(c, p, m.count(c, p) + 1);
        }
        Ok(m)
    }

    #[inline]
    pub fn total_count(&self) -> i32 {
        Piece::ALL_BAR_NONE
            .iter()
            .map(|&p| self.count_piece(p))
            .sum::<i32>()
    }

    pub fn total_count2(&self) -> i32 {
        self.total_count()
    }

    #[inline]
    pub const fn count(&self, c: Color, p: Piece) -> i32 {
        self.counts[c.index()][p.index()] as i32
    }

    #[inline]
    pub fn set_count(&mut self, c: Color, p: Piece, v: i32) {
        self.counts[c][p] = v as MaterialCount;
    }

    #[inline]
    pub fn count_piece(&self, p: Piece) -> i32 {
        self.count(Color::White, p) + self.count(Color::Black, p)
    }

    #[inline]
    pub fn net_piece(&self, p: Piece) -> i32 {
        self.count(Color::White, p) - self.count(Color::Black, p)
    }

    // #[inline]
    // pub fn counts_mut(&mut self, c: Color, p: Piece) -> &mut i32 {
    //     &mut self.counts[c][p]
    // }

    #[inline]
    pub fn minors_and_majors(&self) -> Material {
        let mut m = *self;
        m.set_count(Color::White, Piece::Pawn, 0);
        m.set_count(Color::Black, Piece::Pawn, 0);
        m.set_count(Color::White, Piece::King, 0);
        m.set_count(Color::Black, Piece::King, 0);
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
            .map(|&p| p.centipawns() * self.net_piece(p))
            .sum::<i32>()
    }

    /// removes common material leaving only the advantage material
    pub fn advantage(&self) -> Material {
        let mut advantage = *self;
        for &p in &Piece::ALL_BAR_NONE {
            let common = cmp::min(
                advantage.count(Color::White, p),
                advantage.count(Color::Black, p),
            );
            advantage.set_count(Color::White, p, advantage.count(Color::White, p) - common);
            advantage.set_count(Color::Black, p, advantage.count(Color::Black, p) - common);
        }
        advantage
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
        let ni = self.count_piece(Piece::Knight);
        let bi = 2 * self.count_piece(Piece::Bishop);

        let prq = 3
            * (self.count_piece(Piece::Pawn)
                + self.count_piece(Piece::Rook)
                + self.count_piece(Piece::Queen));
        if ni + bi + prq <= 2 {
            return true;
        }
        if prq == 0 && self.count(w, Piece::Bishop) == 1 && self.count(b, Piece::Bishop) == 1 {
            return true; //case 4
        }
        false
    }

    // 236196
    pub const HASH_VALUES: usize =
        (((((((((((2 + 1) * 3 + 2) * 3 + 2) * 3 + 2) * 3 + 2) * 3) + 2) * 3) + 2) * 9 + 8) * 9)
            + 8
            + 1;

    // hash of no material = 0
    pub const fn hash(&self) -> usize {
        let wq = self.count(Color::White, Piece::Queen) as i32;
        let wr = self.count(Color::White, Piece::Rook) as i32;
        let wb = self.count(Color::White, Piece::Bishop) as i32;
        let wn = self.count(Color::White, Piece::Knight) as i32;
        let wp = self.count(Color::White, Piece::Pawn) as i32;
        if wq > 1 || wr < 0 || wr > 2 || wb < 0 || wb > 2 || wn < 0 || wn > 2 || wp < 0 || wp > 8 {
            return 0;
        }
        let bq = self.count(Color::Black, Piece::Queen) as i32;
        let br = self.count(Color::Black, Piece::Rook) as i32;
        let bb = self.count(Color::Black, Piece::Bishop) as i32;
        let bn = self.count(Color::Black, Piece::Knight) as i32;
        let bp = self.count(Color::Black, Piece::Pawn) as i32;
        if bq > 1 || br < 0 || br > 2 || bb < 0 || bb > 2 || bn < 0 || bn > 2 || bp < 0 || bp > 8 {
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

        hash as usize
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
        m.set_count(Color::White, Piece::Pawn, wp as i32);
        m.set_count(Color::White, Piece::Knight, wn as i32);
        m.set_count(Color::White, Piece::Bishop, wb as i32);
        m.set_count(Color::White, Piece::Rook, wr as i32);
        m.set_count(Color::White, Piece::Queen, wq as i32);
        m.set_count(Color::White, Piece::King, 1);

        m.set_count(Color::Black, Piece::Pawn, bp as i32);
        m.set_count(Color::Black, Piece::Knight, bn as i32);
        m.set_count(Color::Black, Piece::Bishop, bb as i32);
        m.set_count(Color::Black, Piece::Rook, br as i32);
        m.set_count(Color::Black, Piece::Queen, bq as i32);
        m.set_count(Color::Black, Piece::King, 1);
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
            self.set_count(c, Piece::Pawn, self.count(c, Piece::Pawn) - 1);
            self.set_count(c, m.promo_piece(), self.count(c, m.promo_piece()) + 1);
        }
        if m.is_capture() {
            self.set_count(
                c.opposite(),
                m.capture_piece(),
                self.count(c.opposite(), m.capture_piece()) - 1,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // use std::{cmp::Ordering, convert::TryFrom};

    use super::*;
    use crate::{
        catalog::Catalog,
        infra::{black_box, profiler::Profiler},
    };
    // // use crate::{debug, logger::LogInit};

    #[test]
    fn test_material() {
        let board = Catalog::starting_board();
        let mat_full1 = Material::from_board(&board);
        assert_eq!(mat_full1.count(Color::White, Piece::King), 1);
        assert_eq!(mat_full1.count(Color::White, Piece::Pawn), 8);

        let mat_full2 = Material::from_piece_str("PPPPPPPPNNBBRRQKppppppppnnbbrrqk").unwrap();
        assert_eq!(mat_full1, mat_full2);
        assert_eq!(--mat_full1, mat_full2);
        assert_eq!(-&-&mat_full1, mat_full2);
        assert_eq!(mat_full2.total_count(), 32);

        const M1: Material = Material::from_piece_bytes(b"PPPKk");
        let m2 = Material::from_piece_str("KkPPP").unwrap();
        assert_eq!(M1, m2);

        #[allow(non_snake_case)]
        let mat_KBk = Material::from_piece_str("KBk").unwrap();
        assert_ne!(mat_full2, mat_KBk);
        assert_eq!(mat_KBk.flip().to_string(), "Kkb");

        assert!(mat_KBk.is_insufficient());
        assert!(!mat_full2.is_insufficient());

        let mat0 = Material::new();
        // counts and comparisons
        assert_eq!(mat0.count(Color::White, Piece::Pawn), 0);
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
        // count KBk.count(B,n) <   Kkn.count(B, n)
        // count KBk.count(W,B) >   Kkn.count(W, B)

        debug!("{:?} ... {:?}", mat_KBk, mat_Kkn);
        let _b = Piece::ALL_BAR_NONE
            .iter()
            .cartesian_product(&Color::ALL)
            .inspect(|x| {
                debug!("iterating on... {:?}", x);
            })
            .all(|(&p, &c)| mat_KBk.count(c, p) <= mat_Kkn.count(c, p));

        assert_eq!(mat_KBk.partial_cmp(&mat_Kkn), None);
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
        assert_eq!(mat_some.advantage().centipawns(), -275); // R+N-Q = -75, N-b=-0, 2x-P=-200
        let board = Catalog::starting_board();
        assert_eq!(
            board.material().black().minors_and_majors().centipawns(),
            -3200
        );
        assert_eq!(
            board.material().white().minors_and_majors().centipawns(),
            3200
        );
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
        assert_eq!(Material::HASH_VALUES, 236196);

        let mat_part = Material::from_piece_str("KQRBPPPPPkqrrnppppppp").unwrap();
        assert_eq!(Material::maybe_from_hash(mat_part.hash()), mat_part);
    }
    #[test]
    fn bench_material() {
        let board = Catalog::starting_board();

        Profiler::new("material-ctor".into()).benchmark(
            || Material::from_board(&board), //
        );

        let m = Material::from_board(&board);
        Profiler::new("material-total".into()).benchmark(
            || m.total_count(), //
        );

        Profiler::new("material-insuff".into()).benchmark(
            || m.is_insufficient(), //
        );

        {
            let mut prof = Profiler::new("material-manip".into());
            let mut prof2 = Profiler::new("material-count".into());
            let board = Catalog::starting_board();
            let m = Material::from_board(&board);
            prof.start();
            let wp = black_box(m.color(Color::White).count_piece(Piece::Pawn));
            let bb = black_box(m.color(Color::Black).count_piece(Piece::Bishop));
            prof.stop();
            prof2.start();
            let wp2 = black_box(m.count(Color::White, Piece::Pawn));
            prof2.stop();

            assert_eq!(wp, 8);
            assert_eq!(wp2, 8);
            assert_eq!(bb, 2);
        }
    }
}
