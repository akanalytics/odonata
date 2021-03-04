use std::fmt::{self, Write};

use super::{Board, Color, Piece};
use crate::bitboard::Bitboard;

/// BoardBuf is a slow performing facade of convenience methods on board
pub struct BoardBuf {
    board: Board,
    temporary: String,
}

impl BoardBuf {
    pub fn new() -> BoardBuf {
        BoardBuf { board: Board::empty(), temporary: String::new() }
    }

    pub fn adopt(board: Board) -> BoardBuf {
        BoardBuf { board, temporary: String::new() }
    }

    pub fn set_piece_at(&mut self, sq: Bitboard, p: Piece) {
        for bb in self.board.pieces.iter_mut() {
            bb.remove(sq);
        }
        // self.0.pieces(p).remove(sq);
        self.board.pieces[p.index()].insert(sq);
    }

    pub fn set_color_at(&mut self, sq: Bitboard, c: Color) {
        self.board.color(c.opposite()).remove(sq);
        self.board.colors[c.index].insert(sq);
    }

    pub fn color_at(&self, at: Bitboard) -> Option<Color> {
        if self.board.colors[Color::WHITE.index].contains(at) {
            return Some(Color::WHITE);
        } else if self.board.colors[Color::BLACK.index].contains(at) {
            return Some(Color::BLACK);
        }
        None
    }

    pub fn get(&self, bb: Bitboard) -> String {
        let mut res = String::new();
        for sq in bb.iter() {
            let p = self.board.piece_at(sq);
            let ch = match p {
                Piece::None => p.to_upper_char(),
                _ => p.to_char(self.color_at(sq)),
            };
            res.push(ch);
        }
        res
    }

    pub fn set(&mut self, bb: Bitboard, pieces: &str) -> Result<&mut Self, String> {
        if bb.len() != pieces.chars().count() {
            return Err(format!("Bitboard {} and pieces {} have different counts", bb, pieces));
        }
        for (sq, ch) in bb.iter().zip(pieces.chars()) {
            let p = Piece::from_char(ch)?;
            self.set_piece_at(sq, p);
            if p != Piece::None {
                let c = Color::from_piece_char(ch)?;
                self.set_color_at(sq, c);
            } else {
                // FIXME: broken approach - null color??
                self.board.colors[0].remove(sq);
                self.board.colors[1].remove(sq);
            };
        }
        Ok(self)
    }

    pub fn as_board(&self) -> Board {
        self.board
    }

    /// Parses a FEN string to create a board. FEN format is detailed at https://en.wikipedia.org/wiki/Forsyth–Edwards_Notation
    pub fn parse_pieces(fen: &str) -> Result<Self, String> {
        let mut bb = BoardBuf::new();
        let mut pos = String::from(fen);
        for i in 1..=8 {
            pos = pos.replace(i.to_string().as_str(), " ".repeat(i).as_str());
        }
        // pos.retain(|ch| "pPRrNnBbQqKk ".contains(ch));
        let r: Vec<&str> = pos.rsplit('/').collect();
        if r.iter().any(|r| r.chars().count() != 8) || r.len() != 8 {
            return Err(format!("Expected 8 ranks of 8 pieces in fen {}", fen));
        }
        bb.set(Bitboard::all(), &r.concat())?;
        Ok(bb)
    }

    /// 0. Piece placement
    /// 1. Active color
    /// 2. Castling rights
    /// 3. E/P square
    /// 4. Half move clock
    /// 5. Full move counter
    pub fn parse_fen(fen: &str) -> Result<Self, String> {
        let words = fen.split_whitespace().collect::<Vec<_>>();
        if words.len() < 6 {
            return Err(format!("Must specify at least 6 parts in epd/fen {}", fen));
        }
        let mut bb = Self::parse_pieces(words[0])?;
        bb.board.turn = Color::parse(words[1])?;
        //bb.board.castling = CastlingRights::parse(words[1].chars().next().unwrap())?;
        Ok(bb)
    }
}

// impl std::ops::Index<Bitboard> for BoardBuf {
//     type Output = str;
//     fn index(&self, sq: Bitboard) -> &str {
//         self.temporary = self.get(sq);
//         &self.temporary
//     }
// }

// impl  std::ops::IndexMut<Bitboard> for Board {
//     type Output = char;
//     fn index_mut(&mut self, index: Bitboard) -> &mut char {
//     }
// }

impl fmt::Display for Board {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = BoardBuf::adopt(*self);
        for &r in Bitboard::RANKS.iter().rev() {
            fmt.write_str(&b.get(r))?;
            fmt.write_char('\n')?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::globals::constants::*;

    #[allow(non_upper_case_globals)]
    const a1b2: Bitboard = Bitboard::A1.or(Bitboard::B2);

    #[test]
    fn boardbuf() -> Result<(), String> {
        let board = Board::empty();
        assert_eq!(board.kings(), Bitboard::EMPTY);
        assert_eq!(board.us(), Bitboard::EMPTY);
        assert_eq!(board.color_us().is_white, true);

        // assert_eq!(board[a1], 'R');
        let mut buf = BoardBuf::new();
        let board2 = buf
            .set(Bitboard::RANK_2, "PPPPPPPP")?
            .set(a1 | h1, "RR")?
            .set(b1 | g1, "NN")?
            .set(c1 | d1 | e1 | f1, "BQKB")?
            .as_board();
        let board1 = buf.set(Bitboard::RANK_7, "pppppppp")?.set(Bitboard::RANK_8, "rnbqkbnr")?.as_board();
        assert_eq!(buf.get(a1), "R");
        let mut buf2 = BoardBuf::adopt(board2);
        let board2 = buf2.set(Bitboard::RANK_7, "pppppppp")?.set(Bitboard::RANK_8, "rnbqkbnr")?.as_board();
        assert_eq!(board1.to_string(), board2.to_string());
        // let b = hashmap!{ a1+h1 => "R", b1+g1 => "N" };
        // let b = BoardBuf::new().rooks(a1|h1).knights(b1|g1).pawns(rank_2).set("RNBQKBNR", rank_1);
        // let b = BoardBuf::new("rnbqkbnr/
        //     pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
        // let b = BoardBuf::new().k(a1).K(h8).r(a2).R(c3);
        // let b = BoardBuf::new().set(a1=k, rank_2=p, );
        // todo!()
        println!("{}", buf.as_board());
        let fen1 = "1/1/7/8/8/8/PPPPPPPP/RNBQKBNR";
        assert_eq!(BoardBuf::parse_pieces(fen1).err(), Some("Expected 8 ranks of 8 pieces in fen 1/1/7/8/8/8/PPPPPPPP/RNBQKBNR".to_string()));
        assert!(BoardBuf::parse_pieces("8").err().unwrap().starts_with("Expected 8"));
        assert!(BoardBuf::parse_pieces("8/8").err().unwrap().starts_with("Expected 8"));
        assert_eq!(BoardBuf::parse_pieces("X7/8/8/8/8/8/8/8").err(), Some("Unknown piece 'X'".to_string()));
        let buf = BoardBuf::parse_pieces("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        assert_eq!(buf.get(a1), "R");
        assert_eq!(buf.get(Bitboard::FILE_H), "RP....pr");
        Ok(())
    }
}
