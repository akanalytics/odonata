use crate::attacks::{BitboardAttacks, ClassicalBitboard};
use crate::bitboard::{Bitboard, Dir};
use crate::board::{Board, Move, CastlingRights, Color, Piece};
use Vec;


pub struct MoveGen {
    attack_gen: ClassicalBitboard,
}

impl MoveGen {

    // Vec::with_capacity(100).
    fn pseudo_legal_moves(&self, board: &Board, moves: &mut Vec<Move>) {
        let color = board.color_us();
        let them = board.them();
        let us = board.us();
        let occupied = us | them;

        let pawns = board.pawns() & us;

        // non-promoted single-push pawns
        let pawn_push = self.attack_gen.pawn_pushes(occupied, pawns, &color);
        for dest in (pawn_push & !Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_move.opposite());
            let m = Move::Push { dest, src };
            moves.push(m);
        }
        // pawn promos - only makes sense for single push
        for dest in (pawn_push & Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_move.opposite());
            // try and pre-sort promos by likely usefulness
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                let m = Move::Promo { dest, src, promo };
                moves.push(m);
            }
        }
        // pawn_captures
        let (pawn_captures_e, pawn_captures_w) = self.attack_gen.pawn_attacks(pawns, &color);
        for dest in (pawn_captures_e & them & !Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(dest);
            moves.push(Move::Capture { dest, src, mover: Piece::Pawn, capture });
        }
        for dest in (pawn_captures_w & them & !Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_west.opposite());
            let capture = board.piece_at(dest);
            moves.push(Move::Capture { dest, src, mover: Piece::Pawn, capture });
        }

        // pawn capture-promos
        for dest in (pawn_captures_e & them & Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(dest);
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                moves.push(Move::PromoCapture { dest, src, promo, capture });
            }
        }
        for dest in (pawn_captures_w & them & Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_west.opposite());
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                let capture = board.piece_at(dest);
                moves.push(Move::PromoCapture { dest, src, promo, capture });
            }
        }
        // knights
        for src in (board.knights() & us).iter() {
            let attacks = self.attack_gen.knight_attacks(src.first_square()) & !us;
            for dest in attacks.iter() {
                if them.contains(dest) {
                    moves.push(Move::Capture { dest, src, mover: Piece::Knight, capture: board.piece_at(dest) });
                } else {
                    moves.push(Move::Quiet { dest, src, mover: Piece::Knight });
                }
            }
        }
        // sliders
        for src in (board.bishops() & us).iter() {
            let attacks = !us & self.attack_gen.bishop_attacks(occupied, src.first_square());
            // println!("{}\n{}\n{}\n", src, attacks, occupied);
            for dest in attacks.iter() {
                if them.contains(dest) {
                    moves.push(Move::Capture { dest, src, mover: Piece::Bishop, capture: board.piece_at(dest) });
                } else {
                    moves.push(Move::Quiet { dest, src, mover: Piece::Bishop });
                }
            }
        }
        for src in (board.rooks() & us).iter() {
            let attacks = !us & self.attack_gen.rook_attacks(occupied, src.first_square());
            for dest in attacks.iter() {
                if them.contains(dest) {
                    moves.push(Move::Capture { dest, src, mover: Piece::Rook, capture: board.piece_at(dest) });
                } else {
                    moves.push(Move::Quiet { dest, src, mover: Piece::Rook });
                }
            }
        }
        for src in (board.queens() & us).iter() {
            let attacks = !us
                & (self.attack_gen.rook_attacks(occupied, src.first_square())
                    | self.attack_gen.bishop_attacks(occupied, src.first_square()));
            for dest in attacks.iter() {
                if them.contains(dest) {
                    let m = Move::Capture { dest, src, mover: Piece::Queen, capture: board.piece_at(dest) };
                    moves.push(m);
                } else {
                    let m = Move::Quiet { dest, src, mover: Piece::Queen };
                    moves.push(m);
                }
            }
        }
        for src in (board.kings() & us).iter() {
            let attacks = !us & self.attack_gen.king_attacks(src.first_square());
            for dest in attacks.iter() {
                if them.contains(dest) {
                    let m = Move::Capture { dest, src, mover: Piece::King, capture: board.piece_at(dest) };
                    moves.push(m);
                } else {
                    let m = Move::Quiet { dest, src, mover: Piece::King };
                    moves.push(m);
                }
            }
        }
    
        // castling
        // check castling rights (cheap)
        // check king not in check
        // side = +/-2
        // check king+1 and king+2 for being clear on kings side
        // check king-1, king-2, king-3 clear on queens
        // check that king +/- 1 and king +/- 2 isnt in check
        // addMove King +/- 2, add rook -2/+3
        // castling rights
        let king = board.kings() & us;
        let rights = board.castling();

        let right = color.castle_rights_king;
        if rights.contains(right) && !color.kingside_castle_sqs.intersects(occupied) {
            let king_moves = king | color.kingside_castle_sqs;
            if self.attacked_by(king_moves, occupied, board, &color).is_empty() {
                let rook_dest = king.shift(&Dir::E);
                let king_dest = rook_dest.shift(&Dir::E);
                let rook_src = Bitboard::FILE_A & color.back_rank;
                let m = Move::Castle { king_dest, king_src: king, rook_dest, rook_src, right };
                moves.push(m)
            }
        }

        let right = color.castle_rights_queen;
        if rights.contains(right) && !color.queenside_castle_sqs.intersects(occupied) {
            let king_moves = king | color.queenside_castle_sqs;
            if self.attacked_by(king_moves, occupied, board, &color).is_empty() {
                let rook_dest = king.shift(&Dir::W);
                let king_dest = rook_dest.shift(&Dir::W);
                let rook_src = Bitboard::FILE_H & color.back_rank;
                let m = Move::Castle { king_dest, king_src: king, rook_dest, rook_src, right };
                moves.push(m)
            }
        }
    }




    fn attacked_by(&self, target: Bitboard, occ: Bitboard, board: &Board, c: &Color) -> Bitboard {
        let pawns = board.pawns() & board.them();
        let knights = board.knights() & board.them();
        let bishops = board.bishops() & board.them();
        let rooks = board.rooks() & board.them();
        let queens = board.queens() & board.them();
        let kings = board.kings() & board.them();

        let (east, west) = self.attack_gen.pawn_attacks(target, c);
        let mut attackers = (east | west) & pawns;

        let sq = target.first_square();

        attackers |= self.attack_gen.knight_attacks(sq) & knights
            | self.attack_gen.king_attacks(sq) & kings
            | self.attack_gen.bishop_attacks(occ, sq) & (bishops | queens)
            | self.attack_gen.rook_attacks(occ, sq) & (rooks | queens);
        // TODO: en passant!!
        attackers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::globals::constants::*;

    #[test]
    fn pseudo_legal_moves() -> Result<(), String> {
        let mg = MoveGen { attack_gen: ClassicalBitboard::new() };
        let mut moves: Vec<Move> = Vec::new();
        let mut buf = BoardBuf::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        buf.set(a2, ".")?;
        buf.set(d2, ".")?;
        buf.set(d4, "P")?;
        let board = buf.as_board();
        mg.pseudo_legal_moves(&board, &mut moves);
        println!("{}\n{:#?}", board, moves);
        //assert_eq!(format!("{:#?}", moves), "vec![]");
        Ok(())
    }
}
