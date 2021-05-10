use crate::attacks::{BitboardAttacks, ClassicalBitboard};
use crate::bitboard::{Bitboard, Dir};
use crate::board::makemove::MoveMaker;
use crate::board::movegen::{attacked_by, global_classical_bitboard};
use crate::board::Board;
use crate::globals::counts;
use crate::movelist::{Move, MoveExt, MoveList, MoveListExt};
use crate::types::{Color, Piece};
use crate::castling::CastlingRights;
use once_cell::sync::OnceCell;

pub struct Rules;


impl Board {
    pub fn pseudo_legal_moves_ext(&self) -> MoveListExt {
        Rules::pseudo_legal_moves_ext(self)
    }
}


impl Rules {

    pub fn pseudo_legal_moves_ext(b: &Board) -> MoveListExt {
        let mut moves = MoveListExt::new();
        let vec = &mut moves.moves;
        Rules::pawn_push(b, vec);
        Rules::pawn_promos(b, vec);
        Rules::pawn_captures_incl_promo(b, vec);
        Rules::pawn_capture_promos(b, vec);
        Rules::non_pawn(Piece::Knight, b, vec);
        Rules::non_pawn(Piece::Bishop, b, vec);
        Rules::non_pawn(Piece::Rook, b, vec);
        Rules::non_pawn(Piece::Queen,b,  vec);
        Rules::non_pawn(Piece::King, b, vec);
        Rules::castles(b, vec);
        moves
    }

    pub fn non_pawn(p: Piece, b: &Board, moves: &mut Vec<MoveExt>) {
        let attack_gen = global_classical_bitboard();
        let them = b.them();
        let us = b.us();
        let occ = b.occupied();

        for from_sq in (b.pieces(p) & us).squares() {
            let attacks = attack_gen.attacks(p, occ, from_sq) & !us;
            moves.extend(attacks.iter().map(|to| {
                if them.contains(to) {
                    MoveExt::new_capture(p, from_sq.as_bb(), to, b.piece_at(to))
                } else {
                    MoveExt::new_quiet(p, from_sq.as_bb(), to)
                }
            }))
        }
    }

    pub fn castles(b: &Board, moves: &mut Vec<MoveExt>) {
        let c = b.color_us();
        let them = b.them();
        let us = b.us();
        let occ = b.occupied();
        let king = b.kings() & us;
        let rights = b.castling();

        let right = CastlingRights::king_side_right(c);
        if rights.contains(right) && !CastlingRights::king_side_squares(c).intersects(occ) && !king.is_empty()
        {
            let rook_to = king.shift(Dir::E);
            let king_to = rook_to.shift(Dir::E);
            let king_moves = king | rook_to | king_to;
            if attacked_by(king_moves, occ, b).disjoint(them) {
                let m = MoveExt::new_castle(king, king_to, king_to.shift(Dir::E), rook_to, right);
                moves.push(m);
            }
        }

        let right = CastlingRights::queen_side_right(c);
        if rights.contains(right)
            && !CastlingRights::queen_side_squares(c).intersects(occ)
            && !king.is_empty()
        {
            let rook_to = king.shift(Dir::W);
            let king_to = rook_to.shift(Dir::W);
            let king_moves = king | rook_to | king_to;
            if attacked_by(king_moves, occ, b).disjoint(them) {
                let rook_from = king_to.shift(Dir::W).shift(Dir::W);
                let m = MoveExt::new_castle(king, king_to, rook_from, rook_to, right);
                moves.push(m);
            }
        }
    }

