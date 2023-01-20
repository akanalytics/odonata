use crate::{
    bits::{
        bitboard::{Bitboard, Dir},
        castling::CastlingRights,
        square::Square},
    boards::{Board, BoardCalcs},
    globals::constants::*,
    infra::utils::{StringUtils, ToStringOr},
    piece::{Color, Piece}, PreCalc,
};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self},
    str::FromStr,
};

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct BareMove {
    pub mover: Option<Piece>,
    pub from:  Square,
    pub to:    Square,
    pub promo: Option<Piece>,
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
            write!(f, "{}{}{}", self.from.uci(), self.to.uci(), self.promo.to_string_or(""))
        }
    }
}
impl BareMove {
    pub fn null() -> Self {
        Default::default()
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
        let s = s.replace("-", "");
        let mut s = s.replace("x", "");
        let mover = if s.starts_with(['K', 'Q', 'R', 'B', 'N']) {
            let (first, rest) = s.split_at(1);
            let piece = Some(Piece::parse(first)?);
            s = rest.to_string();
            piece
        } else {
            Some(Piece::Pawn)
        };
        Ok(BareMove { mover, ..Self::parse_uci(&s)? })
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

        let pc = PreCalc::default();

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
                if !bd.is_en_passant_square(to)
                && 
                !to.is_in(them) {
                    return Err("pawn capture must be to enemy square or en-passant square");
                }
            }
            if to.is_in(Bitboard::RANKS_18) {
                if mv.promo.is_none() {
                    return Err("pawn move to back rank must be a promotion");
                }
            } else {
                if mv.promo.is_some() {
                    return Err("must move to back rank to be a pawn promotion");
                }
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
            let king_in_between = Square::from_u32((from.index() + to.index()) as u32 / 2);
            if king_in_between.is_in(us|them) || to.is_in(them) {
                return Err("cannot castle when king step/to square is occupied");
            }
            if king_in_between.is_in(us|them) || to.is_in(them) {
                return Err("cannot castle when king-step/to square is occupied");
            }
            // west of king-to is either the king-step square (king side) or the rook step-over sq (queens)
            // in either case it must be empty, but the step-sq situation is checked above hence the specific error
            if to.shift(Dir::W).is_in(us|them) {
                return Err("rook step-square must be empty");
            }
            if BoardCalcs::attacked_by(king_in_between.as_bb(), us|them, bd).intersects(them) {
                return Err("cannot castle when king moves through check");
            }


        } else {
            let atts = pc.attacks(color_us, mover, us, them, from);
            if !to.is_in(atts)  {
                if !(mover == Piece::Pawn && bd.is_en_passant_square(to) && to.is_in(pc.pawn_capture_attacks_from_sq(color_us, from))) {
                    trace!("{mover} {from} -> {to}\n{atts} {bd} {ep}", ep = bd.is_en_passant_square(to));
                    return Err("to-square is not a valid move-to square for the piece");
                }
            }
        }

        // pseudo-legal move - test for in check - castling already handled at this point
        //
        // idea - lightweight make_move - no hash - just enough to check rays of sliders etc
        // this also means that if the king didnt 
        let from_to_bits = from.as_bb() | to.as_bb();
        let mut our_king = bd.kings() & us;

        // make the the occupied sqs reflect any move, and opponents sqs reflect any captures
        let us = us ^ from_to_bits;  
        let them = them - us;


        if !our_king.is_empty() 
            ||
            // if its not a king move and the piece isn't pinned and the king is not in check
            !from.is_in(bd.pinned(color_us)) && mover != Piece::King && !bd.is_in_check(color_us){

            // if the king moves, adjust the king square
            if mover == Piece::King {
                our_king ^= from_to_bits;
            }    
            // king square (or square the king will end up on)
            let ksq = our_king.square();
            
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



impl Board {
    pub fn is_castling_move_legal(&self, mv: BareMove) -> bool {
        let c = self.color_us();
        let right = CastlingRights::from_king_move(mv.to);
        let king = mv.from;
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
            return Move::new_null();
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
            return Move::new_castle(from, to);
        }
        if let Some(promo) = mv.promo {
            if let Some(_capture) = capture_piece {
                return Move::new_promo_capture(from, to, promo, self);
            } else {
                return Move::new_promo(from, to, promo);
            }
        }
        if let Some(_captured) = capture_piece {
            return Move::new_capture(mover, from, to, self);
        }
        // diagonal pawn capture yet to-square is empty
        if mover == Piece::Pawn {
            return Move::new_pawn_move(from, to, self);
        }
        if from == to {
            return Move::new_null();
        }
        Move::new_quiet(mover, from, to)
    }
}

type UMOVE = u16;

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
            .field("promo", &self.promo_piece())
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

// 16 - less 2x6 for from/to = 4 bits = 16 things
#[derive(Copy, Clone)]
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
    // 15, 16
}

