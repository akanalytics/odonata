use crate::bitboard::Bitboard;
use crate::board::Board;
use crate::types::{CastlingRights, Color, Piece};
use crate::utils::StringUtils;
use std::fmt;
use std::ops::{Deref, DerefMut};

// FIXME: public methods
#[derive(Debug, Default, Copy, Clone)]
pub struct Move {
    pub from: Bitboard,
    pub to: Bitboard,
    pub ep: Bitboard,
    pub promo: Piece,
    pub capture: Piece,
    pub mover: Piece,

    pub is_castle: bool,
    pub is_null: bool,
    pub is_drop: bool,
}

impl Move {
    
    
    #[inline]
    pub fn new_null() -> Move {
        Move { is_null: true, ..Default::default() }
    }

    #[inline]
    pub fn to(&self) -> Bitboard {
        self.to
    }

    #[inline]
    pub fn from(&self) -> Bitboard {
        self.from
    }

    #[inline]
    pub fn ep(&self) -> Bitboard {
        self.ep
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.is_null
    }

    #[inline]
    pub fn is_promo(&self) -> bool {
        self.promo != Piece::None
    }

    #[inline]
    pub fn promo_piece(&self) -> Piece {
        self.promo
    }

    #[inline]
    pub fn capture_piece(&self) -> Piece {
        self.capture
    }

    #[inline]
    pub fn mover_piece(&self) -> Piece {
        self.mover
    }

    #[inline]
    pub fn is_capture(&self) -> bool {
        self.capture != Piece::None
    }

    #[inline]
    pub fn is_castle(&self) -> bool {
        self.is_castle
    }

    #[inline]
    pub fn is_drop(&self) -> bool {
        self.is_drop
    }

    #[inline]
    pub fn is_ep_capture(&self) -> bool {
        !self.ep.is_empty() && self.is_capture()
    }

    #[inline]
    pub fn is_pawn_double_push(&self) -> bool {
        !self.ep.is_empty() && !self.is_capture()
    }

    pub fn uci(&self) -> String {
        if self.is_null() {
            return String::from('-');
        }
        let mut res = String::new();
        res.push_str(&self.from.uci());
        res.push_str(&self.to.uci());
        if self.is_promo() {
            res.push(self.promo.to_char(Some(Color::Black)));
        }
        res
    }
    
    pub fn parse_uci(s: &str) -> Result<Move, String> {
        let from = Bitboard::parse_square(s.take_slice(0..2))?;
        let to = Bitboard::parse_square(s.take_slice(2..4))?;
        let promo;
        if let Some(ch) = s.take_char_at(4) {
            promo = Piece::from_char(ch)?;
        } else {
            promo = Piece::None;
        }
        Ok(Move { to, from, promo, ..Default::default() })
    }
}






















impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.uci())
    }
}

#[derive(Debug, Clone, Default)]
pub struct MoveList(Vec<Move>);

// pub struct MoveList(ArrayVec::<[Move; 384]>);
// impl Default for MoveList {
//     fn default() -> MoveList { MoveList::new() }
// }

impl MoveList {
    pub fn new() -> Self {
        MoveList(Vec::with_capacity(250)) // TODO: capacity??
    }

    pub fn sort(&mut self) -> &mut Self {
        self.0.sort_by_key(|m| m.to_string());
        self
    }

}

impl Deref for MoveList {
    type Target = Vec<Move>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MoveList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for MoveList {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let strings: Vec<String> = self.0.iter().map(Move::to_string).collect();
        fmt.write_str(&strings.join(", "))
        // fmt.write_str(self.0.iter().fold(String::new(), |acc, m| acc + m.to_string().as_str()).as_str())
    }
}






#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::*;
    use crate::globals::constants::*;

    #[test]
    fn move_and_movelist() {
        assert_eq!(Move::new_null().to_string(), "-");

        let move_a1b2 = Move { from: a1, to: b2, ..Default::default() };
        let promo_a7a8 = Move { from: a7, to: a8, promo: Piece::Queen, ..Default::default() };
        assert_eq!(move_a1b2.to_string(), "a1b2");
        assert_eq!(promo_a7a8.to_string(), "a7a8q");
        let mut moves = MoveList::new();
        moves.push(move_a1b2);
        moves.push(promo_a7a8);
        assert_eq!(moves.to_string(), "a1b2, a7a8q");

        let move_e2e4 = Move::parse_uci("e2e4").unwrap();
        assert_eq!(move_e2e4.to_string(), "e2e4");

        let move_e7e8 = Move::parse_uci("e7e8p").unwrap();
        assert_eq!(move_e7e8.to_string(), "e7e8p");
    }
}





