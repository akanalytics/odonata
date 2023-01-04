use crate::bits::bitboard::{Bitboard, Dir};
use crate::bits::castling::CastlingRights;
use crate::bits::square::Square;
use crate::board::{Board, BoardCalcs};
use crate::globals::constants::*;
use crate::infra::utils::{Displayable, StringUtils};
use crate::piece::{Color, Piece};
use anyhow::Result;
use anyhow::{anyhow, bail};
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

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct BareMove {
    pub from: Square,
    pub to: Square,
    pub promo: Option<Piece>,
}

impl BareMove {
    #[inline]
    pub fn null() -> Self {
        Default::default()
    }

    pub fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <BareMove as fmt::Display>::fmt(self, f)
    }

    pub fn parse_uci(s: &str) -> anyhow::Result<Self> {
        s.parse()
    }

    pub fn to_uci(&self) -> String {
        Displayable(|fmt| self.fmt_uci(fmt)).to_string()
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.to == self.from
    }

    #[inline]
    pub fn from(&self) -> Square {
        self.from
    }

    #[inline]
    pub fn to(&self) -> Square {
        self.to
    }

    #[inline]
    pub fn promo(&self) -> Option<Piece> {
        self.promo
    }

    pub fn to_san(&self, b: &Board) -> String {
        b.to_san(b.augment_move(*self))
    }

    pub fn is_castle(&self, b: &Board) -> bool {
        self.from.is_in(b.kings()) && CastlingRights::is_castling(self.from, self.to)
    }
}

// impl TryFrom<&str> for BareMove {
//     type Error = anyhow::Error;

//     fn try_from(s: &str) -> Result<Self, Self::Error> {
//         Self::from_str(s)
//     }

// }

impl FromStr for BareMove {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.trim() == "0000" {
            return Ok(Self::null());
        }
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
    pub fn is_castling_move_legal(&self, mv: BareMove) -> bool {
        let c = self.color_us();
        let right = CastlingRights::from_king_move(mv.to());
        let king = mv.from();
        if CastlingRights::is_king_side(&right) {
            if self.castling().contains(right)
                && !CastlingRights::king_side_move_squares(c).intersects(self.occupied())
            {
                let rook_to = king.shift(Dir::E);
                let king_to = rook_to.shift(Dir::E);
                let king_moves = king.as_bb() | rook_to.as_bb() | king_to.as_bb();
                if BoardCalcs::attacked_by(king_moves, self.occupied(), self).disjoint(self.them())
                {
                    return true;
                }
            }
        }
        if CastlingRights::is_queen_side(&right) {
            if self.castling().contains(right)
                && !CastlingRights::queen_side_move_squares(c).intersects(self.occupied())
            {
                let rook_to = king.shift(Dir::W);
                let king_to = rook_to.shift(Dir::W);
                let king_moves = king.as_bb() | rook_to.as_bb() | king_to.as_bb();
                if BoardCalcs::attacked_by(king_moves, self.occupied(), self).disjoint(self.them())
                {
                    return true;
                }
            }
        }
        false
    }

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

type UMOVE = u16;

// FIXME: public methods
#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Move {
    bits: UMOVE,
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Move")
            .field("uci", &self.to_uci())
            .field("from", &self.from())
            .field("to", &self.to())
            .field("is_capture", &self.is_capture())
            // .field("castling", &self.castling_side())
            .field("promo", &self.promo())
            // .field("ep", &self.ep())
            .field(
                "bits",
                &format!("{:064b}", self.bits)
                    .chars()
                    .collect::<Vec<char>>()
                    .chunks(8)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<String>>()
                    .join(" "),
            )
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
    const OFFSET_IS_CAPTURE: i32 = 12; 
    const OFFSET_PROMO: i32 = 13;      // 13, 14, 15
    // const OFFSET_CASTLE: i32 = 28;

