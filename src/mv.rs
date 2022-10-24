use crate::bits::bitboard::Bitboard;
use crate::bits::castling::CastlingRights;
use crate::bits::square::Square;
use crate::board::Board;
use crate::globals::constants::*;
use crate::infra::utils::{StringUtils, Uci};
use crate::piece::{Color, Piece};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::{self};
use std::str::FromStr;

// #[enumflags2::bitflags]
// #[repr(u32)]
// #[derive(Clone, Copy, Debug, Eq, PartialEq, EnumCount, Display, Serialize, Deserialize)]
// pub enum MoveType {
//     EnPassant,
//     CastleQueen,
//     CastleKing,
//     DoublePush,
//     PromoBishop,
//     PromoKnight,
//     PromoRook,
//     PromoQueen,

//     PromoBishop,
//     Unsorted,
//     Capture,
//     GoodCapture,
//     GoodCaptureUpfrontSorted,
//     Killer,
//     CounterMove,
//     Promo,
//     QueenPromo,
//     Evasion,
//     Quiet,
//     QuietOrPromo,
//     QuietUnsorted,
//     BadCapture,
//     Remaining,
//     End,
// }

// pub type MoveTypes = BitFlags<MoveType>;
//lean  / bare

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct BareMove {
    pub from: Square,
    pub to: Square,
    pub promo: Option<Piece>,
}

impl Uci for BareMove {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <BareMove as fmt::Display>::fmt(self, f)
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        s.parse()
    }
}

impl BareMove {
    #[inline]
    pub fn null() -> Self {
        Default::default()
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.to == self.from
    }

    pub fn to_san(&self, b: &Board) -> String {
        b.to_san(&b.augment_move(*self))
    }
}

impl FromStr for BareMove {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let from = Bitboard::parse_square(s.take_slice(0..2))?;
        let to = Bitboard::parse_square(s.take_slice(2..4))?;
        let promo = if let Some(ch) = s.take_char_at(4) {
            Some(Piece::from_char(ch)?)
        } else {
            None
        };
        Ok(BareMove { from, to, promo })
    }
}

impl fmt::Display for BareMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "0000")
        } else {
            write!(f, "{}{}", self.from.uci(), self.to.uci())?;
            if let Some(p) = self.promo {
                write!(f, "{}", p.to_char(Color::Black))?
            }
            Ok(())
        }
    }
}

impl Board {
    pub fn augment_move(&self, mv: BareMove) -> Move {
        if mv.is_null() {
            return Move::NULL_MOVE;
        }
        let from = mv.from;
        let to = mv.to;
        let mover = self.piece(from).unwrap_or_else(|| {
            panic!(
                "move {mv} no piece on {from} for board {fen}",
                fen = self.to_fen()
            )
        });
        let capture_piece = self.piece(to);
        if mover == Piece::King && CastlingRights::is_castling(from, to) {
            let rights = CastlingRights::from_king_move(to);
            return Move::new_castle(from, to, rights);
        }
        if let Some(promo) = mv.promo {
            if let Some(capture) = capture_piece {
                return Move::new_promo_capture(from, to, promo, capture);
            } else {
                return Move::new_promo(from, to, promo);
            }
        }
        if let Some(captured) = capture_piece {
            return Move::new_capture(mover, from, to, captured);
        }
        // diagonal pawn capture yet to-square is empty
        if mover == Piece::Pawn && from.file() != to.file() {
            let file_x = to.file_index() as u32;
            let rank_y = from.rank_index() as u32;
            return Move::new_ep_capture(from, to, Square::from_xy(file_x, rank_y));
        }
        if mover == Piece::Pawn
            && (from.index() + 16 == to.index() || to.index() + 16 == from.index())
        {
            return Move::new_double_push(
                from,
                to,
                Square::from_u32((from.index() as u32 + to.index() as u32) / 2),
            );
        }
        if from == to {
            return Move::new_null();
        }
        Move::new_quiet(mover, from, to)
    }
}

// FIXME: public methods
#[derive(Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    bits: u32,
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Move")
            .field("from", &self.from())
            .field("to", &self.to())
            .field("mover", &self.mover_piece())
            .field("captured", &self.capture_piece())
            .field("castling", &self.castling_side())
            .field("ep", &self.ep())
            .finish()
    }
}

