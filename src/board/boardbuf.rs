use super::Board;
use crate::bitboard::bitboard::Bitboard;
use crate::bitboard::castling::CastlingRights;
use crate::cache::hasher::Hasher;
use crate::types::{Color, Piece};
use anyhow::{bail, Context, Result};

/// BoardBuf is a slow performing facade of convenience methods on board
// #[derive(Clone)]
// pub struct BoardBuf {
//     board: Board,
// }

pub trait BoardBuf {
    fn set_turn(&mut self, c: Color);
    fn set_castling(&mut self, cr: CastlingRights);
    fn set_en_passant(&mut self, sq: Bitboard);
    fn set_fifty_halfmove_clock(&mut self, hmvc: i32);
    fn set_fullmove_number(&mut self, fmvc: i32);
    fn set_piece_at(&mut self, sq: Bitboard, p: Piece);
    fn set_color_at(&mut self, sq: Bitboard, c: Color);
    fn color_at(&self, at: Bitboard) -> Option<Color>;
    fn get(&self, bb: Bitboard) -> String;
    fn set(&mut self, bb: Bitboard, pieces: &str) -> Result<&mut Self>;
    fn parse_piece_placement(fen: &str) -> Result<Board>;
    fn parse_fen(fen: &str) -> Result<Board>;
    fn as_board(&self) -> Board; // FIXME
    fn validate(&self) -> Result<()>;
}

impl BoardBuf for Board {
    // pub fn new_empty() -> BoardBuf {
    //     BoardBuf { board: Board::new_empty() }
    // }

    fn set_turn(&mut self, c: Color) {
        self.turn = c;
        self.calculate_internals();
    }

    fn set_castling(&mut self, cr: CastlingRights) {
        self.castling = cr;
        self.calculate_internals();
    }

    fn set_en_passant(&mut self, sq: Bitboard) {
        self.en_passant = sq;
        self.calculate_internals();
    }

    fn set_fifty_halfmove_clock(&mut self, hmvc: i32) {
        self.fifty_clock = hmvc as u16;
        self.calculate_internals();
    }

    fn set_fullmove_number(&mut self, fmvc: i32) {
        self.fullmove_number = fmvc as u16;
        self.calculate_internals();
    }

    fn set_piece_at(&mut self, sq: Bitboard, p: Piece) {
        self.multiboard.set_piece_at(sq, p);
        self.calculate_internals();
    }

    fn set_color_at(&mut self, sq: Bitboard, c: Color) {
        self.multiboard.set_color_at(sq, Some(c));
        self.calculate_internals();
    }

    fn color_at(&self, at: Bitboard) -> Option<Color> {
        if self.color(Color::White).contains(at) {
            return Some(Color::White);
        } else if self.color(Color::Black).contains(at) {
            return Some(Color::Black);
        }
        None
    }

    fn get(&self, bb: Bitboard) -> String {
        let mut res = String::new();
        for sq in bb.iter() {
            let p = self.piece_at(sq);
            let ch = match p {
                Piece::None => p.to_upper_char(),
                _ => p.to_char(self.color_at(sq)),
            };
            res.push(ch);
        }
        res
    }

    fn set(&mut self, bb: Bitboard, pieces: &str) -> Result<&mut Self> {
        if bb.popcount() != pieces.chars().count() as i32 {
            bail!("Bitboard {} and pieces {} have different counts", bb, pieces);
        }
        for (sq, ch) in bb.iter().zip(pieces.chars()) {
            let p = Piece::from_char(ch)?;
            self.set_piece_at(sq, p);
            if p != Piece::None {
                let c = Color::from_piece_char(ch)?;
                self.set_color_at(sq, c);
            } else {
                // FIXME: broken approach - null color??
                self.multiboard.set_color_at(sq, None);
            }
        }
        self.calculate_internals();
        Ok(self)
    }

    fn as_board(&self) -> Board {
        self.clone()
    }

    fn validate(&self) -> Result<()> {
        if self.black().intersects(self.white()) {
            bail!("White\n{}\n and black\n{}\n are not disjoint", self.white(), self.black());
        }
        let mut bb = Bitboard::all();
        for &p in Piece::ALL_BAR_NONE.iter() {
            bb &= self.pieces(p);
        }
        if !bb.is_empty() {
            bail!("Piece bitboards are not disjoint");
        }

        // if self.fullmove_counter() < self.fifty_halfmove_clock() * 2 {
        //     bail!("Fullmove number (fmvn: {}) < twice half move clock (hmvc: {})", self.fullmove_counter(), self.fifty_halfmove_clock() );
        // }
        let ep = self.en_passant();
        if !ep.is_empty() {
            if !ep.intersects(Bitboard::RANK_3 | Bitboard::RANK_6) {
                bail!("En passant square must be rank 3 or 6 not {}", ep.sq_as_uci());
            }
            let capture_square = ep.shift(self.color_them().forward());
            if !(self.pawns() & self.them()).contains(capture_square) {
                bail!(
                    "En passant square of {} entails a pawn on square {}",
                    ep.sq_as_uci(),
                    capture_square.sq_as_uci()
                );
            }
        }
        if self.hash() != Hasher::default().hash_board(self) {
            bail!("Hash is incorrect");
        }
        Ok(())
    }