    // from = 6 bits
    // to = 6 bits
    // flags = 3 bits// 7 bits
    //   castle

    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_null() {
            write!(f, "0000")
        } else {
            write!(f, "{}{}", self.from().uci(), self.to().uci())?;
            if let Some(p) = self.promo() {
                write!(f, "{}", p.to_char(Color::Black))?
            }
            Ok(())
        }
    }

    #[inline]
    pub const fn to_inner(&self) -> BareMove {
        BareMove {
            from: self.from(),
            to: self.to(),
            promo: self.promo(),
        }
    }

    // /// Ng1-f3 Nb8-c6
    // /// Bb5xNc6 d7xBc6
    // /// d2-d3 Bf8-b4+
    // /// 0-0 Bb4xNc3
    // pub fn parse_lan(mut s: &str) -> Result<Self> {
    //     if s == "0000" {
    //         return Ok(Move::new_null());
    //     }
    //     let gives_check = if let Some(t) = s.strip_suffix("+") {
    //         s = t;
    //         true
    //     } else {
    //         false
    //     };
    //     let mover = if let Some(t) = s.strip_prefix(['P', 'N', 'B', 'R', 'Q', 'K']) {
    //         s = t;
    //         Piece::from_char(s.chars().next().unwrap())?
    //     } else {
    //         Piece::Pawn
    //     };

    //     Err("")
    // }

    pub fn parse_uci(s: &str, b: &Board) -> Result<Self> {
        if s.trim() == "0000" {
            return Ok(Self::new_null());
        }
        let from = Bitboard::parse_square(s.take_slice(0..2))?;
        let to = Bitboard::parse_square(s.take_slice(2..4))?;
        if let Some(ch) = s.take_char_at(4) {
            let _promo = Piece::from_char(ch)?;
            if from.rank_number_as_white(b.color_us()) != 7 {
                bail!("move {s} from {from} sq is not rank 2/7 for board {b}");
            }
            if to.rank_number_as_white(b.color_us()) != 8 {
                bail!("move {s} from {from} sq is not rank 1/8 for board {b}");
            }
        } else {
            let _mover = b
                .piece(from)
                .ok_or_else(|| anyhow!("move {s} no piece on {from} for board {b}"))?;
            if !from.is_in(b.us()) {
                bail!("move {s} has wrong color mover for board {b}");
            }
        }
        b.legal_moves()
            .iter()
            .find(|mv| mv.to_uci() == s)
            .ok_or_else(|| anyhow!("move {s} is not a legal move for board {b}"))
            .cloned()
    }

    #[inline]
    pub const fn new_quiet(_mover: Piece, from: Square, to: Square) -> Move {
        let mut bits = (from.index() as UMOVE & 63) << Self::OFFSET_FROM;
        bits += (to.index() as UMOVE & 63) << Self::OFFSET_TO;
        // bits += (mover.index() as u32) << Self::OFFSET_MOVER;
        // bits += (Square::null().index() as u32 & 127) << Self::OFFSET_EP;
        // bits += 7 << Self::OFFSET_CAPTURE;
        Move { bits }
    }

    #[inline]
    pub fn set_capture(&mut self, _capture: Piece) {
        self.bits |= 1 << Self::OFFSET_IS_CAPTURE;
        // self.bits &= !(7 << Self::OFFSET_CAPTURE);
        // self.bits += (capture.index() as u32) << Self::OFFSET_CAPTURE;
    }

    #[inline]
    pub fn set_promo(&mut self, promo: Piece) {
        self.bits += (promo.index() as UMOVE) << Self::OFFSET_PROMO;
    }

    #[inline]
    pub fn set_en_passant(&mut self, _ep_sq: Square) {
        // self.bits &= !(127 << Self::OFFSET_EP);
        // self.bits += (ep_sq.index() as u32 & 127) << Self::OFFSET_EP;
    }

    #[inline]
    pub fn set_castling_side(&mut self, _castle_side: CastlingRights) {
        // self.bits += (castle_side.bits() as u32) << Self::OFFSET_CASTLE;
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

    pub fn to_san(&self, b: &Board) -> String {
        b.to_san(*self)
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
        Square::from_u32((self.from().index() + self.to().index()) as u32 / 2)
    }

    #[inline]
    pub fn capture_square(&self, b: &Board) -> Square {
        if self.is_ep_capture(b) {
            Square::from_xy(
                self.to().file_index() as u32,
                self.from().rank_index() as u32,
            )
        } else if self.is_capture() {
            self.to()
        } else {
            debug_assert!(false, "capture square {self} on board {b}");
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
    pub fn is_near_promo(&self, bd: &Board) -> bool {
        let c = if self.from() < self.to() {
            Color::White
        } else {
            Color::Black
        };
        self.mover_piece(bd) == Piece::Pawn && self.to().rank_number_as_white(c) == 7
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
    pub fn capture_piece(&self, b: &Board) -> Option<Piece> {
        if !self.is_capture() {
            None
        } else {
            // match (self.bits >> Self::OFFSET_CAPTURE) & 7 {
            //     7 => None,
            //     p => Some(Piece::from_index(p as usize)),
            // }
            b.piece(self.capture_square(b))
        }
    }

    #[inline]
    pub fn mover_piece(&self, bd: &Board) -> Piece {
        bd.piece_unchecked(self.from())
    }

    #[inline]
    pub fn is_capture(&self) -> bool {
        self.bits & (1 << Self::OFFSET_IS_CAPTURE) > 0
        // self.capture_piece(b).is_some()
    }

    #[inline]
    pub fn is_castle(&self, b: &Board) -> bool {
        !self.castling_side(b).is_empty()
    }

    #[inline]
    pub fn castling_side(&self, b: &Board) -> CastlingRights {
        // CastlingRights::from_bits_truncate((self.bits >> Self::OFFSET_CASTLE) as u8)
        if self.mover_piece(b) == Piece::King && CastlingRights::is_castling(self.from(), self.to())
        {
            CastlingRights::from_king_move(self.to())
        } else {
            CastlingRights::NONE
        }
    }

    #[inline]
    pub fn is_ep_capture(&self, b: &Board) -> bool {
        self.mover_piece(b) == Piece::Pawn && self.to().is_in(b.en_passant())
    }

    #[inline]
    pub fn is_pawn_double_push(&self, b: &Board) -> bool {
        self.mover_piece(b) == Piece::Pawn
            && (self.to().index() == self.from().index() + 16
                || self.to().index() + 16 == self.from().index())
    }

    #[inline]
    pub fn rook_move(&self, b: &Board) -> Move {
        if self.is_castle(b) {
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
    pub fn mvv_lva_score(&self, bd: &Board) -> i32 {
        debug_assert!(bd.is_legal_move(*self), "{self} is illegal for board {bd}");
        let mut score = 0;
        if let Some(cap) = self.capture_piece(bd) {
            score += cap.centipawns() * 10 - self.mover_piece(bd).centipawns() / 10;
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
            if !self.ep().is_null() {
                write!(f, " ep:{}", self.ep().uci())?;
            }
            // if let Some(c) = self.capture_piece() {
            //     write!(f, " c:{}", c)?;
            // }
            // if self.is_castle() {
            //     write!(f, " cs:{}", self.castling_side())?;
            // }
            // if self.is_ep_capture() {
            //     write!(f, " e/p cap")?;
            // }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog::Catalog, perft::Perft, Position};
    // use crate::movelist::MoveValidator;

    #[test]
    fn test_parse_uci() -> anyhow::Result<()> {
        let b = Board::starting_pos();
        assert_eq!(Move::parse_uci("a2a3", &b)?.to_uci(), "a2a3");
        assert_eq!(Move::parse_uci("0000", &b).is_err(), false);
        assert_eq!(Move::parse_uci("XYZ", &b).is_err(), true);
        let b = Position::parse_epd(
            r"
            ........
            P.......
            ........
            R.K.....
            ........
            ........
            ........
            .....n.k w KQkq - 1 1",
        )
        .unwrap()
        .board()
        .clone();
        assert_eq!(Move::parse_uci("a7a8q", &b)?.to_uci(), "a7a8q");
        Ok(())
    }

    #[test]
    fn test_move() {
        assert_eq!(Move::new_null().to_string(), "0000");

        let move_castle = Move::new_castle(a1.square(), b2.square(), CastlingRights::WHITE_QUEEN);

        println!("{:#} {:b}", move_castle, move_castle.bits);
        assert_eq!(move_castle.from(), Square::A1);
        assert_eq!(move_castle.to(), Square::B2);

        // ep cant be called on castling move
        // assert_eq!(move_castle.ep(), Square::null());
        // assert_eq!(move_castle.capture_piece(), None);
        assert_eq!(move_castle.promo(), None);
        assert_eq!(move_castle.is_promo(), false);
        assert_eq!(move_castle.is_capture(), false);
        assert_eq!(move_castle.is_null(), false);
        // assert_eq!(move_castle.castling_side(), CastlingRights::WHITE_QUEEN);

        let move_a1b2 = Move::new_quiet(Piece::Bishop, a1.square(), b2.square());

        println!("{:#} {:b}", move_a1b2, move_a1b2.bits);
        assert_eq!(move_a1b2.is_capture(), false);
        assert_eq!(move_a1b2.from(), a1.square());
        assert_eq!(move_a1b2.to(), b2.square());
        // assert_eq!(move_a1b2.mover_piece(), Piece::Bishop);
        assert_eq!(move_a1b2.is_promo(), false);

        let capture_a1b2 =
            Move::new_capture(Piece::Bishop, a1.square(), b2.square(), Piece::Knight);
        assert_eq!(capture_a1b2.is_capture(), true);

        // ep cant be called on castling move
        // assert_eq!(move_a1b2.ep(), Square::null());
        // assert_eq!(move_a1b2.castling_side(), CastlingRights::NONE);

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
        let pos = Position::parse_epd(
            r"
            k.....qb
            .....PnQ
            .....P..
            q.r.....
            .P..pb.r
            ...P..P.
            ......p.
            .K.....Q w - - 1 1",
        )
        .unwrap();
        let bd = pos.board();

        let pxq = bd.parse_san_move("Pxa5").unwrap();
        let pxr = bd.parse_san_move("Pxc5").unwrap();
        let pxb = bd.parse_san_move("Pxf4").unwrap();
        let pxn = bd.parse_san_move("Pxg7").unwrap();
        let pxp = bd.parse_san_move("Pxe4").unwrap();
        let qxp = bd.parse_san_move("Qxg2").unwrap();
        let qxn = bd.parse_san_move("Qxg7").unwrap();
        let qxb = bd.parse_san_move("Qxh8").unwrap();
        let qxr = bd.parse_san_move("Q1xh4").unwrap();
        let qxq = bd.parse_san_move("Qxg8").unwrap();

        let pxq_q = bd.parse_san_move("Pxg8=Q").unwrap();

        let p_q = bd.parse_san_move("f8=Q").unwrap();

        assert_eq!(qxr.capture_piece(&bd), Some(Piece::Rook));
        assert_eq!(qxr.mover_piece(&bd), Piece::Queen);

        assert_eq!(pxq.mvv_lva_score(&bd), 8990);
        assert_eq!(pxr.mvv_lva_score(&bd), 4990);
        assert_eq!(pxb.mvv_lva_score(&bd), 3240);
        assert_eq!(pxn.mvv_lva_score(&bd), 3240);
        assert_eq!(pxp.mvv_lva_score(&bd), 990);

        assert_eq!(qxp.mvv_lva_score(&bd), 910);
        assert_eq!(qxn.mvv_lva_score(&bd), 3160);
        assert_eq!(qxb.mvv_lva_score(&bd), 3160);
        assert_eq!(qxr.mvv_lva_score(&bd), 4910);
        assert_eq!(qxq.mvv_lva_score(&bd), 8910);

        assert_eq!(pxq_q.mvv_lva_score(&bd), 17980);
        assert_eq!(p_q.mvv_lva_score(&bd), 8990);
    }

    #[test]
    fn test_to_san() {
        let mut board = Catalog::starting_board();
        let a2a3 = board.parse_uci_move("a2a3").unwrap();
        let b1c3 = board.parse_uci_move("b1c3").unwrap();
        assert_eq!(board.to_san(a2a3), "a3");
        assert_eq!(board.to_san(b1c3), "Nc3");

        let board = board.set(d3, "p").unwrap();
        let board = board.set(f3, "p").unwrap();

        let c2d3 = board.parse_uci_move("c2d3").unwrap();
        assert_eq!(board.to_san(c2d3), "cxd3");

        let e2d3 = board.parse_uci_move("e2d3").unwrap();
        assert_eq!(board.to_san(e2d3), "exd3");

        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(g1f3), "Nxf3");

        // knight ambiguity
        let board = board.set(g5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(g1f3), "N1xf3");

        // two knights same rank and file as g5
        let board = board.set(e5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g5f3").unwrap();
        assert_eq!(board.to_san(g1f3), "Ng5xf3");

        // remove some minor pieces to allow castling
        let board = board.set(Bitboard::RANK_8, "r...k..r").unwrap();
        board.set_turn(Color::Black);
        let castle_k = board.parse_uci_move("e8g8").unwrap();
        assert_eq!(board.to_san(castle_k), "O-O");
        let castle_q = board.parse_uci_move("e8c8").unwrap();
        assert_eq!(board.to_san(castle_q), "O-O-O");
    }
}