impl MoveFlag {
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
            EnPassantCapture => RegularCapture,
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
        match &self {
            RegularCapture => true,
            PromoCaptureQueen => true,
            PromoCaptureKnight => true,
            PromoCaptureBishop => true,
            PromoCaptureRook => true,
            EnPassantCapture => true,
            _ => false,
        }
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
    #[inline]
    pub fn new_move(
        mover: Piece,
        from: Square,
        to: Square,
        promo: Option<Piece>,
        bd: &Board,
    ) -> Move {
        if let Some(promo) = promo {
            if to.is_in(bd.occupied()) {
                Self::new_promo_capture(from, to, promo, bd)
            } else {
                Self::new_promo(from, to, promo)
            }
        } else if mover == Piece::Pawn {
            Self::new_pawn_move(from, to, bd)
        } else if mover == Piece::King && CastlingRights::is_castling(from, to) {
            Self::new_castle(from, to)
        } else if to.is_in(bd.occupied()) {
            Self::new_capture(mover, from, to, bd)
        } else {
            Self::new_quiet(mover, from, to)
        }
    }

    #[inline]
    pub const fn new_quiet(_mover: Piece, from: Square, to: Square) -> Move {
        let mut bits = (from.index() as UMOVE & 63) << Move::OFFSET_FROM;
        bits += (to.index() as UMOVE & 63) << Move::OFFSET_TO;
        Move { bits }
    }

    const NULL_MOVE: Move = Self::new_quiet(Piece::Pawn, Square::A1, Square::A1);

    #[inline]
    pub const fn new_null() -> Move {
        Self::NULL_MOVE
    }

    #[inline]
    pub fn new_pawn_move(from: Square, to: Square, bd: &Board) -> Move {
        if to.file() != from.file() {
            if bd.is_en_passant_square(to) {
                let mut m = Self::new_quiet(Piece::Pawn, from, to);
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
                Self::new_double_push(from, to, ep)
            } else {
                Self::new_quiet(Piece::Pawn, from, to)
            }
        }
    }

