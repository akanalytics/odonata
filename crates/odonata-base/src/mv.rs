use std::fmt::{self, Debug, Display};
use std::str::FromStr;

use anyhow::{anyhow, bail, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::bits::bitboard::{Bitboard, Dir};
use crate::bits::castling::CastlingRights;
use crate::bits::square::Square;
use crate::boards::BoardCalcs;
use crate::infra::utils::{StringUtils, ToStringOr};
use crate::piece::{Color, Piece};
use crate::prelude::Board;
use crate::{FlipVertical, PreCalc};

// #[derive(Copy, Clone, PartialEq, Eq, Hash)]
// pub struct ReversibleMove {
//     bits: u32,
//     // pub mover:   Piece,
//     // pub from:    Square,
//     // pub to:      Square,
//     // pub promo:   Option<Piece>,
//     // pub capture: Option<Piece>,
//     // pub cr_lost: CastlingRights,
//     // pub ep_file: u8,
// }

// impl fmt::Display for ReversibleMove {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.write_str(&self.to_uci())?;
//         Ok(())
//     }
// }

// impl ReversibleMove {
// 6    const OFFSET_FROM: i32 = 0;
// 6    const OFFSET_TO: i32 = 6;
// 1    const OFFSET_PROMO: i32 = 12; // 1 bits
// 3    const OFFSET_MOVER/PROMO: i32 = 16; // 3 bits

// 3    const OFFSET_CAPTURE: i32 = 19; // 3 bits
// 4    const OFFSET_PRIOR_EP: i32 = 22; // 6 bits
// 2    const OFFSET_PRIOR_CR: i32 = 28; // 4 bits
// 7    hmvc

//     pub const fn to(&self) -> Square {
//         Square::from_u8((self.bits >> Self::OFFSET_TO) as u8 & 63)
//     }

//     pub const fn from(&self) -> Square {
//         Square::from_u8((self.bits >> Self::OFFSET_FROM) as u8 & 63)
//     }

//     pub fn mover_piece(&self, _bd: &Board) -> Piece {
//         ((self.bits >> Self::OFFSET_MOVER) as u8 & 7).into()
//     }

//     const fn flag(&self) -> MoveFlag {
//         MoveFlag::from_index((self.bits >> Move::OFFSET_FLAG) as u16)
//     }

//     pub fn capture_piece(&self, _bd: &Board) -> Option<Piece> {
//         match self.flag().is_capture() {
//             false => None,
//             true => Some(((self.bits >> Self::OFFSET_CAPTURE) as u8 & 7).into()),
//         }
//     }

//     pub fn prior_ep_square(&self) -> Option<Square> {
//         let index = (self.bits >> Self::OFFSET_PRIOR_EP) as u8 & 63;
//         match index {
//             0 => None,
//             u => Some(u.into()),
//         }
//     }

//     pub fn is_castle(&self, _b: &Board) -> bool {
//         self.flag().is_castling()
//         // !self.castling_side(b).is_empty()
//     }

//     pub fn castling_side(&self, b: &Board) -> CastlingRights {
//         // CastlingRights::from_bits_truncate((self.bits >> Self::OFFSET_CASTLE) as u8)
//         if self.is_castle(b) {
//             CastlingRights::from_king_move(self.to())
//         } else {
//             CastlingRights::NONE
//         }
//     }

//     pub const fn rook_move_from_to(&self) -> (Square, Square) {
//         #[allow(non_upper_case_globals)]
//         match self.to().as_bb() {
//             c1 => (Square::A1, Square::D1),
//             g1 => (Square::H1, Square::F1),
//             c8 => (Square::A8, Square::D8),
//             g8 => (Square::H8, Square::F8),
//             _ => (Square::null(), Square::null()),
//         }
//     }

//     pub const fn to_inner(&self) -> BareMove {
//         BareMove {
//             mover: None,
//             from:  self.from(),
//             to:    self.to(),
//             promo: self.promo_piece(),
//         }
//     }

//     /// if mv is invalid then maybe mover is not set
//     pub fn to_inner_with_mover(&self, bd: &Board) -> BareMove {
//         BareMove {
//             mover: bd.piece(self.from()),
//             from:  self.from(),
//             to:    self.to(),
//             promo: self.promo_piece(),
//         }
//     }

//     pub const fn promo_piece(&self) -> Option<Piece> {
//         self.flag().promo_piece()
//     }

//     pub fn validate(&self, bd: &Board) -> std::result::Result<(), &'static str> {
//         // let mv = self.to_inner().validate(bd)?;
//         // if *self != mv {
//         //     return Err("move flags likely wrong");
//         // }
//         Ok(())
//     }

//     pub fn is_valid(&self, bd: &Board) -> bool {
//         self.validate(bd).is_ok()
//     }

//     pub fn mvv_lva_score(&self, bd: &Board) -> i32 {
//         debug_assert!(self.is_valid(bd), "{self} is illegal for board {bd}");
//         let mut score = 0;
//         if let Some(cap) = self.capture_piece(bd) {
//             score += cap.centipawns() * 10 - self.mover_piece(bd).centipawns() / 10;
//         }
//         if let Some(promo) = self.promo_piece() {
//             score += promo.centipawns() * 10 - Piece::Pawn.centipawns() / 10;
//         }
//         score
//     }

//     pub fn to_uci(&self) -> String {
//         self.to_inner().to_string()
//     }

//     #[inline]
//     pub fn castling_rights_lost(&self) -> CastlingRights {
//         if self.is_null() {
//             CastlingRights::NONE
//         } else {
//             // let squares_changing = self.to().as_bb() | self.from().as_bb();
//             CastlingRights::rights_lost(self.from(), self.to())
//         }
//     }

//     #[inline]
//     pub fn is_pawn_double_push(&self, _b: &Board) -> bool {
//         self.flag().is_pawn_double_push()
//         // self.mover_piece(b) == Piece::Pawn
//         //     && (self.to().index() == self.from().index() + 16
//         //         || self.to().index() + 16 == self.from().index())
//     }

//     pub const fn is_promo(&self) -> bool {
//         self.flag().promo_piece().is_some()
//     }

//     #[inline]
//     pub fn is_near_promo(&self, bd: &Board) -> bool {
//         let c = if self.from() < self.to() {
//             Color::White
//         } else {
//             Color::Black
//         };
//         self.mover_piece(bd) == Piece::Pawn && self.to().rank_number_as_white(c) == 7
//     }

//     pub const fn is_null(&self) -> bool {
//         self.to().index() == self.from().index() // partial.eq isn't const
//     }

//     pub fn is_en_passant_capture(&self) -> bool {
//         self.flag().is_en_passant_capture()
//     }

//     // pub fn is_pawn_double_push(&self) -> bool {
//     //     self.flag().is_pawn_double_push()
//     // }

//     pub fn capture_square(&self) -> Square {
//         if !self.is_en_passant_capture() {
//             self.to()
//         } else {
//             Square::from_xy(
//                 self.to().file_index() as u32,
//                 self.from().rank_index() as u32,
//             )
//         }
//     }

//     pub fn castling_rights_delta(&self) -> CastlingRights {
//         CastlingRights::from_bits_retain((self.bits >> Self::OFFSET_PRIOR_CR & 15) as u8)
//     }
// }

// impl ReversibleMove {
//     pub fn parse_uci(s: &str, b: &Board) -> Result<Self> {
//         if s.trim() == "0000" {
//             return Ok(Self::new_null());
//         }
//         let from = Square::parse(s.take_slice(0..2))?;
//         let to = Square::parse(s.take_slice(2..4))?;
//         if let Some(ch) = s.take_char_at(4) {
//             let _promo = Piece::from_char(ch)?;
//             if from.rank_number_as_white(b.color_us()) != 7 {
//                 bail!("move {s} from {from} sq is not rank 2/7 for board {b}");
//             }
//             if to.rank_number_as_white(b.color_us()) != 8 {
//                 bail!("move {s} from {from} sq is not rank 1/8 for board {b}");
//             }
//         } else {
//             let _mover = b
//                 .piece(from)
//                 .ok_or_else(|| anyhow!("move {s} no piece on {from} for board {b}"))?;
//             if !from.is_in(b.us()) {
//                 bail!("move {s} has wrong color mover for board {b}");
//             }
//         }
//         b.legal_moves()
//             .iter()
//             .find(|mv| mv.to_uci() == s)
//             .ok_or_else(|| anyhow!("move {s} is not a legal move for board {b}"))
//             .cloned()
//     }

//     pub fn display(moves: &[Move]) -> String {
//         "[".to_string() + &moves.iter().join(", ") + "]"
//     }
//     pub fn flip_vertical(&mut self) {
//         if self.is_null() {
//             return;
//         }
//         let from = self.from().flip_vertical();
//         let to = self.to().flip_vertical();

//         self.bits &= !(63 << Self::OFFSET_FROM);
//         self.bits &= !(63 << Self::OFFSET_TO);

//         self.bits |= (from.index() as u32 & 63) << Self::OFFSET_FROM;
//         self.bits |= (to.index() as u32 & 63) << Self::OFFSET_TO;
//     }

//     const NULL_MOVE: Move = Self::new_null();

//     #[inline]
//     pub fn new_move(
//         mover: Piece,
//         from: Square,
//         to: Square,
//         promo: Option<Piece>,
//         bd: &Board,
//     ) -> Move {
//         if let Some(promo) = promo {
//             if to.is_in(bd.occupied()) {
//                 Self::new_promo_capture(from, to, promo, bd)
//             } else {
//                 Self::new_promo(from, to, promo, bd)
//             }
//         } else if mover == Piece::Pawn {
//             Self::new_pawn_move(from, to, bd)
//         } else if mover == Piece::King && CastlingRights::is_castling(from, to) {
//             Self::new_castle(from, to, bd)
//         } else if to.is_in(bd.occupied()) {
//             Self::new_capture(mover, from, to, bd)
//         } else {
//             Self::new_quiet(mover, from, to, bd)
//         }
//     }

//     #[inline]
//     pub const fn new_quiet(_mover: Piece, from: Square, to: Square, _bd: &Board) -> Move {
//         let mut bits = (from.index() as UMove & 63) << Move::OFFSET_FROM;
//         bits += (to.index() as UMove & 63) << Move::OFFSET_TO;
//         Move { bits }
//     }

//     #[inline]
//     pub const fn new_null() -> Self {
//         let mut bits = (Square::A1.index() as u32 & 63) << Move::OFFSET_FROM;
//         bits += (Square::A1.index() as u32 & 63) << Move::OFFSET_TO;
//         Self { bits }
//     }

//     #[inline]
//     pub fn new_pawn_move(from: Square, to: Square, bd: &Board) -> Move {
//         if to.file() != from.file() {
//             if bd.is_en_passant_square(to) {
//                 let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
//                 m.set_capture(bd);
//                 m.set_en_passant();
//                 m
//             } else {
//                 // let cap = Piece::Pawn; //bd.piece_unchecked(to);
//                 Self::new_capture(Piece::Pawn, from, to, bd)
//             }
//         } else {
//             // its a push
//             let behind = to.shift(bd.color_us().backward());
//             let ep = behind;
//             if behind.as_bb().disjoint(bd.pawns()) {
//                 // no one behind us => double push
//                 Self::new_double_push(from, to, ep, bd)
//             } else {
//                 Self::new_quiet(Piece::Pawn, from, to, bd)
//             }
//         }
//     }

//     #[inline]
//     fn new_double_push(from: Square, to: Square, ep: Square, bd: &Board) -> Move {
//         let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
//         m.set_double_push(ep);
//         m
//     }

//     #[inline]
//     pub fn new_capture(p: Piece, from: Square, to: Square, bd: &Board) -> Move {
//         let mut m = Self::new_quiet(p, from, to, bd);
//         m.set_capture(bd);
//         m
//     }

//     #[inline]
//     pub fn new_ep_capture(from: Square, to: Square, bd: &Board) -> Move {
//         let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
//         m.set_capture(bd);
//         m.set_en_passant();
//         m
//     }

//     #[inline]
//     pub fn new_promo(from: Square, to: Square, promo: Piece, bd: &Board) -> Move {
//         let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
//         m.set_promo(promo);
//         m
//     }

//     #[inline]
//     pub fn new_promo_capture(from: Square, to: Square, promo: Piece, bd: &Board) -> Move {
//         let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
//         m.set_promo(promo);
//         m.set_capture(bd);
//         m
//     }

//     #[inline]
//     pub fn new_castle(king_from: Square, king_to: Square, bd: &Board) -> Move {
//         let mut m = Self::new_quiet(Piece::King, king_from, king_to, bd);
//         m.set_castling();
//         m
//     }

//     #[inline]
//     fn set_flag(&mut self, flag: MoveFlag) {
//         self.bits &= !(15 << Move::OFFSET_FLAG);
//         self.bits |= (flag.index() as u32) << Self::OFFSET_FLAG;
//     }

//     #[inline]
//     fn set_capture(&mut self, _bd: &Board) {
//         // let cap = bd
//         //     .piece(to)
//         //     .unwrap_or_else(|| panic!("No piece on board {bd} for from:{from} to:{to}"));
//         self.set_flag(self.flag().with_capture());
//     }

//     #[inline]
//     fn set_promo(&mut self, promo: Piece) {
//         self.set_flag(self.flag().with_promo(promo));
//     }

//     #[inline]
//     fn set_en_passant(&mut self) {
//         self.set_flag(MoveFlag::EnPassantCapture);
//     }

//     #[inline]
//     fn set_double_push(&mut self, _ep_sq: Square) {
//         self.set_flag(MoveFlag::PawnDoublePush);
//     }

//     #[inline]
//     fn set_castling(&mut self) {
//         self.set_flag(MoveFlag::Castle);
//     }
// }

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BareMove {
    pub mover: Option<Piece>,
    pub from:  Square,
    pub to:    Square,
    pub promo: Option<Piece>,
}

impl Debug for BareMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl FromStr for BareMove {
    type Err = anyhow::Error;

    /// either uci or lan acceptable eg e4e5, e4xd5, e7e8p, Ra1a8, Bc1xd2
    fn from_str(s: &str) -> Result<Self> {
        Self::parse_uci(s).or_else(|_| Self::parse_lan(s))
    }
}

impl fmt::Display for BareMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "0000")
        } else {
            f.write_str(self.from.uci())?;
            f.write_str(self.to.uci())?;
            if let Some(promo) = self.promo {
                f.write_str(&promo.to_string())?;
            }
            Ok(())
        }
    }
}
impl BareMove {
    pub fn null() -> Self {
        Self {
            mover: None,
            from:  Square::A1,
            to:    Square::A1,
            promo: None,
        }
    }