// piece
// from
// to
// pice2
// from2
// to2
//
// promo/capture
//
// P from
// Q-to
// cap-from
//
// Promo/capture

impl Move {
    const OFFSET_FROM: i32 = 0;
    const OFFSET_TO: i32 = 6;
    const OFFSET_EP: i32 = 12; // 7 bits
    const OFFSET_MOVER: i32 = 19;
    const OFFSET_CAPTURE: i32 = 22;
    const OFFSET_PROMO: i32 = 25;
    const OFFSET_CASTLE: i32 = 28;

    #[inline]
    pub const fn to_inner(&self) -> BareMove {
        BareMove {
            from: self.from(),
            to: self.to(),
            promo: self.promo(),
        }
    }

    #[inline]
    pub const fn new_quiet(mover: Piece, from: Square, to: Square) -> Move {
        let mut bits = (from.index() as u32 & 63) << Self::OFFSET_FROM;
        bits += (to.index() as u32 & 63) << Self::OFFSET_TO;
        bits += (mover.index() as u32) << Self::OFFSET_MOVER;
        bits += (Square::null().index() as u32 & 127) << Self::OFFSET_EP;
        bits += 7 << Self::OFFSET_CAPTURE;
        Move { bits }
    }

    pub fn set_capture(&mut self, capture: Piece) {
        self.bits &= !(7 << Self::OFFSET_CAPTURE);
        self.bits += (capture.index() as u32) << Self::OFFSET_CAPTURE;
    }

    pub fn set_promo(&mut self, promo: Piece) {
        self.bits += (promo.index() as u32) << Self::OFFSET_PROMO;
    }

    pub fn set_en_passant(&mut self, ep_sq: Square) {
        self.bits &= !(127 << Self::OFFSET_EP);
        self.bits += (ep_sq.index() as u32 & 127) << Self::OFFSET_EP;
    }

    pub fn set_castling_side(&mut self, castle_side: CastlingRights) {
        self.bits += (castle_side.bits() as u32) << Self::OFFSET_CASTLE;
    }

    // #[inline]
    // pub const fn new(
    //     from: Square,
    //     to: Square,
    //     ep: Square,
    //     mover: Piece,
    //     capture: Piece,
    //     promo: Piece,
    //     castle_side: CastlingRights,
    // ) -> Move {
    //     // debug_assert!(!from.is_null());
    //     // debug_assert!(!to.is_null());
    //     let mut bits = (from.index() as u32 & 63) << Self::OFFSET_FROM;
    //     bits += (to.index() as u32 & 63) << Self::OFFSET_TO;
    //     bits += (ep.index() as u32 & 127) << Self::OFFSET_EP;
    //     bits += (mover.index() as u32) << Self::OFFSET_MOVER;
    //     bits += (capture.index() as u32) << Self::OFFSET_CAPTURE;
    //     bits += (promo.index() as u32) << Self::OFFSET_PROMO;
    //     bits += (castle_side.bits() as u32) << Self::OFFSET_CASTLE;
    //     Move { bits }
    // }

    pub const NULL_MOVE: Move = Move::new_quiet(Piece::Pawn, Square::A1, Square::A1);

    #[inline]
    pub fn new_null() -> Move {
        Move::NULL_MOVE
    }

    #[inline]
    pub const fn to(&self) -> Square {
        Square::from_u32(((self.bits >> Self::OFFSET_TO) & 63) as u32)
    }

    #[inline]
    pub const fn from(&self) -> Square {
        Square::from_u32(((self.bits >> Self::OFFSET_FROM) & 63) as u32)
    }

    #[inline]
    pub const fn ep(&self) -> Square {
        Square::from_u32(((self.bits >> Self::OFFSET_EP) & 127) as u32)
    }

