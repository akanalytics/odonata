use crate::bitboard::{Bitboard, Dir};
use std::iter::*;
use std::fmt::{self, Write};
use self::movegen::MoveGen;

mod movegen;
mod movemaker;
pub mod boardbuf;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
    pub is_white: bool,
    pub index: usize,
    pub pawn_move: Dir,
    pub pawn_capture_east: Dir,
    pub pawn_capture_west: Dir,
    pub kingside_castle_sqs: Bitboard,
    pub queenside_castle_sqs: Bitboard,
    pub double_push_dest_rank: Bitboard,
    pub castle_rights_queen: CastlingRights,
    pub castle_rights_king: CastlingRights,
    pub back_rank: Bitboard,
}

bitflags! {
    pub struct CastlingRights: u8 {
        const NONE = 0;
        const WHITE_KING = 1 << 0;
        const WHITE_QUEEN = 1 << 1;
        const BLACK_KING = 1 << 2;
        const BLACK_QUEEN = 1 << 3;
        const ALL = Self::WHITE_KING.bits | Self::WHITE_QUEEN.bits | Self::BLACK_KING.bits | Self::BLACK_QUEEN.bits;
    }
}

impl CastlingRights {
    pub fn parse(s: &str) -> Result<CastlingRights, String> {
        let mut castling = CastlingRights::NONE;
        for ch in s.chars() {
            match ch {
                '-' => break,
                'K' => castling |= CastlingRights::WHITE_KING,
                'Q' => castling |= CastlingRights::WHITE_QUEEN,
                'k' => castling |= CastlingRights::BLACK_KING,
                'q' => castling |= CastlingRights::BLACK_QUEEN,
                _ => return Err(format!("Invalid character '{}' in castling rights '{}'", ch, s)),
            }
        }
        Ok(castling)
    }
}



impl Color {
    pub const WHITE: Self = Color {
        is_white: true,
        index: 0,
        pawn_move: Dir::N,
        pawn_capture_east: Dir::NE,
        pawn_capture_west: Dir::NW,
        kingside_castle_sqs: Bitboard::F1.or(Bitboard::G1),
        queenside_castle_sqs: Bitboard::D1.or(Bitboard::C1).or(Bitboard::B1),
        double_push_dest_rank: Bitboard::RANK_4,
        castle_rights_queen: CastlingRights::WHITE_QUEEN,
        castle_rights_king: CastlingRights::WHITE_KING,
        back_rank: Bitboard::RANK_1,
    };
    pub const BLACK: Self = Color {
        is_white: false,
        index: 1,
        pawn_move: Dir::S,
        pawn_capture_east: Dir::SE,
        pawn_capture_west: Dir::SW,
        kingside_castle_sqs: Bitboard::F8.or(Bitboard::G8),
        queenside_castle_sqs: Bitboard::D8.or(Bitboard::C8),
        double_push_dest_rank: Bitboard::RANK_5,
        castle_rights_queen: CastlingRights::BLACK_QUEEN,
        castle_rights_king: CastlingRights::BLACK_KING,
        back_rank: Bitboard::RANK_8,
    };

    pub fn opposite(&self) -> Color {
        [Color::BLACK, Color::WHITE][self.index]
    }

    pub fn parse(s: &str) -> Result<Color, String> {
        match s {
            "w" => Ok(Color::WHITE),
            "b" => Ok(Color::BLACK),
            _ => Err(format!("Invalid color: '{}'", s))
        }
    }

    pub fn from_piece_char(ch : char) -> Result<Color, String> {
        if ch.is_lowercase() {
            return Ok(Color::BLACK);
        }
        else if ch.is_uppercase() {
            return Ok(Color::WHITE);
        }
        Err(format!("Cannot get color for char '{}'", ch))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Piece {
    None = 0,
    Pawn = 1,
    Knight = 2,
    Bishop = 3,
    Rook = 4,
    Queen = 5,
    King = 6,
}

impl Default for Piece {
    fn default() -> Self { Piece::None }
}

impl Piece {
    const ALL: [Piece; 6] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];

    #[inline]
    // pub fn to_upper_char(self) -> &char {
    //     ".PNBRQK".as_bytes()[self as usize] as char 
    // }

    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }

    #[inline]
    pub fn to_upper_char(&self) -> char {
        match self {
            Piece::None => '.',
            Piece::Pawn => 'P',
            Piece::Knight => 'N',
            Piece::Bishop => 'B',
            Piece::Rook => 'R',
            Piece::Queen => 'Q',
            Piece::King => 'K',
        }
    }

    #[inline]
    pub fn from_char(ch: char) -> Result<Piece, String> {
        Ok(match ch.to_ascii_uppercase() {
            '.'|' ' => Piece::None,
            'P' => Piece::Pawn,
            'N' => Piece::Knight,
            'B' => Piece::Bishop,
            'R' => Piece::Rook,
            'Q' => Piece::Queen,
            'K' => Piece::King,
            _ => return Err(format!("Unknown piece '{}'", ch))
        })
    }

    #[inline]
    pub fn to_char(&self, c: Option<Color>) -> char {
        match c {
            None => self.to_upper_char(),
            Some(c) if c.is_white  => self.to_upper_char(),
            Some(_) => self.to_upper_char().to_ascii_lowercase(),
        }
    }    
}



#[derive(Copy, Clone)]
pub struct Board {
    pieces: [Bitboard; 7],
    colors: [Bitboard; 2],
    castling: CastlingRights,
    en_passant: Bitboard,
    turn: Color,
    fifty_clock: u16,
    fullmove_count: u16,
}



#[derive(Debug, Default)]
pub struct Move {
    from: Bitboard,
    to: Bitboard,
    ep: Bitboard,
    promo: Piece,
    capture: Piece,
    mover: Piece,

    is_castle: bool,
    is_null: bool,
    is_drop: bool,
}


impl Move {
    pub fn is_promo(&self) -> bool {
        self.promo != Piece::None
    }

    pub fn is_capture(&self) -> bool {
        self.capture != Piece::None
    }

}


impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut res = String::new();
        res.push_str(self.from.uci().as_str());
        res.push_str(self.to.uci().as_str());
        if self.is_promo() {
            res.push(self.promo.to_char(Some(Color::BLACK)));
        }
        write!(f, "{}", res)
    }
}








#[derive(Debug, Default)]
pub struct MoveList(Vec<Move>);

impl MoveList {
    pub fn new() -> Self {
        MoveList(Vec::with_capacity(250))  // TODO: capacity??
    }

    pub fn sort(&mut self) -> &mut Self {
        self.0.sort_by_key(|m| m.to_string());
        self
    }
}

use std::ops::{Deref, DerefMut};

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



#[derive(Debug)]
pub enum MoveEnum {
    Promo { dest: Bitboard, src: Bitboard, promo: Piece },
    PromoCapture { dest: Bitboard, src: Bitboard, promo: Piece, capture: Piece },
    EnPassant { dest: Bitboard, src: Bitboard, capture_sq: Bitboard },
    Push { dest: Bitboard, src: Bitboard },
    Castle { king_dest: Bitboard, king_src: Bitboard, rook_dest: Bitboard, rook_src: Bitboard, right: CastlingRights },
    Quiet { dest: Bitboard, src: Bitboard, mover: Piece },
    Capture { dest: Bitboard, src: Bitboard, mover: Piece, capture: Piece },
    Null(),
    // DropAdd { dest: Bitboard, piece: Piece },
    // DropRemove { dest: Bitboard, piece: Piece },
}



impl Board {
    pub fn empty() -> Board {
        Board {
            pieces: [Bitboard::EMPTY; 7],
            colors: [Bitboard::EMPTY; 2],
            castling: CastlingRights::ALL,
            en_passant: Bitboard::EMPTY,
            turn: Color::WHITE,
            fullmove_count: 0,
            fifty_clock: 0,
        }
    }

    
    // pub fn new() -> Board {
    //     let board = Board {
    //         pieces: [Bitboard::EMPTY; 7],
    //         colors: [Bitboard::EMPTY; 2],
    //         ..Board::empty()
    //     };
    //     board
    // }



    // fn piece_and_color_at(&self, at: Bitboard) -> (Piece, Color) {
    //     for p in &Piece::ALL {
    //         if self.pieces[*p as usize].contains(at) {
    //             let c = if self.colors[Color::WHITE.index].contains(at) { Color::WHITE } else { Color::BLACK };
    //             return (*p, c);
    //         }
    //     }
    //     (Piece::None, Color::BLACK)  
    // }