    pub fn is_null(&self) -> bool {
        self.to == self.from
    }

    pub fn is_castle(&self, b: &Board) -> bool {
        self.from.is_in(b.kings()) && CastlingRights::is_castling(self.from, self.to)
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        if s.trim() == "0000" {
            return Ok(Self::null());
        }
        let from = Square::parse(s.take_slice(0..2))?;
        let to = Square::parse(s.take_slice(2..4))?;
        let promo = if let Some(ch) = s.take_char_at(4) {
            Some(Piece::from_char(ch)?)
        } else {
            None
        };
        Ok(BareMove {
            mover: None,
            from,
            to,
            promo,
        })
    }

    // lan = Ra1a4 Ra1-a4 Ra1xa4 e4e5 e4-e5 e4xd5 e7e8Q
    fn parse_lan(s: &str) -> anyhow::Result<Self> {
        if s.trim() == "0000" {
            return Ok(Self::null());
        }
        let s = s.replace('-', "");
        let mut s = s.replace('x', "");
        let mover = if s.starts_with(['K', 'Q', 'R', 'B', 'N']) {
            let (first, rest) = s.split_at(1);
            let piece = Some(Piece::parse(first)?);
            s = rest.to_string();
            piece
        } else {
            Some(Piece::Pawn)
        };
        Ok(BareMove {
            mover,
            ..Self::parse_uci(&s)?
        })
    }

