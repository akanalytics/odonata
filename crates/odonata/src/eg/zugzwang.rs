use serde::{Deserialize, Serialize};

use crate::{boards::Board, PreCalc};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Zugzwang {
    #[default]
    Disabled,
    NonPawn,
    NonPawnOrFreePawn,
    NonPawnOrKingSpace,
    NonPawnNonPinned,
    NonPawnOrPawnMove1,
    NonPawnOrPawnMove2,
    NonPawnNonPinnedOrFreePawn,
    NonPawnNonPinnedOrPawnMove1,
    NonPawnNonPinnedOrPawnMove2,
}

impl Zugzwang {
    pub fn is_maybe_zugzwang(&self, b: &Board) -> bool {
        match self {
            Zugzwang::Disabled => false,
            Zugzwang::NonPawn => ((b.line_pieces() | b.knights()) & b.us()).is_empty(),
            Zugzwang::NonPawnNonPinned => {
                (((b.line_pieces() | b.knights()) & b.us()) - b.pinned(b.color_us())).is_empty()
            }
            Zugzwang::NonPawnNonPinnedOrFreePawn => {
                Self::NonPawnNonPinned.is_maybe_zugzwang(b) && {
                    let their_pawns = b.pawns() & b.them();
                    let their_king_sq = (b.kings() & b.them()).square();
                    let their_atts = their_pawns.shift(b.color_them().pawn_capture_east())
                        | their_pawns.shift(b.color_them().pawn_capture_west())
                        | PreCalc::instance().king_attacks(their_king_sq);
                    let our_pawns = b.pawns() & b.us();
                    let our_pawn_defenders = our_pawns
                        & (our_pawns.shift(b.color_them().pawn_capture_east())
                            | our_pawns.shift(b.color_them().pawn_capture_west()));
                    let our_free_pawns = our_pawns - our_pawn_defenders;
                    let our_pawn_moves =
                        our_free_pawns.shift(b.color_us().forward()) - b.occupied();
                    let our_safe_pawn_moves = our_pawn_moves - their_atts;
                    our_safe_pawn_moves.popcount() < 1
                }
            }
            Zugzwang::NonPawnOrFreePawn => {
                Self::NonPawn.is_maybe_zugzwang(b) && {
                    let their_pawns = b.pawns() & b.them();
                    let their_king_sq = (b.kings() & b.them()).square();
                    let their_atts = their_pawns.shift(b.color_them().pawn_capture_east())
                        | their_pawns.shift(b.color_them().pawn_capture_west())
                        | PreCalc::instance().king_attacks(their_king_sq);
                    let our_pawns = b.pawns() & b.us();
                    let our_pawn_defenders = our_pawns
                        & (our_pawns.shift(b.color_them().pawn_capture_east())
                            | our_pawns.shift(b.color_them().pawn_capture_west()));
                    let our_free_pawns = our_pawns - our_pawn_defenders;
                    let our_pawn_moves =
                        our_free_pawns.shift(b.color_us().forward()) - b.occupied();
                    let our_safe_pawn_moves = our_pawn_moves - their_atts;
                    our_safe_pawn_moves.popcount() < 1
                }
            }
            Zugzwang::NonPawnNonPinnedOrPawnMove1 => {
                Self::NonPawnNonPinned.is_maybe_zugzwang(b) && {
                    let their_pawns = b.pawns() & b.them();
                    let their_king_sq = (b.kings() & b.them()).square();
                    let their_atts = their_pawns.shift(b.color_them().pawn_capture_east())
                        | their_pawns.shift(b.color_them().pawn_capture_west())
                        | PreCalc::instance().king_attacks(their_king_sq);
                    let our_pawn_moves =
                        (b.pawns() & b.us()).shift(b.color_us().forward()) - b.occupied();
                    let our_safe_pawn_moves = our_pawn_moves - their_atts;
                    our_safe_pawn_moves.popcount() < 1
                }
            }
            Zugzwang::NonPawnNonPinnedOrPawnMove2 => {
                Self::NonPawnNonPinned.is_maybe_zugzwang(b) && {
                    let their_pawns = b.pawns() & b.them();
                    let their_king_sq = (b.kings() & b.them()).square();
                    let their_atts = their_pawns.shift(b.color_them().pawn_capture_east())
                        | their_pawns.shift(b.color_them().pawn_capture_west())
                        | PreCalc::instance().king_attacks(their_king_sq);
                    let our_pawn_moves =
                        (b.pawns() & b.us()).shift(b.color_us().forward()) - b.occupied();
                    let our_safe_pawn_moves = our_pawn_moves - their_atts;
                    our_safe_pawn_moves.popcount() < 2
                }
            }
            Zugzwang::NonPawnOrPawnMove1 => {
                Self::NonPawn.is_maybe_zugzwang(b) && {
                    let their_pawns = b.pawns() & b.them();
                    let their_king_sq = (b.kings() & b.them()).square();
                    let their_atts = their_pawns.shift(b.color_them().pawn_capture_east())
                        | their_pawns.shift(b.color_them().pawn_capture_west())
                        | PreCalc::instance().king_attacks(their_king_sq);
                    let our_pawn_moves =
                        (b.pawns() & b.us()).shift(b.color_us().forward()) - b.occupied();
                    let our_safe_pawn_moves = our_pawn_moves - their_atts;
                    our_safe_pawn_moves.popcount() < 1
                }
            }
            Zugzwang::NonPawnOrPawnMove2 => {
                Self::NonPawn.is_maybe_zugzwang(b) && {
                    let their_pawns = b.pawns() & b.them();
                    let their_king_sq = (b.kings() & b.them()).square();
                    let their_atts = their_pawns.shift(b.color_them().pawn_capture_east())
                        | their_pawns.shift(b.color_them().pawn_capture_west())
                        | PreCalc::instance().king_attacks(their_king_sq);
                    let our_pawn_moves =
                        (b.pawns() & b.us()).shift(b.color_us().forward()) - b.occupied();
                    let our_safe_pawn_moves = our_pawn_moves - their_atts;
                    our_safe_pawn_moves.popcount() < 2
                }
            }
            Zugzwang::NonPawnOrKingSpace => {
                Self::NonPawn.is_maybe_zugzwang(b) && {
                    let our_king_sq = (b.kings() & b.us()).square();
                    let area =
                        PreCalc::instance().within_chebyshev_distance_inclusive(our_king_sq, 2);
                    (area & b.occupied()).popcount() > 1
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_log::test, Color, Position};

    #[test]
    fn test_zugzwang_lone_king() {
        let mut b = Position::parse_epd(
            r"
            k.......
            ........
            ........
            ........
            ........
            ........
            ........
            KN...... b - - 1 1",
        )
        .unwrap()
        .board()
        .clone();
        assert_eq!(Zugzwang::Disabled.is_maybe_zugzwang(&b), false);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnNonPinned.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnOrPawnMove1.is_maybe_zugzwang(&b), true);
        b.set_turn(Color::White);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), false);
    }