    pub fn pawn_promos(b: &Board, moves: &mut Vec<MoveExt>) {
        let attack_gen = global_classical_bitboard();
        let c = b.color_us();
        let occ = b.occupied();
        let us = b.us();
        let pawn_push = attack_gen.pawn_pushes(occ, b.pawns() & us, &c);
        for to in (pawn_push & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(c.pawn_move().opposite());
            // try and pre-sort promos by likely usefulness
            moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                MoveExt::new_promo(from, to, p)
            ));
        }
    }


    //     // pawn capture-promos
    //     for to in (pawn_captures_e & them & Bitboard::PROMO_RANKS).iter() {
    //         let from = to.shift(color.pawn_capture_east().opposite());
    //         let capture = board.piece_at(to);
    //         for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
    //             // MoveEnum::PromoCapture { to, from, promo, capture });
    //             let m = Move { from, to, mover: Piece::Pawn, capture, promo, ..Default::default() };
    //             moves.push(m);
    //         }
    //     }
    //     for to in (pawn_captures_w & them & Bitboard::PROMO_RANKS).iter() {
    //         let from = to.shift(color.pawn_capture_west().opposite());
    //         let capture = board.piece_at(to);
    //         for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
    //             // MoveEnum::PromoCapture { to, from, promo, capture });
    //             let m = Move { from, to, mover: Piece::Pawn, capture, promo, ..Default::default() };
    //             moves.push(m);
    //         }
    //     }




    pub fn pawn_push(b: &Board, moves: &mut Vec<MoveExt>) {
        // non-promoted single-push pawns
        let attack_gen = global_classical_bitboard();
        let c = b.color_us();
        let occ = b.occupied();
        let us = b.us();
        let pawns = b.pawns() & us;
        let pawn_push = attack_gen.pawn_pushes(occ, pawns, &c);
        for to in (pawn_push & !Bitboard::PROMO_RANKS).iter() {
            let behind = to.shift(c.pawn_move().opposite());
            if !pawns.contains(behind) {
                // must have been double push
                let ep = behind;
                let from = behind.shift(c.pawn_move().opposite());
                let m = MoveExt::new_double_push(from, to, ep);
                moves.push(m);
            } else {
                let m = MoveExt::new_quiet(Piece::Pawn, behind, to);
                // FIXME!!!! // { from, to, ep, mover: Piece::Pawn, ..Default::default() };
                moves.push(m);
            }
        }
    }


    pub fn pawn_capture_promos(b: &Board, moves: &mut Vec<MoveExt>) {
        let attack_gen = global_classical_bitboard();
        let c = b.color_us();
        let us = b.us();
        let them = b.them();
        let pawns = b.pawns() & us;

        let (pawn_captures_e, pawn_captures_w) = attack_gen.pawn_attacks(pawns, c);
        for to in (pawn_captures_e & them).iter() {
            let from = to.shift(c.pawn_capture_east().opposite());
            let captured = b.piece_at(to);
            if Bitboard::PROMO_RANKS.contains(to) {
                moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                    MoveExt::new_promo_capture(from, to, p, captured)
                ));
            }
            else {
                // let m = MoveExt::new_capture(Piece::Pawn, from, to, captured);
                // moves.push(m);
            }
        }
        for to in (pawn_captures_w & them).iter() {
            let from = to.shift(c.pawn_capture_west().opposite());
            let captured = b.piece_at(to);
            if Bitboard::PROMO_RANKS.contains(to) {
                moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                    MoveExt::new_promo_capture(from, to, p, captured)
                ));
            }
            else {
                // let m = MoveExt::new_capture(Piece::Pawn, from, to, captured);
                // moves.push(m);
            }
        }
    }

    pub fn pawn_captures_incl_promo(b: &Board, moves: &mut Vec<MoveExt>) {
        let attack_gen = global_classical_bitboard();
        let c = b.color_us();
        let us = b.us();
        let them = b.them();
        let pawns = b.pawns() & us;

        let (pawn_captures_e, pawn_captures_w) = attack_gen.pawn_attacks(pawns, c);
        for to in (pawn_captures_e & them).iter() {
            let from = to.shift(c.pawn_capture_east().opposite());
            let captured = b.piece_at(to);
            if Bitboard::PROMO_RANKS.contains(to) {
                // moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                //     MoveExt::new_promo_capture(from, to, p, captured)
                // ));
            }
            else {
                let m = MoveExt::new_capture(Piece::Pawn, from, to, captured);
                moves.push(m);
            }
        }
        for to in (pawn_captures_w & them).iter() {
            let from = to.shift(c.pawn_capture_west().opposite());
            let captured = b.piece_at(to);
            if Bitboard::PROMO_RANKS.contains(to) {
                // moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                //     MoveExt::new_promo_capture(from, to, p, captured)
                // ));
            }
            else {
                let m = MoveExt::new_capture(Piece::Pawn, from, to, captured);
                moves.push(m);
            }
        }
        // e/p pawn_captures
        let ep = b.en_passant();
        if ep.intersects(pawn_captures_e) {
            let from = ep.shift(c.pawn_capture_east().opposite());
            let capture_square = ep.shift(c.opposite().pawn_move());
            let m =MoveExt::new_ep_capture(from, ep, capture_square);
            moves.push(m);
        }
        if ep.intersects(pawn_captures_w) {
            let from = ep.shift(c.pawn_capture_west().opposite());
            let capture_square = ep.shift(c.opposite().pawn_move());
            let m =MoveExt::new_ep_capture(from, ep, capture_square);
            moves.push(m);
        }
    }

}



    // }