    pub fn validate(&self, bd: &Board) -> Result<Move, &'static str> {
        let mv = self;
        let color_us = bd.color_us();
        if mv.is_null() {
            if bd.is_in_check(color_us) {
                return Err("null move invalid when in check");
            }
            return Ok(Move::new_null());
        }
        let from = mv.from;
        let to = mv.to;
        let us = bd.us();
        let them = bd.them();

        if !from.is_in(us) {
            return Err("from-sq is not our color");
        }
        let Some(mover) = bd.piece(from) else {
            return Err("no mover piece on board");
        };

        if let Some(mv_mover) = mv.mover {
            if mv_mover != mover {
                return Err("piece on to-square doesn't match mover (from move)");
            }
        }

        if to.is_in(us) {
            return Err("to-sq is our color");
        }

        // let _capture_piece = bd.piece(to);

        if let Some(promo) = mv.promo {
            if mover != Piece::Pawn {
                return Err("mover piece (on board from-sq) must be a pawn");
            }
            if promo == Piece::Pawn || promo == Piece::King {
                return Err("cannot promote to a king or a pawn");
            }
        }

        let pc = PreCalc::instance();

        // check pawn movement
        if mover == Piece::Pawn {
            if (pc.strictly_between(from, to) & bd.occupied()).any() {
                return Err("cannot double push when skip square is occupied");
            }
            if from.file_index() == to.file_index() {
                // if its a push - to-sq must be empty
                if to.is_in(bd.occupied()) {
                    return Err("pawn push must be to an empty square");
                }
            } else {
                // if its a pawn capture - to-sq must be en-passant-sq or enemy
                if !bd.is_en_passant_square(to) && !to.is_in(them) {
                    return Err("pawn capture must be to enemy square or en-passant square");
                }
            }
            if to.is_in(Bitboard::RANKS_18) {
                if mv.promo.is_none() {
                    return Err("pawn move to back rank must be a promotion");
                }
            } else if mv.promo.is_some() {
                return Err("must move to back rank to be a pawn promotion");
            }
        }

        if mv.is_castle(bd) {
            let rights = CastlingRights::from_king_move(to);
            if !bd.castling().contains(rights) {
                return Err("no castling rights");
            }
            if bd.is_in_check(color_us) {
                return Err("cannot castle when in check");
            }
            let king_in_between = unsafe { Square::from_u8_unchecked((from.index() + to.index()) as u8 / 2) };
            if king_in_between.is_in(us | them) || to.is_in(them) {
                return Err("cannot castle when king step/to square is occupied");
            }
            if king_in_between.is_in(us | them) || to.is_in(them) {
                return Err("cannot castle when king-step/to square is occupied");
            }
            // west of king-to is either the king-step square (king side) or the rook step-over sq (queens)
            // in either case it must be empty, but the step-sq situation is checked above hence the specific error
            if to.shift(Dir::W).is_in(us | them) {
                return Err("rook step-square must be empty");
            }
            if BoardCalcs::attacked_by(king_in_between.as_bb(), us | them, bd).intersects(them) {
                return Err("cannot castle when king moves through check");
            }
        } else {
            let atts = pc.attacks(color_us, mover, us, them, from);
            #[allow(clippy::nonminimal_bool)]
            if !to.is_in(atts)
                && !(mover == Piece::Pawn
                    && bd.is_en_passant_square(to)
                    && to.is_in(pc.pawn_capture_attacks_from_sq(color_us, from)))
            {
                trace!(
                    "{mover} {from} -> {to}\n{atts} {bd} {ep}",
                    ep = bd.is_en_passant_square(to)
                );
                return Err("to-square is not a valid move-to square for the piece");
            }
        }

        // pseudo-legal move - test for in check - castling already handled at this point
        //
        // idea - lightweight make_move - no hash - just enough to check rays of sliders etc
        // this also means that if the king didnt
        let from_to_bits = from.as_bb() | to.as_bb();
        let mut our_king = bd.kings() & us;

        // make the the occupied sqs reflect any move,
        let us = us ^ from_to_bits;

        // or remove any en passant captures from them, in case the captured pawn (via e/p capture ) was blocking an attack
        let them = if bd.is_en_passant_square(to)
            && from.is_in(bd.pawns())
            && !to.is_in(them)
            && from.file_index() != to.file_index()
        {
            let capture_sq = Square::from_xy(to.file_index() as u32, from.rank_index() as u32);
            them - capture_sq.as_bb()
        } else {
            // .. and opponents sqs reflect any regular captures
            them - us
        };

        // if its not a king move and the piece isn't pinned and the king is not in check and its not an ep capture
        let skip_king_attacked_test = !from.is_in(bd.pinned(color_us))
            && mover != Piece::King
            && !bd.is_in_check(color_us)
            && !bd.is_en_passant_square(to);
        if our_king.any() && !skip_king_attacked_test {
            // if the king moves, adjust the king square
            if mover == Piece::King {
                our_king ^= from_to_bits;
            }

            debug_assert!(our_king.popcount() == 1, "move {mv} on board {bd} king square failure");

            // king square (or square the king will end up on)
            let ksq = our_king.find_first_square().unwrap();

            // note that the king_sq itself wont be seen as an attack so capturing a piece is fine
            let occ = us | them;
            if (pc.rook_attacks(occ, ksq) & bd.rooks_or_queens() & them).any() {
                return Err("king could be attacked by rook or queen");
            }

            if (pc.knight_attacks(ksq) & bd.knights() & them).any() {
                return Err("king could be attacked by knight");
            }

            if (pc.bishop_attacks(occ, ksq) & bd.bishops_or_queens() & them).any() {
                return Err("king could be attacked by bishop or queen");
            }

            if (pc.pawn_attackers(our_king, bd.color_them()) & bd.pawns() & them).any() {
                return Err("king could be attacked by pawns");
            }

            if (pc.king_attacks(ksq) & bd.kings() & them).any() {
                return Err("king could be attacked by opponent king");
            }
        }

        Ok(Move::new_move(mover, from, to, mv.promo, bd))
    }
}

impl BareMove {
    pub fn pack_14bits(&self) -> u64 {
        if self.is_null() {
            return 0;
        }
        self.from.index() as u64
            + ((self.to.index() as u64) << 6)
            + if let Some(p) = self.promo {
                (Self::pack_promo_2bits(p) & 3) << 12
            } else {
                0
            }
    }
    fn pack_promo_2bits(p: Piece) -> u64 {
        match p {
            Piece::Knight => 0,
            Piece::Bishop => 1,
            Piece::Rook => 2,
            Piece::Queen => 3,
            _ => unreachable!(),
        }
    }

    fn unpack_promo_2bits(index: u64) -> Piece {
        match index {
            0 => Piece::Knight,
            1 => Piece::Bishop,
            2 => Piece::Rook,
            3 => Piece::Queen,
            _ => unreachable!(),
        }
    }

