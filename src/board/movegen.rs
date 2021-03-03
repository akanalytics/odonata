use crate::attacks::{BitboardAttacks, ClassicalBitboard};
use crate::bitboard::{Bitboard, Dir};
use crate::board::{Board, MoveEnum, Move, MoveList, CastlingRights, Color, Piece};
use Vec;


pub struct MoveGen {
    attack_gen: ClassicalBitboard,
}

impl MoveGen {

    pub fn new() -> MoveGen {
        MoveGen { attack_gen: ClassicalBitboard::new() }
    }


    // Vec::with_capacity(100).
    pub fn pseudo_legal_moves(&self, board: &Board, moves: &mut MoveList) {
        let color = board.color_us();
        let them = board.them();
        let us = board.us();
        let occupied = us | them;

        let pawns = board.pawns() & us;

        // non-promoted single-push pawns
        let pawn_push = self.attack_gen.pawn_pushes(occupied, pawns, &color);
        for to in (pawn_push & !Bitboard::PROMO_RANKS).iter() {
            let mut from = to.shift(&color.pawn_move.opposite());
            let mut ep = Bitboard::EMPTY;
            if !pawns.contains(from) {
                // must have been double push
                ep = from;
                from = to.shift(&color.pawn_move.opposite());
            }
            // let m = MoveEnum::Push { to, from };
            let m = Move { from, to, ep, mover: Piece::Pawn, ..Default::default() };
            moves.push(m);
        }
        // pawn promos - only makes sense for single push
        for to in (pawn_push & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_move.opposite());
            // try and pre-sort promos by likely usefulness
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                let m = Move { from, to, promo, mover: Piece::Pawn, ..Default::default() };
                // let m = MoveEnum::Promo { to, from, promo };
                moves.push(m);
            }
        }
        // pawn_captures
        let (pawn_captures_e, pawn_captures_w) = self.attack_gen.pawn_attacks(pawns, &color);
        
        for to in (pawn_captures_e & them & !Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(to);
            let m = Move { from, to, mover: Piece::Pawn, capture, ..Default::default() };
            // MoveEnum::Capture { to, from, mover: Piece::Pawn, capture });
            moves.push(m); 
        }
        for to in (pawn_captures_w & them & !Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_west.opposite());
            let capture = board.piece_at(to);
            let m = Move { from, to, mover: Piece::Pawn, capture, ..Default::default() };
            // MoveEnum::Capture { to, from, mover: Piece::Pawn, capture };
            moves.push(m);
        }

        // pawn capture-promos
        for to in (pawn_captures_e & them & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(to);
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                // MoveEnum::PromoCapture { to, from, promo, capture });
                let m = Move { from, to, mover: Piece::Pawn, capture, promo, ..Default::default() };
                moves.push(m);
            }
        }
        for to in (pawn_captures_w & them & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_west.opposite());
            let capture = board.piece_at(to);
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                // MoveEnum::PromoCapture { to, from, promo, capture });
                let m = Move { from, to, mover: Piece::Pawn, capture, promo, ..Default::default() };
                moves.push(m);
            }
        }
        // knights
        for from in (board.knights() & us).iter() {
            let attacks = self.attack_gen.knight_attacks(from.first_square()) & !us;
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move { from, to, mover: Piece::Knight, capture: board.piece_at(to), ..Default::default() };
                    // MoveEnum::Capture { to, from, mover: Piece::Knight, capture: board.piece_at(to) }
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Knight, ..Default::default() };
                    // MoveEnum::Quiet { to, from, mover: Piece::Knight }
                    moves.push(m);
                }
            }
        }
        // sliders
        for from in (board.bishops() & us).iter() {
            let attacks = !us & self.attack_gen.bishop_attacks(occupied, from.first_square());
            // println!("{}\n{}\n{}\n", from, attacks, occupied);
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move { from, to, mover: Piece::Bishop, capture: board.piece_at(to), ..Default::default() };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Bishop, ..Default::default() };
                    moves.push(m);
                }
            }
        }
        for from in (board.rooks() & us).iter() {
            let attacks = !us & self.attack_gen.rook_attacks(occupied, from.first_square());
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move { from, to, mover: Piece::Rook, capture: board.piece_at(to), ..Default::default() };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Rook, ..Default::default() };
                    moves.push(m);
                }
            }
        }
        for from in (board.queens() & us).iter() {
            let attacks = !us
                & (self.attack_gen.rook_attacks(occupied, from.first_square())
                    | self.attack_gen.bishop_attacks(occupied, from.first_square()));
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move { from, to, mover: Piece::Queen, capture: board.piece_at(to), ..Default::default() };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Queen, ..Default::default() };
                    moves.push(m);
                }
            }
        }
        for from in (board.kings() & us).iter() {
            let attacks = !us & self.attack_gen.king_attacks(from.first_square());
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move { from, to, mover: Piece::King, capture: board.piece_at(to), ..Default::default() };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::King, ..Default::default() };
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
        // addMoveEnum King +/- 2, add rook -2/+3
        // castling rights
        let king = board.kings() & us;
        let rights = board.castling();

        let right = color.castle_rights_king;
        if rights.contains(right) && !color.kingside_castle_sqs.intersects(occupied) {
            let king_moves = king | color.kingside_castle_sqs;
            if self.attacked_by(king_moves, occupied, board, &color).is_empty() {
                let rook_to = king.shift(&Dir::E);
                let king_to = rook_to.shift(&Dir::E);
                // let rook_from = Bitboard::FILE_A & color.back_rank;
                // let m = MoveEnum::Castle { king_dest, king_from: king, rook_dest, rook_from, right };
                let m = Move { from: king, to:king_to, mover: Piece::King, is_castle: true, ..Default::default() };
                moves.push(m);
            }
        }

        let right = color.castle_rights_queen;
        if rights.contains(right) && !color.queenside_castle_sqs.intersects(occupied) {
            let king_moves = king | color.queenside_castle_sqs;
            if self.attacked_by(king_moves, occupied, board, &color).is_empty() {
                let rook_to = king.shift(&Dir::W);
                let king_to = rook_to.shift(&Dir::W);
                // let rook_from = Bitboard::FILE_H & color.back_rank;
                // let m = MoveEnum::Castle { king_dest, king_from: king, rook_dest, rook_from, right };
                let m = Move { from: king, to:king_to, mover: Piece::King, is_castle: true, ..Default::default() };
                moves.push(m);
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
        let mg = MoveGen::new();
        let mut moves = MoveList::new();
        let mut buf = BoardBuf::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        buf.set(a2, ".")?;
        buf.set(d2, ".")?;
        buf.set(d4, "P")?;
        let board = buf.as_board();
        mg.pseudo_legal_moves(&board, &mut moves);
        assert_eq!(moves.len(), 32);
        println!("{}\n{:#?}", board, moves);
        //assert_eq!(format!("{:#?}", moves), "vec![]");
        Ok(())
    }

    #[test]
    fn test_pawns() {
        let board = BoardBuf::parse_fen("8/8/8/8/8/8/P7/8 w - - id 'lone P'").unwrap().as_board();
        let moves = board.pseudo_legal_moves();
        assert_eq!(moves.len(), 2);
        assert_eq!(moves.to_string(), "a2a3, a2a4");

//         b = Parser(cls).parse_board_epd("8/p7/8/8/8/8/8/8 b - - id 'lone P flipped'")
//         self.assert_pseudo_legal_moves(b, ["a7a5","a7a6"])

//         b = Parser(cls).parse_board_epd("8/8/8/8/8/p7/P7/8 w - - id "PP"')
//         self.assert_pseudo_legal_moves(b, [])

//         b = Parser(cls).parse_board_epd("8/8/8/8/8/8/PPP5/8 w - - id "PPP"")
//         self.assert_pseudo_legal_moves(b, ["a2a3", "a2a4", "b2b3", "b2b4", "c2c3", "c2c4"])

//         b = Parser(cls).parse_board_epd("8/8/8/8/8/p1p5/1P6/8 w - - id "P capture white"")
//         self.assert_pseudo_legal_moves(b, ["b2a3", "b2b3", "b2b4", "b2c3"])

//         b = Parser(cls).parse_board_epd("8/1p6/P1P5/8/8/8/1P6/8 b - - id "P capture black"")
//         self.assert_pseudo_legal_moves(b, ["b7a6", "b7b6", "b7b5", "b7c6"])

//         b = Parser(cls).parse_board_epd("8/8/p6p/1N6/8/8/8/8 b - - id "PxN black"")
//         self.assert_pseudo_legal_moves(b, ["a6a5", "a6b5", "h6h5"])
// }



    }
}
