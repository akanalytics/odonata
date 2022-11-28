use crate::board::Board;
use crate::board::BoardCalcs;
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
    pub promo: bool,
}

impl Default for See {
    fn default() -> Self {
        Self {
            enabled: true,
            promo: false,
        }
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
        debug_assert!(mv.is_capture());
        debug_assert!(board.us().contains(mv.from().as_bb()));
        debug_assert!(board.them().contains(mv.capture_square().as_bb()));

        let bb = PreCalc::default();
        let mut gain: [i32; 40] = [0; 40];
        let mut d = 0;
        let mut from = mv.from().as_bb();
        let to = mv.to().as_bb();
        let mut occ = board.black() | board.white();
        let mut attacker_color = board.color_us();
        let mut attackers_bw = BoardCalcs::attacked_by(to, occ, board); // will include the current 'mv' attacker
        let mut attackers_xray = BoardCalcs::attacked_by(to, Bitboard::EMPTY, board); // will include the current 'mv' attacker
        attackers_xray -= board.non_line_pieces() | attackers_bw;

        gain[0] = MaterialBalance::CLASSICAL_WEIGHTS[mv.capture_piece().unwrap()].s() as i32;
        while from.any() {
            let mut mover = board.piece_unchecked(from.first_square());
            // check for a pawn promo during capture
            if self.promo && mover == Piece::Pawn && to.intersects(Bitboard::RANKS_18) {
                mover = Piece::Queen;
                gain[d] += MaterialBalance::CLASSICAL_WEIGHTS[Piece::Queen].s() as i32;
                // TODO not quite right coz of pawn loss
            }
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
                // last move was king, but he cant move into check
                // so break before adding another trophy gain
                break;
            }
            d += 1;
            gain[d] = MaterialBalance::CLASSICAL_WEIGHTS[mover].s() as i32 - gain[d - 1];
            // what you are taking less what opp has
            // println!("{}\n{}: mover: {} from: {:?} for spec gain {:?}\n{}",board.to_fen(), d, mover, from, gain, attackers);
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
    use crate::{catalog::Catalog, eval::eval::Eval, Position};
    // use crate::movelist::MoveValidator;
    use anyhow::Result;
    use test_log::test;

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

    #[test]
    fn test_see2() -> Result<()> {
        let e = Eval::new();

        let b = Position::parse_epd(
            r"
            .......k
            ........
            ........
            ..r.....
            ..R.....
            ........
            ........
            K....... w - - 1 1",
        )?
        .board()
        .clone();
        let mv = b.parse_san_move("Rc5")?;
        let see = e.see.eval_move_see(&b, mv);
        assert_eq!(see, Piece::Rook.centipawns());

        let b = Position::parse_epd(
            r"
            .......k
            ........
            ...b....
            ..r.....
            .B......
            ........
            ........
            K....... w - - 1 1",
        )?
        .board()
        .clone();
        let mv = b.parse_san_move("Bc5")?;
        let see = e.see.eval_move_see(&b, mv);
        assert_eq!(see, Piece::Rook.centipawns() - Piece::Bishop.centipawns());

        //
        // without promos, we just appear to be a rook up
        // but we are a rook+queen less a pawn up
        let b = Position::parse_epd(
            r"
            .r.....k
            P.......
            ........
            ........
            ........
            ........
            ........
            K....... w - - 1 1",
        )?
        .board()
        .clone();
        let mut e2 = Eval::new();
        e2.see.promo = false;
        let mv = b.parse_san_move("Pxb8=Q")?;
        let see = e2.see.eval_move_see(&b, mv);
        assert_eq!(see, Piece::Rook.centipawns());

        //
        // without promos, we just appear to be a rook up as promo'd
        // "pawn" is not retaken
        // with promos Pxr=Q, nxQ, Rxn   => +r+Q-Q+n
        let b = Position::parse_epd(
            r"
            .r.....k
            PR.n....
            ........
            ........
            ........
            ........
            ........
            K....... w - - 1 1",
        )?
        .board()
        .clone();
        let mut e2 = Eval::new();
        e2.see.promo = true;
        let mv = b.parse_san_move("Pxb8=Q")?;
        let see = e2.see.eval_move_see(&b, mv);
        assert_eq!(see, Piece::Rook.centipawns() + Piece::Knight.centipawns());

        //
        // Qxr (+r), rxQ (-Q), nxr (+r) (+2r-q)
        let b = Position::parse_epd(
            r"
            .r...r.k
            PQ.N....
            ........
            ........
            ........
            ........
            ........
            K....... w - - 1 1",
        )?
        .board()
        .clone();
        let mut e2 = Eval::new();
        e2.see.promo = false;
        let mv = b.parse_san_move("Qxb8")?;
        let see = e2.see.eval_move_see(&b, mv);
        assert_eq!(
            see,
            2 * Piece::Rook.centipawns() - Piece::Queen.centipawns()
        );
        Ok(())
    }
}
