use crate::bitboard::{Bitboard, Dir};
use crate::globals::*;
use std::iter::*;


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
    };

    pub fn opposite(&self) -> Color {
        [Color::BLACK, Color::WHITE][self.index]
    }

    pub fn from_char(ch : char) -> Color {
        if ch.is_lowercase() {
            return Color::BLACK;
        }
        else if ch.is_uppercase() {
            return Color::WHITE;
        }
        panic!(format!("Cannot get color for char '{}'", ch))
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
    pub fn from_char(ch: char) -> Piece {
        match ch.to_ascii_uppercase() {
            '.'|' ' => Piece::None,
            'P' => Piece::Pawn,
            'N' => Piece::Knight,
            'B' => Piece::Bishop,
            'R' => Piece::Rook,
            'Q' => Piece::Queen,
            'K' => Piece::King,
            _ => panic!("")
        }

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
    move_count: u16,
    fifty_clock: u16,
}

impl Board {
    pub fn empty() -> Board {
        Board {
            pieces: [Bitboard::EMPTY; 7],
            colors: [Bitboard::EMPTY; 2],
            castling: CastlingRights::ALL,
            en_passant: Bitboard::EMPTY,
            turn: Color::WHITE,
            move_count: 0,
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

    pub fn piece_at(&self, sq: Bitboard) -> Piece {
        for &p in &Piece::ALL {
            if self.pieces(p).contains(sq) {
                return p;
            }
        }
        Piece::None
    }

}


/// BoardBuf is a slow performing facade of convenience methods on board
struct BoardBuf(Board);

impl BoardBuf {
    pub fn new() -> BoardBuf {
        BoardBuf(Board::empty())
    }



    pub fn set_piece_at(&mut self, sq: Bitboard, p: Piece) {
        for bb in self.0.pieces.iter_mut() {
            bb.remove(sq);
        } 
        // self.0.pieces(p).remove(sq);
        self.0.pieces(p).insert(sq);
    }

    pub fn set_color_at(&mut self, sq: Bitboard, c: Color) {
        self.0.color(c.opposite()).remove(sq);
        self.0.color(c).insert(sq);
    }


    pub fn color_at(&self, at: Bitboard) -> Option<Color> {
        if self.0.colors[Color::WHITE.index].contains(at) {
            return Some(Color::WHITE);
        }
        else if self.0.colors[Color::BLACK.index].contains(at) {
            return Some(Color::BLACK);
        }
        None
    }

    pub fn get(&self, bb: Bitboard) -> String {
        let mut res = String::new();
        for sq in bb.iter() {
            let p = self.0.piece_at(sq);
            let c = self.color_at(sq);
            let ch = p.to_char(c);
            res.push(ch);
        }
        res
    }

    pub fn set(&mut self, bb: Bitboard, pieces: &str) -> &mut Self {
        assert!(pieces.len() == 1 || pieces.len() == bb.len() as usize);
        for (sq, ch) in bb.iter().zip(pieces.chars()) {
            let p = Piece::from_char(ch);
            let c = Color::from_char(ch);
            self.set_piece_at(sq, p);
            self.set_color_at(sq, c);
        }
        self
    }

    pub fn as_board(&self) -> Board {
        self.0
    }
}




// impl  std::ops::IndexMut<Bitboard> for Board {
//     type Output = char;
//     fn index_mut(&mut self, index: Bitboard) -> &mut char {
//     }
// }

// impl std::ops::Index<Bitboard> for Board {
//     type Output = char;
//     fn index(&self, sq: Bitboard) -> &char {
//         self.piece_at(sq).to_upper_char()
//     }
// }


#[cfg(test)]
mod tests {
    use super::*;

    use crate::globals::constants::*;

    #[allow(non_upper_case_globals)]
    const a1b2: Bitboard = Bitboard::A1.or(Bitboard::B2);


    #[test]
    fn test_piece() {
        assert_eq!(Piece::Pawn.to_upper_char(), 'P');
        assert_eq!(Piece::King.to_char(Some(Color::BLACK)), 'k');
    }

    #[test]
    fn test_boardbuf() {
        let board = Board::empty();
        assert_eq!(board.kings(), Bitboard::EMPTY);
        assert_eq!(board.us(), Bitboard::EMPTY);
        assert_eq!(board.color_us().is_white, true);

        // assert_eq!(board[a1], 'R');
        let mut buf = BoardBuf::new();
        buf.set(Bitboard::RANK_2, "P").set(a1|h1, "RR").set(b1|g1, "NN").set(c1|d1|e1|f1, "BQKB");
        assert_eq!(buf.get(a1), "R");
        // let b = hashmap!{ a1+h1 => "R", b1+g1 => "N" };
        // let b = BoardBuf::new().rooks(a1|h1).knights(b1|g1).pawns(rank_2).set("RNBQKBNR", rank_1);
        // let b = BoardBuf::new("rnbqkbnr/
        //     pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
        // let b = BoardBuf::new().k(a1).K(h8).r(a2).R(c3);
        // let b = BoardBuf::new().set(a1=k, rank_2=p, );
        // todo!()

    }
}
