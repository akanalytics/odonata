use crate::bits::bitboard::Bitboard;
use crate::bits::precalc::PreCalc;
use crate::board::Board;
use crate::types::Color;

pub struct BoardCalcs {}

impl BoardCalcs {
    // no king on board => no attackers
    #[inline]
    pub fn checkers_of(board: &Board, king_color: Color) -> Bitboard {
        let us = board.color(king_color);
        let them = board.color(king_color.opposite());
        let our_king = board.kings() & us;
        // debug_assert!(!our_king.is_empty(), "king ({}) not found {}", king_color, board);
        let occ = us | them;
        Self::attacked_by(our_king, occ, board) & them
    }

    #[inline]
    pub fn pinned_and_discoverers(b: &Board, king_color: Color) -> (Bitboard, Bitboard) {
        let kings = b.kings() & b.color(king_color);
        if kings.is_empty() {
            return Default::default();
        };
        let color_us = b.color(king_color);
        let color_them = b.color(king_color.opposite());
        let king_sq = kings.square();

        let xray_checkers = Self::attacked_by(kings, Bitboard::EMPTY, b) & color_them;
        let mut pinned = Bitboard::empty();
        let mut discoverers = Bitboard::empty();
        for ch in xray_checkers.squares() {
            let ray = PreCalc::default().strictly_between(ch, king_sq);
            if ray.disjoint(color_them) && (ray & color_us).popcount() == 1 {
                pinned |= ray & color_us;
            } else if ray.disjoint(color_us) && (ray & color_them).popcount() == 1 {
                discoverers |= ray & color_them;
            }
        }
        (pinned, discoverers)
    }

    /// all attacks
    #[inline]
    pub fn all_attacks_on(board: &Board, us: Color, occ: Bitboard) -> Bitboard {
        let opponent = us.opposite();
        let pawns = board.pawns() & board.color(opponent);
        let knights = board.knights() & board.color(opponent);
        let bishops = board.bishops() & board.color(opponent);
        let rooks = board.rooks() & board.color(opponent);
        let queens = board.queens() & board.color(opponent);
        let kings = board.kings() & board.color(opponent);

        let attack_gen = PreCalc::default();
        let (east, west) = attack_gen.pawn_attacks(pawns, opponent);
        let mut threats = east | west;

        for p in knights.iter() {
            let sq = p.square();
            threats |= attack_gen.knight_attacks(sq);
        }
        for p in (bishops | queens).iter() {
            let sq = p.square();
            threats |= attack_gen.bishop_attacks(occ, sq);
        }

        for p in (rooks | queens).iter() {
            let sq = p.square();
            threats |= attack_gen.rook_attacks(occ, sq);
        }

        for p in kings.iter() {
            let sq = p.square();
            threats |= attack_gen.king_attacks(sq);
        }
        threats
    }

    #[inline]
    pub fn attacked_by(targets: Bitboard, occ: Bitboard, board: &Board) -> Bitboard {
        if targets.is_empty() {
            return Bitboard::empty();
        }
        let pawns = board.pawns();
        let knights = board.knights();
        let bishops = board.bishops();
        let rooks = board.rooks();
        let queens = board.queens();
        let kings = board.kings();

        let attack_gen = PreCalc::default();
        let white = attack_gen.pawn_attackers(targets, Color::White) & pawns & board.white();
        let black = attack_gen.pawn_attackers(targets, Color::Black) & pawns & board.black();
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

    // fn attacked_by_colorX(targets: Bitboard, occ: Bitboard, board: &Board, opponent: Color) -> Bitboard {
    //     let pawns = board.pawns() & board.color(opponent);
    //     let knights = board.knights() & board.color(opponent);
    //     let bishops = board.bishops() & board.color(opponent);
    //     let rooks = board.rooks() & board.color(opponent);
    //     let queens = board.queens() & board.color(opponent);
    //     let kings = board.kings() & board.color(opponent);

    //     let attack_gen = global_classical_bitboard();
    //     let (east, west) = attack_gen.pawn_attacks(pawns, opponent);
    //     let mut attackers = (east | west) & targets;

    //     for each in targets.iter() {
    //         let sq = each.first_square();
    //         attackers |= attack_gen.knight_attacks(sq) & knights
    //             | attack_gen.king_attacks(sq) & kings
    //             | attack_gen.bishop_attacks(occ, sq) & (bishops | queens)
    //             | attack_gen.rook_attacks(occ, sq) & (rooks | queens);
    //         // TODO: en passant!!
    //     }
    //     debug!("opponent:{}\n{}target\n{}attackers\n{}", opponent, board, targets, attackers);

    //     attackers
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::board::*;
    use crate::catalog::*;
    use crate::globals::constants::*;

    #[test]
    fn test_threats_to() {
        let board = Board::parse_fen("k5r1/3q1p2/4b2r/1n6/6pp/b2N3n/8/K1QR4 w - - 0 1")
            .unwrap()
            .as_board();
        let bb = BoardCalcs::all_attacks_on(&board, Color::White, board.occupied());
        println!("{}", !bb);
        assert_eq!(
            !bb,
            a1 | b1
                | d1
                | e1
                | f1
                | h1
                | c2
                | d2
                | e2
                | g2
                | h2
                | e3
                | a4
                | e4
                | a5
                | e5
                | a6
                | b6
                | h6
                | g8
        );
    }

    #[test]
    fn test_attacked_by() {
        let board = Board::parse_fen("5Q2/8/7p/4P1p1/8/3NK1P1/8/8 w - - 0 1")
            .unwrap()
            .as_board();
        let bb = BoardCalcs::attacked_by(f4, board.white() | board.black(), &board);
        println!("{}", bb);
        assert_eq!(bb, g3 | g5 | e3 | d3 | f8);
    }

    #[test]
    fn test_pinned() {
        let positions = Catalog::pins();
        for pos in positions {
            let pins = BoardCalcs::pinned_and_discoverers(pos.board(), pos.board().color_us()).0;
            assert_eq!(pins, pos.sq().unwrap(), "{}", pos.board());
        }
    }

    #[test]
    fn test_discovered_check() {
        let positions = Catalog::discovered_check();
        for pos in positions {
            let discoverers = BoardCalcs::pinned_and_discoverers(pos.board(), pos.board().color_us()).1;
            assert_eq!(discoverers, pos.sq().unwrap(), "{}", pos.board());
            let discoverers = BoardCalcs::pinned_and_discoverers(pos.board(), pos.board().color_us()).1;
            assert_eq!(discoverers, pos.sq().unwrap(), "{}", pos.board());
        }
    }
}