    pub fn unpack_14bits(bits: u64) -> BareMove {
        // works for null move
        let from = Square::from_u32(bits as u32 & 63);
        let to = Square::from_u32((bits >> 6) as u32 & 63);
        let piece_index = (bits >> 12) & 3;
        let promo = match piece_index {
            0 => None,
            pi => Some(Self::unpack_promo_2bits(pi)),
        };
        BareMove {
            mover: None,
            to,
            from,
            promo,
        }
    }
}

type UMove = u32;

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Move {
    bits: UMove,
}

impl Move {
    const OFFSET_FROM: i32 = 0;
    const OFFSET_TO: i32 = 6;
    const OFFSET_FLAG: i32 = 12; // 4 bits
    const OFFSET_MOVER: i32 = 16; // 3 bits
    const OFFSET_CAPTURE: i32 = 19; // 3 bits
                                    // const OFFSET_PRIOR_EP: i32 = 22; // 6 bits
                                    // const OFFSET_PRIOR_CR: i32 = 28; // 4 bits
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !f.alternate() {
            write!(f, "{}{}{}", self.from(), self.to(), self.promo_piece().to_string_or(""))
        } else {
            let bytes = self.bits.to_be_bytes();
            let bits = format!(
                "{:0<8b}_{:0<8b}_{:0<8b}_{:0<8b}",
                bytes[0], bytes[1], bytes[2], bytes[3]
            );
            f.debug_struct("Move")
                .field("bits", &bits)
                .field("uci", &self.to_uci())
                .field("from", &self.from())
                .field("to", &self.to())
                .field("mover", &self.mover())
                .field("promo", &self.promo_piece())
                .field("flag", &self.flag())
                .finish()
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_uci())?;
        Ok(())
    }
}

// 16 - less 2x6 for from/to = 4 bits = 16 things
#[derive(Copy, Clone, Debug)]
enum MoveFlag {
    Quiet              = 0,
    Castle,
    PromoKnight,
    PromoBishop,
    PromoRook,
    PromoQueen,
    PawnDoublePush,
    Unused1,

    PromoCaptureKnight = 8,
    PromoCaptureBishop,
    PromoCaptureRook,
    PromoCaptureQueen,
    RegularCapture,
    EnPassantCapture,
    Unused2,
    Unused3,
}

impl MoveFlag {
    const FLAG_MASK: u16 = 0b1111;
    const fn from_index(index: u16) -> MoveFlag {
        use MoveFlag::*;
        match index {
            0 => Quiet,
            1 => Castle,
            2 => PromoKnight,
            3 => PromoBishop,
            4 => PromoRook,
            5 => PromoQueen,
            6 => PawnDoublePush,
            7 => Unused1,

            8 => PromoCaptureKnight,
            9 => PromoCaptureBishop,
            10 => PromoCaptureRook,
            11 => PromoCaptureQueen,
            12 => RegularCapture,
            13 => EnPassantCapture,
            14 => Unused2,
            15 => Unused3,

            _ => unreachable!(),
        }
    }

    const fn index(self) -> u16 {
        self as u16
    }

    const fn promo_piece(self) -> Option<Piece> {
        use MoveFlag::*;
        match self {
            PromoQueen | PromoCaptureQueen => Some(Piece::Queen),
            PromoKnight | PromoCaptureKnight => Some(Piece::Knight),
            PromoBishop | PromoCaptureBishop => Some(Piece::Bishop),
            PromoRook | PromoCaptureRook => Some(Piece::Rook),
            _ => None,
        }
    }

    const fn with_capture(self) -> Self {
        use MoveFlag::*;
        match self {
            Quiet => RegularCapture,
            EnPassantCapture => EnPassantCapture,
            RegularCapture => RegularCapture,
            PromoQueen => PromoCaptureQueen,
            PromoKnight => PromoCaptureKnight,
            PromoBishop => PromoCaptureBishop,
            PromoRook => PromoCaptureRook,
            _ => unreachable!(),
        }
    }

    const fn with_promo(self, p: Piece) -> Self {
        use MoveFlag::*;

        let flag = match p {
            Piece::Queen => PromoQueen,
            Piece::Rook => PromoRook,
            Piece::Bishop => PromoBishop,
            Piece::Knight => PromoKnight,
            _ => unreachable!(),
        };
        if self.is_capture() {
            flag.with_capture()
        } else {
            flag
        }
    }

    const fn is_capture(self) -> bool {
        use MoveFlag::*;
        matches!(
            &self,
            RegularCapture
                | PromoCaptureQueen
                | PromoCaptureKnight
                | PromoCaptureBishop
                | PromoCaptureRook
                | EnPassantCapture
        )
    }

    const fn is_castling(self) -> bool {
        matches!(self, Self::Castle)
    }

    const fn is_pawn_double_push(self) -> bool {
        matches!(self, Self::PawnDoublePush)
    }

    const fn is_en_passant_capture(self) -> bool {
        matches!(self, Self::EnPassantCapture)
    }
}

impl Move {
    pub fn display(moves: &[Move]) -> String {
        "[".to_string() + &moves.iter().join(", ") + "]"
    }
    pub fn flip_vertical(&mut self) {
        if self.is_null() {
            return;
        }
        let from = self.from().flip_vertical();
        let to = self.to().flip_vertical();

        self.bits &= !(63 << Move::OFFSET_FROM);
        self.bits &= !(63 << Move::OFFSET_TO);

        self.bits |= (from.index() as UMove & 63) << Move::OFFSET_FROM;
        self.bits |= (to.index() as UMove & 63) << Move::OFFSET_TO;
    }

    const NULL_MOVE: Move = Self::new_null();

    #[inline]
    pub fn new_move(mover: Piece, from: Square, to: Square, promo: Option<Piece>, bd: &Board) -> Move {
        if let Some(promo) = promo {
            if to.is_in(bd.occupied()) {
                Self::new_promo_capture(from, to, promo, bd)
            } else {
                Self::new_promo(from, to, promo, bd)
            }
        } else if mover == Piece::Pawn {
            Self::new_pawn_move(from, to, bd)
        } else if mover == Piece::King && CastlingRights::is_castling(from, to) {
            Self::new_castle(from, to, bd)
        } else if to.is_in(bd.occupied()) {
            Self::new_capture(mover, from, to, bd)
        } else {
            Self::new_quiet(mover, from, to, bd)
        }
    }

    #[inline]
    pub const fn new_quiet(mover: Piece, from: Square, to: Square, _bd: &Board) -> Move {
        let mut bits = (from.index() as UMove & 63) << Move::OFFSET_FROM;
        bits += (to.index() as UMove & 63) << Move::OFFSET_TO;
        bits += (mover.index() as UMove & 15) << Move::OFFSET_MOVER;
        // bits += 7 << Move::OFFSET_CAPTURE;

        Move { bits }
    }

    #[inline]
    pub const fn new_null() -> Move {
        let mut bits = (Square::A1.index() as UMove & 63) << Move::OFFSET_FROM;
        bits += (Square::A1.index() as UMove & 63) << Move::OFFSET_TO;
        // bits += 7 << Move::OFFSET_CAPTURE;
        Move { bits }
    }

    #[inline]
    pub fn new_pawn_move(from: Square, to: Square, bd: &Board) -> Move {
        if to.file_index() != from.file_index() {
            if bd.is_en_passant_square(to) {
                let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
                m.set_capture(bd);
                m.set_en_passant();
                m
            } else {
                // let cap = Piece::Pawn; //bd.piece_unchecked(to);
                Self::new_capture(Piece::Pawn, from, to, bd)
            }
        } else {
            // its a push
            let behind = to.shift(bd.color_us().backward());
            let ep = behind;
            if behind.as_bb().disjoint(bd.pawns()) {
                // no one behind us => double push
                Self::new_double_push(from, to, ep, bd)
            } else {
                Self::new_quiet(Piece::Pawn, from, to, bd)
            }
        }
    }

