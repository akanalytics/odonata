use crate::bitboard::{Bitboard, Dir};
use crate::globals::*;
use std::iter::*;
use std::fmt::{self, Write};


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

    pub fn from_char(ch : char) -> Result<Color, String> {
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
pub struct BoardBuf(Board);

impl BoardBuf {
    pub fn new() -> BoardBuf {
        BoardBuf(Board::empty())
    }



    pub fn set_piece_at(&mut self, sq: Bitboard, p: Piece) {
        for bb in self.0.pieces.iter_mut() {
            bb.remove(sq);
        } 
        // self.0.pieces(p).remove(sq);
        self.0.pieces[p.index()].insert(sq);
    }

    pub fn set_color_at(&mut self, sq: Bitboard, c: Color) {
        self.0.color(c.opposite()).remove(sq);
        self.0.colors[c.index].insert(sq);
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
            let ch = match p {
                Piece::None => p.to_upper_char(),
                _ => p.to_char(self.color_at(sq)),
            };
            res.push(ch);
        }
        res
    }

    pub fn set(&mut self, bb: Bitboard, pieces: &str) -> Result<&mut Self, String> {
        if bb.len()!= pieces.chars().count() {
            return Err(format!("Bitboard {} and pieces {} have different counts", bb, pieces));
        }
        for (sq, ch) in bb.iter().zip(pieces.chars()) {
            let p = Piece::from_char(ch)?;
            self.set_piece_at(sq, p);
            if p != Piece::None {
                let c = Color::from_char(ch)?;
                self.set_color_at(sq, c);
            } else {
                // FIXME: broken approach - null color??
                self.0.colors[0].remove(sq);
                self.0.colors[1].remove(sq);
            };
        }
        Ok(self)
    }

    pub fn as_board(&self) -> Board {
        self.0
    }





    pub fn parse_fen(fen: &str) -> Result<Self, String> {
        let mut bb = BoardBuf::new();
        let mut words = fen.split_whitespace();
        if let Some(part) = words.next() {
            let mut pos = String::from(part);   
            for i in 1..=8 {
                pos = pos.replace(i.to_string().as_str(), " ".repeat(i).as_str());
            }
            // pos.retain(|ch| "pPRrNnBbQqKk ".contains(ch));
            let r: Vec<&str> = pos.rsplit('/').collect();
            if r.iter().any(|r| r.chars().count() != 8) || r.len() != 8 {
                return Err(format!("Expected 8 ranks of 8 pieces in fen {}", fen));
            }
            bb.set( Bitboard::all(), &r.concat() )?;
        }
        Ok(bb)
    }
}




        //     // assert 8
        //     for r in ranks {
        //         for ch in r.chars() {
        //             match ch {
        //                 ch in digit {
        //                     add "." * digit
        //                 }
        //                 ch in valid Piece. add p
        //             }

        //         }
        //     }
        // } 
        // else {
        //     return Err("Fen string lacks position part.");
        // }
    //}
    // sqs = ''
    // valid_digits = "12345678"
    // valid_pieces = "pnbrqkPNBRQK"
    // ranks_8_to_1 = fen_part1.split('/')
    // if len(ranks_8_to_1) != 8:
    //     raise ValueError(f'Invalid FEN {fen_part1}: Expected 8 ranks in position part but found {len(ranks_8_to_1)}')
    // for rank in ranks_8_to_1:
    //     row = ''
    //     for p in rank:
    //         if p in valid_digits:
    //             row += ' ' * int(p)
    //         elif p in valid_pieces:
    //             row += p
    //         else:
    //             raise ValueError(f'Invalid FEN {fen_part1} in row of "{rank}" unexpected "{p}"')
    //     # weve captured all the pieces/squares in this row
    //     if len(row) != 8:
    //         raise ValueError(f'Invalid FEN {fen_part1} in row of "{rank}" expected 8 pieces but found {len(row)}')
    //     sqs += row
    // grid = "\n".join([sqs[r*8:r*8 + 8] for r in range(8)])
    // return grid





impl fmt::Display for Board {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = BoardBuf(*self);
        for &r in Bitboard::RANKS.iter().rev() {
            fmt.write_str(&b.get(r))?;
            fmt.write_char('\n')?;
        }
        Ok(())
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
    fn piece() {
        assert_eq!(Piece::Pawn.to_upper_char(), 'P');
        assert_eq!(Piece::King.to_char(Some(Color::BLACK)), 'k');
        assert_eq!(Piece::King.to_char(None), 'K');
    }

    #[test]
    fn boardbuf() -> Result<(),String> {
        let board = Board::empty();
        assert_eq!(board.kings(), Bitboard::EMPTY);
        assert_eq!(board.us(), Bitboard::EMPTY);
        assert_eq!(board.color_us().is_white, true);

        // assert_eq!(board[a1], 'R');
        let mut buf = BoardBuf::new();
        let x = buf.set(Bitboard::RANK_2, "PPPPPPPP")?.set(a1|h1, "RR")?.set(b1|g1, "NN")?.set(c1|d1|e1|f1, "BQKB")?;
        buf.set(Bitboard::RANK_7, "pppppppp")?.set(Bitboard::RANK_8, "rnbqkbnr")?;
        assert_eq!(buf.get(a1), "R");
        // let b = hashmap!{ a1+h1 => "R", b1+g1 => "N" };
        // let b = BoardBuf::new().rooks(a1|h1).knights(b1|g1).pawns(rank_2).set("RNBQKBNR", rank_1);
        // let b = BoardBuf::new("rnbqkbnr/
        //     pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
        // let b = BoardBuf::new().k(a1).K(h8).r(a2).R(c3);
        // let b = BoardBuf::new().set(a1=k, rank_2=p, );
        // todo!()
        println!("{}", buf.as_board() );
        assert!( BoardBuf::parse_fen("1/1/7/8/8/8/PPPPPPPP/RNBQKBNR").err().unwrap().starts_with("Expected 8"));
        assert!( BoardBuf::parse_fen("8").err().unwrap().starts_with("Expected 8"));
        assert!( BoardBuf::parse_fen("8/8").err().unwrap().starts_with("Expected 8"));
        assert_eq!( BoardBuf::parse_fen("X7/8/8/8/8/8/8/8").err(), Some("Unknown piece 'X'".to_string()));
        let buf = BoardBuf::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        assert_eq!(buf.get(a1), "R");
        assert_eq!(buf.get(Bitboard::FILE_H), "RP....pr");
        Ok(())
    }

    fn board_bitboards() -> Result<(),String> {
        let board = BoardBuf::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap().as_board();
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