    #[test]
    fn test_zugzwang_pinned() {
        let mut b = Position::parse_epd(
            r"
            kn.....R
            ........
            ........
            ........
            ........
            ........
            ........
            KN...... b - - 1 1",
        )
        .unwrap()
        .board()
        .clone();
        assert_eq!(Zugzwang::Disabled.is_maybe_zugzwang(&b), false);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), false);
        assert_eq!(Zugzwang::NonPawnNonPinned.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnOrPawnMove1.is_maybe_zugzwang(&b), false);
        b.set_turn(Color::White);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), false);
    }

    #[test]
    fn test_zugzwang_unblocked_pawn() {
        let mut b = Position::parse_epd(
            r"
            k.......
            ........
            p.......
            ........
            ........
            ........
            ........
            KN...... b - - 1 1",
        )
        .unwrap()
        .board()
        .clone();
        assert_eq!(Zugzwang::Disabled.is_maybe_zugzwang(&b), false);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnNonPinned.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnOrPawnMove1.is_maybe_zugzwang(&b), false);
        b.set_turn(Color::White);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), false);
    }

    #[test]
    fn test_zugzwang_blocked_pawn() {
        let mut b = Position::parse_epd(
            r"
            k.......
            ........
            p.......
            P.......
            ........
            ........
            ........
            KN...... b - - 1 1",
        )
        .unwrap()
        .board()
        .clone();
        assert_eq!(Zugzwang::Disabled.is_maybe_zugzwang(&b), false);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnNonPinned.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnOrPawnMove1.is_maybe_zugzwang(&b), true);
        b.set_turn(Color::White);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), false);
    }

    #[test]
    fn test_zugzwang_attacked_pawn_stop() {
        let mut b = Position::parse_epd(
            r"
            k.......
            ........
            p.......
            ........
            .P......
            ........
            ........
            KN...... b - - 1 1",
        )
        .unwrap()
        .board()
        .clone();
        assert_eq!(Zugzwang::Disabled.is_maybe_zugzwang(&b), false);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnNonPinned.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnOrPawnMove1.is_maybe_zugzwang(&b), true);
        b.set_turn(Color::White);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), false);
    }

    #[test]
    fn test_zugzwang_king_space() {
        let mut b = Position::parse_epd(
            r"
            k.......
            ........
            .P......
            ........
            ........
            ........
            ........
            K....... b - - 1 1",
        )
        .unwrap()
        .board()
        .clone();
        assert_eq!(Zugzwang::Disabled.is_maybe_zugzwang(&b), false);
        assert_eq!(Zugzwang::NonPawn.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnNonPinned.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnOrPawnMove1.is_maybe_zugzwang(&b), true);
        assert_eq!(Zugzwang::NonPawnOrKingSpace.is_maybe_zugzwang(&b), true);
        b.set_turn(Color::White);
        assert_eq!(Zugzwang::NonPawnOrKingSpace.is_maybe_zugzwang(&b), false);
    }
}
