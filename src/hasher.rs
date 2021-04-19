use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use crate::types::{Color, Piece, CastlingRights};
use crate::board::Board;
use crate::board::boardbuf::BoardBuf;
use crate::board::movegen::MoveGen;
use crate::board::makemove::MoveMaker;
use crate::movelist::Move;
use crate::globals::constants::*;
use std::fmt;

// CPW:
// One number for each piece at each square
// One number to indicate the side to move is black
// Four numbers to indicate the castling rights, though usually 16 (2^4) are used for speed
// Eight numbers to indicate the file of a valid En passant square, if any
//
// https://web.archive.org/web/20071031100138/http://www.brucemo.com/compchess/programming/zobrist.htm
//
#[derive(Debug, Eq, PartialEq)]
pub struct Hasher {
    seed: u64,
    piece_squares: [[[u64; 64]; 6]; 2],  // [colour][piece][square]
    side: u64,
    castling: [u64; 4],
    ep: [u64; 8],
}


impl fmt::Display for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Zobrist({})", self.seed)?;
        if f.alternate() {
            writeln!(f)?;
            for c in &Color::ALL {
                for p in &Piece::ALL {
                    for sq in 0..64 {
                        writeln!(f, "[{}][{}][{:2}] = {:x}", c, p, sq, self.piece_squares[c.index()][p.index()][sq])?;
                    }
                }
            }
            writeln!(f, "side = {:x}", self.side)?;
            for cr in CastlingRights::iter() {
                writeln!(f, "castling[{}] = {:x}", cr, self.castling[cr.index()])?;
            }
            for sq in 0..self.ep.len() {
                writeln!(f, "ep[{}] = {:x}", sq, self.ep[sq])?;
            }
        }
        Ok(())
    }
}

// https://docs.rs/rand/0.8.3/rand/rngs/struct.StdRng.html
// For a secure reproducible generator, we recommend use of the rand_chacha crate directly.
// hence https://crates.io/crates/rand_chacha
//
// I think the motivation is to ensure cross platform reproducibility - which I want
//
impl Hasher {
    pub fn new(seed: u64) -> Self {
        let mut rng = ChaChaRng::seed_from_u64(seed);
        let mut h = Hasher {
            seed,
            piece_squares: [[[0;64];6];2],
            side: 0,
            castling: [0;4],
            ep: [0;8],
        };
        // let i = rng.gen::<u64>();
        
        // fill seems fine to use "On big-endian platforms this performs 
        // byte-swapping to ensure portability of results from reproducible generators."
        for c in &Color::ALL {
            for p in &Piece::ALL {
                rng.fill(&mut h.piece_squares[c.index()][p.index()]);
            }
        }
        h.side = rng.gen();
        rng.fill(&mut h.castling);
        rng.fill(&mut h.ep);
        h
    }

    pub fn hash_board(&self, b: &Board) -> u64 {
        let mut hash = b.color_us().chooser_wb(0, self.side);
        for cr in CastlingRights::iter() {
            if b.castling().contains(*cr) {
                hash ^= self.castling[cr.index()];
            }
        }
        if !b.en_passant().is_empty() {
            hash ^= self.ep[(b.en_passant().bits() & 7) as usize];
        }    
        for p in Piece::ALL.iter() {
            for bb in b.pieces(*p).iter() {
                let sq = bb.first_square();
                if b.color(Color::White).contains(bb) {
                    hash ^= self.piece_squares[Color::White][p.index()][sq];
                } else {
                hash ^= self.piece_squares[Color::Black][p.index()][sq];
                }
            }
        }
        hash
    }


    pub fn hash_move(&self, m: &Move, pre_move: &Board) -> u64 {
        // either we're moving to an empty square or its a capture
        let us = pre_move.color_us();
        let them = pre_move.color_them();
        let mut hash = self.side;
        if !pre_move.en_passant().is_empty() {
            hash ^= self.ep[pre_move.en_passant().last_square()];
        }

        if m.is_capture() {
            if m.is_ep_capture() {
                // ep capture is like capture but with capture piece on *ep* square not *dest*
                hash ^=self.piece_squares[them][m.capture_piece()][m.ep().last_square()];
            } else {
                // regular capture
                hash ^=self.piece_squares[them][m.capture_piece()][m.to().last_square()];
            }
        }

        hash ^=self.piece_squares[us][m.mover_piece()][m.from().last_square()];
        hash ^=self.piece_squares[them][m.mover_piece()][m.to().last_square()];

        if m.mover_piece() == Piece::Pawn && m.is_pawn_double_push() {
            hash ^= self.ep[(m.ep().bits() & 7) as usize];
        }

        if m.is_promo() {
            hash ^=self.piece_squares[us][Piece::Pawn][m.from().last_square()];
            hash ^=self.piece_squares[us][m.promo_piece()][m.to().last_square()];
        }

        // castling *moves*
        if m.is_castle() {
            let rook_from;
            let rook_to;

            #[allow(non_upper_case_globals)]
            match m.to() {
                c1 => {
                    rook_from = a1;
                    rook_to= d1;
                }
                g1 => {
                    rook_from = h1;
                    rook_to =f1;
                }
                c8 => {
                    rook_from=  a8;
                    rook_to =d8;
                }
                g8 => {
                    rook_from= h8;
                    rook_to= f8;
                }
                _ => panic!("Castling move from square {}", m.to()),
            }
            hash ^=self.piece_squares[us][Piece::Rook][rook_from.last_square()];
            hash ^=self.piece_squares[us][Piece::Rook][rook_to.last_square()];
        }
        hash
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn test_hasher_display() {
        // println!("{:?}", Hasher::new(1));
        println!("{}", Hasher::new(1));
        println!("{:#}", Hasher::new(1));
        assert_eq!( format!("{:x}",Hasher::new(1).ep[7]), "fc31329afb42f9a7");
    }

    #[test]
    fn test_hash_board() {
        let hasher = Hasher::new(1);
        let b = Catalog::starting_position();
        assert_eq!(format!("{:x}", hasher.hash_board(&b)), "5deb2bf6a1e5765");
    }

    #[test]
    fn test_hash_move() {
        let hasher = Hasher::new(1);
        let bd1 = Catalog::starting_position();
        let moves = bd1.legal_moves(); 
        let hash_bd1 = hasher.hash_board(&bd1);
        for mv in moves.iter() {
            let hash_mv = hasher.hash_move(mv, &bd1);
            let hash_bd2 = hasher.hash_board(&bd1.make_move(mv));
            println!("Move: {}", mv);
            assert_eq!(hash_bd1 ^ hash_mv, hash_bd2);
        }
    }
}