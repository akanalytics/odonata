use crate::bits::bitboard::Bitboard;
use crate::boards::Board;
use crate::cache::hasher::Hasher;
use crate::infra::black_box;
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::piece::{Piece, Repeats};
use crate::search::node::{Counter, Timing};
use crate::variation::Variation;

use std::cell::Cell;


impl Board {

    pub fn make_moves_old(&self, var: &Variation) -> Board {
        let mut b = self.clone();
        for mv in var.moves() {
            b = b.make_move(mv);
        }
        b
    }

    pub fn make_moves(&self, var: &Variation) -> Board {
        let mut b = self.clone();
        for mv in var.moves() {
            debug_assert!(
                b.validate().is_ok(),
                "Invalid board before move {mv} from {var} on board {b} (base board {self:#})"
            );
            debug_assert!(
                b.is_legal_move(mv),
                "Move {mv} from {var} on board {b} (base board {self:#}) is invalid"
            );
            b = b.make_move(mv);
        }
        b
    }

    pub fn make_move(&self, m: Move) -> Board {
        Metrics::incr(Counter::MakeMove);
        let t = Metrics::timing_start();
        // either we're moving to an empty square or its a capture
        debug_assert!(
            m.is_null()
                || ((self.white() | self.black()) & m.to().as_bb()).is_empty()
                || m.is_capture(),
            "Non-empty to:sq for non-capture {:?} board \n{} white \n{} black\n{}",
            m,
            self,
            self.white(),
            self.black()
        );

        let mut b = Board {
            en_passant: self.en_passant(),
            turn: self.turn,
            fullmove_number: self.fullmove_number + self.turn.chooser_wb(0, 1),
            half_move_clock: self.half_move_clock + 1,
            repetition_count: Cell::new(Repeats::default()),
            threats_to: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            checkers_of: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            pinned: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            discoverer: [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ],
            // material: Cell::<_>::new(self.material()),
            // moves: self.moves.clone(),
            pieces: self.pieces.clone(),
            colors: self.colors.clone(),
            castling: self.castling,
            hash: self.hash,
            ply: self.ply,
        };
        debug_assert!(
            self.validate().is_ok(),
            "board {self} failed validation before move {m} with '{}'\n{self:#}",
            self.validate().unwrap_err()
        );

        #[cfg(debug_assertions)]
        let old_board = self.clone();

        #[cfg(debug_assertions)]
        let move_details = format!(
            "Move:\ncapture_square: {cs}\nis_capture:{ic}\nis_ep_capture:{ep}\ncapture_piece:{cp:?}\n",
            // san = m.to_san(&b),
            cs = m.capture_square(&b),
            ic = m.is_capture(),
            cp = m.capture_piece(&b),
            ep = m.is_ep_capture(&b)
        );

        b.apply_move(m);
        #[cfg(debug_assertions)]
        debug_assert!(
            b.validate().is_ok(),
            "board {b} failed validation after move {m} with '{}'\n{b:#}\n{old_board:#}\n{move_details}",
            b.validate().unwrap_err()
        );

        Metrics::profile(t, Timing::TimingMakeMove);

        debug_assert!(
            b.hash == Hasher::default().hash_board(&b),
            "\n{self}.make_move({m}) = \n{b} inconsistent incremental hash {:x} (should be {:x})",
            b.hash,
            Hasher::default().hash_board(&b),
        );

        b
    }

    #[inline]
    pub fn apply_move(&mut self, m: Move) {
        let mut b = self;
        let move_hash = Hasher::default().hash_move(m, b);
        b.hash = b.hash ^ move_hash;
        // now hash calculated - we can adjust these
        b.turn = b.turn.opposite();
        if m.is_null() {
            b.en_passant = Bitboard::EMPTY;
            return;
        }

        let mover = m.mover_piece(b);

        let mut en_passant = Bitboard::EMPTY;
        if mover == Piece::Pawn {
            b.half_move_clock = 0;
            if m.is_pawn_double_push(b) {
                en_passant = m.ep().as_bb();
            }
        }

        if let Some(c) = m.capture_piece(b) {
            b.half_move_clock = 0;
            if m.is_ep_capture(b) {
                // ep capture is like capture but with capture piece on *ep* square not *dest*
                b.remove_piece(m.capture_square(b).as_bb(), c, b.turn);
            } else {
                // regular capture
                debug_assert!(c != Piece::King, "king captured by move {m} on board \n{b}");
                b.remove_piece(m.to().as_bb(), c, b.turn);
            }
        }

        // safely set e/p flag now that weve handled the capture
        b.en_passant = en_passant;

        // castling *moves*
        if m.is_castle(b) {
            // rules say no reset of fifty clock
            // king move already handled, castling rights handled below, just the rook move

            let (rook_from, rook_to) = m.rook_move_from_to();
            // let rook_from_to = rook_from.as_bb() ^ rook_to.as_bb();
            b.move_piece(
                rook_from.as_bb(),
                rook_to.as_bb(),
                Piece::Rook,
                b.turn.opposite(),
            )
        }

        // clear one bit and set another for the move using xor
        if !m.is_null() {
            // let from_to_bits = m.from().as_bb() | m.to().as_bb();
            b.move_piece(m.from().as_bb(), m.to().as_bb(), mover, b.turn.opposite());
        }

        if let Some(promo) = m.promo() {
            // fifty clock handled by pawn move above;
            b.change_piece(m.to().as_bb(), Piece::Pawn, promo);
            // pawn has already moved
        }

        // castling *rights*
        //  if a piece moves TO (=capture) or FROM the rook squares - appropriate castling rights are lost
        //  if a piece moves FROM the kings squares, both castling rights are lost
        //  possible with a rook x rook capture that both sides lose castling rights
        b.castling -= m.castling_rights_lost();
        // b.castling ^= m.castling_side();
    }

