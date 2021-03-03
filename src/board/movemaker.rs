use crate::bitboard::{Bitboard};
use crate::board::{Board, MoveEnum, Move, CastlingRights, Color, Piece};
use crate::globals::constants::*;



// #[derive(Debug, Default)]
// pub struct Move {
//     from: Bitboard,
//     to: Bitboard,
//     promo: Piece,
//     capture: Piece,

//     is_castle: bool,
//     is_en_passant: bool,
//     is_promo: bool,
//     is_null: bool,
//     is_drop: bool,
// }


impl Board {


    pub fn make_move(&self, m: Move) -> Board {
        let mut board = Board {
            en_passant: Bitboard::EMPTY,
            turn: self.turn.opposite(),
            move_count: if self.turn == Color::BLACK { self.move_count + 1 } else { self.move_count },
            fifty_clock: self.fifty_clock + 1,
            .. *self
        };

        // clear one bit and set another for the move using xor
        let from_to_bits = m.from | m.to;
        board.pieces[m.mover.index()] ^= from_to_bits;
        board.colors[self.turn.index] ^= from_to_bits;

        if m.mover == Piece::Pawn {
            board.fifty_clock = 0;
            // set en_passant flag
            if m.ep != Bitboard::EMPTY {
                // m.to.shift()
            }
        } 

        if m.ep != Bitboard::EMPTY {
            // ep capture is like capture but with capture piece on ep square not dest  
            board.fifty_clock = 0;
            board.pieces[m.capture.index()].remove(m.ep);
            board.colors[board.turn.index].remove(m.ep);
        } 
        else if m.capture != Piece::None {
            board.fifty_clock = 0;
            board.pieces[m.capture.index()].remove(m.to);
            board.colors[board.turn.index].remove(m.to);
        }

        if m.promo != Piece::None {
            // fifty clock handled by pawn move above;
            board.pieces[Piece::Pawn.index()].remove(m.to);  // pawn has already moved
            board.pieces[m.promo.index()].insert(m.to);
        }

        // castling *moves*
        if m.is_castle {
            // rules say no reset of fifty clock
            // king move already handled, castling rights handled below, just the rook move
            let rook_from_to; 

            #[allow(non_upper_case_globals)]
            match m.to {
                c1 => {
                    debug_assert!(board.castling.contains(CastlingRights::WHITE_KING));
                    rook_from_to = a1 | d1;
                } 
                g1 => {
                    debug_assert!(board.castling.contains(CastlingRights::WHITE_QUEEN));
                    rook_from_to = h1 | f1;
                }
                c8 => {
                    debug_assert!(board.castling.contains(CastlingRights::BLACK_KING));
                    rook_from_to = a8 | d8;
                } 
                g8 => {
                    debug_assert!(board.castling.contains(CastlingRights::BLACK_QUEEN));
                    rook_from_to = h8 | f8;
                }
                _ => panic!(format!("Castling move from square {}", m.to))
            }
            board.pieces[Piece::Rook.index()] ^= rook_from_to;
            board.colors[self.turn.index] ^= rook_from_to;
        }

        
        // castling *rights*
        //  if a piece moves TO (=capture) or FROM the rook squares - appropriate castling rights are lost
        //  if a piece moves FROM the kings squares, both castling rights are lost
        //  possible with a rook x rook capture that both sides lose castling rights
        if m.from == e1 {
            board.castling.remove( CastlingRights::WHITE_KING | CastlingRights::WHITE_QUEEN);
        }
        else if m.from == a1 || m.to == a1 {
            board.castling.remove(CastlingRights::WHITE_QUEEN);
        }
        else if m.from == h1 || m.to == h1 {
            board.castling.remove(CastlingRights::WHITE_KING);
        }
        
        if m.from == e8 {
            board.castling.remove( CastlingRights::BLACK_KING | CastlingRights::BLACK_QUEEN);
        }
        else if m.from == a8 || m.to == a8 {
            board.castling.remove( CastlingRights::BLACK_QUEEN);
        }
        else if m.from == h8 || m.to == h8 {
            board.castling.remove( CastlingRights::BLACK_KING);
        }
        board
    }






    pub fn make_move_enum(&self, m: MoveEnum) -> Board {
        let mut board = Board {
            en_passant: Bitboard::EMPTY,
            turn: self.turn.opposite(),
            move_count: if self.turn == Color::BLACK { self.move_count + 1 } else { self.move_count },
            fifty_clock: self.fifty_clock + 1,
            .. *self
        };
        match m {
            MoveEnum::Quiet{ dest, src, mover } => {
                // clear one bit and set another for the move
                let from_to_bits = dest ^ src;
                board.pieces[mover.index()] ^= from_to_bits;
                board.colors[self.turn.index] ^= from_to_bits;
            },
            MoveEnum::Push { dest, src } => {
                board.fifty_clock = 0;
                let from_to_bits = dest ^ src;
                board.pieces[Piece::Pawn.index()] ^= from_to_bits;
                board.colors[self.turn.index] ^= from_to_bits;
            }
            MoveEnum::Null() => {
            },
            MoveEnum::Capture { dest, src, mover, capture } => {
                board.fifty_clock = 0;
                board.pieces[capture.index()].remove(dest);
                board.colors[board.turn.index].remove(dest);

                let from_to_bits = dest ^ src;
                board.pieces[mover.index()] ^= from_to_bits;
                board.colors[self.turn.index] ^= from_to_bits;
            },
            MoveEnum::Promo { dest, src, promo } => {
                board.fifty_clock = 0;
                let from_to_bits = dest ^ src;
                board.pieces[Piece::Pawn.index()].remove(src);
                board.pieces[promo.index()].insert(dest);
                board.colors[self.turn.index] ^= from_to_bits;
            },
            MoveEnum::PromoCapture { dest, src, promo, capture } => {
                board.fifty_clock = 0;
                board.pieces[capture.index()].remove(dest);
                board.colors[board.turn.index].remove(dest);

                let from_to_bits = dest ^ src;
                board.pieces[Piece::Pawn.index()].remove(src);
                board.pieces[promo.index()].insert(dest);
                board.colors[self.turn.index] ^= from_to_bits;
            },
            MoveEnum::EnPassant { dest, src, capture_sq } => {
                board.fifty_clock = 0;
                board.pieces[Piece::Pawn.index()].remove(capture_sq);
                board.colors[board.turn.index].remove(capture_sq);

                let from_to_bits = dest ^ src;
                board.pieces[Piece::Pawn.index()] ^= from_to_bits;
                board.colors[self.turn.index] ^= from_to_bits;
            }
            MoveEnum::Castle { king_dest, king_src, rook_dest, rook_src, right } => {
                // NO reset of fifty clock. MoveEnum king then move rook.
                board.castling.remove(right);
                let from_to_bits = king_dest ^ king_src;
                board.pieces[Piece::King.index()] ^= from_to_bits;
                board.colors[self.turn.index] ^= from_to_bits;

                let from_to_bits = rook_dest ^ rook_src;
                board.pieces[Piece::King.index()] ^= from_to_bits;
                board.colors[self.turn.index] ^= from_to_bits;
            }
 

        }
        board
    }
}
