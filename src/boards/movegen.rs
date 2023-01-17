use anyhow::anyhow;
use itertools::Itertools;

use crate::{
    bits::{bitboard::Bitboard, precalc::PreCalc, Square},
    boards::{rules::LegalMoves, Board},
    infra::metric::*,
    movelist::MoveList,
    mv::{BareMove, Move},
    piece::Piece,
    search::node::{Counter, Timing},
};

// fn is_in_check(&self, king_color: Color) -> bool;
// fn will_check_them(&self, mv: &Move) -> bool;
// fn is_legal_move(&self, mv: &Move) -> bool;
// fn pseudo_legal_moves(&self) -> MoveList;
// fn legal_moves(&self) -> impl Iterator<Item=Move>;
// fn legal_capture_moves(&self) -> MoveList;
// fn threats_to(&self, c: Color) -> Bitboard;

impl Board {
    pub fn is_pseudo_legal_and_legal_move(&self, m: Move) -> bool {
        let t = Metrics::timing_start();
        let ret = self.is_pseudo_legal_move(m) && self.is_legal_move(m);
        Metrics::profile(t, Timing::TimingPseudoLegalAndLegal);
        ret
    }

    pub fn validate_pseudo_legal_and_legal_move(&self, m: Move) -> Result<(), &'static str> {
        if !self.is_pseudo_legal_move(m) {
            return Err("Move is not pseudo-legal");
        }
        if !self.is_legal_move(m) {
            return Err("Move is not legal");
        }
        Ok(())
    }

    pub fn is_pseudo_legal_move(&self, m: Move) -> bool {
        if !self.is_pseudo_legal_baremove(&m.to_inner()) {
            return false;
        }
        // if m.mover_piece(self) != self.piece_unchecked(m.from()) {
        //     return false;
        // }

        if m.is_capture() {
            debug_assert!(
                !m.capture_square(self).is_null(),
                "Move {m} on board {self} has null capture square"
            );
            if !m.capture_square(self).is_in(self.them()) {
                trace!("Capture sq not opponents piece");
                return false;
            }
        } else {
            if m.to().is_in(self.occupied()) {
                trace!("Quiet move has occupied to-sq");
                return false;
            }
            // if its not a capture, it can't be an e/p capture
            if m.is_ep_capture(self) {
                trace!("Quiet move is e/p capture");
                return false;
            }
        }

        let double_push = m.mover_piece(self) == Piece::Pawn
            && (m.to().index() == m.from().index() + 16 || m.to().index() + 16 == m.from().index());
        if double_push {
            if !m.is_pawn_double_push(self) {
                trace!("Looks like a double push but not flagged as one");
                return false;
            }
        }
        if m.is_pawn_double_push(self) {
            if !double_push {
                trace!("Double push not from a pawn square");
                return false;
            }
        }

        // else {
        //     let cap = m.capture_square(self);
        //     if cap.is_null() {
        //         return false;
        //     }
        //     if !cap.is_in(self.them()) {
        //         return false;
        //     }
        // if !m.ep().is_null() && m.mover_piece(self) != Piece::Pawn {
        //     return false;
        // }
        if let Some(c) = m.capture_piece(self) {
            if !m.is_ep_capture(self) {
                if !m.to().is_in(self.them()) {
                    trace!("Capture has to-sq thats not opponent occupied");
                    return false;
                }
                if c != self.piece_unchecked(m.to()) {
                    // FIXME! allow capture of another type of piece?
                    trace!("Capture has wrong piece");
                    return false;
                }
            }
            // } else if !m.ep().is_in(self.them() & self.pawns()) {
            //     return false;
            // }
        }
        true
    }

    pub fn is_pseudo_legal_baremove(&self, m: &BareMove) -> bool {
        let precalc = PreCalc::default();

        let mover = self.piece(m.from());
        // check piece move
        let Some(mover) = mover  else {
            return false;
        };
        if let Some(pp) = m.promo() {
            if mover != Piece::Pawn {
                return false;
            }
            if !m.to().as_bb().intersects(Bitboard::RANKS_18) {
                // TODO! exact promo rank for white/black
                return false;
            }
            if pp == Piece::King || pp == Piece::Pawn {
                return false;
            }
        }

        if m.from().is_in(self.line_pieces()) {
            if precalc.between(m.from(), m.to()).is_empty() {
                // to/from dont share a diagonal or orthogonal
                return false;
            }
            if precalc
                .strictly_between(m.from(), m.to())
                .intersects(self.occupied())
            {
                return false;
            }
        }
        if m.from().is_in(self.pawns()) {
            if (precalc.strictly_between(m.from(), m.to()) & self.occupied()).any() {
                return false;
            }
            if self.is_en_passant_square(m.to())
                && m.to()
                    .is_in(precalc.pawn_attacks_from_sq(self.color_us(), m.from()))
            {
                return true;
            }
            if m.to().is_in(Bitboard::RANKS_18) && m.promo().is_none() {
                return false;
            }
        }

        if m.is_castle(self) && self.is_castling_move_legal(*m) {
            return true;
        }

        let destinations =
            precalc.attacks(self.color_us(), mover, self.us(), self.them(), m.from());
        if !m.to().is_in(destinations) {
            // println!("Returning false for {m} on board {self}");
            return false;
        }
        // println!("Returning true for {m} on board {self}");
        // println!(
        //     " in fn: dests = {d} e = {e}, cast = {c}",
        //     d = m.to().is_in(destinations),
        //     e = m.to().is_in(destinations | self.en_passant()),
        //     c = m.is_castle(self)
        // );
        true
    }

    pub fn validate_moves(&self, moves: &[Move]) -> anyhow::Result<()> {
        let mut bd = self.clone();
        for &mv in moves.iter() {
            if !mv.is_null() && !bd.is_pseudo_legal_and_legal_move(mv) {
                return Err(anyhow!(
                    "var: {var} on {self}: move {mv:?} is not valid for board {bd}",
                    var = moves.iter().join(" ")
                ));
            }
            bd = bd.make_move(mv);
        }
        Ok(())
    }

    // the move is pseudo legal
    pub fn is_legal_move(&self, mv: Move) -> bool {
        if !self.is_pseudo_legal_baremove(&mv.to_inner()) {}

        if !self.is_legal_baremove(&mv.to_inner()) {
            return false;
        }
        true
    }

    // assumes the move known is pseudo legal. We just check for putting king in check
    pub fn is_legal_baremove(&self, mv: &BareMove) -> bool {
        if mv.is_null() {
            return false;
        }
        if !mv.from().is_in(self.us()) {
            return false;
        }

        // castling and kings moves already done above
        let mut us = self.us();
        let mut kings = self.kings() & us;
        if kings.is_empty() {
            return true; // a test position without king on the board - we allow
        }

        // idea - lightweight make_move - no hash - just enough to check rays of sliders etc
        let mut them = self.them();
        let from_to_bits = mv.from().as_bb() | mv.to().as_bb();
        us ^= from_to_bits;

        if mv.from.is_in(kings) {
            kings ^= from_to_bits;
        }
        let sq = kings.square();

        // regular capture
        if mv.to().is_in(self.them()) {
            them.remove(mv.to().as_bb());
        } else if mv.from().is_in(self.pawns())
            && !mv.to().is_in(self.them())
            && mv.from().file_index() != mv.to().file_index()
        {
            // ep capture is like capture but with capture piece on *ep* square not *dest*
            let sq = Square::from_xy(mv.to().file_index() as u32, mv.from().rank_index() as u32);
            debug_assert!(
                sq.is_in(self.them()),
                "ep capture square not occupied for move {mv} on board {self}"
            );
            them.remove(sq.as_bb());
        }

        // in (rough) order of computation cost / likelyhood - this code from "attacked_by"
        // their pieces wont have moved, but they may have been taken
        // we rely on self.pieces & them NOT being affected by our move other than by capture

        let attack_gen = PreCalc::default();
        let occ = us | them;
        if (attack_gen.rook_attacks(occ, sq) & (self.rooks() | self.queens()) & them).any() {
            return false;
        }

        if (attack_gen.knight_attacks(sq) & self.knights() & them).any() {
            return false;
        }

        if (attack_gen.bishop_attacks(occ, sq) & (self.bishops() | self.queens()) & them).any() {
            return false;
        }

        if (attack_gen.pawn_attackers(kings, self.color_them()) & self.pawns() & them).any() {
            return false;
        }

        if (attack_gen.king_attacks(sq) & self.kings() & them).any() {
            return false;
        }

        // FIXME! need to check castling

        true
    }

    // #[inline]
    // pub fn legal_moves_for_each(&self, f: impl FnMut(Piece, Square, Bitboard))  {
    //     Metrics::incr(Counter::MoveGen);
    //     Rules::legals_for(self, f);
    // }

    #[inline]
    pub fn legal_moves_with(&self, f: impl FnMut(Move)) {
        Metrics::incr(Counter::MoveGen);
        LegalMoves::new(self, Bitboard::all(), f);
    }

    #[inline]
    pub fn legal_moves(&self) -> MoveList {
        Metrics::incr(Counter::MoveGen);
        let mut moves = MoveList::new();
        LegalMoves::new(self, Bitboard::all(), |mv| moves.push(mv));
        moves
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::{
        catalog::*,
        globals::constants::*,
        infra::{black_box, profiler::PerfProfiler},
        other::Perft,
        Color,
    };
    use anyhow::Result;
    use test_log::test;
    // use crate::movelist::MoveValidator;

    #[test]
    fn test_is_pseudo_legal_move() {
        let bd = Board::parse_fen("8/k7/8/8/4Q3/8/8/1K6 w - - 9 4").unwrap();
        let mv = Move::new_quiet(Piece::King, Square::B1, Square::H1);
        assert_eq!(bd.is_pseudo_legal_move(mv), false, "{mv} is legal");

        let bd = Board::parse_fen("5rk1/6p1/Qp1q3p/3pr2P/Pp2p1P1/1Pp1P3/2P2P2/3RR1K1 b - g3 0 4")
            .unwrap();
        let mv = Move::new_quiet(Piece::Pawn, Square::E4, Square::G3);
        let precalc = PreCalc::default();
        let mover = mv.mover_piece(&bd);
        let destinations = precalc.attacks(bd.color_us(), mover, bd.us(), bd.them(), mv.from());

        if !mv.to().is_in(destinations)
            && !bd.is_en_passant_square(mv.to())
            && !mv.to_inner().is_castle(&bd)
        {
            println!("bad attack");
        }
        println!(
            "bad attack cast={c} dest={d}",
            c = mv.to_inner().is_castle(&bd),
            d = mv.to().is_in(destinations)
        );
        assert_eq!(bd.is_pseudo_legal_move(mv), false, "{mv} is legal");
    }

    #[test]
    fn test_pseudo_legal_moves() -> Result<()> {
        let mut buf =
            Board::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        buf.set(a2, ".")?;
        buf.set(d2, ".")?;
        buf.set(d4, "P")?;
        let board = buf;
        let moves = board.legal_moves();
        assert_eq!(moves.len(), 32);
        // println!("{}\n{:#?}", board, moves);
        // assert_eq!(format!("{:#?}", moves), "vec![]");
        Ok(())
    }

    #[test]
    fn pawn_moves() {
        let bd = Board::parse_fen("8/8/8/8/8/8/P7/8 w - - 0 0 id 'lone P'").unwrap();
        let mut moves = bd.legal_moves();
        println!("{}\n{:#?}", bd, moves);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4");

        let bd = Board::parse_fen("8/p7/8/8/8/8/8/8 b - - 0 0 id 'lone P flipped'").unwrap();
        let mut moves = bd.legal_moves();
        assert_eq!(moves.sort().to_string(), "a7a5, a7a6");

        let bd = Board::parse_fen("8/8/8/8/8/p7/P7/8 w - - 0 0 id PP").unwrap();
        let mut moves = bd.legal_moves();
        assert_eq!(moves.sort().to_string(), "");

        let bd = Board::parse_fen("8/8/8/8/8/8/PPP5/8 w - - 0 0 id PPP").unwrap();
        let mut moves = bd.legal_moves();
        assert_eq!(
            moves.sort().to_string(),
            "a2a3, a2a4, b2b3, b2b4, c2c3, c2c4"
        );

        let bd = Board::parse_fen("8/8/8/8/8/p1p5/1P6/8 w - - 0 0 id P 'capture white'").unwrap();
        let mut moves = bd.legal_moves();
        assert_eq!(moves.sort().to_string(), "b2a3, b2b3, b2b4, b2c3");

        let bd = Board::parse_fen("8/1p6/P1P5/8/8/8/1P6/8 b - - 0 0 id 'P capture black'").unwrap();
        let mut moves = bd.legal_moves();
        assert_eq!(moves.sort().to_string(), "b7a6, b7b5, b7b6, b7c6");

        let bd = Board::parse_fen("8/8/p6p/1N6/8/8/8/8 b - - 0 0 id 'PxN black'").unwrap();
        let mut moves = bd.legal_moves();
        assert_eq!(moves.sort().to_string(), "a6a5, a6b5, h6h5");
    }

    #[test]
    fn pawn_en_passant() {
        let bd = Board::parse_fen("8/8/8/pP6/8/8/8/8 w - a6 0 0 id en-passant-#1").unwrap();
        assert_eq!(bd.en_passant_square(), Some(a6.square()));
        assert_eq!(bd.legal_moves().sort().to_string(), "b5a6, b5b6");
        let bd = Board::parse_fen("8/8/8/PpP5/8/8/8/8 w - b6 0 0 id 'en passant #2'").unwrap();
        assert_eq!(
            bd.legal_moves().sort().to_string(),
            "a5a6, a5b6, c5b6, c5c6"
        );
    }

    #[test]
    fn pawn_promotions() {
        let bd = Board::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'").unwrap();
        assert_eq!(
            bd.legal_moves().sort().to_string(),
            "a1a2, a1b1, a1b2, a7a8b, a7a8n, a7a8q, a7a8r"
        );
    }

    #[test]
    fn rook_moves() {
        let bd = Board::parse_fen("8/8/8/8/8/8/8/R7 w - - 0 0 id 'R'").unwrap();
        assert_eq!(
            bd.legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1a7, a1a8, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let bd = Board::parse_fen("8/p7/P7/8/8/8/8/R7 w - - 0 0 id 'R with P blocking'").unwrap();
        assert_eq!(
            bd.legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let bd = Board::parse_fen("8/p7/p7/8/8/8/8/R7 w - - 0 0 id 'R with p to capture'").unwrap();
        assert_eq!(
            bd.legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );
    }

    #[test]
    fn knight_moves() {
        let bd = Board::parse_fen("8/8/8/3N4/8/8/8/8 w - - 0 0 id 'N d5'").unwrap();
        assert_eq!(
            bd.legal_moves().sort().to_string(),
            "d5b4, d5b6, d5c3, d5c7, d5e3, d5e7, d5f4, d5f6"
        );
    }

    #[test]
    fn bishop_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/8/B7 w - - 0 0 id 'B a1'").unwrap();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "a1b2, a1c3, a1d4, a1e5, a1f6, a1g7, a1h8"
        );

        let board = Board::parse_fen("8/8/8/8/8/8/1B6/8 w - - 0 0 id 'B b2'").unwrap();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "b2a1, b2a3, b2c1, b2c3, b2d4, b2e5, b2f6, b2g7, b2h8"
        );
    }

    #[test]
    fn moves_in_check() {
        let bd = Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
            .unwrap();
        assert!(bd.en_passant_square().is_none());
        // FIXME! assert b1.validate_move(Move.parse('e8e7'))
        // assert!(bd.pseudo_legal_moves().contains(Move.parse("e8e7")));
    }

    #[test]
    fn test_is_in_check() {
        let fen = "r4r1k/p1ppqpb1/bn2pnp1/3PN2Q/1p2P3/2N4p/PPPBBPPP/2R1K2R b K - 3 2";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::Black), true);

        let fen = "k7/8/8/8/8/8/7K/R7 w - - 0 0 id 'check #1'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::Black), true, "\n{}", bd);
        assert_eq!(bd.is_in_check(Color::White), false);

        let fen = "8/k2P3R/8/8/8/8/8/7K w - - 0 0 id 'pawn blocks rook #1'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::Black), false);

        let fen = "k7/8/8/8/8/8/7K/7B w - - 0 0 id 'check Bishop #2'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(crate::Color::Black), true);

        let fen = "k7/8/2p5/8/8/8/7K/7B w - - 0 0 id 'check blocked bishop #3'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::Black), false);

        let fen = "k7/7R/2p5/8/8/8/7K/7B w - - 0 0 id 'check blocked bishop #4'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::Black), false);

        let fen = "k7/8/8/8/8/8/7K/7Q w - - 0 0 id 'check Queen #1'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::Black), true);

        let fen = "k7/8/1N6/8/8/8/7K/8 w - - 0 0 id 'check N #5'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::Black), true);

        // pawn capture+promo attack
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q2/PPPBBPpP/1R3K1R w kq - 0 2";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::White), true, "\n{}", bd);

        // checks by king
        let fen = "8/8/8/8/8/8/1k6/K7 w - - 0 0 id 'check by king!'";
        let bd = Board::parse_fen(fen).unwrap();
        assert_eq!(bd.is_in_check(Color::White), true);
    }

    #[test]
    fn test_legal_move() -> Result<()> {
        // https://lichess.org/editor/8/8/8/8/8/8/6r1/7K
        let fen = "8/8/8/8/8/8/6r1/7K w - - 0 0 id 'rook+king'";
        let board = Board::parse_fen(fen).unwrap();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "h1g2",
            "{:#}",
            board
        );
        let mov_h1g2 = board.parse_uci_move("h1g2")?;
        assert_eq!(board.is_legal_move(mov_h1g2), true);
        // println!("{}", Metrics::to_string());

        for b in Catalog::stalemates().iter() {
            assert_eq!(
                b.legal_moves().to_string(),
                "".to_string(),
                "{}",
                b.to_fen()
            );
        }

        let board = Catalog::starting_board();
        let mv = board.parse_uci_move("e2e3").unwrap();
        assert!(board.is_legal_move(mv), "{:?}", mv);
        Ok(())
    }

    #[test]
    fn test_legal_variation() -> Result<()> {
        let b = Catalog::starting_board();
        let bd2 = Catalog::test_position();
        let mvs = b.parse_uci_variation("a2a3 e7e6 b2b4")?;
        assert_eq!(b.validate_moves(&mvs.moves().collect_vec()).is_ok(), true);
        assert_eq!(
            bd2.board()
                .validate_moves(&mvs.moves().collect_vec())
                .is_ok(),
            false
        );
        Ok(())
    }

    #[test]
    fn test_is_valid_move() {
        let b = Catalog::starting_board();
        let a2sq = a2.square();
        let a3sq = a3.square();
        let a6sq = a6.square();
        let a7sq = a7.square();
        assert_eq!(
            b.is_pseudo_legal_move(Move::new_quiet(Piece::Pawn, a2sq, a3sq)),
            true
        );
        assert_eq!(
            b.is_pseudo_legal_move(Move::new_quiet(Piece::Pawn, a2sq, a6sq)),
            false
        );
        assert_eq!(
            b.is_pseudo_legal_move(Move::new_quiet(Piece::Pawn, a7sq, a6sq)),
            false
        );
        assert_eq!(
            b.is_pseudo_legal_move(Move::new_quiet(Piece::Pawn, a7sq, a6sq)),
            false
        );
        // as capture piece no longer stored
        // assert_eq!(
        //     b.is_pseudo_legal_move(&Move::new_capture(Piece::Pawn, a2sq, a3sq, Piece::Pawn)),
        //     false
        // );
        for &mv in b.legal_moves().iter() {
            assert!(b.is_legal_move(mv));
            assert!(b.is_pseudo_legal_move(mv), "{mv:#}");
        }
    }

    #[test]
    fn test_is_legal_variation() {
        let b =
            Board::from_str("b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12")
                .unwrap();
        let var = b.parse_uci_variation("d3d1 b4c6 e2c4").unwrap();
        assert_eq!(var.len(), 3);
        assert!(b.validate_moves(&var.moves().collect_vec()).is_ok());
    }

    #[test]
    fn test_catalog_moves() {
        let positions = Catalog::moves();
        for pos in positions {
            let mut lm: Vec<String> = pos
                .board()
                .legal_moves()
                .uci()
                .split_ascii_whitespace()
                .map(str::to_string)
                .collect();
            lm.sort();
            let lm = lm.join(" ");

            let mut expected: Vec<String> = pos
                .try_get("c1")
                .unwrap()
                .value_uci()
                .split_ascii_whitespace()
                .map(str::to_string)
                .collect();
            expected.sort();
            let expected = expected.join(" ");

            assert_eq!(lm, expected, "{} {:#}", pos, pos.board());
        }
    }

    #[test]
    fn bench_movegen() {
        let mut starting_pos = Board::starting_pos();

        let mut legal_moves = PerfProfiler::new("board.legal_moves".into());
        let mut legal_moves_into = PerfProfiler::new("board.legal_moves_into".into());

        let mut func = |bd: &Board, _mv: Move| {
            legal_moves.benchmark(|| black_box(bd).legal_moves());
            let mut dest = MoveList::new();
            legal_moves_into.benchmark(|| black_box(bd).legal_moves_with(|mv| dest.push(mv)));
        };
        Perft::perft_with(&mut starting_pos, 3, &mut func);
    }
}
