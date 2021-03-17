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


    // pub fn parse_san(s: &str, board: &Board) -> Result<Move, String> {
    
    
    //     //  convert 0's to O's
    //     //  Wikipedia:
    //     //    Castling is indicated by the special notations 0-0 (for kingside castling) and 0-0-0 (queenside castling).
    //     //    While the FIDE standard [5] is to use the digit zero (0-0 and 0-0-0), PGN uses the uppercase letter O (O-O and O-O-O).[6]
    //     let orig = s;
    //     s = orig.replace("0","O");

    //     // Checkmate at the completion of moves is represented by the symbol "#" in standard FIDE notation and PGN.
    //     // The word mate is commonly used instead; occasionally a double dagger (‡)
    //     s = s.replace("mate","#");
    //     s = s.replace("‡","#");

    //     // strip whitespace
    //     s = s.replace(" ","");


    //     // regex from https://stackoverflow.com/questions/40007937/regex-help-for-chess-moves-san
    //     // /^([NBRQK])?([a-h])?([1-8])?(x)?([a-h][1-8])(=[NBRQK])?(\+|#)?$|^O-O(-O)?$/
    //     // which claims... 'This was unit tested against 2599 cases'
    //     //
    //     // change
    //     //   convert python  : $ to \Z
    //     //   allow "-"       : (\-|x)
    //     //   allow lc promos : [nbrqkNBRQK]
    //     //
    //     // r"^([NBRQK])?([a-h])?([1-8])?(\-|x)?([a-h][1-8])(=[NBRQ])?(\+|#)?\Z|^O-O(-O)?\Z"
    //     //
    //     let re = Regex::new(r#"(?x)    # x flag to allow whitespace and comments
    //         ^
    //         ([NBRQK])?      # piece - grp(1)
    //         ([a-h])?        # src square rank grp(2)
    //         ([1-8])?        # src square file grp(3)
    //         (\-|x)?         # move or capture grp(4)
    //         ([a-h][1-8])?   # square - both rank and file grp(5)
    //         (=[NBRQ])?      # promo grp(6)
    //         (\+|\#)?        # check or checkmate grp(7)
    //         \Z
    //         |               # OR
    //         ^O-O(-O)?\Z     #   or castling king (or queens) side and eol
    //         "#)?;

    //     let caps = re.captures(s)?;
    //     // if not match:
    //     //     raise ValueError(f"Move {orig} is invalid - wrong format")

    //     // parse checkmates
    //     let checkmate = s.contains('#');

    //     // parse castling
    //     castling = false
    //     if s == "O-O" or s == "O-O-O" {
    //         // side = Pieces.to_colour("K" if s == 'O-O' else 'Q', self.board.turn)
    //         //return self.check_legal_move(MoveGenerator.CASTLING_KING_MOVE[side], orig)
    //     }
         
    //     let legal_moves = board.legal_moves();
    //     caps.get(0).unwrap().as_str()
    //     let mut piece = caps.get(1);
    //     let src_rank = caps.get(2);
    //     let src_file = caps.get(3);
    //     // move or capture = grp 4
    //     let dst_square = caps.get(5);
    //     let promo = caps.get(6);
    //     let checks = caps.get(7);

    //     // if one square is given, its the destination not the source
    //     if dst_square == None && if let Some(src_rank) = src_rank {
    //         if let Some(src_file) = src_file {
    //             dst_square = src_rank + src_file
    //             src_rank = ""
    //             src_file = ""

    //     // pawn prefixs are omiited
    //     if piece == None {
    //         piece = Some("P");
    //     }

    //     // strategy: search through legal moves looking for a move that fits the criteria
    //     // this is slow but easy to understand. Castling has already been dealt with
    //     let matching_moves: Vec<Move> = vec![];
    //     for lm in legal_moves:
    //         if dst_square and lm.dst != Square.parse(dst_square):
    //             continue
    //         if src_rank and lm.src.rank() != src_rank:
    //             continue
    //         if src_file and lm.src.file() != src_file:
    //             continue
    //         if piece and self.board.piece_at(lm.src).upper() != piece:
    //             continue
    //         if promo and lm.promotion.upper != promo:
    //             continue
    //         matching_moves.append(lm)

    //     if not matching_moves:
    //         raise ValueError(f"Move {orig} is invalid - not a legal move")

    //     if len(matching_moves) > 1:
    //         raise ValueError(f"Move {orig} is ambiguous - moves {Move.names(matching_moves)}")


    //     # FIXME: warnings on non-captures, non-checkmates etc
    //     return matching_moves[0]
    
    // }






















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





