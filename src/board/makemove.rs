use crate::bitboard::Bitboard;
use crate::board::{Board, CastlingRights, Color, Piece};
use crate::board::{Move};
use crate::globals::constants::*;

pub trait MoveMaker {
    fn make_move(&self, m: &Move) -> Board;
}

impl MoveMaker for Board {
    fn make_move(&self, m: &Move) -> Board {
        // either we're moving to an empty square or its a capture
        debug_assert!(
            ((self.white() | self.black()) & m.to).is_empty() || m.is_capture(),
            "Move-to sq must be empty or its a capture {} on board \n{}",
            m,
            self
        );
        let mut board = Board {
            en_passant: Bitboard::EMPTY,
            turn: self.turn.opposite(),
            fullmove_count: if self.turn == Color::BLACK { self.fullmove_count + 1 } else { self.fullmove_count },
            fifty_clock: self.fifty_clock + 1,
            moves: self.moves.clone(),
            ..*self
        };

        board.moves.push(*m);

        // clear one bit and set another for the move using xor
        let from_to_bits = m.from | m.to;
        board.pieces[m.mover.index()] ^= from_to_bits;
        board.colors[self.turn.index] ^= from_to_bits;

        if m.is_capture() {
            if m.is_ep_capture() {
                // ep capture is like capture but with capture piece on ep square not dest
                board.fifty_clock = 0;
                board.pieces[m.capture.index()].remove(m.ep);
                board.colors[board.turn.index].remove(m.ep);
            } else {
                // regular capture
                debug_assert!(m.capture != Piece::King, "king captured by move {} on board \n{}", m, self);
                board.fifty_clock = 0;
                board.pieces[m.capture.index()].remove(m.to);
                board.colors[board.turn.index].remove(m.to);
            }
        }

        if m.mover == Piece::Pawn {
            board.fifty_clock = 0;
            if m.is_pawn_double_push() {
                board.en_passant = m.ep;
            } else {
                board.en_passant = Bitboard::EMPTY;
            }
        }

        if m.is_promo() {
            // fifty clock handled by pawn move above;
            board.pieces[Piece::Pawn.index()].remove(m.to); // pawn has already moved
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
                    debug_assert!(board.castling.contains(CastlingRights::WHITE_QUEEN));
                    rook_from_to = a1 | d1;
                }
                g1 => {
                    debug_assert!(board.castling.contains(CastlingRights::WHITE_KING));
                    rook_from_to = h1 | f1;
                }
                c8 => {
                    debug_assert!(board.castling.contains(CastlingRights::BLACK_QUEEN));
                    rook_from_to = a8 | d8;
                }
                g8 => {
                    debug_assert!(board.castling.contains(CastlingRights::BLACK_KING));
                    rook_from_to = h8 | f8;
                }
                _ => panic!(format!("Castling move from square {}", m.to)),
            }
            board.pieces[Piece::Rook.index()] ^= rook_from_to;
            board.colors[self.turn.index] ^= rook_from_to;
        }

        // castling *rights*
        //  if a piece moves TO (=capture) or FROM the rook squares - appropriate castling rights are lost
        //  if a piece moves FROM the kings squares, both castling rights are lost
        //  possible with a rook x rook capture that both sides lose castling rights
        if m.from == e1 {
            board.castling.remove(CastlingRights::WHITE_KING | CastlingRights::WHITE_QUEEN);
        } else if m.from == a1 || m.to == a1 {
            board.castling.remove(CastlingRights::WHITE_QUEEN);
        } else if m.from == h1 || m.to == h1 {
            board.castling.remove(CastlingRights::WHITE_KING);
        }
        if m.from == e8 {
            board.castling.remove(CastlingRights::BLACK_KING | CastlingRights::BLACK_QUEEN);
        } else if m.from == a8 || m.to == a8 {
            board.castling.remove(CastlingRights::BLACK_QUEEN);
        } else if m.from == h8 || m.to == h8 {
            board.castling.remove(CastlingRights::BLACK_KING);
        }
        board
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::board::catalog::*;
    use crate::board::movegen::*;

    #[test]
    fn test_make_move() -> Result<(), String> {
        let board = Catalog::starting_position();
        // let mut m = Move::parse("e2e4")?;
        let mov = board.validate_uci_move("e2e4")?;
        let board2 = board.make_move(&mov);
        assert_eq!(board2.to_fen(), "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        Ok(())
    }

    #[test]
    fn test_try_move_promotion() {
        let mut board = BoardBuf::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'").unwrap().as_board();
        board = board.make_move(&board.validate_uci_move("a7a8q").unwrap());
        assert_eq!(BoardBuf::adopt(board.clone()).get(a8), "Q");
        assert_eq!(BoardBuf::adopt(board).get(a7), ".");
    }

    #[test]
    fn test_castling_rights() {
        // check castling rights parsed-from and returned-in fen
        let epd = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1 id: 'castling1'";
        let board = BoardBuf::parse_fen(epd).unwrap().as_board();
        assert_eq!(board.castling().to_string(), "KQkq");

        // rook takes rook, so both sides lose queens side castling grights
        let board = board.make_move(&board.validate_uci_move("a1a8").unwrap());
        assert_eq!(board.castling().to_string(), "Kk");
    }

    #[test]
    fn test_castling() {
        let epd = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq e3 0 1 id: 'castling1'";
        let board = BoardBuf::parse_fen(epd).unwrap().as_board();
        // casle kings side for w and then b
        let board = board.make_move(&board.validate_uci_move("e1g1").unwrap());
        let board = board.make_move(&board.validate_uci_move("e8g8").unwrap());
        assert_eq!(board.to_fen(), "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R4RK1 w - - 2 2");

        // castle queens side
        let board = BoardBuf::parse_fen(epd).unwrap().as_board();
        let board = board.make_move(&board.validate_uci_move("e1c1").unwrap());
        let board = board.make_move(&board.validate_uci_move("e8c8").unwrap());
        assert_eq!(board.to_fen(), "2kr3r/pppppppp/8/8/8/8/PPPPPPPP/2KR3R w - - 2 2");

        // rook moves queens side for w and then b, losing q-side castling rights
        let board = BoardBuf::parse_fen(epd).unwrap().as_board();
        let board = board.make_move(&board.validate_uci_move("a1b1").unwrap());
        let board = board.make_move(&board.validate_uci_move("a8b8").unwrap());
        assert_eq!(board.to_fen(), "1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R w Kk - 2 2");
    }
}