    pub fn castling(&self) -> CastlingRights {
        self.castling
    }

    pub fn pieces(&self, p: Piece) -> Bitboard {
        self.pieces[p as usize]
    }

    pub fn pawns(&self) -> Bitboard {
        self.pieces(Piece::Pawn)
    }

    pub fn knights(&self) -> Bitboard {
        self.pieces(Piece::Knight)
    }

    pub fn bishops(&self) -> Bitboard {
        self.pieces(Piece::Bishop)
    }

    pub fn rooks(&self) -> Bitboard {
        self.pieces(Piece::Rook)
    }

    pub fn queens(&self) -> Bitboard {
        self.pieces(Piece::Queen)
    }

    pub fn kings(&self) -> Bitboard {
        self.pieces(Piece::King)
    }

    pub fn color(&self, c: Color) -> Bitboard {
        self.colors[c.index]
    }

    pub fn white(&self) -> Bitboard {
        self.colors[Color::WHITE.index]
    }

    pub fn black(&self) -> Bitboard {
        self.colors[Color::BLACK.index]
    }

    pub fn color_us(&self) -> Color {
        self.turn
    }

    pub fn color_them(&self) -> Color {
        self.turn.opposite()
    }

    pub fn them(&self) -> Bitboard {
        self.color(self.turn.opposite())
    }

    pub fn us(&self) -> Bitboard {
        self.color(self.turn)
    }

    pub fn fifty_halfmove_clock(&self) -> u32 {
        self.fifty_clock.into()
    }

    pub fn fullmove_counter(&self) -> u32 {
        self.fullmove_count.into()
    }

    pub fn piece_at(&self, sq: Bitboard) -> Piece {
        for &p in &Piece::ALL {
            if self.pieces(p).contains(sq) {
                return p;
            }
        }
        Piece::None
    }

    pub fn pseudo_legal_moves(&self) -> MoveList {
        let mut moves = MoveList::new();
        MoveGen::new().pseudo_legal_moves(self, &mut moves);
        moves
    }


}









#[cfg(test)]
mod tests {
    use super::*;
    use super::boardbuf::*;

    use crate::globals::constants::*;

    #[test]
    fn color() {
        assert_eq!(Color::parse("w"), Ok(Color::WHITE));
        assert_eq!(Color::parse("b"), Ok(Color::BLACK));
        assert_eq!(Color::parse("B"), Err("Invalid color: 'B'".to_string()));
        assert_eq!(Piece::King.to_char(Some(Color::BLACK)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');

    }

    #[test]
    fn piece() {
        assert_eq!(Piece::Pawn.to_upper_char(), 'P');
        assert_eq!(Piece::King.to_char(Some(Color::BLACK)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');

    }

    #[test]
    fn move_and_movelist() {
        let move_a1b2 = Move { from:a1, to:b2, ..Default::default() };
        let promo_a7a8 = Move { from:a7, to:a8, promo: Piece::Queen, ..Default::default() };
        assert_eq!(move_a1b2.to_string(), "a1b2" );
        assert_eq!(promo_a7a8.to_string(), "a7a8q" );
        let mut moves = MoveList::new();
        moves.push(move_a1b2);
        moves.push(promo_a7a8);
        assert_eq!(moves.to_string(), "a1b2, a7a8q");
    }

    #[test]
    fn board_bitboards() -> Result<(),String> {
        let board = BoardBuf::parse_pieces("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap().as_board();
        assert_eq!(board.color_us(), Color::WHITE);
        assert_eq!(board.color_them(), Color::BLACK);
        // assert_eq!(board.en_passant(), Bitboard::empty());
        // assert_eq!(board.move_count(), 0);
        assert_eq!(board.pawns() & board.us(), Bitboard::RANK_2);
        assert_eq!(board.rooks() & board.them(), a8|h8);
        assert_eq!(board.bishops() & board.us(), c1|f1);
        assert_eq!(board.them(), Bitboard::RANK_7 | Bitboard::RANK_8 );
        Ok(())
    }
}