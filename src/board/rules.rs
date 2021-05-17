use crate::bitboard::attacks::{BitboardAttacks, BitboardDefault};
use crate::bitboard::bitboard::{Bitboard, Dir, Square};
use crate::board::boardcalcs::BoardCalcs;
use crate::board::Board;
use crate::movelist::{Move, MoveList};
use crate::types::{Piece};
use crate::bitboard::castling::CastlingRights;

pub struct Rules;



impl Rules {


    // pub fn x(b: &Board) {
    //     let mut capture_mask = Bitboard::all();
    //     let mut push_mask    = Bitboard::all();
        
    //     let checkers = b.checkers_of(b.color_us());

    //     if checkers.popcount() == 1 {
    //         // if ony one checker, we can evade check by capturing it
    //         capture_mask = checkers;
        
    //         // If the piece giving check is a slider, we can evade check by blocking it
    //         if board.line_pieces().intersects(checkers) {
    //             push_mask = opponent_slider_rays_to_square(king_square, board);
    //         } else {
    //             // if the piece is not a slider, we can only evade check by capturing
    //             push_mask = Bitboard::EMPTY; 
    //         }
    //     }        
    // }



    pub fn king_legal(b: &Board, moves: &mut MoveList) {
        let attack_gen = BitboardDefault::default();
        let them = b.them();
        let us = b.us();
        let occ = b.occupied();
        let our_kings = b.kings() & us;
        let king_danger = BoardCalcs::threats_to(b, b.color_us(), occ - our_kings );
        
        if !our_kings.is_empty() {
            let king_sq = (b.kings() & us).square();
            let attacks = attack_gen.non_pawn_attacks(b.color_us(), Piece::King, us, them, king_sq) & !us - king_danger;
            moves.extend(attacks.squares().map(|to| {
                if to.is_in(them) {
                    Move::new_capture(Piece::King, king_sq, to, b.piece_at(to.as_bb())).set_legal()
                } else {
                    Move::new_quiet(Piece::King, king_sq, to).set_legal()
                }
            }))
        }
    }

    pub fn legal_for(p: Piece, b: &Board, moves: &mut MoveList) {
        let us = b.us();
        let them = b.them();
        let our_kings = b.kings() & us;
        if our_kings.is_empty() {
            return Self::non_pawn(p, b, moves);
        }
        let checkers = b.checkers_of(b.color_us());
        if checkers.popcount() >= 2 {
            return; // only king moves
        } 
        
        let the_checker = checkers.square();
        let king_sq = our_kings.square();
        let gen = BitboardDefault::default();
        let occ = b.occupied();
        if  checkers.popcount() == 1 {
            // Captr checker -> pseudos-capture-sq & piece not pinned & (checker)  			            
            // Block checker -> pseudos-to & piece not pinned & (xrays to checker excl)  			
            let blocking = gen.between(king_sq, the_checker);
            for fr in (b.pieces(p) & us & !b.pinned()).squares() {
                let attacks = blocking & gen.non_pawn_attacks(b.color_us(), p, us, them, fr) & !us;
                Self::add_moves(attacks, p, fr, b, moves);
            }
            return;
        } else {
            // not in check
            for fr in (b.pieces(p) & us).squares() {
                let attacks = gen.non_pawn_attacks(b.color_us(),p, us, them, fr) & !us;
                if !fr.is_in(b.pinned()) {
                    // all non pinned pieces 
                    Self::add_moves(attacks, p, fr, b, moves);
                } else {
                    // Pinned -> psuedo-to in ray along king and pinner incl                
                    let blocking = gen.line_through(fr, king_sq);
                    // will exlude knights anyway
                    Self::add_moves(attacks & blocking, p, fr, b, moves);
                }
            }
        }
    }

    #[inline]
    fn add_moves(dests: Bitboard,  p: Piece, fr: Square, b: &Board, moves: &mut MoveList ) {
        moves.extend(dests.squares().map(|to| {
            if to.is_in(b.them()) {
                Move::new_capture(p, fr, to, b.piece_at(to.as_bb()))
            } else {
                Move::new_quiet(p, fr, to)
            }
        }))
    }



