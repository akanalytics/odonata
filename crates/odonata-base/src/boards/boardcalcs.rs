use crate::{
    bits::{bitboard::Bitboard, precalc::PreCalc},
    piece::{Color, FlipSide},
    prelude::Board,
};

pub struct BoardCalcs {}

impl BoardCalcs {
    // no king on board => no attackers
    #[inline]
    pub fn checkers_of(bd: &Board, king_color: Color) -> Bitboard {
        let us = bd.color(king_color);
        let them = bd.color(king_color.flip_side());
        let our_king = bd.kings() & us;
        // if our_king.is_empty() {
        //     return Bitboard::empty();
        // };
        // debug_assert!(!our_king.is_empty(), "king ({}) not found {}", king_color,
        // board);
        let occ = us | them;
        Self::attacked_by(our_king, occ, bd) & them
    }

    #[inline]
    pub fn pinned_and_discoverers(bd: &Board, king_color: Color) -> (Bitboard, Bitboard) {
        let kings = bd.kings() & bd.color(king_color);
        if kings.is_empty() {
            return Default::default();
        };
        let color_us = bd.color(king_color);
        let color_them = bd.color(king_color.flip_side());
        let king_sq = kings.square();

        let pc = PreCalc::instance();
        // let xray_sliding_checkers = Self::attacked_by(kings, Bitboard::EMPTY, bd) &
        // color_them;

        let xray_sliding_checkers = color_them
            & (pc.bishop_xray_attacks(king_sq) & bd.bishops_or_queens()
                | pc.rook_xray_attacks(king_sq) & bd.rooks_or_queens());
        let mut pinned = Bitboard::empty();
        let mut discoverers = Bitboard::empty();
        for checker in xray_sliding_checkers.squares() {
            let ray = pc.strictly_between(checker, king_sq);
            if ray.disjoint(color_them) && (ray & color_us).exactly_one() {
                pinned |= ray & color_us;
            } else if ray.disjoint(color_us) && (ray & color_them).exactly_one() {
                discoverers |= ray & color_them;
            }
        }
        (pinned, discoverers)
    }

    /// all attacks
    #[inline]
    pub fn all_attacks_on(bd: &Board, us: Color, occ: Bitboard) -> Bitboard {
        let opponent = us.flip_side();
        let pawns = bd.pawns() & bd.color(opponent);
        let knights = bd.knights() & bd.color(opponent);
        let bishops_or_queens = bd.bishops_or_queens() & bd.color(opponent);
        let rooks_or_queens = bd.rooks_or_queens() & bd.color(opponent);
        let kings = bd.kings() & bd.color(opponent);

        let attack_gen = PreCalc::instance();
        let (east, west) = attack_gen.pawn_attacks_ew(pawns, opponent);
        let mut threats = east | west;

        for sq in knights.squares() {
            threats |= attack_gen.knight_attacks(sq);
        }
        for sq in (bishops_or_queens).squares() {
            threats |= attack_gen.bishop_attacks(occ, sq);
        }

        for sq in (rooks_or_queens).squares() {
            threats |= attack_gen.rook_attacks(occ, sq);
        }

        for sq in kings.squares() {
            threats |= attack_gen.king_attacks(sq);
        }
        threats
    }

    #[inline]
    pub fn attacked_by(targets: Bitboard, occ: Bitboard, bd: &Board) -> Bitboard {
        if targets.is_empty() {
            return Bitboard::empty();
        }
        let pawns = bd.pawns();
        let knights = bd.knights();
        let bishops = bd.bishops();
        let rooks = bd.rooks();
        let queens = bd.queens();
        let kings = bd.kings();

        let attack_gen = PreCalc::instance();
        let white = attack_gen.pawn_attackers(targets, Color::White) & pawns & bd.white();
        let black = attack_gen.pawn_attackers(targets, Color::Black) & pawns & bd.black();
        let mut attackers = white | black;

        for each in targets.iter() {
            let sq = each.square();
            attackers |= attack_gen.knight_attacks(sq) & knights
                | attack_gen.king_attacks(sq) & kings
                | attack_gen.bishop_attacks(occ, sq) & (bishops | queens)
                | attack_gen.rook_attacks(occ, sq) & (rooks | queens);
        }
        // debug!("{}target\n{}attackers\n{}", board, targets, attackers);

        attackers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::*, globals::constants::*, infra::profiler::PerfProfiler, mv::Move, other::Perft,
    };
    use std::hint::black_box;
    use test_log::test;

