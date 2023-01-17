use crate::{
    bits::{
        bitboard::{Bitboard, Dir},
        castling::CastlingRights,
        precalc::PreCalc,
        square::Square,
    },
    boards::{boardcalcs::BoardCalcs, Board},
    infra::metric::Metrics,
    movelist::MoveList,
    mv::Move,
    piece::Piece,
    search::node::Timing,
};

pub struct Moves {
    pub piece:      Piece,
    pub from:       Square,
    pub to:         Bitboard,
    pub castle:     bool,
    pub promo:      bool,
    pub en_passant: bool,
}

impl Moves {
    pub fn len(&self) -> usize {
        self.to.popcount() as usize
            * match self.promo {
                true => 4,
                false => 1,
            }
    }
}

#[derive(Debug)]
pub struct LegalMoves<'a, F: FnMut(&mut Moves)> {
    board: &'a Board,
    filt:  F,
    list:  &'a mut MoveList,
}

impl<'a, F> LegalMoves<'a, F>
where
    F: FnMut(&mut Moves),
{
    pub fn new(board: &'a Board, _mask: Bitboard, filt: F, list: &'a mut MoveList) -> Self {
        let mut me = Self { board, filt, list };
        me.generate();
        me
    }

    // pub fn append_to_movelist(&self, moves: &mut MoveList) {
    //     for &(p, from, bb) in self.legal_moves.iter() {
    //         if p == Piece::Pawn {
    //             #[allow(clippy::collapsible_else_if)]
    //             if Bitboard::RANKS_18.contains(bb) {
    //                 for to in bb.squares() {
    //                     if to.is_in(self.board.them()) {
    //                         // try and pre-sort promos by likely usefulness
    //                         // let cap = self.board.piece_unchecked(to);
    //                         moves.push(Move::new_promo_capture(from, to, Piece::Queen, self.board));
    //                         moves.push(Move::new_promo_capture(
    //                             from,
    //                             to,
    //                             Piece::Knight,
    //                             self.board,
    //                         ));
    //                         moves.push(Move::new_promo_capture(from, to, Piece::Rook, self.board));
    //                         moves.push(Move::new_promo_capture(
    //                             from,
    //                             to,
    //                             Piece::Bishop,
    //                             self.board,
    //                         ));
    //                     } else {
    //                         moves.push(Move::new_promo(from, to, Piece::Queen));
    //                         moves.push(Move::new_promo(from, to, Piece::Knight));
    //                         moves.push(Move::new_promo(from, to, Piece::Rook));
    //                         moves.push(Move::new_promo(from, to, Piece::Bishop));
    //                     }
    //                 }
    //             } else {
    //                 for to in bb.squares() {
    //                     moves.push(Move::new_pawn_move(from, to, self.board));
    //                 }
    //             }
    //         } else if p == Piece::King && from.is_in((bb << 2) | (bb >> 2)) {
    //             moves.push(Move::new_castle(
    //                 from,
    //                 bb.first_square(),
    //                 CastlingRights::NONE,
    //             ));
    //         } else {
    //             for to in bb.squares() {
    //                 if to.is_in(self.board.them()) {
    //                     moves.push(Move::new_capture(p, from, to, self.board));
    //                 } else {
    //                     moves.push(Move::new_quiet(p, from, to));
    //                 }
    //             }
    //         }
    //     }
    // }

    // fn capture_to_movelist<'a>(
    //     moves: &'a mut MoveList,
    //     bd: &'a Board,
    // ) -> impl FnMut(Piece, Square, Bitboard) + 'a {
    //     |p, from, bb| {
    //         if p != Piece::Pawn {
    //             for to in bb.squares() {
    //                 if to.is_in(bd.them()) {
    //                     moves.push(Move::new_capture(p, from, to, bd));
    //                 } else {
    //                     moves.push(Move::new_quiet(p, from, to));
    //                 }
    //             }
    //         } else {
    //             #[allow(clippy::collapsible_else_if)]
    //             if Bitboard::RANKS_18.contains(bb) {
    //                 for to in bb.squares() {
    //                     if to.is_in(bd.them()) {
    //                         // try and pre-sort promos by likely usefulness
    //                         // let cap = bd.piece_unchecked(to);
    //                         moves.push(Move::new_promo_capture(from, to, Piece::Queen, bd));
    //                         moves.push(Move::new_promo_capture(from, to, Piece::Knight, bd));
    //                         moves.push(Move::new_promo_capture(from, to, Piece::Rook, bd));
    //                         moves.push(Move::new_promo_capture(from, to, Piece::Bishop, bd));
    //                     } else {
    //                         moves.push(Move::new_promo(from, to, Piece::Queen));
    //                         moves.push(Move::new_promo(from, to, Piece::Knight));
    //                         moves.push(Move::new_promo(from, to, Piece::Rook));
    //                         moves.push(Move::new_promo(from, to, Piece::Bishop));
    //                     }
    //                 }
    //             } else {
    //                 for to in bb.squares() {
    //                     moves.push(Move::new_pawn_move(from, to, bd));
    //                 }
    //             }
    //         }
    //     }
    // }

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

    pub fn king_legal(&mut self) {
        let bd = self.board;
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

        let mut moves = Moves {
            piece:      Piece::King,
            from:       king_sq,
            to:         attacks,
            castle:     false,
            en_passant: false,
            promo:      false,
        };
        (self.filt)(&mut moves);
        for to in moves.to.squares() {
            if to.is_in(them) {
                self.list
                    .push(Move::new_capture(Piece::King, king_sq, to, bd));
            } else {
                self.list.push(Move::new_quiet(Piece::King, king_sq, to));
            }
        }
    }

    pub fn generate(&mut self) {
        let t = Metrics::timing_start();
        let b = self.board;
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
            return self.king_legal();
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
                    let attacks = gen.attacks(b.color_us(), p, us, them, fr) & !us;
                    if !fr.is_in(pinned) {
                        // all non pinned pieces
                        // DONE
                        let mut moves = Moves {
                            piece:      p,
                            from:       fr,
                            to:         attacks,
                            castle:     false,
                            en_passant: false,
                            promo:      false,
                        };
                        (self.filt)(&mut moves);
                        self.add_moves(moves.to, p, fr, b);
                    } else {
                        // Pinned -> psuedo-to in ray along king and pinner incl
                        let blocking = gen.line_through(fr, king_sq);
                        // will exlude knights anyway
                        // DONE
                        let mut moves = Moves {
                            piece:      p,
                            from:       fr,
                            to:         attacks & blocking,
                            castle:     false,
                            en_passant: false,
                            promo:      false,
                        };
                        (self.filt)(&mut moves);
                
                        self.add_moves(moves.to, p, fr, b);
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
        let checkers = bd.checkers_of(us);
        if checkers.popcount() == 1 {
            // any non-pinned pawn can capture the checker
            if capture_sq == checkers {
                let fr_e = to.shift(them.pawn_capture_west());
                if (fr_e & bd.pawns() & bd.us() & !bd.pinned(us)).any() {
                    self.list
                        .push(Move::new_ep_capture(fr_e.square(), to.square(), bd));
                }
                let fr_w = to.shift(them.pawn_capture_east());
                if (fr_w & bd.pawns() & bd.us() & !bd.pinned(us)).any() {
                    self.list
                        .push(Move::new_ep_capture(fr_w.square(), to.square(), bd));
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
                    self.list.push(m);
                }
            }
        }
    }

    #[inline]
    // DONE
    fn add_moves(&mut self, dests: Bitboard, p: Piece, fr: Square, bd: &Board) {
        if p != Piece::Pawn {
            for to in dests.squares() {
                if to.is_in(bd.them()) {
                    self.list.push(Move::new_capture(p, fr, to, bd))
                } else {
                    self.list.push(Move::new_quiet(p, fr, to))
                }
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if Bitboard::RANKS_18.contains(dests) {
                self.add_moves_pawn_promo(dests, fr, bd);
            } else {
                for to in dests.squares() {
                    self.list.push(Move::new_pawn_move(fr, to, bd));
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
                let _cap = bd.piece_unchecked(to);
                self.list
                    .push(Move::new_promo_capture(fr, to, Piece::Queen, bd));
                self.list
                    .push(Move::new_promo_capture(fr, to, Piece::Knight, bd));
                self.list
                    .push(Move::new_promo_capture(fr, to, Piece::Rook, bd));
                self.list
                    .push(Move::new_promo_capture(fr, to, Piece::Bishop, bd));
            } else {
                self.list.push(Move::new_promo(fr, to, Piece::Queen));
                self.list.push(Move::new_promo(fr, to, Piece::Knight));
                self.list.push(Move::new_promo(fr, to, Piece::Rook));
                self.list.push(Move::new_promo(fr, to, Piece::Bishop));
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
            if BoardCalcs::attacked_by(king_moves, occ, b).disjoint(them) {
                let m = Move::new_castle(
                    king_sq,
                    king_to.square(),
                    // king_to.square().shift(Dir::E),
                    // rook_to.square(),
                    right,
                );
                self.list.push(m);
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
                self.list.push(m);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_king_legal() {
        let b = Board::parse_piece_placement("rk6/8/8/8/8/8/8/1K6").unwrap();
        let mut moves = MoveList::new();
        let lm = LegalMoves::new(&b, Bitboard::all(), |_| {}, &mut moves);
        moves.retain(|m| b.piece(m.from()) == Some(Piece::King));
        assert_eq!(moves.sort().to_string(), "b1b2, b1c1, b1c2");

        let b = Board::parse_piece_placement("rk6/8/8/8/8/8/K7/8").unwrap();
        let mut moves = MoveList::new();
        let lm = LegalMoves::new(&b, Bitboard::all(), |_| {}, &mut moves);
        // xray prevents a2a1
        assert_eq!(moves.sort().to_string(), "a2b1, a2b2, a2b3");
    }
}