    #[inline]
    pub fn capture_square(&self) -> Square {
        if self.is_ep_capture() {
            self.ep()
        } else if self.is_capture() {
            self.to()
        } else {
            Square::null()
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.to().index() == self.from().index()
    }

    #[inline]
    pub const fn is_promo(&self) -> bool {
        (self.bits >> Self::OFFSET_PROMO) & 7 != 0
    }

    #[inline]
    pub fn is_near_promo(&self) -> bool {
        let c = if self.from() < self.to() {
            Color::White
        } else {
            Color::Black
        };
        self.mover_piece() == Piece::Pawn && self.to().rank_number_as_white(c) == 7
    }

    // #[inline]
    // pub const fn promo_piece(&self) -> Piece {
    //     Piece::from_index((self.bits >> Self::OFFSET_PROMO) as usize & 7)
    // }

    #[inline]
    pub const fn promo(&self) -> Option<Piece> {
        match (self.bits >> Self::OFFSET_PROMO) & 7 {
            0 => None,
            i => Some(Piece::from_index(i as usize)),
        }
    }

    #[inline]
    pub const fn capture_piece(&self) -> Option<Piece> {
        match (self.bits >> Self::OFFSET_CAPTURE) & 7 {
            7 => None,
            p => Some(Piece::from_index(p as usize)),
        }
    }

    #[inline]
    pub const fn mover_piece(&self) -> Piece {
        Piece::from_index((self.bits >> Self::OFFSET_MOVER) as usize & 7)
    }

    #[inline]
    pub fn is_capture(&self) -> bool {
        self.capture_piece().is_some()
    }

    #[inline]
    pub const fn is_castle(&self) -> bool {
        !self.castling_side().is_empty()
    }

    #[inline]
    pub const fn castling_side(&self) -> CastlingRights {
        CastlingRights::from_bits_truncate((self.bits >> Self::OFFSET_CASTLE) as u8)
    }

    #[inline]
    pub fn is_ep_capture(&self) -> bool {
        !self.ep().is_null() && self.is_capture()
    }

    #[inline]
    pub fn is_pawn_double_push(&self) -> bool {
        !self.ep().is_null() && !self.is_capture()
    }

    #[inline]
    pub fn rook_move(&self) -> Move {
        if self.is_castle() {
            let (from, to) = self.rook_move_from_to();
            Move::new_quiet(Piece::Rook, from, to)
        } else {
            Move::NULL_MOVE
        }
    }

    #[inline]
    pub const fn rook_move_from_to(&self) -> (Square, Square) {
        #[allow(non_upper_case_globals)]
        match self.to().as_bb() {
            c1 => (a1.square(), d1.square()),
            g1 => (h1.square(), f1.square()),
            c8 => (a8.square(), d8.square()),
            g8 => (h8.square(), f8.square()),
            _ => (Square::null(), Square::null()),
        }
    }

    #[inline]
    pub fn castling_rights_lost(&self) -> CastlingRights {
        if self.is_null() {
            CastlingRights::NONE
        } else {
            let squares_changing = self.to().as_bb() | self.from().as_bb();
            CastlingRights::rights_lost(squares_changing)
        }
    }

    #[inline]
    pub fn new_pawn_move(from: Square, to: Square, b: &Board) -> Move {
        if to.is_in(b.them()) {
            let cap = b.piece_unchecked(to);
            Move::new_capture(Piece::Pawn, from, to, cap)
        } else {
            // its a push
            let behind = to.shift(b.color_us().backward());
            let ep = behind;
            if behind.as_bb().disjoint(b.pawns()) {
                // no one behind us => double push
                Move::new_double_push(from, to, ep)
            } else {
                Move::new_quiet(Piece::Pawn, from, to)
            }
        }
    }

    #[inline]
    pub fn new_double_push(from: Square, to: Square, ep: Square) -> Move {
        let mut m = Move::new_quiet(Piece::Pawn, from, to);
        m.set_en_passant(ep);
        m
    }

    #[inline]
    pub fn new_capture(p: Piece, from: Square, to: Square, captured: Piece) -> Move {
        let mut m = Move::new_quiet(p, from, to);
        m.set_capture(captured);
        m
    }

    #[inline]
    pub fn new_ep_capture(from: Square, to: Square, captured_sq: Square) -> Move {
        let mut m = Move::new_quiet(Piece::Pawn, from, to);
        m.set_capture(Piece::Pawn);
        m.set_en_passant(captured_sq);
        m
    }

    #[inline]
    pub fn new_promo(from: Square, to: Square, promo: Piece) -> Move {
        let mut m = Move::new_quiet(Piece::Pawn, from, to);
        m.set_promo(promo);
        m
    }

    #[inline]
    pub fn new_promo_capture(from: Square, to: Square, promo: Piece, capture: Piece) -> Move {
        let mut m = Move::new_quiet(Piece::Pawn, from, to);
        m.set_promo(promo);
        m.set_capture(capture);
        m
    }

    #[inline]
    pub fn new_castle(king_from: Square, king_to: Square, castling_side: CastlingRights) -> Move {
        let mut m = Move::new_quiet(Piece::King, king_from, king_to);
        m.set_castling_side(castling_side);
        m
    }

    #[inline]
    pub fn mvv_lva_score(&self) -> i32 {
        let mut score = 0;
        if let Some(cap) = self.capture_piece() {
            score += cap.centipawns() * 10 - self.mover_piece().centipawns() / 10;
        }
        if let Some(promo) = self.promo() {
            score += promo.centipawns() * 10 - Piece::Pawn.centipawns() / 10;
        }
        score
    }

    pub fn to_uci(&self) -> String {
        self.to_inner().to_string()
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_uci())?;
        if f.alternate() {
            write!(f, " m:{}", self.mover_piece())?;

            if !self.ep().is_null() {
                write!(f, " ep:{}", self.ep().uci())?;
            }
            if let Some(c) = self.capture_piece() {
                write!(f, " c:{}", c)?;
            }
            if self.is_castle() {
                write!(f, " cs:{}", self.castling_side())?;
            }
            if self.is_ep_capture() {
                write!(f, " e/p cap")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog::Catalog, perft::Perft};
    // use crate::movelist::MoveValidator;

    #[test]
    fn test_baremove() -> anyhow::Result<()> {
        assert_eq!(BareMove::parse_uci("a1b3")?.to_uci(), "a1b3");
        assert_eq!(BareMove::parse_uci("0000").is_err(), true);
        assert_eq!(BareMove::parse_uci("XYZ").is_err(), true);
        assert_eq!(BareMove::parse_uci("a1a7q")?.to_uci(), "a1a7q");
        Ok(())
    }

    #[test]
    fn test_move() {
        assert_eq!(Move::new_null().to_string(), "0000");

        let move_castle = Move::new_castle(a1.square(), b2.square(), CastlingRights::WHITE_QUEEN);

        println!("{:#} {:b}", move_castle, move_castle.bits);
        assert_eq!(move_castle.from(), Square::A1);
        assert_eq!(move_castle.to(), Square::B2);
        assert_eq!(move_castle.ep(), Square::null());
        assert_eq!(move_castle.capture_piece(), None);
        assert_eq!(move_castle.promo(), None);
        assert_eq!(move_castle.is_promo(), false);
        assert_eq!(move_castle.is_capture(), false);
        assert_eq!(move_castle.is_null(), false);
        assert_eq!(move_castle.castling_side(), CastlingRights::WHITE_QUEEN);

        let move_a1b2 = Move::new_quiet(Piece::Bishop, a1.square(), b2.square());

        println!("{:#} {:b}", move_a1b2, move_a1b2.bits);
        assert_eq!(move_a1b2.from(), a1.square());
        assert_eq!(move_a1b2.to(), b2.square());
        assert_eq!(move_a1b2.mover_piece(), Piece::Bishop);
        assert_eq!(move_a1b2.is_promo(), false);
        assert_eq!(move_a1b2.ep(), Square::null());
        assert_eq!(move_a1b2.castling_side(), CastlingRights::NONE);

        let promo_a7a8 = Move::new_promo(a7.square(), a8.square(), Piece::Queen);

        assert_eq!(move_a1b2.to_string(), "a1b2");
        assert_eq!(promo_a7a8.to_string(), "a7a8q");

        let move_e2e4 = "e2e4".parse::<BareMove>().unwrap();
        assert_eq!(move_e2e4.to_string(), "e2e4");

        let move_e7e8 = BareMove::from_str("e7e8p").unwrap();
        assert_eq!(move_e7e8.to_string(), "e7e8p");

        let board = Catalog::starting_board();
        assert_eq!(board.parse_san_move("Nc3").unwrap().to_string(), "b1c3");
        assert_eq!(board.parse_san_move("c3").unwrap().to_string(), "c2c3");
        assert_eq!(board.parse_san_move("c2c4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("c2-c4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("Pc4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("Pc2c4").unwrap().to_string(), "c2c4");
    }

    #[test]
    fn test_to_and_from_detail() {
        let positions = Catalog::win_at_chess();
        let mut count = 0;
        for pos in positions {
            let mut func = |bd: &Board, mv: Move| {
                assert_eq!(bd.augment_move(mv.to_inner()), mv);
                *&mut count += 1
            };
            Perft::perft_fn(&mut pos.board().clone(), 3, &mut func);
        }
        println!("Count = {count}");
    }

    #[test]
    fn test_mvv_lva() {
        let pxq = Move::new_capture(Piece::Pawn, Square::A1, Square::A2, Piece::Queen);
        let pxr = Move::new_capture(Piece::Pawn, Square::A1, Square::A2, Piece::Rook);
        let pxb = Move::new_capture(Piece::Pawn, Square::A1, Square::A2, Piece::Bishop);
        let pxn = Move::new_capture(Piece::Pawn, Square::A1, Square::A2, Piece::Knight);
        let pxp = Move::new_capture(Piece::Pawn, Square::A1, Square::A2, Piece::Pawn);
        let qxp = Move::new_capture(Piece::Queen, Square::A1, Square::A2, Piece::Pawn);
        let qxn = Move::new_capture(Piece::Queen, Square::A1, Square::A2, Piece::Knight);
        let qxb = Move::new_capture(Piece::Queen, Square::A1, Square::A2, Piece::Bishop);
        let qxr = Move::new_capture(Piece::Queen, Square::A1, Square::A2, Piece::Rook);
        let qxq = Move::new_capture(Piece::Queen, Square::A1, Square::A2, Piece::Queen);

        let pxq_q = Move::new_promo_capture(Square::A1, Square::A2, Piece::Queen, Piece::Queen);

        let p_q = Move::new_promo(Square::A1, Square::A2, Piece::Queen);

        assert_eq!(qxr.capture_piece(), Some(Piece::Rook));
        assert_eq!(qxr.mover_piece(), Piece::Queen);

        assert_eq!(pxq.mvv_lva_score(), 8990);
        assert_eq!(pxr.mvv_lva_score(), 4990);
        assert_eq!(pxb.mvv_lva_score(), 3240);
        assert_eq!(pxn.mvv_lva_score(), 3240);
        assert_eq!(pxp.mvv_lva_score(), 990);

        assert_eq!(qxp.mvv_lva_score(), 910);
        assert_eq!(qxn.mvv_lva_score(), 3160);
        assert_eq!(qxb.mvv_lva_score(), 3160);
        assert_eq!(qxr.mvv_lva_score(), 4910);
        assert_eq!(qxq.mvv_lva_score(), 8910);

        assert_eq!(pxq_q.mvv_lva_score(), 17980);
        assert_eq!(p_q.mvv_lva_score(), 8990);
    }

    #[test]
    fn test_to_san() {
        let mut board = Catalog::starting_board();
        let a2a3 = board.parse_uci_move("a2a3").unwrap();
        let b1c3 = board.parse_uci_move("b1c3").unwrap();
        assert_eq!(board.to_san(&a2a3), "a3");
        assert_eq!(board.to_san(&b1c3), "Nc3");

        let board = board.set(d3, "p").unwrap();
        let board = board.set(f3, "p").unwrap();

        let c2d3 = board.parse_uci_move("c2d3").unwrap();
        assert_eq!(board.to_san(&c2d3), "cxd3");

        let e2d3 = board.parse_uci_move("e2d3").unwrap();
        assert_eq!(board.to_san(&e2d3), "exd3");

        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "Nxf3");

        // knight ambiguity
        let board = board.set(g5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "N1xf3");

        // two knights same rank and file as g5
        let board = board.set(e5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g5f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "Ng5xf3");

        // remove some minor pieces to allow castling
        let board = board.set(Bitboard::RANK_8, "r...k..r").unwrap();
        board.set_turn(Color::Black);
        let castle_k = board.parse_uci_move("e8g8").unwrap();
        assert_eq!(board.to_san(&castle_k), "O-O");
        let castle_q = board.parse_uci_move("e8c8").unwrap();
        assert_eq!(board.to_san(&castle_q), "O-O-O");
    }
}
