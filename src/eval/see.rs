use crate::board::boardcalcs::BoardCalcs;
use crate::board::Board;
use crate::eval::material_balance::MaterialBalance;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::{Bitboard, Piece, PreCalc};
use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct See {
    pub enabled: bool,
}

impl Default for See {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Component for See {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl See {
    // a rusty version of https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
    // Since we dont remove material from the board, the phase will never be right, so we just
    // use classical material values
    //
    // using n=b=325 rather than n=325, b=350 gives +6 Elo
    //
    pub fn eval_move_see(&self, board: &Board, mv: Move) -> i32 {
        debug_assert!(!mv.is_null());
        debug_assert!(board.us().contains(mv.from().as_bb()));
        debug_assert!(board.them().contains(mv.capture_square().as_bb()));
        debug_assert!(mv.is_capture());

        let bb = PreCalc::default();
        let mut gain: [i32; 40] = [0; 40];
        let mut d = 0;
        let mut from = mv.from().as_bb();
        let mut occ = board.black() | board.white();
        let mut attacker_color = board.color_us();
        // let mut attackers_bw = board.attacked_by(mv.to().as_bb());  // will include the current 'mv' attacker
        let mut attackers_bw = BoardCalcs::attacked_by(mv.to().as_bb(), occ, board); // will include the current 'mv' attacker
        let mut attackers_xray = BoardCalcs::attacked_by(mv.to().as_bb(), Bitboard::EMPTY, board); // will include the current 'mv' attacker
        attackers_xray -= board.non_line_pieces() | attackers_bw;

        gain[0] = MaterialBalance::CLASSICAL_WEIGHTS[mv.capture_piece()].s() as i32;
        while from.any() {
            let mover = board.piece_at(from);
            attackers_bw -= from;
            occ -= from;
            attacker_color = attacker_color.opposite();

            // xray attackers
            // we move some pieces from xray into attackers - these are all line pieces
            // alternatively use attacked by bishops & B&Q and attacked-by-rooks & R & Q and move them into attackers
            for sq in (attackers_xray & board.color(attacker_color)).squares() {
                if bb.strictly_between(sq, mv.to()).disjoint(occ) {
                    attackers_xray -= sq.as_bb();
                    attackers_bw |= sq.as_bb();
                }
            }

            from = board.least_valuable_piece(attackers_bw & board.color(attacker_color));
            if mover == Piece::King && from.any() {
                // king is the last attacker, but he cant move into check
                // so break before adding another trophy gain
                break;
            }
            d += 1;
            gain[d] = MaterialBalance::CLASSICAL_WEIGHTS[mover].s() as i32 - gain[d - 1]; // what you are taking less what opp has
                                                                                          // eprintln!("{}\n{}: mover: {} from: {:?} for spec gain {:?}\n{}",board.to_fen(), d, mover, from, gain, attackers);
                                                                                          // if cmp::max(-gain[d-1], gain[d]) < 0 {
                                                                                          //     break; // safely prune as from here on its zero
                                                                                          // }

            if d > 38 {
                // warn!("{} {}", mv, board.to_fen());
                break;
            }
        }

        // so  1=wp:  bn x p, b x n,  r x b, q x r
        //   0=pawn = 1
        //   1=n    = 3-1 = 2
        //   2=b    = 3 - 2 = 1
        //   3=r    = 5 - 1 = 4
        //   4=q    = 9 - 4 = 5

        while d >= 2 {
            gain[d - 2] = -cmp::max(-gain[d - 2], gain[d - 1]);
            d -= 1;
        }
        gain[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    // use crate::movelist::MoveValidator;
    use anyhow::Result;

    #[test]
    fn test_see() -> Result<()> {
        let see = See::default();

        let positions = Catalog::see();
        // let pos = Position::find_by_id("pawn fork", &positions ).unwrap();
        for pos in positions {
            let b = pos.board();
            let mv = pos.sm()?;
            let ce = pos.ce()?;
            assert_eq!(see.eval_move_see(&b, mv), ce, "pos {}", pos);
        }
        Ok(())
    }
}
