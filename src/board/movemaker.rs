use crate::bitboard::Bitboard;
use crate::board::{Board, CastlingRights, Color, Piece};
use crate::board::{Move, MoveEnum};
use crate::globals::constants::*;








trait MoveValidator {
    fn validate_uci_move(&self, mv: &str) -> Result<Move, String>;
    fn validate_san_move(&self, mv: &str) -> Result<Move, String>;
}


impl MoveValidator for Board {
    fn validate_uci_move(&self, mv: &str ) -> Result<Move, String> {
        // FIXME! *legal* moves
        let mut moves = self.pseudo_legal_moves();
        if let Some(pos) = moves.iter().position(|m| m.uci() == mv) {
            return Ok(moves.remove(pos));
        }
        Err(format!("Move {} is not legal", mv))
    }


    fn validate_san_move(&self, mv: &str) -> Result<Move, String> {
        Err("Not implemented".into())
    }
}






trait MoveMaker {
    fn make_move(&self, m: Move) -> Board;
}

impl MoveMaker for Board {
    fn make_move(&self, m: Move) -> Board {
        let mut board = Board {
            en_passant: Bitboard::EMPTY,
            turn: self.turn.opposite(),
            fullmove_count: if self.turn == Color::BLACK { self.fullmove_count + 1 } else { self.fullmove_count },
            fifty_clock: self.fifty_clock + 1,
            ..*self
        };

        // clear one bit and set another for the move using xor
        let from_to_bits = m.from | m.to;
        board.pieces[m.mover.index()] ^= from_to_bits;
        board.colors[self.turn.index] ^= from_to_bits;

        if m.mover == Piece::Pawn {
            board.fifty_clock = 0;
            // ep + !capture means its a double push and set the en_passant square
            if !m.ep.is_empty() {
                if !m.is_capture() {
                    board.en_passant = m.ep;
                } else {
                    board.en_passant = Bitboard::EMPTY;
                }
            }
        }

        if !m.ep.is_empty() {
            // ep capture is like capture but with capture piece on ep square not dest
            board.fifty_clock = 0;
            board.pieces[m.capture.index()].remove(m.ep);
            board.colors[board.turn.index].remove(m.ep);
        } else if m.capture != Piece::None {
            board.fifty_clock = 0;
            board.pieces[m.capture.index()].remove(m.to);
            board.colors[board.turn.index].remove(m.to);
        }

        if m.promo != Piece::None {
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
    use crate::board::*;
    use crate::globals::constants::*;

    #[test]
    fn test_make_move() -> Result<(), String> {
        let board = Catalog::starting_position();
        // let mut m = Move::parse("e2e4")?;
        let mov = board.validate_uci_move("e2e4")?;
        let board2 = board.make_move(mov);
        assert_eq!(board2.to_fen(), "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        Ok(())
    }
}
//     # test operators + and ==
//     b2 = cls()
//     b2._init()
//     b2 = b2.clone().make_move(Parser().parse_move("e2e4"))
//     assert b2 == b
//     b = cls()
//     b._init()
//     b = b.clone().make_move(Move.parse('a2a4'))
//     assert b[a4] == 'P' and b[a2] == ''
// }
/*
    fn test_try_move_promotion() {}
        b = Parser().parse_board_epd("8/P7/8/8/8/8/7k/K7 w - - id 'promos #1'")
        b = b.clone().make_move( Parser().parse_move('a7a8q'))
        assert b[a8] == 'Q' and b[a7] == ''
    }
    @pytest.mark.parametrize("cls", [BoardOfBits])
    fn test_castling_rights() {
        epd = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1 id: 'castling1'"
        b1 = Parser(cls).parse_board_epd(epd)
        assert b1.castling_rights == "KQkq"

        # rook takes rook, so both sides lose queens side castling grights
        b2 = b1.clone().make_move(Move.parse('a1a8'))
        assert b2.castling_rights == "Kk"
    }

    fn test_castling() {
        epd = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq e3 0 1 id: 'castling1'"
        # Kings side
        b = Parser().parse_board_epd(epd)
        b = BoardMailbox(b)
        assert b.get_castling_side(Parser().parse_move('e1g1')) == "K"
        b.try_move( Parser().parse_move('e1g1'))
        b.try_move( Parser().parse_move('e8g8')) # casle kings side for w and then b
        assert b.to_fen() == "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R4RK1 w - - 2 2"

        # Queens side
        b = Parser().parse_board_epd(epd)
        b = BoardMailbox(b)
        assert b.get_castling_side(Parser().parse_move('e1c1')) == "Q"
        b.try_move( Parser().parse_move('e1c1'))
        b.try_move( Parser().parse_move('e8c8')) # casle queens side for w and then b
        assert b.to_fen() == "2kr3r/pppppppp/8/8/8/8/PPPPPPPP/2KR3R w - - 2 2"

        # Queens side rook moves
        b = Parser().parse_board_epd(epd)
        b = BoardMailbox(b)
        assert b.get_castling_side(Parser().parse_move('e1c1')) == "Q"
        b.try_move( Parser().parse_move('a1b1'))
        b.try_move( Parser().parse_move('a8b8')) # rook moves queens side for w and then b
        assert b.to_fen() == "1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R w Kk - 2 2"
    }
*/