    /// Parses a FEN string to create a board. FEN format is detailed at https://en.wikipedia.org/wiki/Forsyth–Edwards_Notation
    /// terminology of "piece placement data" from http://kirill-kryukov.com/chess/doc/fen.html
    fn parse_piece_placement(fen: &str) -> Result<Self> {
        let mut bb = Board::new_empty();
        let mut pos = String::from(fen);
        for i in 1..=8 {
            pos = pos.replace(i.to_string().as_str(), " ".repeat(i).as_str());
        }
        // pos.retain(|ch| "pPRrNnBbQqKk ".contains(ch));
        let r: Vec<&str> = pos.rsplit('/').collect();
        if r.iter().any(|r| r.chars().count() != 8) || r.len() != 8 {
            bail!("Expected 8 ranks of 8 pieces in fen {}", fen);
        }
        bb.set(Bitboard::all(), &r.concat())?;
        bb.calculate_internals();
        Ok(bb)
    }

    /// 0. Piece placement
    /// 1. Active color
    /// 2. Castling rights
    /// 3. E/P square
    /// 4. Half move clock
    /// 5. Full move counter
    fn parse_fen(fen: &str) -> Result<Self> {
        let words = fen.split_whitespace().collect::<Vec<_>>();
        if words.len() < 6 {
            bail!("Must specify at least 6 parts in epd/fen '{}'", fen);
        }
        let mut bb = Self::parse_piece_placement(words[0])?;
        bb.turn = Color::parse(words[1])?;
        bb.castling = CastlingRights::parse(words[2])?;
        bb.en_passant = if words[3] == "-" {
            Bitboard::EMPTY
        } else {
            Bitboard::parse_square(words[3])?.as_bb()
        };
        bb.fifty_clock = words[4].parse().context(format!("Invalid halfmove clock '{}'", words[4]))?;
        bb.fullmove_number = words[5].parse().context(format!("Invalid fullmove count '{}'", words[5]))?;
        bb.calculate_internals();
        bb.validate()?;
        Ok(bb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::constants::*;

    //
    // interface designs....
    //
    // let b = hashmap!{ a1+h1 => "R", b1+g1 => "N" };
    // let b = BoardBuf::new().rooks(a1|h1).knights(b1|g1).pawns(rank_2).set("RNBQKBNR", rank_1);
    // let b = BoardBuf::new("rnbqkbnr/
    //     pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
    // let b = BoardBuf::new().k(a1).K(h8).r(a2).R(c3);
    // let b = BoardBuf::new().set(a1=k, rank_2=p, );
    // todo!()

    #[test]
    fn boardbuf_sets() -> Result<()> {
        let board = Board::new_empty();
        assert_eq!(board.kings(), Bitboard::EMPTY);
        assert_eq!(board.us(), Bitboard::EMPTY);
        assert_eq!(board.color_us(), Color::White);

        // assert_eq!(board[a1], 'R');
        let mut board1 = Board::new_empty();
        board1 = board1
            .set(Bitboard::RANK_2, "PPPPPPPP")?
            .set(a1 | h1, "RR")?
            .set(b1 | g1, "NN")?
            .set(c1 | d1 | e1 | f1, "BQKB")?
            .as_board();
        board1
            .set(Bitboard::RANK_7, "pppppppp")?
            .set(Bitboard::RANK_8, "rnbqkbnr")?
            .as_board();
        assert_eq!(board1.get(a1), "R");
        let str1 = board1.to_string();
        let mut board2 = board1;
        let board2 = board2
            .set(Bitboard::RANK_7, "pppppppp")?
            .set(Bitboard::RANK_8, "rnbqkbnr")?
            .as_board();
        assert_eq!(str1, board2.to_string());
        println!("{}", board2.as_board());
        Ok(())
    }

    #[test]
    fn parse_piece() -> Result<()> {
        let fen1 = "1/1/7/8/8/8/PPPPPPPP/RNBQKBNR";
        assert_eq!(
            Board::parse_piece_placement(fen1).unwrap_err().to_string(),
            "Expected 8 ranks of 8 pieces in fen 1/1/7/8/8/8/PPPPPPPP/RNBQKBNR"
        );
        assert!(Board::parse_piece_placement("8").unwrap_err().to_string().starts_with("Expected 8"));
        assert!(Board::parse_piece_placement("8/8")
            .unwrap_err()
            .to_string()
            .starts_with("Expected 8"));
        assert_eq!(
            Board::parse_piece_placement("X7/8/8/8/8/8/8/8").unwrap_err().to_string(),
            "Unknown piece 'X'"
        );
        let buf = Board::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        assert_eq!(buf.get(a1), "R");
        assert_eq!(buf.get(Bitboard::FILE_H), "RP....pr");
        Ok(())
    }

    #[test]
    fn parse_fen() -> Result<()> {
        let b = Board::parse_fen("7k/8/8/8/8/8/8/7K b KQkq - 45 100")?.as_board();
        assert_eq!(b.color_us(), Color::Black);
        assert_eq!(b.fullmove_number(), 100);
        assert_eq!(b.fifty_halfmove_clock(), 45);
        assert_eq!(b.castling(), CastlingRights::all());
        Ok(())
    }
    #[test]
    fn parse_invalid_fen() -> Result<()> {
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K B Qkq - 45 100").unwrap_err().to_string(),
            "Invalid color: 'B'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b XQkq - 45 100").unwrap_err().to_string(),
            "Invalid character 'X' in castling rights 'XQkq'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b - - fifty 100").unwrap_err().to_string(),
            "Invalid halfmove clock 'fifty'".to_string()
        );
        assert_eq!(
            Board::parse_fen("7k/8/8/8/8/8/8/7K b - - 50 full").unwrap_err().to_string(),
            "Invalid fullmove count 'full'".to_string()
        );
        Ok(())
    }
}
