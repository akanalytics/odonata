use std::cmp;

use odonata_base::boards::BoardCalcs;
use odonata_base::infra::component::Component;
use odonata_base::prelude::*;
use odonata_base::PreCalc;
use serde::{Deserialize, Serialize};

use crate::eval::weight::Weight;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct See {
    pub enabled: bool,
    pub promo:   bool,
}

impl Default for See {
    fn default() -> Self {
        Self {
            enabled: true,
            promo:   true,
        }
    }
}

impl Configurable for See {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.promo.set(p.get("promo"))?;
        Ok(p.is_modified())
    }
}

impl Component for See {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

pub const CLASSICAL_WEIGHTS: [Weight; Piece::len()] = [
    Weight::from_i32(Piece::Pawn.centipawns(), Piece::Pawn.centipawns()),
    Weight::from_i32(Piece::Knight.centipawns(), Piece::Knight.centipawns()),
    Weight::from_i32(Piece::Bishop.centipawns(), Piece::Bishop.centipawns()),
    Weight::from_i32(Piece::Rook.centipawns(), Piece::Rook.centipawns()),
    Weight::from_i32(Piece::Queen.centipawns(), Piece::Queen.centipawns()),
    Weight::from_i32(Piece::King.centipawns(), Piece::King.centipawns()),
];

impl See {
    // a rusty version of https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
    // Since we dont remove material from the board, the phase will never be right, so we just
    // use classical material values
    //
    // using n=b=325 rather than n=325, b=350 gives +6 Elo
    //
    pub fn eval_move_see(&self, board: &Board, mv: Move) -> i32 {
        if !mv.is_capture() {
            return 0;
        }
        debug_assert!(!mv.is_null());
        debug_assert!(mv.is_capture());
        debug_assert!(board.us().contains(mv.from().as_bb()));
        debug_assert!(board.them().contains(mv.capture_square(board).as_bb()));

        let bb = PreCalc::instance();
        let mut gain: [i32; 40] = [0; 40];
        let mut d = 0;
        let mut attacker = Some(mv.from());
        let to = mv.to().as_bb();
        let mut occ = board.black() | board.white();
        let mut attacker_color = board.color_us();
        let mut attackers_bw = BoardCalcs::attacked_by(to, occ, board); // will include the current 'mv' attacker
        let mut attackers_xray = BoardCalcs::attacked_by(to, Bitboard::EMPTY, board); // will include the current 'mv' attacker
        attackers_xray -= board.non_line_pieces() | attackers_bw;

        gain[0] = CLASSICAL_WEIGHTS[mv.capture_piece(board).unwrap()].s() as i32;
        while let Some(from) = attacker {
            let mut mover = board.piece_unchecked(from);
            // check for a pawn promo during capture
            if self.promo && mover == Piece::Pawn && to.intersects(Bitboard::RANKS_18) {
                mover = Piece::Queen;
                gain[d] += CLASSICAL_WEIGHTS[Piece::Queen].s() as i32;
                // TODO not quite right coz of pawn loss
            }
            attackers_bw -= from.as_bb();
            occ -= from.as_bb();
            attacker_color = attacker_color.flip_side();

            // xray attackers
            // we move some pieces from xray into attackers - these are all line pieces
            // alternatively use attacked by bishops & B&Q and attacked-by-rooks & R & Q and move them into attackers
            for sq in (attackers_xray & board.color(attacker_color)).squares() {
                if bb.strictly_between(sq, mv.to()).disjoint(occ) {
                    attackers_xray -= sq.as_bb();
                    attackers_bw |= sq.as_bb();
                }
            }

            attacker = board.least_valuable_piece(attackers_bw & board.color(attacker_color));
            if mover == Piece::King && attacker.is_some() {
                // last move was king, and after exchange there is an attack on the king, but he cant move into check
                // so break before adding the gain
                break;
            }
            d += 1;
            gain[d] = CLASSICAL_WEIGHTS[mover].s() as i32 - gain[d - 1];
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
    use odonata_base::catalog::Catalog;
    use odonata_base::infra::profiler::PerfProfiler;
    use odonata_base::other::tags::EpdOps as _;
    use test_log::test;

    use super::*;

    #[test]
    fn test_see() {
        let see = See::default();

        // let pos = Position::find_by_id("pawn fork", &positions ).unwrap();
        for epd in Catalog::see() {
            let b = epd.board();
            let mv = epd.mv("sm").unwrap();
            let ce = epd.int("ce").unwrap() as i32;
            assert_eq!(see.eval_move_see(&b, mv), ce, "{epd}");
        }
    }

    #[test]
    fn test_see2() -> Result<()> {
        let see = See::default();

        let b = Board::parse_diagram(
            r"
            .......k
            ........
            ........
            ..r.....
            ..R.....
            ........
            ........
            K....... w - - 1 1",
        )?;
        let mv = b.parse_san_move("Rc5")?;
        let see_value = see.eval_move_see(&b, mv);
        assert_eq!(see_value, Piece::Rook.centipawns());

        let b = Board::parse_diagram(
            r"
            .......k
            ........
            ...b....
            ..r.....
            .B......
            ........
            ........
            K....... w - - 1 1",
        )?;
        let mv = b.parse_san_move("Bc5")?;
        let see_value = see.eval_move_see(&b, mv);
        assert_eq!(see_value, Piece::Rook.centipawns() - Piece::Bishop.centipawns());

        // without promos, we just appear to be a rook up
        // but we are a rook+queen less a pawn up
        let b = Board::parse_diagram(
            r"
            .r.....k
            P.......
            ........
            ........
            ........
            ........
            ........
            K....... w - - 1 1",
        )?;
        let see = See {
            promo: false,
            ..See::default()
        };
        let mv = b.parse_san_move("Pxb8=Q")?;
        let see_value = see.eval_move_see(&b, mv);
        assert_eq!(see_value, Piece::Rook.centipawns());

        // without promos, we just appear to be a rook up as promo'd
        // "pawn" is not retaken
        // with promos Pxr=Q, nxQ, Rxn   => +r+Q-Q+n
        let b = Board::parse_diagram(
            r"
            .r.....k
            PR.n....
            ........
            ........
            ........
            ........
            ........
            K....... w - - 1 1",
        )?;
        let see = See {
            promo: true,
            ..See::default()
        };
        let mv = b.parse_san_move("Pxb8=Q")?;
        let see_value = see.eval_move_see(&b, mv);
        assert_eq!(see_value, Piece::Rook.centipawns() + Piece::Knight.centipawns());

        // Qxr (+r), rxQ (-Q), nxr (+r) (+2r-q)
        let b = Board::parse_diagram(
            r"
            .r...r.k
            PQ.N....
            ........
            ........
            ........
            ........
            ........
            K....... w - - 1 1",
        )?;
        let see = See {
            promo: false,
            ..See::default()
        };

        let mv = b.parse_san_move("Qxb8")?;
        let see_value = see.eval_move_see(&b, mv);
        assert_eq!(see_value, 2 * Piece::Rook.centipawns() - Piece::Queen.centipawns());
        Ok(())
    }

    #[test]
    fn bench_see() {
        let mut pr = PerfProfiler::new("see");
        let see = See::default();
        let mut score = 0;
        for epd in Catalog::win_at_chess().iter() {
            let b = epd.board();
            let captures = b
                .legal_moves()
                .iter()
                .filter(|mv| mv.is_capture())
                .cloned()
                .collect_vec();
            for mv in captures.into_iter() {
                score += pr.bench(|| see.eval_move_see(&b, mv));
            }
        }
        assert_eq!(score, -301_275);
    }
}