    #[inline]
    pub fn new_double_push(from: Square, to: Square, ep: Square, bd: &Board) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
        m.set_double_push(ep);
        m
    }

    #[inline]
    pub fn new_capture(p: Piece, from: Square, to: Square, bd: &Board) -> Move {
        let mut m = Self::new_quiet(p, from, to, bd);
        m.set_capture(bd);
        m
    }

    #[inline]
    pub fn new_ep_capture(from: Square, to: Square, bd: &Board) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
        m.set_capture(bd);
        m.set_en_passant();
        m
    }

    #[inline]
    pub fn new_promo(from: Square, to: Square, promo: Piece, bd: &Board) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
        m.set_promo(promo);
        m
    }

    #[inline]
    pub fn new_promo_capture(from: Square, to: Square, promo: Piece, bd: &Board) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to, bd);
        m.set_promo(promo);
        m.set_capture(bd);
        m
    }

    #[inline]
    pub fn new_castle(king_from: Square, king_to: Square, bd: &Board) -> Move {
        let mut m = Self::new_quiet(Piece::King, king_from, king_to, bd);
        m.set_castling();
        m
    }

    #[inline]
    fn set_flag(&mut self, flag: MoveFlag) {
        self.bits &= !(15 << Move::OFFSET_FLAG);
        self.bits |= (flag.index() as UMove) << Self::OFFSET_FLAG;
    }

    #[inline]
    fn set_capture(&mut self, _b: &Board) {
        // let cap = bd.piece(self.capture_square(bd)).unwrap_or_else(|| {
        //     panic!(
        //         "No piece on board {bd} for from:{} to:{}",
        //         self.from(),
        //         self.to()
        //     )
        // });

        // self.bits |= (cap.index() as UMove) << Self::OFFSET_CAPTURE;
        self.set_flag(self.flag().with_capture());
    }

    #[inline]
    fn set_promo(&mut self, promo: Piece) {
        self.set_flag(self.flag().with_promo(promo));
    }

    #[inline]
    fn set_en_passant(&mut self) {
        self.set_flag(MoveFlag::EnPassantCapture);
    }

    #[inline]
    fn set_double_push(&mut self, _ep_sq: Square) {
        self.set_flag(MoveFlag::PawnDoublePush);
    }

    #[inline]
    fn set_castling(&mut self) {
        self.set_flag(MoveFlag::Castle);
    }
}

// const OFFSET_FROM: i32 = 0;
// const OFFSET_TO: i32 = 6;
// const OFFSET_FLAG: i32 = 12; // 4 bits
// const OFFSET_MOVER: i32 = 16; // 3 bits
// const OFFSET_CAPTURE: i32 = 19; // 3 bits
// const OFFSET_PRIOR_EP: i32 = 22; // 6 bits
// const OFFSET_PRIOR_CR: i32 = 28; // 4 bits

// pub const fn to(&self) -> Square {
//     Square::from_u8((self.bits >> Self::OFFSET_TO) as u8 & 63)
// }

// pub const fn from(&self) -> Square {
//     Square::from_u8((self.bits >> Self::OFFSET_FROM) as u8 & 63)
// }

// pub fn mover_piece(&self, _bd: &Board) -> Piece {
//     ((self.bits >> Self::OFFSET_MOVER) as u8 & 7).into()
// }

// const fn flag(&self) -> MoveFlag {
//     MoveFlag::from_index((self.bits >> Move::OFFSET_FLAG) as u16)
// }

// pub fn capture_piece(&self, _bd: &Board) -> Option<Piece> {
//     match self.flag().is_capture() {
//         false => None,
//         true => Some(((self.bits >> Self::OFFSET_CAPTURE) as u8 & 7).into()),
//     }
// }

// pub fn prior_ep_square(&self) -> Option<Square> {
//     let index = (self.bits >> Self::OFFSET_PRIOR_EP) as u8 & 63;
//     match index {
//         0 => None,
//         u => Some(u.into()),
//     }
// }