    #[inline]
    fn new_double_push(from: Square, to: Square, ep: Square) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to);
        m.set_double_push(ep);
        m
    }

    #[inline]
    pub fn new_capture(p: Piece, from: Square, to: Square, bd: &Board) -> Move {
        let mut m = Self::new_quiet(p, from, to);
        m.set_capture(bd);
        m
    }

    #[inline]
    pub fn new_ep_capture(from: Square, to: Square, bd: &Board) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to);
        m.set_capture(bd);
        m.set_en_passant();
        m
    }

    #[inline]
    pub fn new_promo(from: Square, to: Square, promo: Piece) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to);
        m.set_promo(promo);
        m
    }

    #[inline]
    pub fn new_promo_capture(from: Square, to: Square, promo: Piece, bd: &Board) -> Move {
        let mut m = Self::new_quiet(Piece::Pawn, from, to);
        m.set_promo(promo);
        m.set_capture(bd);
        m
    }

    #[inline]
    pub fn new_castle(king_from: Square, king_to: Square) -> Move {
        let mut m = Self::new_quiet(Piece::King, king_from, king_to);
        m.set_castling();
        m
    }

    #[inline]
    fn set_flag(&mut self, flag: MoveFlag) {
        self.bits &= !(15 << Move::OFFSET_FLAG);
        self.bits |= flag.index() << Move::OFFSET_FLAG;
    }

    #[inline]
    fn set_capture(&mut self, _bd: &Board) {
        // let cap = bd
        //     .piece(to)
        //     .unwrap_or_else(|| panic!("No piece on board {bd} for from:{from} to:{to}"));
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

impl Move {
    const OFFSET_FROM: i32 = 0;
    const OFFSET_TO: i32 = 6;
    const OFFSET_FLAG: i32 = 12;

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
        MoveFlag::from_index(self.bits >> Move::OFFSET_FLAG)
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
    pub const fn double_push_en_passant_square(&self) -> Square {
        Square::from_u32((self.from().index() + self.to().index()) as u32 / 2)
    }

    #[inline]
    pub fn is_ep_capture(&self, _b: &Board) -> bool {
        self.flag().is_en_passant_capture()
        // !self.is_null() && self.mover_piece(b) == Piece::Pawn && self.to().is_in(b.en_passant())
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
            // debug_assert!(false, "capture square {self} on board {b}");
            Square::null()
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
        self.mover_piece(bd) == Piece::Pawn && self.to().rank_number_as_white(c) == 7
    }

    // #[inline]
    // pub const fn promo_piece(&self) -> Piece {
    //     Piece::from_index((self.bits >> Self::OFFSET_PROMO) as usize & 7)
    // }

    #[inline]
    pub const fn promo_piece(&self) -> Option<Piece> {
        self.flag().promo_piece()
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
        (self.flag().index() & 8) != 0
    }

    #[inline]
    pub fn is_castle(&self, _b: &Board) -> bool {
        self.flag().is_castling()
        // !self.castling_side(b).is_empty()
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
        // self.mover_piece(b) == Piece::Pawn
        //     && (self.to().index() == self.from().index() + 16
        //         || self.to().index() + 16 == self.from().index())
    }

    #[inline]
    pub fn rook_move(&self, b: &Board) -> Move {
        if self.is_castle(b) {
            let (from, to) = self.rook_move_from_to();
            Move::new_quiet(Piece::Rook, from, to)
        } else {
            Move::new_null()
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
            // let squares_changing = self.to().as_bb() | self.from().as_bb();
            CastlingRights::rights_lost(self.from(), self.to())
        }
    }

    #[inline]
    pub fn mvv_lva_score(&self, bd: &Board) -> i32 {
        debug_assert!(bd.is_legal_move(*self), "{self} is illegal for board {bd}");
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

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_uci())?;
        if f.alternate() {}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::iter::once;

    use super::*;
    use crate::{
        catalog::Catalog,
        infra::{black_box, profiler::PerfProfiler},
        other::Perft,
        Position,
    };
    use test_log::test;
    // use crate::movelist::MoveValidator;

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
        assert_eq!( mv.mover, Some(Piece::Rook) );
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
    fn test_validate_move() -> anyhow::Result<()>   {

        let p = Position::parse_epd(
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
        let b = p.board();
        assert_eq!("f4f5".parse::<BareMove>()?.validate(b).unwrap().to_uci(), "f4f5");
        assert_eq!("h4h5".parse::<BareMove>()?.validate(b).is_err(), true);
        assert_eq!("h5g6".parse::<BareMove>()?.validate(b).unwrap().to_uci(), "h5g6"); // ep capture
        assert_eq!("Ke1g1".parse::<BareMove>()?.validate(b).unwrap().to_uci(), "e1g1"); // castle K
        assert_eq!("Ke1c1".parse::<BareMove>()?.validate(b).is_err(), true); // castle Q
        assert_eq!("Ra1a7".parse::<BareMove>()?.validate(b).unwrap().to_uci(), "a1a7"); // rook attack
        assert_eq!("Ra1a8".parse::<BareMove>()?.validate(b).is_err(), true); // rook x-ray attack
        assert_eq!("Ra1b2".parse::<BareMove>()?.validate(b).is_err(), true); // rook diagonal
        assert_eq!("Ke1d1".parse::<BareMove>()?.validate(b).unwrap().to_uci(), "e1d1"); // king move
        assert_eq!("Ke1e2".parse::<BareMove>()?.validate(b).is_err(), true); // king in check
        assert_eq!("Re3e2".parse::<BareMove>()?.validate(b).unwrap().to_uci(), "e3e2"); // pinned piece ok
        assert_eq!("Re3f3".parse::<BareMove>()?.validate(b).is_err(), true); // pinned piece
        assert_eq!("0000".parse::<BareMove>()?.validate(b).unwrap().to_uci(), "0000"); // null move

        test_all_legal_and_illegal_moves(&b);
        for pos in Catalog::bratko_kopec().iter() {
            test_all_legal_and_illegal_moves(&pos.board());
        }
        Ok(())
    }


    fn test_all_legal_and_illegal_moves(b: &Board)    {
        let pieces = Piece::ALL.iter().map(|&p| Some(p)).chain(once(None));
        for mover in pieces.clone() {
            for from in Square::all() {
                for to in Square::all() {
                    for promo in pieces.clone() {
                        let bm = BareMove { mover, from, to, promo};
                        if bm.is_null() {
                            continue;
                        }
                        let mut found = false;
                        for lm in b.legal_moves().iter() {
                            if lm.to() == bm.to && lm.from() == bm.from && lm.promo_piece() == bm.promo && (Some(lm.mover_piece(b)) == bm.mover || bm.mover == None) {
                                assert_eq!(bm.validate(b).expect(&format!("{bm} {bm:?}")).to_uci(), lm.to_uci());
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

        let mut validate_good = PerfProfiler::new("baremove: validate good".into());
        let mut validate_bad = PerfProfiler::new("baremove: validate bad".into());
        let mut validate_old = PerfProfiler::new("baremove: validate old".into());

        let mut func = |bd: &Board, mv: Move| {
            let bm = mv.to_inner();
            _ = validate_good.benchmark(|| black_box(bm).validate(black_box(bd)));
            _ = validate_bad.benchmark(|| black_box(bm).validate(black_box(&wrong)));
            _ = validate_old.benchmark(|| black_box(bd).is_pseudo_legal_and_legal_move(black_box(mv)));
        };
        Perft::perft_with(&mut starting_pos, 2, &mut func);
    }


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
    fn bench_move() {
        let mut starting_pos = Catalog::perft_kiwipete().0;

        let mut clone = PerfProfiler::new("move: clone".into());
        let mut from = PerfProfiler::new("move: from".into());
        let mut to = PerfProfiler::new("move: to".into());
        let mut from_index = PerfProfiler::new("move: from_index".into());
        let mut capture_sq = PerfProfiler::new("move: capture_sq".into());
        let mut mover = PerfProfiler::new("move: mover".into());
        let mut capture_piece = PerfProfiler::new("move: capture_piece".into());
        let mut is_ep = PerfProfiler::new("move: is_ep".into());
        let mut is_capture = PerfProfiler::new("move: is_capture".into());
        let mut is_castle = PerfProfiler::new("move: is_castle".into());
        let mut is_double_push = PerfProfiler::new("move: is_double_push".into());
        let mut new_pawn_move = PerfProfiler::new("move: new_pawn_move".into());
        let mut ep_square = PerfProfiler::new("move: en_passant_square".into());

        let mut func = |bd: &Board, mv: Move| {
            let index = mv.flag().index();
            clone.benchmark(|| black_box(mv).clone());
            from.benchmark(|| black_box(mv).from());
            to.benchmark(|| black_box(mv).to());
            from_index.benchmark(|| MoveFlag::from_index(black_box(index)));
            capture_sq.benchmark(|| black_box(mv).capture_square(black_box(bd)));
            mover.benchmark(|| black_box(mv).mover_piece(black_box(bd)));
            capture_piece.benchmark(|| black_box(mv).capture_piece(black_box(bd)));
            is_ep.benchmark(|| black_box(mv).is_ep_capture(black_box(bd)));
            is_capture.benchmark(|| black_box(mv).is_capture());
            is_castle.benchmark(|| black_box(mv).is_castle(black_box(bd)));
            is_double_push.benchmark(|| black_box(mv).is_pawn_double_push(black_box(bd)));
            ep_square.benchmark(|| black_box(mv).double_push_en_passant_square());
            if mv.from().is_in(bd.pawns()) {
                new_pawn_move.benchmark(|| {
                    Move::new_pawn_move(black_box(mv).from(), black_box(mv).to(), black_box(bd))
                });
            }
        };
        Perft::perft_with(&mut starting_pos, 2, &mut func);
    }

    #[test]
    fn test_move() {
        assert_eq!(Move::new_null().to_string(), "0000");

        let move_castle = Move::new_castle(a1.square(), b2.square());

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

        let move_a1b2 = Move::new_quiet(Piece::Bishop, a1.square(), b2.square());

        println!("{:#} {:b}", move_a1b2, move_a1b2.bits);
        assert_eq!(move_a1b2.is_capture(), false);
        assert_eq!(move_a1b2.from(), a1.square());
        assert_eq!(move_a1b2.to(), b2.square());
        // assert_eq!(move_a1b2.mover_piece(), Piece::Bishop);
        assert_eq!(move_a1b2.is_promo(), false);

        let capture_a1b2 = Move::new_capture(
            Piece::Bishop,
            a1.square(),
            b2.square(),
            &Board::starting_pos(),
        );
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
            Perft::perft_with(&mut pos.board().clone(), 3, &mut func);
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