    pub fn undo_move(&mut self, m: Move) {
        if true {
            let mut b = self;
            let move_hash = Hasher::default().hash_move(m, b);
            b.hash = b.hash ^ move_hash;

            // now hash calculated - we can adjust these
            b.turn = b.turn.opposite();
            b.en_passant = Bitboard::EMPTY;

            if let Some(c) = m.capture_piece(b) {
                b.half_move_clock = 0;
                if m.is_ep_capture(b) {
                    // ep capture is like capture but with capture piece on *ep* square not *dest*
                    b.remove_piece(m.capture_square(b).as_bb(), c, b.turn);
                } else {
                    // regular capture
                    debug_assert!(
                        c != Piece::King,
                        "king captured by move {} on board \n{}",
                        m,
                        b
                    );
                    b.remove_piece(m.to().as_bb(), c, b.turn);
                }
            }

            // clear one bit and set another for the move using xor
            if !m.is_null() {
                // let from_to_bits = m.from().as_bb() | m.to().as_bb();
                b.move_piece(
                    m.to().as_bb(),
                    m.from().as_bb(),
                    m.mover_piece(b),
                    b.turn.opposite(),
                );
            }

            if m.mover_piece(b) == Piece::Pawn {
                b.half_move_clock = 0;
                if m.is_pawn_double_push(b) {
                    b.en_passant = m.ep().as_bb();
                }
            }

            if let Some(promo) = m.promo() {
                // fifty clock handled by pawn move above;
                b.change_piece(m.to().as_bb(), Piece::Pawn, promo);
                // pawn has already moved
            }

            // castling *moves*
            if m.is_castle(b) {
                // rules say no reset of fifty clock
                // king move already handled, castling rights handled below, just the rook move

                let (rook_from, rook_to) = m.rook_move_from_to();
                // let rook_from_to = rook_from.as_bb() ^ rook_to.as_bb();
                b.move_piece(
                    rook_from.as_bb(),
                    rook_to.as_bb(),
                    Piece::Rook,
                    b.turn.opposite(),
                )
            }

            // castling *rights*
            //  if a piece moves TO (=capture) or FROM the rook squares - appropriate castling rights are lost
            //  if a piece moves FROM the kings squares, both castling rights are lost
            //  possible with a rook x rook capture that both sides lose castling rights
            b.castling -= m.castling_rights_lost();
        }
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::globals::constants::*;
    use crate::infra::profiler::PerfProfiler;
    use crate::other::Perft;
    use anyhow::Result;

    #[test]
    fn test_make_move() -> Result<()> {
        let board = Catalog::starting_board();
        // let mut m = Move::parse("e2e4")?;
        let mov = board.parse_uci_move("e2e4")?;
        assert_eq!(board.total_halfmoves(), 0);

        let board = board.make_move(mov);
        assert_eq!(
            board.to_fen(),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
        assert_eq!(board.total_halfmoves(), 1);

        let board = board.make_move(board.parse_uci_move("a7a6").unwrap());
        assert_eq!(board.total_halfmoves(), 2);

        let board = board.make_move(board.parse_uci_move("e4e5").unwrap());
        assert_eq!(board.total_halfmoves(), 3);

        let board = board.make_move(board.parse_uci_move("a6a5").unwrap());
        assert_eq!(board.total_halfmoves(), 4);
        Ok(())
    }

    #[test]
    fn make_move_ep() -> Result<()> {
        let board1 = Catalog::perft_cpw_number3().0.clone();
        // double push - ep sq should be set
        let mov1 = board1.parse_uci_move("e2e4")?;
        let board2 = board1.make_move(mov1);
        assert_eq!(board2.to_fen(), "8/2p5/3p4/KP5r/1R2Pp1k/8/6P1/8 b - e3 0 1");
        // ep capture is not valid as leaves king in check
        assert!(board2.parse_uci_move("f4e3").is_err());
        // , e4, "EP square for e/p capture move is square the captured piece is on");
        Ok(())
    }

    #[test]
    fn test_try_move_promotion() {
        let mut board = Board::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'").unwrap();
        board = board.make_move(board.parse_uci_move("a7a8q").unwrap());
        assert_eq!(board.get(a8), "Q");
        assert_eq!(board.get(a7), ".");
    }

    #[test]
    fn test_castling_rights() {
        // check castling rights parsed-from and returned-in fen
        let epd = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1 id: 'castling1'";
        let board = Board::parse_fen(epd).unwrap();
        board.validate().unwrap();
        assert_eq!(board.castling().to_string(), "KQkq");

        // rook takes rook, so both sides lose queens side castling grights
        let mv = board.parse_uci_move("a1a8").unwrap();
        let board = board.make_move(mv);
        assert_eq!(board.castling().to_string(), "Kk");
    }

    #[test]
    fn test_castling() {
        let epd = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1 id: 'castling1'";
        let board = Board::parse_fen(epd).unwrap();
        // casle kings side for w and then b
        assert!(!board.is_in_check(board.color_us()));
        assert!(board.checkers_of(board.color_us()).is_empty());
        assert_eq!(board.total_halfmoves(), 0);
        assert_eq!(board.legal_moves().len(), 16 + 5 + 2 + 2); // 16P, 5R, 2K, OO, OOO

        let board = board.make_move(board.parse_uci_move("e1g1").unwrap());
        assert_eq!(board.total_halfmoves(), 1);

        let board = board.make_move(board.parse_uci_move("e8g8").unwrap());
        assert_eq!(
            board.to_fen(),
            "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R4RK1 w - - 2 2"
        );
        assert_eq!(board.total_halfmoves(), 2);

        // castle queens side
        let board = Board::parse_fen(epd).unwrap();
        let board = board.make_move(board.parse_uci_move("e1c1").unwrap());
        let board = board.make_move(board.parse_uci_move("e8c8").unwrap());
        assert_eq!(
            board.to_fen(),
            "2kr3r/pppppppp/8/8/8/8/PPPPPPPP/2KR3R w - - 2 2"
        );

        // rook moves queens side for w and then b, losing q-side castling rights
        let board = Board::parse_fen(epd).unwrap();
        let board = board.make_move(board.parse_uci_move("a1b1").unwrap());
        let board = board.make_move(board.parse_uci_move("a8b8").unwrap());
        assert_eq!(
            board.to_fen(),
            "1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R w Kk - 2 2"
        );
    }

    #[test]
    fn bench_make_move() {
        let mut b = Catalog::starting_board();
        let mv = b.parse_san_move("e4").unwrap();
        PerfProfiler::new("move: apply_move".to_string()).benchmark(|| _ = b.apply_move(mv));
        let b = Catalog::starting_board();
        PerfProfiler::new("move: make_move".to_string()).benchmark(|| b.make_move(mv));

        let mut prof = PerfProfiler::new("move: perft_make_move".into());
        let mut func = |b: &Board, mv: Move| _ = prof.benchmark(|| b.make_move(mv));
        let mut b = Catalog::starting_board();
        Perft::perft_fn(&mut b, 3, &mut func);
    }

    #[test]
    fn bench_cell() {
        let mut cells: [Cell<Bitboard>; 32] = <_>::default();
        PerfProfiler::new("cell_default".to_string()).benchmark(|| {
            cells = <_>::default();
            cells.len()
        });

        let mut cells: [Cell<Option<Bitboard>>; 32] = <_>::default();
        PerfProfiler::new("cell_option_bitboard".to_string()).benchmark(|| {
            cells = <_>::default();
            cells.len()
        });

        let mut cells: [Cell<Bitboard>; 32] = <_>::default();
        PerfProfiler::new("cell_bitboard".to_string()).benchmark(|| {
            cells = [
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
                Cell::<_>::new(Bitboard::niche()),
            ];
            cells.len()
        });
    }
}

pub fn func() {
    // let mut cells: [Cell<Bitboard>; 32] = <_>::default();
    // let cells: Vec<Cell<Bitboard>> =  vec![Default::default();32];
    // black_box(cells);
    // u32 = 4 byte
    // u64 = 64 bit = 8 bytes
    // u128 = 128 bit = 16 byte = X reg
    // u256 = 32 bytes = Y reg
    let _: [u32; 32] = black_box(Default::default());
    // black_box(black_box(Bitboard::RANKS_18).first_square());
}
