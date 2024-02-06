use anyhow::anyhow;
use itertools::Itertools;

use crate::{
    bits::{
        bitboard::{Bitboard, Dir},
        castling::CastlingRights,
        precalc::PreCalc,
        square::Square,
    },
    boards::{boardcalcs::BoardCalcs, Board},
    domain::node::{Counter, Timing},
    infra::metric::Metrics,
    movelist::MoveList,
    mv::Move,
    piece::Piece,
};

#[derive(Debug)]
pub struct LegalMoves<'a, F: FnMut(Move)> {
    board:        &'a Board,
    capture_mask: Bitboard,
    callback:     F,
}

impl Board {
    pub fn validate_moves(&self, moves: &[Move]) -> anyhow::Result<()> {
        let mut bd = self.clone();
        for &mv in moves.iter() {
            if let Err(e) = mv.validate(&bd) {
                return Err(anyhow!(
                    "var: {var} on {self}: move {mv} invalid for board {bd}: {e}",
                    var = moves.iter().join(" ")
                ));
            }
            bd = bd.make_move(mv);
        }
        Ok(())
    }

    #[inline]
    pub fn legal_moves_to_with(&self, to: Bitboard, f: impl FnMut(Move)) {
        Metrics::incr(Counter::MoveGen);
        LegalMoves::new(self, to, f);
    }

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

impl<'a, F> LegalMoves<'a, F>
where
    F: FnMut(Move),
{
    pub fn new(board: &'a Board, capture_mask: Bitboard, callback: F) -> Self {
        let mut me = Self {
            board,
            capture_mask,
            callback,
        };
        me.generate();
        me
    }

    pub fn king_legal(&mut self) {
        let bd = self.board;
        let us = bd.us();
        let our_kings = bd.kings() & us;
        if our_kings.is_empty() {
            return;
        }
        let attack_gen = PreCalc::instance();
        let them = bd.them();
        let occ = bd.occupied();
        let king_sq = (bd.kings() & us).square();
        let king_att = attack_gen.king_attacks(king_sq);
        let king_danger = BoardCalcs::all_attacks_on(bd, bd.color_us(), occ - our_kings);
        let attacks = (king_att & !us & self.capture_mask) - king_danger;
        for to in attacks.squares() {
            if to.is_in(them) {
                (self.callback)(Move::new_capture(Piece::King, king_sq, to, bd));
            } else {
                (self.callback)(Move::new_quiet(Piece::King, king_sq, to));
            }
        }
    }

    pub fn generate(&mut self) {
        let t = Metrics::timing_start();
        let b = self.board;

        let us = b.us();
        let them = b.them();
        let our_kings = b.kings() & us;
        // if our_kings.is_empty() {
        //     return Self::pseudo_legals(b, moves);
        // }
        let checkers = b.checkers_of(b.color_us());
        if checkers.popcount() >= 2 {
            return self.king_legal();
        }
        let gen = PreCalc::instance();
        if checkers.popcount() == 1 {
            let the_checker = checkers.square();
            let king_sq = our_kings.square();
            // Captr checker -> capture-sq & piece not pinned & (checker)
            // Block checker -> to & piece not pinned & (xrays to checker excl)
            let blocking = gen.between(king_sq, the_checker) | checkers; // "| checkers" is for knight checkers
            for &p in Piece::ALL_BAR_KING.iter() {
                for fr in (b.pieces(p) & us & !b.pinned(b.color_us())).squares() {
                    let attacks = self.capture_mask
                        & blocking
                        & gen.attacks(b.color_us(), p, us, them, fr)
                        & !us;
                    self.add_moves(attacks, p, fr, b);
                }
            }
            self.add_moves_en_passant();
            self.king_legal();
        } else {
            let pinned = b.pinned(b.color_us());
            let king_sq = if pinned.is_empty() {
                Square::null()
            } else {
                our_kings.square()
            };
            for &p in Piece::ALL_BAR_KING.iter() {
                // not in check
                for fr in (b.pieces(p) & us).squares() {
                    let attacks =
                        self.capture_mask & gen.attacks(b.color_us(), p, us, them, fr) & !us;
                    if !fr.is_in(pinned) {
                        // all non pinned pieces
                        self.add_moves(attacks, p, fr, b);
                    } else {
                        // Pinned -> psuedo-to in ray along king and pinner incl
                        let blocking = gen.line_through(fr, king_sq);
                        // will exlude knights anyway
                        self.add_moves(attacks & blocking, p, fr, b);
                    }
                }
            }
            self.add_moves_en_passant();
            self.king_legal();
            self.castles();
        }
        Metrics::profile(t, Timing::TimingMoveGen);
    }

    pub fn add_moves_en_passant(&mut self) {
        let bd = self.board;
        let Some(to) = bd.en_passant_square() else {
            return;
        };
        // if (bd.us() & bd.pawns() & Bitboard::RANKS_45).is_empty() {
        //     return;
        // }
        let us = bd.color_us();
        let them = bd.color_them();
        let to = to.as_bb();
        let capture_sq = to.shift(them.forward());
        if capture_sq.disjoint(self.capture_mask) {
            return;
        }
        let checkers = bd.checkers_of(us);
        if checkers.popcount() == 1 {
            // any non-pinned pawn can capture the checker
            if capture_sq == checkers {
                let fr_e = to.shift(them.pawn_capture_west());
                if (fr_e & bd.pawns() & bd.us() & !bd.pinned(us)).any() {
                    (self.callback)(Move::new_ep_capture(fr_e.square(), to.square(), bd));
                }
                let fr_w = to.shift(them.pawn_capture_east());
                if (fr_w & bd.pawns() & bd.us() & !bd.pinned(us)).any() {
                    (self.callback)(Move::new_ep_capture(fr_w.square(), to.square(), bd));
                }
            }
        } else if checkers.popcount() == 0 {
            let fr_e = to.shift(them.pawn_capture_west());
            let fr_w = to.shift(them.pawn_capture_east());
            for fr in ((fr_e | fr_w) & bd.pawns() & bd.us()).squares() {
                // this optimization is not valid, as a bishop can pin a pawn in the direction of capture
                // which allows the pawn to capture
                // if fr.is_in(b.pinned()) {
                //     continue;
                // }
                // special case: will removing the capture piece AND moving the pawn result in check
                let m = Move::new_ep_capture(fr, to.square(), bd);
                if m.is_valid(bd) {
                    (self.callback)(m);
                }
            }
        }
    }

    #[inline]
    fn add_moves(&mut self, dests: Bitboard, p: Piece, fr: Square, bd: &Board) {
        if p != Piece::Pawn {
            for to in dests.squares() {
                if to.is_in(bd.them()) {
                    (self.callback)(Move::new_capture(p, fr, to, bd))
                } else {
                    (self.callback)(Move::new_quiet(p, fr, to))
                }
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if Bitboard::RANKS_18.contains(dests) {
                self.add_moves_pawn_promo(dests, fr, bd);
            } else {
                for to in dests.squares() {
                    (self.callback)(Move::new_pawn_move(fr, to, bd));
                }
            }
        }
    }

    #[inline]
    // DONE
    fn add_moves_pawn_promo(&mut self, dests: Bitboard, fr: Square, bd: &Board) {
        for to in dests.squares() {
            if to.is_in(bd.them()) {
                // try and pre-sort promos by likely usefulness
                (self.callback)(Move::new_promo_capture(fr, to, Piece::Queen, bd));
                (self.callback)(Move::new_promo_capture(fr, to, Piece::Knight, bd));
                (self.callback)(Move::new_promo_capture(fr, to, Piece::Rook, bd));
                (self.callback)(Move::new_promo_capture(fr, to, Piece::Bishop, bd));
            } else {
                (self.callback)(Move::new_promo(fr, to, Piece::Queen));
                (self.callback)(Move::new_promo(fr, to, Piece::Knight));
                (self.callback)(Move::new_promo(fr, to, Piece::Rook));
                (self.callback)(Move::new_promo(fr, to, Piece::Bishop));
            }
        }
    }

    pub fn castles(&mut self) {
        let b = self.board;
        let c = b.color_us();
        let them = b.them();
        let us = b.us();
        let occ = b.occupied();
        let king = b.kings() & us;
        if king.is_empty() {
            // allow no kings
            return;
        }
        let king_sq = king.square();
        let rights = b.castling();

        let right = CastlingRights::king_side_right(c);
        if rights.contains(right) && !CastlingRights::king_side_move_squares(c).intersects(occ) {
            let rook_to = king.shift(Dir::E);
            let king_to = rook_to.shift(Dir::E);
            let king_moves = king | rook_to | king_to;
            if BoardCalcs::attacked_by(king_moves, occ, b).disjoint(them)
                && king_to.intersects(self.capture_mask)
            {
                let m = Move::new_castle(king_sq, king_to.square());
                (self.callback)(m);
            }
        }

        let right = CastlingRights::queen_side_right(c);
        if rights.contains(right) && !CastlingRights::queen_side_move_squares(c).intersects(occ) {
            let rook_to = king.shift(Dir::W);
            let king_to = rook_to.shift(Dir::W);
            let king_moves = king | rook_to | king_to;
            if BoardCalcs::attacked_by(king_moves, occ, b).disjoint(them)
                && king_to.intersects(self.capture_mask)
            {
                let king_to = king_to.square();
                // let rook_from = king_to.shift(Dir::W).shift(Dir::W);
                let m = Move::new_castle(king_sq, king_to);
                (self.callback)(m);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bits::Square, catalog::*, globals::constants::*, infra::profiler::PerfProfiler,
        other::Perft, Color, Piece, PreCalc,
    };
    use anyhow::Result;
    use std::{hint::black_box, str::FromStr};
    use test_log::test;

    #[test]
    fn test_king_legal() {
        let b = Board::parse_piece_placement("rk6/8/8/8/8/8/8/1K6").unwrap();
        let mut moves = MoveList::new();
        let _lm = LegalMoves::new(&b, Bitboard::all(), |mv| moves.push(mv));
        moves.retain(|m| b.piece(m.from()) == Some(Piece::King));
        assert_eq!(moves.sort().to_string(), "b1b2, b1c1, b1c2");

        let b = Board::parse_piece_placement("rk6/8/8/8/8/8/K7/8").unwrap();
        let mut moves = MoveList::new();
        let _lm = LegalMoves::new(&b, Bitboard::all(), |mv| moves.push(mv));
        // xray prevents a2a1
        assert_eq!(moves.sort().to_string(), "a2b1, a2b2, a2b3");
    }

    #[test]
    fn test_is_pseudo_legal_move() {
        let bd = Board::parse_fen("8/k7/8/8/4Q3/8/8/1K6 w - - 9 4").unwrap();
        let mv = Move::new_quiet(Piece::King, Square::B1, Square::H1);
        assert_eq!(mv.to_inner().validate(&bd).is_ok(), false, "{mv} is legal");

        let bd = Board::parse_fen("5rk1/6p1/Qp1q3p/3pr2P/Pp2p1P1/1Pp1P3/2P2P2/3RR1K1 b - g3 0 4")
            .unwrap();
        let mv = Move::new_quiet(Piece::Pawn, Square::E4, Square::G3);
        let precalc = PreCalc::instance();
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
        assert_eq!(mv.to_inner().validate(&bd).is_ok(), false, "{mv} is legal");
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
    fn test_legal_move_perft() {
        let mut starting_pos = Catalog::perft_kiwipete().0;

        // check all these legal moves are indeed identified as legal
        let mut func = |bd: &Board, mv: Move| {
            assert!(mv.to_inner().validate(bd).is_ok(), "{bd} move: {mv:?}");
        };
        Perft::<true>::perft_with(&mut starting_pos, 4, &mut func);

        // check all these moves which are illegal are indeed identified as illegal
        let wrong_board = Catalog::perft_kiwipete().0;
        let mvs = wrong_board.legal_moves();
        let mut func = |_: &Board, mv: Move| {
            // where bm has no mover
            assert_eq!(
                mvs.iter().any(|m| m.to_inner() == mv.to_inner()),
                mv.to_inner().validate(&wrong_board).is_ok(),
                "{wrong_board} move: {mv:?} legals: {mvs:?}",
            );
            // where bm has mover
            assert_eq!(
                mvs.iter()
                    .any(|m| m.to_inner_with_mover(&wrong_board)
                        == mv.to_inner_with_mover(&wrong_board)),
                mv.to_inner_with_mover(&wrong_board)
                    .validate(&wrong_board)
                    .is_ok(),
                "{wrong_board} move: {mv:?} legals: {mvs:?}",
            );
        };
        Perft::<true>::perft_with(&mut starting_pos, 4, &mut func);
    }

    #[test]
    fn test_legal_move() -> Result<()> {
        // https://lichess.org/editor/8/8/8/8/8/8/6r1/7K
        let fen = "8/8/8/8/8/8/6r1/7K w - - 0 0 id 'rook+king'";
        let board = Board::parse_fen(fen).unwrap();
        assert_eq!(board.legal_moves().sort().to_string(), "h1g2", "{board:#}",);
        let mov_h1g2 = board.parse_uci_move("h1g2")?;
        assert_eq!(mov_h1g2.is_valid(&board), true);
        // println!("{}", Metrics::to_string());

        for epd in Catalog::stalemates().iter() {
            assert_eq!("", epd.board().legal_moves().to_string(), "{epd}",);
        }

        let board = Catalog::starting_board();
        let mv = board.parse_uci_move("e2e3").unwrap();
        assert!(mv.is_valid(&board), "{:?}", mv);
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
    fn test_is_a_valid_move() {
        let b = Catalog::starting_board();
        let a2sq = a2.square();
        let a3sq = a3.square();
        let a6sq = a6.square();
        let a7sq = a7.square();
        assert_eq!(
            Move::new_quiet(Piece::Pawn, a2sq, a3sq)
                .to_inner()
                .validate(&b)
                .is_ok(),
            true
        );
        assert_eq!(
            Move::new_quiet(Piece::Pawn, a2sq, a6sq)
                .to_inner()
                .validate(&b)
                .is_ok(),
            false
        );
        assert_eq!(
            Move::new_quiet(Piece::Pawn, a7sq, a6sq)
                .to_inner()
                .validate(&b)
                .is_ok(),
            false
        );
        assert_eq!(
            Move::new_quiet(Piece::Pawn, a7sq, a6sq)
                .to_inner()
                .validate(&b)
                .is_ok(),
            false
        );
        // as capture piece no longer stored
        // assert_eq!(
        //     &Move::new_capture(Piece::Pawn, a2sq, a3sq, Piece::Pawn)),
        //     false
        // );
        for &mv in b.legal_moves().iter() {
            assert!(mv.is_valid(&b));
            assert!(mv.to_inner().validate(&b).is_ok(), "{mv:#}");
        }
    }

    #[test]
    fn test_is_legal_variation() {
        let b =
            Board::from_str("b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12")
                .unwrap();
        let var = b.parse_uci_variation("d3d1 b4c6 e2c4").unwrap();
        assert_eq!(var.len(), 3);
        b.validate_moves(&var.moves().collect_vec()).unwrap();
    }

    #[test]
    fn test_catalog_moves() {
        for epd in Catalog::moves() {
            let lm = epd
                .board()
                .legal_moves()
                .to_uci()
                .split_ascii_whitespace()
                .map(str::to_string)
                .sorted()
                .join(" ");

            let expected = epd
                .tag("c1")
                .unwrap()
                .split_ascii_whitespace()
                .map(str::to_string)
                .sorted()
                .join(" ");

            assert_eq!(lm, expected, "{epd} {:#}", epd.board());
        }
    }

    #[test]
    fn bench_movegen() {
        let mut starting_pos = Board::starting_pos();

        let mut legal_moves = PerfProfiler::new("board.legal_moves");
        let mut legal_moves_into = PerfProfiler::new("board.legal_moves_into");

        let mut func = |bd: &Board, _mv: Move| {
            legal_moves.bench(|| black_box(bd).legal_moves());
            let mut dest = MoveList::new();
            legal_moves_into.bench(|| black_box(bd).legal_moves_with(|mv| dest.push(mv)));
        };
        Perft::<true>::perft_with(&mut starting_pos, 3, &mut func);
    }
}