    #[test]
    fn test_threats_to() {
        let board = Board::parse_fen("k5r1/3q1p2/4b2r/1n6/6pp/b2N3n/8/K1QR4 w - - 0 1").unwrap();
        let bb = BoardCalcs::all_attacks_on(&board, Color::White, board.occupied());
        println!("{}", !bb);
        let ans = a1 | b1 | d1 | e1 | f1 | h1 | c2 | d2 | e2 | g2;
        let ans = ans | h2 | e3 | a4 | e4 | a5 | e5 | a6 | b6 | h6 | g8;
        assert_eq!(!bb, ans);
    }

    #[test]
    fn test_attacked_by() {
        let board = Board::parse_fen("5Q2/8/7p/4P1p1/8/3NK1P1/8/8 w - - 0 1").unwrap();
        let bb = BoardCalcs::attacked_by(f4, board.white() | board.black(), &board);
        println!("{bb}");
        assert_eq!(bb, g3 | g5 | e3 | d3 | f8);
    }

    #[test]
    fn test_pinned() {
        for epd in Catalog::pins() {
            let pins = BoardCalcs::pinned_and_discoverers(&epd.board(), epd.board().color_us()).0;
            assert_eq!(pins, epd.bitboard("Sq").unwrap(), "{epd}");
        }
    }

    #[test]
    fn test_discovered_check() {
        let positions = Catalog::discovered_check();
        for epd in positions {
            let discoverers =
                BoardCalcs::pinned_and_discoverers(&epd.board(), epd.board().color_us()).1;
            assert_eq!(discoverers, epd.bitboard("Sq").unwrap(), "{epd}");
            let discoverers =
                BoardCalcs::pinned_and_discoverers(&epd.board(), epd.board().color_us()).1;
            assert_eq!(discoverers, epd.bitboard("Sq").unwrap(), "{epd}");
        }
    }

    #[test]
    fn bench_pins_and_disc() {
        let mut prof = PerfProfiler::new("bench_pins_and_disc");
        for pos in Catalog::win_at_chess() {
            let bd = pos.board();
            prof.bench(|| BoardCalcs::pinned_and_discoverers(black_box(&bd), bd.color_us()));
        }
    }

    #[test]
    fn bench_calcs() {
        let mut starting_pos = Catalog::perft_kiwipete().0;
        let mut pinned_calc = PerfProfiler::new("boardcalcs: pinned calc");
        let mut pinned_cached = PerfProfiler::new("boardcalcs: pinned cached");
        let mut discoverer_calc = PerfProfiler::new("boardcalcs: discoverer calc");
        let mut discoverer_cached = PerfProfiler::new("boardcalcs: discoverer cached");
        let mut checkers_of_calc = PerfProfiler::new("boardcalcs: checkers_of calc");
        let mut checkers_of_cached = PerfProfiler::new("boardcalcs: checkers_of cached");
        let mut threats_to_calc = PerfProfiler::new("boardcalcs: threats_to calc");
        let mut threats_to_cached = PerfProfiler::new("boardcalcs: threats_to cached");

        let mut func = |bd: &Board, mv: Move| {
            let bd = black_box(bd.make_move(mv));
            pinned_calc.bench(|| bd.pinned(Color::White));
            pinned_cached.bench(|| bd.pinned(Color::White));

            discoverer_calc.bench(|| bd.discoverer(Color::White));
            discoverer_cached.bench(|| bd.discoverer(Color::White));

            checkers_of_calc.bench(|| bd.checkers_of(Color::White));
            checkers_of_cached.bench(|| bd.checkers_of(Color::White));

            threats_to_calc.bench(|| bd.all_attacks_on(Color::White));
            threats_to_cached.bench(|| bd.all_attacks_on(Color::White));
        };
        const BULK_COUNT: bool = true;
        Perft::<BULK_COUNT>::perft_with(&mut starting_pos, 2, &mut func);
    }
}
