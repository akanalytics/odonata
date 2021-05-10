use crate::bitboard::Bitboard;
use crate::board::Board;
use crate::globals::constants::*;
use crate::hasher::Hasher;
use crate::movelist::{Move, MoveExt};
use crate::types::{Piece};
use crate::castling::CastlingRights;

use std::cell::Cell;

pub trait MoveMaker {
    fn make_move_ext(&mut self, mv: &MoveExt);
    fn undo_move_ext(&mut self, mv: &MoveExt);

    fn make_move(&self, m: &Move) -> Board;
    fn undo_move(&self, m: &Move);
}

impl MoveMaker for Board {


    fn make_move_ext(&mut self, mv: &MoveExt) {
        let them = self.turn.opposite();
        self.pieces[mv.p1.index()] ^= mv.f1 ^ mv.t1;
        self.pieces[mv.p2.index()] ^= mv.f2;
        self.pieces[mv.p3.index()] ^= mv.t3;
        self.pieces[mv.p4.index()] ^= mv.f4;
        self.threats_to[0].set(Bitboard::niche());
        self.threats_to[1].set(Bitboard::niche());
        self.checkers_of[0].set(Bitboard::niche());
        self.checkers_of[1].set(Bitboard::niche());
        self.repetition_count.set(0);

        self.fifty_clock += 1;
        if mv.p1 == Piece::Pawn || mv.is_capture() {
            self.fifty_clock = 0;
        }
        self.colors[self.turn] ^= mv.f1 ^ mv.t1 ^ mv.t3 ^ mv.f4;
        self.colors[them] ^= mv.f2;
        
        // self.castling ^= mv.castle;

        // self.castling -= Bitboard::ROOK_AND_KING_SQS & (mv.f1 | mv.t3);

        self.fullmove_number += self.turn.chooser_wb(0, 1);
        self.turn = them;

        //self.hash ^= Hasher::default().hash_move(mv, self);
        // debug_assert!(
        //     self.hash == Hasher::default().hash_board(self),
        //     "make_move_ext({:?}) inconsistent incremental hash {:x} (should be {:x}",
        //     mv, 
        //     self.hash,
        //     Hasher::default().hash_board(self),
        // );

    }

    fn undo_move_ext(&mut self, _mv: &MoveExt) {
        // *self.pieces_mut(mv.p1) ^= mv.f1 ^ mv.t1;
        // *self.pieces_mut(mv.p2) ^= mv.f2;
        // *self.pieces_mut(mv.p3) ^= mv.t3;
        // self.turn = self.turn.opposite();
    }




    fn make_move(&self, m: &Move) -> Board {
        // either we're moving to an empty square or its a capture
        debug_assert!(
            ((self.white() | self.black()) & m.to()).is_empty() || m.is_capture(),
            "Non-empty to:sq for non-capture {:?} board \n{} white \n{} black\n{}",
            m,
            self,
            self.white(),
            self.black()
        );
        let mut b = Board {
            en_passant: Bitboard::EMPTY,
            turn: self.turn.opposite(),
            fullmove_number: self.fullmove_number + self.turn.chooser_wb(0, 1),
            fifty_clock: self.fifty_clock + 1,
            repetition_count: Cell::new(0),
            threats_to: [Cell::<_>::new(Bitboard::niche()), Cell::<_>::new(Bitboard::niche())],
            checkers_of: [Cell::<_>::new(Bitboard::niche()), Cell::<_>::new(Bitboard::niche())],
            // moves: self.moves.clone(),
            ..*self
        };

        // board.moves.push(*m);

        if m.is_capture() {
            if m.is_ep_capture() {
                // ep capture is like capture but with capture piece on *ep* square not *dest*
                b.fifty_clock = 0;
                b.pieces[m.capture_piece()].remove(m.ep());
                b.colors[b.turn].remove(m.ep());
            } else {
                // regular capture
                debug_assert!(
                    m.capture_piece() != Piece::King,
                    "king captured by move {} on board \n{}",
                    m,
                    self
                );
                b.fifty_clock = 0;
                b.pieces[m.capture_piece()].remove(m.to());
                b.colors[b.turn].remove(m.to());
            }
        }

        // clear one bit and set another for the move using xor
        let from_to_bits = m.from() | m.to();
        b.pieces[m.mover_piece()] ^= from_to_bits;
        b.colors[self.turn] ^= from_to_bits;

        if m.mover_piece() == Piece::Pawn {
            b.fifty_clock = 0;
            if m.is_pawn_double_push() {
                b.en_passant = m.ep();
            }
        }

        if m.is_promo() {
            // fifty clock handled by pawn move above;
            b.pieces[Piece::Pawn].remove(m.to()); // pawn has already moved
            b.pieces[m.promo_piece()].insert(m.to());
        }

        // castling *moves*
        if m.is_castle() {
            // rules say no reset of fifty clock
            // king move already handled, castling rights handled below, just the rook move
            let rook_from_to;

            #[allow(non_upper_case_globals)]
            match m.to() {
                c1 => {
                    debug_assert!(b.castling.contains(CastlingRights::WHITE_QUEEN));
                    rook_from_to = a1 | d1;
                }
                g1 => {
                    debug_assert!(b.castling.contains(CastlingRights::WHITE_KING));
                    rook_from_to = h1 | f1;
                }
                c8 => {
                    debug_assert!(b.castling.contains(CastlingRights::BLACK_QUEEN));
                    rook_from_to = a8 | d8;
                }
                g8 => {
                    debug_assert!(b.castling.contains(CastlingRights::BLACK_KING));
                    rook_from_to = h8 | f8;
                }
                _ => panic!("Castling move from square {}", m.to()),
            }
            b.pieces[Piece::Rook] ^= rook_from_to;
            b.colors[self.turn] ^= rook_from_to;
        }

        // castling *rights*
        //  if a piece moves TO (=capture) or FROM the rook squares - appropriate castling rights are lost
        //  if a piece moves FROM the kings squares, both castling rights are lost
        //  possible with a rook x rook capture that both sides lose castling rights
        let squares_changing = m.to() | m.from();
        b.castling.adjust( squares_changing);

        // if m.from() == e1 {
        //     b.castling.remove(CastlingRights::WHITE_KING | CastlingRights::WHITE_QUEEN);
        // } else if m.from() == a1 || m.to() == a1 {
        //     b.castling.remove(CastlingRights::WHITE_QUEEN);
        // } else if m.from() == h1 || m.to() == h1 {
        //     b.castling.remove(CastlingRights::WHITE_KING);
        // }
        // if m.from() == e8 {
        //     b.castling.remove(CastlingRights::BLACK_KING | CastlingRights::BLACK_QUEEN);
        // } else if m.from() == a8 || m.to() == a8 {
        //     b.castling.remove(CastlingRights::BLACK_QUEEN);
        // } else if m.from() == h8 || m.to() == h8 {
        //     b.castling.remove(CastlingRights::BLACK_KING);
        // }

        let move_hash = Hasher::default().hash_move(m, self);
        b.hash = self.hash ^ move_hash;
        debug_assert!(
            b.hash == Hasher::default().hash_board(&b),
            "\n{}.make_move({}) = \n{} inconsistent incremental hash {:x} (should be {:x}",
            self,
            m, 
            b,
            b.hash,
            Hasher::default().hash_board(&b),
        );
        b
    }

