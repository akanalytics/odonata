use crate::bits::bitboard::{Bitboard, Dir};
use crate::bits::castling::CastlingRights;
use crate::bits::precalc::PreCalc;
use crate::bits::square::Square;
use crate::boards::boardcalcs::BoardCalcs;
use crate::boards::Board;
use crate::infra::metric::Metrics;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::piece::Piece;
use crate::search::node::Timing;

pub struct Rules;

fn capture_to_movelist<'a>(
    moves: &'a mut MoveList,
    bd: &'a Board,
) -> impl FnMut(Piece, Square, Bitboard) + 'a {
    |p, from, bb| {
        if p != Piece::Pawn {
            for to in bb.squares() {
                if to.is_in(bd.them()) {
                    moves.push(Move::new_capture(p, from, to, bd));
                } else {
                    moves.push(Move::new_quiet(p, from, to));
                }
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if Bitboard::RANKS_18.contains(bb) {
                for to in bb.squares() {
                    if to.is_in(bd.them()) {
                        // try and pre-sort promos by likely usefulness
                        // let cap = bd.piece_unchecked(to);
                        moves.push(Move::new_promo_capture(from, to, Piece::Queen, bd));
                        moves.push(Move::new_promo_capture(from, to, Piece::Knight, bd));
                        moves.push(Move::new_promo_capture(from, to, Piece::Rook, bd));
                        moves.push(Move::new_promo_capture(from, to, Piece::Bishop, bd));
                    } else {
                        moves.push(Move::new_promo(from, to, Piece::Queen));
                        moves.push(Move::new_promo(from, to, Piece::Knight));
                        moves.push(Move::new_promo(from, to, Piece::Rook));
                        moves.push(Move::new_promo(from, to, Piece::Bishop));
                    }
                }
            } else {
                for to in bb.squares() {
                    moves.push(Move::new_pawn_move(from, to, bd));
                }
            }
        }
    }
}

impl Rules {
    // pub fn pseudo_legals(b: &Board, moves: &mut MoveList) {
    //     Rules::pawn_captures_incl_promo(b, moves);
    //     Rules::pawn_promos(b, moves);
    //     Rules::pawn_push(b, moves);
    //     // Rules::pawn_capture_promos(b, moves);
    //     Rules::non_pawn(Piece::Knight, b, moves);
    //     Rules::non_pawn(Piece::Bishop, b, moves);
    //     Rules::non_pawn(Piece::Rook, b, moves);
    //     Rules::non_pawn(Piece::Queen, b, moves);
    //     // Rules::non_pawn(Piece::King,b,  moves);
    //     Rules::king_legal(b, moves);
    //     Rules::castles(b, moves);
    // }

    pub fn king_legal(bd: &Board, moves: &mut MoveList) {
        let us = bd.us();
        let our_kings = bd.kings() & us;
        if our_kings.is_empty() {
            return;
        }
        let attack_gen = PreCalc::default();
        let them = bd.them();
        let occ = bd.occupied();
        let king_sq = (bd.kings() & us).square();
        let king_att = attack_gen.king_attacks(king_sq);
        let king_danger = BoardCalcs::all_attacks_on(bd, bd.color_us(), occ - our_kings);
        let attacks = (king_att & !us) - king_danger;

        // DONE
        for to in attacks.squares() {
            if to.is_in(them) {
                moves.push(Move::new_capture(Piece::King, king_sq, to, bd));
            } else {
                moves.push(Move::new_quiet(Piece::King, king_sq, to));
            }
        }
    }