impl Move {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_null() {
            write!(f, "0000")
        } else {
            write!(f, "{}{}", self.from().uci(), self.to().uci())?;
            if let Some(p) = self.promo_piece() {
                write!(f, "{}", p.to_char(Color::Black))?
            }
            Ok(())
        }
    }

    #[inline]
    pub const fn to_inner(&self) -> BareMove {
        BareMove {
            mover: None,
            from:  self.from(),
            to:    self.to(),
            promo: self.promo_piece(),
        }
    }

    /// if mv is invalid then maybe mover is not set
    pub fn to_inner_with_mover(&self, bd: &Board) -> BareMove {
        BareMove {
            mover: bd.piece(self.from()),
            from:  self.from(),
            to:    self.to(),
            promo: self.promo_piece(),
        }
    }

    pub fn validate(&self, bd: &Board) -> std::result::Result<(), &'static str> {
        let mv = self.to_inner().validate(bd)?;
        if *self != mv {
            return Err("move flags likely wrong");
        }
        Ok(())
    }

    pub fn is_valid(&self, bd: &Board) -> bool {
        self.validate(bd).is_ok()
    }

    pub fn parse_uci(s: &str, b: &Board) -> Result<Self> {
        if s.trim() == "0000" {
            return Ok(Self::new_null());
        }
        let from = Square::parse(s.take_slice(0..2))?;
        let to = Square::parse(s.take_slice(2..4))?;
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
    const fn flag(&self) -> MoveFlag {
        MoveFlag::from_index((self.bits >> Move::OFFSET_FLAG) as u16 & 15)
    }

    pub fn to_san(&self, b: &Board) -> String {
        b.to_san(*self)
    }

    #[inline]
    pub const fn to(&self) -> Square {
        unsafe { Square::from_u8_unchecked(((self.bits >> Self::OFFSET_TO) & 63) as u8) }
    }

    #[inline]
    pub const fn from(&self) -> Square {
        unsafe { Square::from_u8_unchecked(((self.bits >> Self::OFFSET_FROM) & 63) as u8) }
    }

    #[inline]
    pub const fn double_push_en_passant_square(&self) -> Square {
        unsafe { Square::from_u8_unchecked((self.from().index() + self.to().index()) as u8 / 2) }
    }

    #[inline]
    pub fn is_ep_capture(&self, _b: &Board) -> bool {
        self.flag().is_en_passant_capture()
        // !self.is_null() && self.mover_piece(b) == Piece::Pawn && self.to().is_in(b.en_passant())
    }

    #[inline]
    pub fn capture_square(&self, b: &Board) -> Square {
        if self.is_ep_capture(b) {
            // the ep square IS NOT the capture square. The capture sq is in front of it (w)
            Square::from_xy(self.to().file_index() as u32, self.from().rank_index() as u32)
        } else {
            self.to()
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.to().index() == self.from().index() // partial.eq isn't const
    }

    #[inline]
    pub const fn is_promo(&self) -> bool {
        self.flag().promo_piece().is_some()
    }

    #[inline]
    pub fn is_near_promo(&self, bd: &Board) -> bool {
        let c = if self.from() < self.to() {
            Color::White
        } else {
            Color::Black
        };
        self.from().is_in(bd.pawns()) && self.to().rank_number_as_white(c) == 7
    }

    // #[inline]
    // pub const fn promo_piece(&self) -> Piece {
    //     Piece::from_index((self.bits >> Self::OFFSET_PROMO) as usize & 7)
    // }

    #[inline]
    pub const fn promo_piece(&self) -> Option<Piece> {
        self.flag().promo_piece()
    }

    pub fn mover_piece(&self, _bd: &Board) -> Piece {
        ((self.bits >> Self::OFFSET_MOVER) as u8 & 7).into()
    }

    pub fn mover(&self) -> Piece {
        ((self.bits >> Self::OFFSET_MOVER) as u8 & 7).into()
    }
    // pub fn mover_piece(&self, bd: &Board) -> Piece {
    //     bd.piece_unchecked(self.from())
    // }

    // pub fn capture_piece(&self, _bd: &Board) -> Option<Piece> {
    //     match self.flag().is_capture() {
    //         false => None,
    //         true => Some(((self.bits >> Self::OFFSET_CAPTURE) as u8 & 7).into()),
    //     }
    // }

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

    // #[inline]
    // pub fn mover_piece(&self, bd: &Board) -> Piece {
    //     bd.piece_unchecked(self.from())
    // }

    #[inline]
    pub fn is_capture(&self) -> bool {
        (self.flag().index() & 8) != 0
        // self.to().is_in(bd.occupied())
        // || self.from().is_in(bd.pawns()) && self.to().file_index() != self.from().file_index()
    }

    #[inline]
    pub fn is_castle(&self, _b: &Board) -> bool {
        self.flag().is_castling()
        // self.from().is_in(Bitboard::CASTLING)
        //     && self.to().is_in(Bitboard::CASTLING)
        //     && self.from().is_in(b.kings())
    }

    #[inline]
    pub fn castling_side(&self, b: &Board) -> CastlingRights {
        // CastlingRights::from_bits_truncate((self.bits >> Self::OFFSET_CASTLE) as u8)
        if self.is_castle(b) {
            CastlingRights::from_king_move(self.to())
        } else {
            CastlingRights::NONE
        }
    }

    #[inline]
    pub fn is_pawn_double_push(&self, _b: &Board) -> bool {
        self.flag().is_pawn_double_push()
        // self.from().is_in(Bitboard::RANKS_27)
        //     && self.to().is_in(Bitboard::RANKS_45)
        //     && self.from().is_in(b.pawns())
    }

    // #[inline]
    // pub fn rook_move(&self, b: &Board) -> Move {
    //     if self.is_castle(b) {
    //         let (from, to) = self.rook_move_from_to();
    //         Move::new_quiet(Piece::Rook, from, to)
    //     } else {
    //         Move::new_null()
    //     }
    // }

    #[inline]
    pub const fn rook_move_from_to(&self) -> (Square, Square) {
        match self.to() {
            Square::C1 => (Square::A1, Square::D1),
            Square::G1 => (Square::H1, Square::F1),
            Square::C8 => (Square::A8, Square::D8),
            Square::G8 => (Square::H8, Square::F8),
            _ => panic!("not a valid castling king-to square"),
        }
    }

    #[inline]
    pub fn castling_rights_lost(&self) -> CastlingRights {
        if self.is_null() {
            CastlingRights::NONE
        } else {
            // let squares_changing = self.to().as_bb() | self.from().as_bb();
            CastlingRights::rights_lost(self.from(), self.to())
        }
    }

    #[inline]
    pub fn mvv_lva_score(&self, bd: &Board) -> i32 {
        debug_assert!(self.is_valid(bd), "{self} is illegal for board {bd}");
        let mut score = 0;
        if let Some(cap) = self.capture_piece(bd) {
            score += cap.centipawns() * 10 - self.mover_piece(bd).centipawns() / 10;
        }
        if let Some(promo) = self.promo_piece() {
            score += promo.centipawns() * 10 - Piece::Pawn.centipawns() / 10;
        }
        score
    }

    pub fn to_uci(&self) -> String {
        self.to_inner().to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::iter::once;

    use test_log::test;

    use super::*;
    use crate::catalog::Catalog;
    use crate::infra::profiler::PerfProfiler;
    use crate::other::Perft;

    #[test]
    fn test_baremove() {
        assert_eq!(BareMove::parse_uci("a2a3").unwrap().to_string(), "a2a3");
        assert_eq!(BareMove::parse_uci("a2a3Q").unwrap().to_string(), "a2a3q");
        assert_eq!(BareMove::parse_uci("0000").unwrap().to_string(), "0000");

        assert_eq!(BareMove::parse_lan("0000").unwrap().to_string(), "0000");
        assert_eq!(BareMove::parse_lan("a2a3").unwrap().to_string(), "a2a3");
        assert_eq!(BareMove::parse_lan("a2-a3").unwrap().to_string(), "a2a3");
        assert_eq!(BareMove::parse_lan("e4xd5").unwrap().to_string(), "e4d5");
        assert_eq!(BareMove::parse_lan("e7e8q").unwrap().to_string(), "e7e8q");
        assert_eq!(BareMove::parse_lan("e7e8q").unwrap().to_string(), "e7e8q");
        assert_eq!(BareMove::parse_lan("Re7e8").unwrap().to_string(), "e7e8");
        let mv = BareMove::parse_lan("Re7e8").unwrap();
        assert_eq!(mv.mover, Some(Piece::Rook));
        assert_eq!(BareMove::parse_lan("").is_err(), true);
        assert_eq!(BareMove::parse_lan("X").is_err(), true);
        assert_eq!(BareMove::parse_lan("Be5").is_err(), true);

        assert_eq!("a2a3".parse::<BareMove>().unwrap().to_string(), "a2a3");
        assert_eq!("Re7e8".parse::<BareMove>().unwrap().to_string(), "e7e8");

        // parses as uci which doesn't have a mover
        assert_eq!("a2a3".parse::<BareMove>().unwrap().mover, None);

        // parses as lan (as uci forbids '-') which has a mover
        assert_eq!("a2-a3".parse::<BareMove>().unwrap().mover, Some(Piece::Pawn));
    }

    #[test]
    fn test_validate_move() -> anyhow::Result<()> {
        let b = Board::parse_fen("1r5k/8/7r/K1pP3r/7r/8/8/8 w - c6 0 1").unwrap();
        assert_eq!("d5c6".parse::<BareMove>()?.validate(&b).is_err(), true); // ep capture discovered check

        let b = Board::parse_diagram(
            r"
            k.......
            p.......
            ........
            ......pP
            ....rPpP
            ....R...
            .......r
            R...K..R w K g6 1 1",
        )
        .unwrap();
        assert_eq!("f4f5".parse::<BareMove>()?.validate(&b).unwrap().to_uci(), "f4f5");
        assert_eq!("h4h5".parse::<BareMove>()?.validate(&b).is_err(), true);
        assert_eq!("h5g6".parse::<BareMove>()?.validate(&b).unwrap().to_uci(), "h5g6"); // ep capture
        assert_eq!("Ke1g1".parse::<BareMove>()?.validate(&b).unwrap().to_uci(), "e1g1"); // castle K
        assert_eq!("Ke1c1".parse::<BareMove>()?.validate(&b).is_err(), true); // castle Q
        assert_eq!("Ra1a7".parse::<BareMove>()?.validate(&b).unwrap().to_uci(), "a1a7"); // rook attack
        assert_eq!("Ra1a8".parse::<BareMove>()?.validate(&b).is_err(), true); // rook x-ray attack
        assert_eq!("Ra1b2".parse::<BareMove>()?.validate(&b).is_err(), true); // rook diagonal
        assert_eq!("Ke1d1".parse::<BareMove>()?.validate(&b).unwrap().to_uci(), "e1d1"); // king move
        assert_eq!("Ke1e2".parse::<BareMove>()?.validate(&b).is_err(), true); // king in check
        assert_eq!("Re3e2".parse::<BareMove>()?.validate(&b).unwrap().to_uci(), "e3e2"); // pinned piece ok
        assert_eq!("Re3f3".parse::<BareMove>()?.validate(&b).is_err(), true); // pinned piece
        assert_eq!("0000".parse::<BareMove>()?.validate(&b).unwrap().to_uci(), "0000"); // null move

        test_all_legal_and_illegal_moves(&b);
        for pos in Catalog::bratko_kopec().iter() {
            test_all_legal_and_illegal_moves(&pos.board());
        }
        Ok(())
    }

    // helper method
    fn test_all_legal_and_illegal_moves(b: &Board) {
        let pieces = Piece::ALL.iter().map(|&p| Some(p)).chain(once(None));
        for mover in pieces.clone() {
            for from in Square::all() {
                for to in Square::all() {
                    for promo in pieces.clone() {
                        let bm = BareMove { mover, from, to, promo };
                        if bm.is_null() {
                            continue;
                        }
                        let mut found = false;
                        for lm in b.legal_moves().iter() {
                            if lm.to() == bm.to
                                && lm.from() == bm.from
                                && lm.promo_piece() == bm.promo
                                && (Some(lm.mover_piece(b)) == bm.mover || bm.mover.is_none())
                            {
                                assert_eq!(
                                    bm.validate(b).unwrap_or_else(|_| panic!("{bm} {bm:?}")).to_uci(),
                                    lm.to_uci()
                                );
                                found = true;
                            }
                        }
                        if !found {
                            assert_eq!(bm.validate(b).is_err(), true, "bm expected illegal: {bm} on board {b}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn bench_validate_move() {
        let mut starting_pos = Catalog::perft_kiwipete().0;
        let wrong = Catalog::perft_kiwipete().0;

        let mut validate_good = PerfProfiler::new("baremove: validate good");
        let mut validate_bad = PerfProfiler::new("baremove: validate bad");

        let mut func = |bd: &Board, mv: Move| {
            let bm = mv.to_inner();
            _ = validate_good.bench(|| black_box(bm).validate(black_box(bd)));
            _ = validate_bad.bench(|| black_box(bm).validate(black_box(&wrong)));
        };
        Perft::<true>::perft_with(&mut starting_pos, 2, &mut func);
    }

    #[test]
    fn test_parse_mv_uci() -> anyhow::Result<()> {
        let b = Board::starting_pos();
        assert_eq!(Move::parse_uci("a2a3", &b)?.to_uci(), "a2a3");
        assert_eq!(Move::parse_uci("0000", &b).is_err(), false);
        assert_eq!(Move::parse_uci("XYZ", &b).is_err(), true);
        let b = Board::parse_diagram(
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
        .unwrap();
        assert_eq!(Move::parse_uci("a7a8q", &b)?.to_uci(), "a7a8q");
        Ok(())
    }

    #[test]
    fn test_move_basics() {
        // QUIET
        let b = Catalog::perft_kiwipete().0;
        let mv = Move::new_quiet(Piece::Bishop, Square::D2, Square::E3, &b);
        // let mv2 = bd.parse_uci_move("e2d3").unwrap();
        assert_eq!(mv.from(), Square::D2, "{mv:#?}");
        assert_eq!(mv.to(), Square::E3, "{mv:#?}");
        assert_eq!(mv.mover_piece(&b), Piece::Bishop, "{mv:#?}");
        assert_eq!(mv.capture_piece(&b), None, "{mv:#?}");
        assert_eq!(mv.is_promo(), false, "{mv:#?}");
        // assert_eq!(mv.capture_square(&b), Square::E3, "{mv:#?}");
        assert_eq!(mv.is_pawn_double_push(&b), false, "{mv:#?}");
        let b = b.make_move(mv);
        assert_eq!(
            b.to_fen(),
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N1BQ1p/PPP1BPPP/R3K2R b KQkq - 1 1"
        );
        assert!(b.validate().is_ok());

        // PAWN DOUBLE PUSH
        let b = Catalog::perft_kiwipete().0;
        let mv = b.parse_san_move("a4").unwrap();
        assert_eq!(mv.from(), Square::A2, "{mv:#?}");
        assert_eq!(mv.to(), Square::A4, "{mv:#?}");
        assert_eq!(mv.mover_piece(&b), Piece::Pawn, "{mv:#?}");
        assert_eq!(mv.capture_piece(&b), None, "{mv:#?}");
        assert_eq!(mv.is_promo(), false, "{mv:#?}");
        // assert_eq!(mv.capture_square(&b), Square::A4, "{mv:#?}");
        assert_eq!(mv.is_pawn_double_push(&b), true, "{mv:#?}");
        let b = b.make_move(mv);
        assert_eq!(
            b.to_fen(),
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/Pp2P3/2N2Q1p/1PPBBPPP/R3K2R b KQkq a3 0 1"
        );
        assert!(b.validate().is_ok());

        // CAPTURE
        let b = Catalog::perft_kiwipete().0;
        let mv = Move::new_capture(Piece::Queen, Square::F3, Square::F6, &b);
        assert_eq!(mv.from(), Square::F3, "{mv:#?}");
        assert_eq!(mv.to(), Square::F6, "{mv:#?}");
        assert_eq!(mv.mover_piece(&b), Piece::Queen, "{mv:#?}");
        assert_eq!(mv.capture_piece(&b), Some(Piece::Knight), "{mv:#?}");
        assert_eq!(mv.capture_piece(&b), Some(Piece::Knight), "{mv:#?}");
        assert_eq!(mv.is_promo(), false, "{mv:#?}");
        assert_eq!(mv.is_pawn_double_push(&b), false, "{mv:#?}");
        let b = b.make_move(mv);
        assert_eq!(
            b.to_fen(),
            "r3k2r/p1ppqpb1/bn2pQp1/3PN3/1p2P3/2N4p/PPPBBPPP/R3K2R b KQkq - 0 1"
        );
        assert!(b.validate().is_ok());

        // EP CAPTURE
        let b: Board = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/Pp2P3/2N2Q1p/1PPBBPPP/R3K2R b KQkq a3 0 1"
            .parse()
            .unwrap();
        let mv = Move::new_ep_capture(Square::B4, Square::A3, &b);
        assert_eq!(mv.from(), Square::B4, "{mv:#?}");
        assert_eq!(mv.to(), Square::A3, "{mv:#?}");
        assert_eq!(mv.mover_piece(&b), Piece::Pawn, "{mv:#?}");
        assert_eq!(mv.capture_piece(&b), Some(Piece::Pawn), "{mv:#?}");
        assert_eq!(mv.is_promo(), false, "{mv:#?}");
        assert_eq!(mv.is_ep_capture(&b), true, "{mv:#?}");
        assert_eq!(mv.capture_square(&b), Square::A4, "{mv:#?}");
        assert_eq!(mv.is_pawn_double_push(&b), false, "{mv:#?}");
        let b = b.make_move(mv);
        assert_eq!(
            b.to_fen(),
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/4P3/p1N2Q1p/1PPBBPPP/R3K2R w KQkq - 0 2"
        );
        assert!(b.validate().is_ok());

        // PROMO WITH CAPTURE
        let b: Board = "n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1".parse().unwrap();
        let mv = Move::new_promo_capture(Square::G2, Square::F1, Piece::Queen, &b);
        assert_eq!(mv.from(), Square::G2, "{mv:#?}");
        assert_eq!(mv.to(), Square::F1, "{mv:#?}");
        assert_eq!(mv.is_promo(), true, "{mv:#?}");
        assert_eq!(mv.mover_piece(&b), Piece::Pawn, "{mv:#?}");
        assert_eq!(mv.capture_piece(&b), Some(Piece::Knight), "{mv:#?}");
        assert_eq!(mv.is_ep_capture(&b), false, "{mv:#?}");
        assert_eq!(mv.capture_square(&b), Square::F1, "{mv:#?}");
        assert_eq!(mv.is_pawn_double_push(&b), false, "{mv:#?}");
        let b = b.make_move(mv);
        assert_eq!(b.to_fen(), "n1n5/PPPk4/8/8/8/8/4Kp1p/5q1N w - - 0 2");
        assert!(b.validate().is_ok());

        let b: Board = "rn1qr1nk/ppp1ppbp/3p2p1/3P4/2P3b1/2N2N2/PP2PPPP/K1BRQB1R b - - 0 5"
            .parse()
            .unwrap();
        let mv = Move::new_double_push(Square::E7, Square::E5, Square::E6, &b);
        assert_eq!(mv.from(), Square::E7, "{mv:#?}");
        assert_eq!(mv.to(), Square::E5, "{mv:#?}");
        assert_eq!(mv.is_promo(), false, "{mv:#?}");
        assert_eq!(mv.mover_piece(&b), Piece::Pawn, "{mv:#?}");
        assert_eq!(mv.capture_piece(&b), None, "{mv:#?}");
        assert_eq!(mv.is_ep_capture(&b), false, "{mv:#?}");
        assert_eq!(mv.capture_square(&b), Square::E5, "{mv:#?}");
        assert_eq!(mv.is_pawn_double_push(&b), true, "{mv:#?}");
        let b = b.make_move(mv);
        // ensure e/p square has been set
        assert_eq!(
            b.to_fen(),
            "rn1qr1nk/ppp2pbp/3p2p1/3Pp3/2P3b1/2N2N2/PP2PPPP/K1BRQB1R w - e6 0 6"
        );
    }

    #[test]
    fn bench_move() {
        let mut starting_pos = Catalog::perft_kiwipete().0;

        let mut clone = PerfProfiler::new("move: clone");
        let mut from = PerfProfiler::new("move: from");
        let mut to = PerfProfiler::new("move: to");
        let mut from_index = PerfProfiler::new("move: from_index");
        let mut capture_sq = PerfProfiler::new("move: capture_sq");
        let mut mover = PerfProfiler::new("move: mover");
        let mut capture_piece = PerfProfiler::new("move: capture_piece");
        let mut is_ep = PerfProfiler::new("move: is_ep");
        let mut is_capture = PerfProfiler::new("move: is_capture");
        let mut is_castle = PerfProfiler::new("move: is_castle");
        let mut is_double_push = PerfProfiler::new("move: is_double_push");
        let mut new_pawn_move = PerfProfiler::new("move: new_pawn_move");
        let mut ep_square = PerfProfiler::new("move: en_passant_square");

        let mut func = |bd: &Board, mv: Move| {
            let index = mv.flag().index();
            #[allow(clippy::clone_on_copy)]
            clone.bench(|| black_box(mv).clone());
            from.bench(|| black_box(mv).from());
            to.bench(|| black_box(mv).to());
            from_index.bench(|| MoveFlag::from_index(black_box(index)));
            capture_sq.bench(|| black_box(mv).capture_square(black_box(bd)));
            mover.bench(|| black_box(mv).mover_piece(black_box(bd)));
            capture_piece.bench(|| black_box(mv).capture_piece(black_box(bd)));
            is_ep.bench(|| black_box(mv).is_ep_capture(black_box(bd)));
            is_capture.bench(|| black_box(mv).is_capture());
            is_castle.bench(|| black_box(mv).is_castle(black_box(bd)));
            is_double_push.bench(|| black_box(mv).is_pawn_double_push(black_box(bd)));
            ep_square.bench(|| black_box(mv).double_push_en_passant_square());
            if mv.from().is_in(bd.pawns()) {
                new_pawn_move.bench(|| Move::new_pawn_move(black_box(mv).from(), black_box(mv).to(), black_box(bd)));
            }
        };
        Perft::<true>::perft_with(&mut starting_pos, 2, &mut func);
    }

    #[test]
    fn test_move() {
        let bd = &Board::starting_pos();

        assert_eq!(Move::new_null().to_string(), "0000");

        let move_castle = Move::new_castle(Square::A1, Square::B2, bd);

        println!("{:#} {:b}", move_castle, move_castle.bits);
        assert_eq!(move_castle.from(), Square::A1);
        assert_eq!(move_castle.to(), Square::B2);

        // ep cant be called on castling move
        // assert_eq!(move_castle.ep(), Square::null());
        // assert_eq!(move_castle.capture_piece(), None);
        assert_eq!(move_castle.promo_piece(), None);
        assert_eq!(move_castle.is_promo(), false);
        assert_eq!(move_castle.is_capture(), false);
        assert_eq!(move_castle.is_null(), false);
        // assert_eq!(move_castle.castling_side(), CastlingRights::WHITE_QUEEN);

        let mut flip_move_castle = move_castle;
        flip_move_castle.flip_vertical();
        assert_eq!(flip_move_castle.from(), Square::A8);
        assert_eq!(flip_move_castle.to(), Square::B7);
        assert_eq!(flip_move_castle.is_promo(), false);
        assert_eq!(flip_move_castle.is_capture(), false);
        assert_eq!(flip_move_castle.is_null(), false);

        let move_a1b2 = Move::new_quiet(Piece::Bishop, Square::A1, Square::B2, bd);

        println!("{:#} {:b}", move_a1b2, move_a1b2.bits);
        assert_eq!(move_a1b2.is_capture(), false);
        assert_eq!(move_a1b2.from(), Square::A1);
        assert_eq!(move_a1b2.to(), Square::B2);
        // assert_eq!(move_a1b2.mover_piece(), Piece::Bishop);
        assert_eq!(move_a1b2.is_promo(), false);

        let capture_a1b2 = Move::new_capture(Piece::Bishop, Square::A1, Square::B2, &Board::starting_pos());
        assert_eq!(capture_a1b2.is_capture(), true);

        // ep cant be called on castling move
        // assert_eq!(move_a1b2.ep(), Square::null());
        // assert_eq!(move_a1b2.castling_side(), CastlingRights::NONE);

        let promo_a7a8 = Move::new_promo(Square::A7, Square::A8, Piece::Queen, &Board::starting_pos());

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
                assert_eq!(mv.to_inner().validate(bd).unwrap(), mv);
                count += 1
            };
            Perft::<true>::perft_with(&mut pos.board().clone(), 3, &mut func);
        }
        println!("Count = {count}");
    }

    #[test]
    fn test_mvv_lva() {
        let bd = Board::parse_diagram(
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
        let board = Catalog::starting_board();
        let a2a3 = board.parse_uci_move("a2a3").unwrap();
        let b1c3 = board.parse_uci_move("b1c3").unwrap();
        assert_eq!(board.to_san(a2a3), "a3");
        assert_eq!(board.to_san(b1c3), "Nc3");

        let mut bb = board.into_builder();
        bb.set(Bitboard::D3, "p").unwrap();
        bb.set(Bitboard::F3, "p").unwrap();
        let board = bb.build();

        let c2d3 = board.parse_uci_move("c2d3").unwrap();
        assert_eq!(board.to_san(c2d3), "cxd3");

        let e2d3 = board.parse_uci_move("e2d3").unwrap();
        assert_eq!(board.to_san(e2d3), "exd3");

        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(g1f3), "Nxf3");

        // knight ambiguity
        let mut bb = board.into_builder();
        bb.set(Bitboard::G5, "N").unwrap();
        let board = bb.build();
        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(g1f3), "N1xf3");

        // two knights same rank and file as g5
        let mut bb = board.into_builder();
        bb.set(Bitboard::E5, "N").unwrap();
        let board = bb.build();
        let g1f3 = board.parse_uci_move("g5f3").unwrap();
        assert_eq!(board.to_san(g1f3), "Ng5xf3");

        // remove some minor pieces to allow castling
        let mut bb = board.into_builder();
        bb.set(Bitboard::RANK_8, "r...k..r").unwrap();
        bb.set_turn(Color::Black);
        let board = bb.build();
        let castle_k = board.parse_uci_move("e8g8").unwrap();
        assert_eq!(board.to_san(castle_k), "O-O");
        let castle_q = board.parse_uci_move("e8c8").unwrap();
        assert_eq!(board.to_san(castle_q), "O-O-O");
    }
}