    fn undo_move(&self, _m: &Move) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::BoardBuf;
    use crate::board::*;
    use crate::catalog::*;
    use crate::movelist::MoveValidator;

    #[test]
    fn test_make_move() -> Result<(), String> {
        let board = Catalog::starting_position();
        // let mut m = Move::parse("e2e4")?;
        let mov = board.parse_uci_move("e2e4")?;
        assert_eq!(board.total_halfmoves(), 0);

        let board = board.make_move(&mov);
        assert_eq!(board.to_fen(), "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        assert_eq!(board.total_halfmoves(), 1);

        let board = board.make_move(&board.parse_uci_move("a7a6").unwrap());
        assert_eq!(board.total_halfmoves(), 2);

        let board = board.make_move(&board.parse_uci_move("e4e5").unwrap());
        assert_eq!(board.total_halfmoves(), 3);

        let board = board.make_move(&board.parse_uci_move("a6a5").unwrap());
        assert_eq!(board.total_halfmoves(), 4);
        Ok(())
    }

    #[test]
    fn make_move_ep() -> Result<(), String> {
        let board1 = Catalog::perft_cpw_number3().0.clone();
        // double push - ep sq should be set
        let mov1 = board1.parse_uci_move("e2e4")?;
        let board2 = board1.make_move(&mov1);
        assert_eq!(board2.to_fen(), "8/2p5/3p4/KP5r/1R2Pp1k/8/6P1/8 b - e3 0 1");
        // ep capture is not valid as leaves king in check
        assert!(board2.parse_uci_move("f4e3").is_err());
        // , e4, "EP square for e/p capture move is square the captured piece is on");
        Ok(())
    }

    #[test]
    fn test_try_move_promotion() {
        let mut board = Board::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'").unwrap().as_board();
        board = board.make_move(&board.parse_uci_move("a7a8q").unwrap());
        assert_eq!(board.get(a8), "Q");
        assert_eq!(board.get(a7), ".");
    }

    #[test]
    fn test_castling_rights() {
        // check castling rights parsed-from and returned-in fen
        let epd = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1 id: 'castling1'";
        let board = Board::parse_fen(epd).unwrap().as_board();
        board.validate().unwrap();
        assert_eq!(board.castling().to_string(), "KQkq");

        // rook takes rook, so both sides lose queens side castling grights
        let mv = board.parse_uci_move("a1a8").unwrap();
        let board = board.make_move(&mv);
        assert_eq!(board.castling().to_string(), "Kk");
    }

    #[test]
    fn test_castling() {
        let epd = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1 id: 'castling1'";
        let board = Board::parse_fen(epd).unwrap().as_board();
        // casle kings side for w and then b
        assert!(!board.is_in_check(board.color_us()));
        assert!(board.checkers_of(board.color_us()).is_empty());
        assert_eq!(board.total_halfmoves(), 0);
        assert_eq!(board.legal_moves().len(), 16+5+2+2); // 16P, 5R, 2K, OO, OOO 

        let board = board.make_move(&board.parse_uci_move("e1g1").unwrap());
        assert_eq!(board.total_halfmoves(), 1);

        let board = board.make_move(&board.parse_uci_move("e8g8").unwrap());
        assert_eq!(board.to_fen(), "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R4RK1 w - - 2 2");
        assert_eq!(board.total_halfmoves(), 2);
        
        // castle queens side
        let board = Board::parse_fen(epd).unwrap().as_board();
        let board = board.make_move(&board.parse_uci_move("e1c1").unwrap());
        let board = board.make_move(&board.parse_uci_move("e8c8").unwrap());
        assert_eq!(board.to_fen(), "2kr3r/pppppppp/8/8/8/8/PPPPPPPP/2KR3R w - - 2 2");

        // rook moves queens side for w and then b, losing q-side castling rights
        let board = Board::parse_fen(epd).unwrap().as_board();
        let board = board.make_move(&board.parse_uci_move("a1b1").unwrap());
        let board = board.make_move(&board.parse_uci_move("a8b8").unwrap());
        assert_eq!(board.to_fen(), "1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R w Kk - 2 2");
    }
}