    pub fn non_pawn(p: Piece, b: &Board, moves: &mut MoveList ) {
        let attack_gen = BitboardDefault::default();
        let us = b.us();
        let occ = b.occupied();

        for fr in (b.pieces(p) & us).squares() {
            let attacks = attack_gen.non_pawn_attacks(b.color_us(),p, b.us(), b.them(), fr) & !us;
            Self::add_moves(attacks, p, fr, b, moves);
        }
    }

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
        if rights.contains(right) && !CastlingRights::king_side_squares(c).intersects(occ) && !king.is_empty()
        {
            let rook_to = king.shift(Dir::E);
            let king_to = rook_to.shift(Dir::E);
            let king_moves = king | rook_to | king_to;
            if BoardCalcs::attacked_by(king_moves, occ, b).disjoint(them) {
                let m = Move::new_castle(king_sq, king_to.square(), king_to.square().shift(Dir::E), rook_to.square(), right).set_legal();
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
            if BoardCalcs::attacked_by(king_moves, occ, b).disjoint(them) {
                let king_to = king_to.square();
                let rook_from = king_to.shift(Dir::W).shift(Dir::W);
                let m = Move::new_castle(king_sq, king_to, rook_from, rook_to.square(), right).set_legal();
                moves.push(m);
            }
        }
    }

    pub fn pawn_promos(b: &Board, moves: &mut MoveList) {
        let attack_gen = BitboardDefault::default();
        let c = b.color_us();
        let occ = b.occupied();
        let us = b.us();
        let pawn_push = attack_gen.pawn_pushes(occ, b.pawns() & us, c);
        for to in (pawn_push & Bitboard::PROMO_RANKS).squares() {
            let from = to.shift(c.pawn_move().opposite());
            // try and pre-sort promos by likely usefulness
            moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                Move::new_promo(from, to, p)
            ));
        }
    }



    pub fn pawn_push(b: &Board, moves: &mut MoveList) {
        // non-promoted single-push pawns
        let attack_gen = BitboardDefault::default();
        let c = b.color_us();
        let occ = b.occupied();
        let us = b.us();
        let pawns = b.pawns() & us;
        let pawn_push = attack_gen.pawn_pushes(occ, pawns, c);
        for to in (pawn_push & !Bitboard::PROMO_RANKS).squares() {
            let behind = to.shift(c.pawn_move().opposite());
            if !behind.is_in(pawns) {
                // must have been double push
                let ep = behind;
                let from = behind.shift(c.pawn_move().opposite());
                let m = Move::new_double_push(from, to, ep);
                moves.push(m);
            } else {
                let m = Move::new_quiet(Piece::Pawn, behind, to);
                // FIXME!!!! // { from, to, ep, mover: Piece::Pawn, ..Default::default() };
                moves.push(m);
            }
        }
    }



    pub fn pawn_captures_incl_promo(b: &Board, moves: &mut MoveList) {
        let attack_gen = BitboardDefault::default();
        let c = b.color_us();
        let us = b.us();
        let them = b.them();
        let pawns = b.pawns() & us;

        let (pawn_captures_e, pawn_captures_w) = attack_gen.pawn_attacks(pawns, c);
        for to in (pawn_captures_e & them).squares() {
            let from = to.shift(c.pawn_capture_east().opposite());
            let captured = b.piece_at(to.as_bb());
            if to.is_in(Bitboard::PROMO_RANKS) {
                moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                    Move::new_promo_capture(from, to, p, captured)
                ));
            }
            else {
                let m = Move::new_capture(Piece::Pawn, from, to, captured);
                moves.push(m);
            }
        }
        for to in (pawn_captures_w & them).squares() {
            let from = to.shift(c.pawn_capture_west().opposite());
            let captured = b.piece_at(to.as_bb());
            if to.is_in(Bitboard::PROMO_RANKS) {
                moves.extend( [Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop].iter().map(|&p|
                    Move::new_promo_capture(from, to, p, captured)
                ));
            }
            else {
                let m = Move::new_capture(Piece::Pawn, from, to, captured);
                moves.push(m);
            }
        }
        // e/p pawn_captures
        let ep = b.en_passant();
        if !ep.is_empty() {
            let ep = ep.square();
            if ep.is_in(pawn_captures_e) {
                let from = ep.shift(c.pawn_capture_east().opposite());
                let capture_square = ep.shift(c.opposite().pawn_move());
                let m =Move::new_ep_capture(from, ep, capture_square);
                moves.push(m);
            }
            if ep.is_in(pawn_captures_w) {
                let from = ep.shift(c.pawn_capture_west().opposite());
                let capture_square = ep.shift(c.opposite().pawn_move());
                let m =Move::new_ep_capture(from, ep, capture_square);
                moves.push(m);
            }

        }
    }
}



  
#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::board::*;


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
