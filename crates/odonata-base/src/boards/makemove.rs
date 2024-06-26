use crate::boards::hasher::Hasher;
use crate::domain::node::{Counter, Timing};
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::piece::{FlipSide, Piece};
use crate::prelude::Board;
use crate::variation::Variation;

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
                "invalid board before move {mv} from {var} on board {b} (base board {self:#})"
            );
            debug_assert!(
                mv.is_valid(&b),
                "invalid make moves {mv} in var {var} on board {b} (base board {self:#})"
            );
            b = b.make_move(mv);
        }
        b
    }

    pub fn make_move_str(&self, mv: &str) -> anyhow::Result<Board> {
        Ok(self.make_move(self.parse_san_move(mv)?))
    }

    pub fn make_moves_str(&self, var: &str) -> anyhow::Result<Board> {
        Ok(self.make_moves(&self.parse_san_variation(var)?))
    }

    pub fn make_move(&self, m: Move) -> Board {
        Metrics::incr(Counter::MakeMove);
        // either we're moving to an empty square or its a capture
        debug_assert!(
            m.is_null() || ((self.white() | self.black()) & m.to().as_bb()).is_empty() || m.is_capture(),
            "Non-empty to:sq for non-capture {:?} board \n{} white \n{} black\n{}",
            m,
            self,
            self.white(),
            self.black()
        );

        let mut b = Board {
            en_passant:      self.en_passant_square(),
            turn:            self.turn,
            fullmove_number: self.fullmove_number,
            halfmove_clock: self.halfmove_clock,
            threats_to:      Default::default(),
            checkers_of:     Default::default(),
            pinned:          Default::default(),
            discoverer:      Default::default(),
            // material: Cell::<_>::new(self.material()),
            // moves: self.moves.clone(),
            pieces:          self.pieces,
            colors:          self.colors,
            castling:        self.castling,
            hash:            self.hash,
            ply:             self.ply,
        };
        self.make_move_into(m, &mut b);
        b
    }

    pub fn copy_from(&mut self, b: &Board) {
        self.en_passant = b.en_passant_square();
        self.turn = b.turn;
        self.fullmove_number = b.fullmove_number;
        self.halfmove_clock = b.halfmove_clock;
        self.threats_to = Default::default();
        self.checkers_of = Default::default();
        self.pinned = Default::default();
        self.discoverer = Default::default();
        // material: Cell::<_>::new(b.material()),
        // moves= b.moves.clone(),
        self.pieces = b.pieces;
        self.colors = b.colors;
        self.castling = b.castling;
        self.hash = b.hash;
        self.ply = b.ply;
    }

    pub fn copy_from_v2(bds: &mut [Board], i: usize, j: usize) {
        bds[i].en_passant = bds[j].en_passant_square();
        bds[i].turn = bds[j].turn;
        bds[i].fullmove_number = bds[j].fullmove_number;
        bds[i].halfmove_clock = bds[j].halfmove_clock;
        bds[i].threats_to = Default::default();
        bds[i].checkers_of = Default::default();
        bds[i].pinned = Default::default();
        bds[i].discoverer = Default::default();
        // material: Cell::<_>::new(bds[j].material()),
        // moves= bds[j].moves.clone(),
        bds[i].pieces = bds[j].pieces;
        bds[i].colors = bds[j].colors;
        bds[i].castling = bds[j].castling;
        bds[i].hash = bds[j].hash;
        bds[i].ply = bds[j].ply;
    }

    pub fn make_move_into(&self, m: Move, b: &mut Board) {
        let t = Metrics::timing_start();
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
            cs = m.capture_square(b),
            ic = m.is_capture(),
            cp = m.capture_piece(b),
            ep = m.is_ep_capture(b)
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
            b.hash == Hasher::instance().hash_board(b),
            "\n{self}.make_move({m}) = \n{b} inconsistent incremental hash {:x} (should be {:x})",
            b.hash,
            Hasher::instance().hash_board(b),
        );
    }

    #[inline]
    pub fn apply_move(&mut self, m: Move) {
        let b = self;
        b.fullmove_number += b.turn.chooser_wb(0, 1);
        b.halfmove_clock += 1;
        b.ply += 1;
        let move_hash = Hasher::instance().hash_move(m, b);
        b.hash ^= move_hash;
        // now hash calculated - we can adjust these
        b.turn = b.turn.flip_side();
        if m.is_null() {
            b.en_passant = None;
            return;
        }

        let mover = m.mover_piece(b);

        let mut en_passant = None;
        if mover == Piece::Pawn {
            b.halfmove_clock = 0;
            if m.is_pawn_double_push(b) {
                en_passant = Some(m.double_push_en_passant_square());
            }
        }

        // if let Some(c) = m.capture() {
        //     b.half_move_clock = 0;
        //     debug_assert!(c != Piece::King, "king captured by move {m} on board \n{b}");
        //     b.toggle_piece(m.capture_square(b), c, b.turn);
        // }

        if let Some(c) = m.capture_piece(b) {
            b.halfmove_clock = 0;
            if m.is_ep_capture(b) {
                // ep capture is like capture but with capture piece on *ep* square not *dest*
                b.toggle_piece(m.capture_square(b).as_bb(), c, b.turn);
            } else {
                // regular capture
                debug_assert!(c != Piece::King, "king captured by move {m} on board \n{b}");
                b.toggle_piece(m.to().as_bb(), c, b.turn);
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
            b.move_piece(rook_from.as_bb(), rook_to.as_bb(), Piece::Rook, b.turn.flip_side())
        }

        // clear one bit and set another for the move using xor
        // let from_to_bits = m.from().as_bb() | m.to().as_bb();
        b.move_piece(m.from().as_bb(), m.to().as_bb(), mover, b.turn.flip_side());

        if let Some(promo) = m.promo_piece() {
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
}

#[cfg(test)]

mod tests {
    use std::cell::Cell;

    use anyhow::Result;

    use super::*;
    use crate::catalog::*;
    use crate::infra::profiler::PerfProfiler;
    use crate::Bitboard;

    #[test]
    fn test_make_move() -> Result<()> {
        let board = Catalog::starting_board();
        // let mut m = Move::parse("e2e4")?;
        let mov = board.parse_uci_move("e2e4")?;
        assert_eq!(board.total_halfmove_ply(), 0);

        let board = board.make_move(mov);
        assert_eq!(
            board.to_fen(),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
        assert_eq!(board.total_halfmove_ply(), 1);

        let board = board.make_move(board.parse_uci_move("a7a6").unwrap());
        assert_eq!(board.total_halfmove_ply(), 2);

        let board = board.make_move(board.parse_uci_move("e4e5").unwrap());
        assert_eq!(board.total_halfmove_ply(), 3);

        let board = board.make_move(board.parse_uci_move("a6a5").unwrap());
        assert_eq!(board.total_halfmove_ply(), 4);
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
        assert_eq!(board.get(Bitboard::A8), "Q");
        assert_eq!(board.get(Bitboard::A7), ".");
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
        assert_eq!(board.total_halfmove_ply(), 0);
        assert_eq!(board.legal_moves().len(), 16 + 5 + 2 + 2); // 16P, 5R, 2K, OO, OOO

        let board = board.make_move(board.parse_uci_move("e1g1").unwrap());
        assert_eq!(board.total_halfmove_ply(), 1);

        let board = board.make_move(board.parse_uci_move("e8g8").unwrap());
        assert_eq!(board.to_fen(), "r4rk1/pppppppp/8/8/8/8/PPPPPPPP/R4RK1 w - - 2 2");
        assert_eq!(board.total_halfmove_ply(), 2);

        // castle queens side
        let board = Board::parse_fen(epd).unwrap();
        let board = board.make_move(board.parse_uci_move("e1c1").unwrap());
        let board = board.make_move(board.parse_uci_move("e8c8").unwrap());
        assert_eq!(board.to_fen(), "2kr3r/pppppppp/8/8/8/8/PPPPPPPP/2KR3R w - - 2 2");

        // rook moves queens side for w and then b, losing q-side castling rights
        let board = Board::parse_fen(epd).unwrap();
        let board = board.make_move(board.parse_uci_move("a1b1").unwrap());
        let board = board.make_move(board.parse_uci_move("a8b8").unwrap());
        assert_eq!(board.to_fen(), "1r2k2r/pppppppp/8/8/8/8/PPPPPPPP/1R2K2R w Kk - 2 2");
    }

    #[test]
    fn bench_make_move() {
        let mut b = Catalog::starting_board();
        let mv = b.parse_san_move("e4").unwrap();
        PerfProfiler::new("movegen: apply_move").bench(|| b.apply_move(mv));
        let b = Catalog::starting_board();
        PerfProfiler::new("movegen: make_move").bench(|| b.make_move(mv));
    }

    #[test]
    fn bench_cell() {
        let mut cells: [Cell<Bitboard>; 32] = <_>::default();
        PerfProfiler::new("cell_default").bench(|| {
            cells = <_>::default();
            cells.len()
        });

        let mut cells: [Cell<Option<Bitboard>>; 32] = <_>::default();
        PerfProfiler::new("cell_option_bitboard").bench(|| {
            cells = <_>::default();
            cells.len()
        });

        let mut cells: [Cell<Bitboard>; 32] = <_>::default();
        PerfProfiler::new("cell_bitboard").bench(|| {
            cells = [
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
                Cell::<_>::new(Bitboard::all()),
            ];
            cells.len()
        });
    }
}