    pub fn legals_for(b: &Board, moves: &mut MoveList) {
        let t = Metrics::timing_start();
        // profile_method!("move generation");
        // let _g = hprof::enter("move generation");

        let us = b.us();
        let them = b.them();
        let our_kings = b.kings() & us;
        // if our_kings.is_empty() {
        //     return Self::pseudo_legals(b, moves);
        // }
        let checkers = b.checkers_of(b.color_us());
        if checkers.popcount() >= 2 {
            return Self::king_legal(b, moves);
        }
        let the_checker = checkers.square();
        let gen = PreCalc::default();
        if checkers.popcount() == 1 {
            let king_sq = our_kings.square();
            // Captr checker -> capture-sq & piece not pinned & (checker)
            // Block checker -> to & piece not pinned & (xrays to checker excl)
            let blocking = gen.between(king_sq, the_checker) | checkers; // "| checkers" is for knight checkers
            for &p in Piece::ALL_BAR_KING.iter() {
                for fr in (b.pieces(p) & us & !b.pinned(b.color_us())).squares() {
                    let attacks = blocking & gen.attacks(b.color_us(), p, us, them, fr) & !us;
                    // DONE
                    Self::add_moves(attacks, p, fr, b, moves);
                }
            }
            Self::add_moves_en_passant(b, moves);
            Self::king_legal(b, moves);
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
                    let attacks = gen.attacks(b.color_us(), p, us, them, fr) & !us;
                    if !fr.is_in(pinned) {
                        // all non pinned pieces
                        // DONE
                        Self::add_moves(attacks, p, fr, b, moves);
                    } else {
                        // Pinned -> psuedo-to in ray along king and pinner incl
                        let blocking = gen.line_through(fr, king_sq);
                        // will exlude knights anyway
                        // DONE
                        Self::add_moves(attacks & blocking, p, fr, b, moves);
                    }
                }
            }
            Self::add_moves_en_passant(b, moves);
            Self::king_legal(b, moves);
            Self::castles(b, moves);
        }
        Metrics::profile(t, Timing::TimingMoveGen);
    }

    pub fn add_moves_en_passant(bd: &Board, moves: &mut MoveList) {
        if bd.en_passant().is_empty() {
            return;
        }
        let them = bd.color_them();
        let to = bd.en_passant();
        let capture_sq = to.shift(them.forward());
        let checkers = bd.checkers_of(bd.color_us());
        if checkers.popcount() == 1 {
            // any non-pinned pawn can capture the checker
            if capture_sq == checkers {
                let fr_e = to.shift(them.pawn_capture_west());
                if (fr_e & bd.pawns() & bd.us() & !bd.pinned(bd.color_us())).any() {
                    moves.push(Move::new_ep_capture(fr_e.square(), to.square(), bd));
                }
                let fr_w = to.shift(them.pawn_capture_east());
                if (fr_w & bd.pawns() & bd.us() & !bd.pinned(bd.color_us())).any() {
                    moves.push(Move::new_ep_capture(fr_w.square(), to.square(), bd));
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
                if bd.is_legal_move(m) {
                    moves.push(m);
                }
            }
        }
    }

    #[inline]
    // DONE
    fn add_moves(dests: Bitboard, p: Piece, fr: Square, bd: &Board, moves: &mut MoveList) {
        if p != Piece::Pawn {
            for to in dests.squares() {
                if to.is_in(bd.them()) {
                    moves.push(Move::new_capture(p, fr, to, bd))
                } else {
                    moves.push(Move::new_quiet(p, fr, to))
                }
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if Bitboard::RANKS_18.contains(dests) {
                Self::add_moves_pawn_promo(dests, fr, bd, moves);
            } else {
                for to in dests.squares() {
                    moves.push(Move::new_pawn_move(fr, to, bd));
                }
            }
        }
    }

    #[inline]
    // DONE
    fn add_moves_pawn_promo(dests: Bitboard, fr: Square, bd: &Board, moves: &mut MoveList) {
        for to in dests.squares() {
            if to.is_in(bd.them()) {
                // try and pre-sort promos by likely usefulness
                let _cap = bd.piece_unchecked(to);
                moves.push(Move::new_promo_capture(fr, to, Piece::Queen, bd));
                moves.push(Move::new_promo_capture(fr, to, Piece::Knight, bd));
                moves.push(Move::new_promo_capture(fr, to, Piece::Rook, bd));
                moves.push(Move::new_promo_capture(fr, to, Piece::Bishop, bd));
            } else {
                moves.push(Move::new_promo(fr, to, Piece::Queen));
                moves.push(Move::new_promo(fr, to, Piece::Knight));
                moves.push(Move::new_promo(fr, to, Piece::Rook));
                moves.push(Move::new_promo(fr, to, Piece::Bishop));
            }
        }
    }

    // pub fn non_pawn(p: Piece, b: &Board, moves: &mut MoveList) {
    //     let attack_gen = PreCalc::default();
    //     let us = b.us();
    //     for fr in (b.pieces(p) & us).squares() {
    //         let attacks = attack_gen.non_pawn_attacks(b.color_us(), p, b.us(), b.them(), fr) & !us;
    //         Self::add_moves(attacks, p, fr, b, moves);
    //     }
    // }

    pub fn castles(b: &Board, moves: &mut MoveList) {
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
            if BoardCalcs::attacked_by(king_moves, occ, b).disjoint(them) {
                let m = Move::new_castle(
                    king_sq,
                    king_to.square(),
                    // king_to.square().shift(Dir::E),
                    // rook_to.square(),
                    right,
                );
                moves.push(m);
            }
        }

        let right = CastlingRights::queen_side_right(c);
        if rights.contains(right) && !CastlingRights::queen_side_move_squares(c).intersects(occ) {
            let rook_to = king.shift(Dir::W);
            let king_to = rook_to.shift(Dir::W);
            let king_moves = king | rook_to | king_to;
            if BoardCalcs::attacked_by(king_moves, occ, b).disjoint(them) {
                let king_to = king_to.square();
                // let rook_from = king_to.shift(Dir::W).shift(Dir::W);
                let m = Move::new_castle(
                    king_sq, king_to, // rook_from,
                    // rook_to.square(),
                    right,
                );
                moves.push(m);
            }
        }
    }

    // pub fn pawn_promos(b: &Board, moves: &mut MoveList) {
    //     let attack_gen = PreCalc::default();
    //     let c = b.color_us();
    //     let occ = b.occupied();
    //     let us = b.us();
    //     let pawn_push = attack_gen.pawn_pushes(occ, b.pawns() & us, c);
    //     for to in (pawn_push & Bitboard::PROMO_RANKS).squares() {
    //         let from = to.shift(c.forward().opposite());
    //         // try and pre-sort promos by likely usefulness
    //         moves.extend(
    //             [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop]
    //                 .iter()
    //                 .map(|&p| Move::new_promo(from, to, p)),
    //         );
    //     }
    // }

    // pub fn pawn_push(b: &Board, moves: &mut MoveList) {
    //     // non-promoted single-push pawns
    //     let attack_gen = PreCalc::default();
    //     let c = b.color_us();
    //     let occ = b.occupied();
    //     let us = b.us();
    //     let pawns = b.pawns() & us;
    //     let pawn_push = attack_gen.pawn_pushes(occ, pawns, c);
    //     for to in (pawn_push & !Bitboard::PROMO_RANKS).squares() {
    //         let behind = to.shift(c.forward().opposite());
    //         if !behind.is_in(pawns) {
    //             // must have been double push
    //             let ep = behind;
    //             let from = behind.shift(c.forward().opposite());
    //             let m = Move::new_double_push(from, to, ep);
    //             moves.push(m);
    //         } else {
    //             let m = Move::new_quiet(Piece::Pawn, behind, to);
    //             //  { from, to, ep, mover: Piece::Pawn, ..Default::default() };
    //             moves.push(m);
    //         }
    //     }
    // }

    // pub fn pawn_captures_incl_promo(b: &Board, moves: &mut MoveList) {
    //     let attack_gen = PreCalc::default();
    //     let c = b.color_us();
    //     let us = b.us();
    //     let them = b.them();
    //     let pawns = b.pawns() & us;

    //     let (pawn_captures_e, pawn_captures_w) = attack_gen.pawn_attacks(pawns, c);
    //     for to in (pawn_captures_e & them).squares() {
    //         let from = to.shift(c.pawn_capture_east().opposite());
    //         let captured = b.piece_at(to.as_bb());
    //         if to.is_in(Bitboard::PROMO_RANKS) {
    //             moves.extend(
    //                 [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop]
    //                     .iter()
    //                     .map(|&p| Move::new_promo_capture(from, to, p, captured)),
    //             );
    //         } else {
    //             let m = Move::new_capture(Piece::Pawn, from, to, captured);
    //             moves.push(m);
    //         }
    //     }
    //     for to in (pawn_captures_w & them).squares() {
    //         let from = to.shift(c.pawn_capture_west().opposite());
    //         let captured = b.piece_at(to.as_bb());
    //         if to.is_in(Bitboard::PROMO_RANKS) {
    //             moves.extend(
    //                 [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop]
    //                     .iter()
    //                     .map(|&p| Move::new_promo_capture(from, to, p, captured)),
    //             );
    //         } else {
    //             let m = Move::new_capture(Piece::Pawn, from, to, captured);
    //             moves.push(m);
    //         }
    //     }
    //     // e/p pawn_captures
    //     let ep = b.en_passant();
    //     if !ep.is_empty() {
    //         let ep = ep.square();
    //         if ep.is_in(pawn_captures_e) {
    //             let from = ep.shift(c.pawn_capture_east().opposite());
    //             let capture_square = ep.shift(c.opposite().forward());
    //             let m = Move::new_ep_capture(from, ep, capture_square);
    //             moves.push(m);
    //         }
    //         if ep.is_in(pawn_captures_w) {
    //             let from = ep.shift(c.pawn_capture_west().opposite());
    //             let capture_square = ep.shift(c.opposite().forward());
    //             let m = Move::new_ep_capture(from, ep, capture_square);
    //             moves.push(m);
    //         }
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_king_legal() {
        let b = Board::parse_piece_placement("rk6/8/8/8/8/8/8/1K6").unwrap();
        let mut moves = MoveList::new();
        Rules::king_legal(&b, &mut moves);
        assert_eq!(moves.sort().to_string(), "b1b2, b1c1, b1c2");
        let b = Board::parse_piece_placement("rk6/8/8/8/8/8/K7/8").unwrap();
        let mut moves = MoveList::new();
        Rules::king_legal(&b, &mut moves);
        // xray prevents a2a1
        assert_eq!(moves.sort().to_string(), "a2b1, a2b2, a2b3");
    }
}