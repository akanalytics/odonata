use crate::attacks::{BitboardAttacks, ClassicalBitboard};
use crate::bitboard::{Bitboard, Dir};
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::globals::counts;
use crate::movelist::{Move, MoveList, MoveExt};
use crate::types::{CastlingRights, Color, Piece};
use once_cell::sync::OnceCell;
use crate::board::movegen::global_classical_bitboard;

pub struct Rules;

impl Rules {
    pub fn knight_moves(b: &Board, moves: &mut Vec<MoveExt>) {

        let attack_gen = global_classical_bitboard();
        let them = b.them();
        let us = b.us();

         
        for from_sq in (b.knights() & us).squares() {
            let attacks = attack_gen.knight_attacks(from_sq) & !us;
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = MoveExt::new_capture(Piece::Knight,
                        from_sq.as_bb(), 
                        to, 
                        b.piece_at(to));
                    moves.push(m);
                } else {
                    
                    let m = MoveExt::new_quiet(Piece::Knight,
                        from_sq.as_bb(),
                          to);
                    moves.push(m);
                }
            }
        }
    }

    
}