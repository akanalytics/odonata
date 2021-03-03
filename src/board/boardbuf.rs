use std::fmt::{self, Write};

use super::{Board,Piece,Color};
use crate::bitboard::Bitboard;



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


}
