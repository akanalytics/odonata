use crate::board::Board;
use crate::eval::eval::Eval;
use crate::eval::score::Score;
use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::search::node::{Event, Node};
use crate::{Algo, Bitboard};
// use crate::{debug, logger::LogInit};
use crate::types::{MoveType, MoveTypes, Piece, Ply};
use serde::{Deserialize, Serialize};
use std::{cmp, fmt};

// 5+0.04 => 165/1600
// 10+0.08 => 82/1189 (3000 sims)

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Futility {
    pub alpha_enabled: bool,
    pub beta_enabled: bool,
    prune_remaining: bool,
    in_check: bool,
    giving_check: bool,
    discoverer: bool, 
    first_move: bool,
    prune_extensions: bool,
    prune_fw_node: bool,
    prune_alpha_mate: bool,
    prune_beta_mate: bool,
    max_pawn_rank: u8,
    max_depth: Ply,
    max_depth_captures: Ply,
    margin_qs: i32,
    margin1: i32,
    margin2: i32,
    margin3: i32,
    move_types_forbidden: MoveTypes,
}

impl Component for Futility {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl Default for Futility {
    fn default() -> Self {
        Futility {
            alpha_enabled: true,
            beta_enabled: false,
            prune_remaining: false,
            in_check: true,
            giving_check: false,
            discoverer: false,
            first_move: false,
            prune_extensions: false,
            prune_fw_node: false,
            prune_alpha_mate: false,
            prune_beta_mate: false,
            max_pawn_rank: 7,
            max_depth: 2, // not sure > 2 really makes sense
            max_depth_captures: 2,
            margin_qs: 0,
            margin1: 100,
            margin2: 250,
            margin3: 1500,
            move_types_forbidden: MoveTypes::EMPTY, // MoveType::Hash | MoveType::Killer, // HK
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FutilityMeasure {
    eval: Score,
    margin: Score,
}

// # PLAYER                :  RATING  ERROR  POINTS  PLAYED   (%)  CFS(%)     W     D     L  D(%)
// 1 0.3.44:fes=MBPOW      :       4      7  2747.5    5424  50.7      54  2053  1389  1982  25.6
// 2 0.3.44:fes=MBPT       :       3      7  2741.0    5422  50.6      53  2026  1430  1966  26.4
// 3 0.3.44:fes=MBPOWS     :       3      7  2737.5    5422  50.5      56  2055  1365  2002  25.2
// 4 0.3.44                :       2      7  2730.0    5422  50.4      68  2037  1386  1999  25.6
// 5 0.3.44:fes=MBPOWST    :      -1      7  2706.0    5422  49.9      51  2018  1376  2028  25.4
// 6 0.3.44:fes=MBPO       :      -1      7  2704.0    5422  49.9      97  1992  1424  2006  26.3
// 7 0.3.44:fes=MB         :     -11      6  2612.0    5422  48.2     ---  1904  1416  2102  26.1

impl Algo {
    #[inline]
    pub fn standing_pat(&mut self, b: &Board, n: &mut Node, standing_pat: Score) -> Option<Score> {
        if !n.is_qs() {
            return None;
        }

        if n.depth == 0 {
            let outcome = b.outcome();
            if outcome.is_game_over() {
                if outcome.is_draw() {
                    self.counts.inc(n, Event::NodeLeafDraw);
                    return Some(b.pov_score(self.eval.w_eval_draw(b, n)));
                }
                // depth 0: we have considered a full width move search to get here so a winning
                // result is valid. Beyond depth 0 it is not.
                if let Some(c) = outcome.winning_color() {
                    self.counts.inc(n, Event::NodeLeafWinLoss);
                    if c == b.color_us() {
                        return Some(Score::we_win_in(n.ply));
                    } else {
                        return Some(Score::we_lose_in(n.ply));
                    }
                }
            }
        }
        if standing_pat >= n.beta && !b.is_in_check(b.color_us()) {
            self.counts.inc(n, Event::PruneStandingPat);
            return Some(standing_pat);
        }

        if standing_pat > n.alpha && !b.is_in_check(b.color_us()) {
            n.alpha = standing_pat;
        }

        if self.qsearch_disabled {
            self.counts.inc(n, Event::NodeLeafQuietEval);
            return Some(standing_pat);
        }
        None
    }

    #[inline]
    pub fn can_futility_prune_at_node(&self, b: &Board, node: &Node) -> bool {
        if (!self.futility.alpha_enabled && !self.futility.beta_enabled)
            ||
            node.ply == 0   // dont prune at root node
            ||
            // node.alpha + Score::from_cp(1) == node.beta // not in PVS
            // ||
            node.depth > self.max_depth // dont prune too far away from leaf nodes
            ||
            (!self.futility.prune_alpha_mate && node.alpha.is_mate())  
            ||
            (!self.futility.prune_beta_mate && node.beta.is_mate())  
            ||
            (!self.futility.prune_fw_node && node.is_fw())  // VER:0.4.14
            ||
            (!self.futility.in_check && b.is_in_check(b.color_us()))
        {
            return false;
        }
        true
    }

    // for each move at a prunable node, see if its worth pruning
    // if this move isnt a capture then even if the opponents move IS a capture our score will still
    // likely fall short of alpha
    // for futility at depth = 3, conceivably our second move could be a capture so risky to prune
    //
    // obviously even prunign at depth=2, this move could be a quite move that attacks a piece and means quiese
    // changes the score dramatically - so futility pruning at depth = 1/2 is not without downside
    //
    #[inline]
    pub fn can_futility_prune_move(
        &mut self,
        mv: Move,
        mv_num: u32,
        mt: MoveType,
        before: &Board,
        after: &Board,
        eval: Score,
        n: &Node,
        ext: Ply,
    ) -> Option<Score> {
        if !self.futility.alpha_enabled {
            return None;
        }
        if !self.futility.prune_extensions && ext != 0 {
            return None;
        }
        if self.futility.move_types_forbidden.contains(mt) {
            return None;
        }

        if !self.futility.first_move && mv_num <= 1 {
            return None;
        }

        if !n.alpha.is_numeric() {
            return None;
        }

        // we allow mate scores as we clamp the
        //  results of eval (which could be a mate score 
        // if taken from tt)
        // 
        // if !eval.is_numeric() {
        //     return None;
        // }

        if !self.futility.discoverer && mv.from().is_in(before.discoverer(before.color_them())) {
            return None;
        }


        // gives check a more precise and costly version of discoverers
        if !self.futility.giving_check && after.is_in_check(after.color_us()) {
            return None;
        }

        // position wise, passed pawn promos make a huge impact so exclude them
        if mv.mover_piece() == Piece::Pawn
            && mv.from().rank_number_as_white(before.color_us()) > self.futility.max_pawn_rank as usize
        {
            return None;
        }

        if n.depth > self.futility.max_depth_captures && mv.is_capture() {
            return None;
        }

        if !self.can_futility_prune_at_node(before, n) {
            return None;
        }

        // safety margin depends on how far away we are from leaf node
        let margin = Score::from_cp(match n.depth {
            d if d <= 0 => self.futility.margin_qs,
            1 => self.futility.margin1,
            2 => self.futility.margin2,
            3 => self.futility.margin3,
            _ => self.futility.margin1 + self.futility.margin2 + self.futility.margin3,
        });

        // not a capture or promo => gain = 0
        let gain = before.eval_move_material(&self.eval, &mv);

        // fail low pruning
        let est_score = eval.clamp_score() + margin + gain;
        if est_score <= n.alpha {
            let category = match n.depth {
                d if d <= 0 => Event::PruneFutilityD0,
                1 => Event::PruneFutilityD1,
                2 => Event::PruneFutilityD2,
                _ => Event::PruneFutilityD3,
            };
            self.counts.inc(n, category);
            self.explain_futility(&before, mv, mt, est_score, &n, category);
            return Some(est_score);
        }
        None
    }

    // at depth = 1, assuming captures sorted by MVVLVA, then once one falls below futility,
    // then the rest will, and quiet
    // moves will also be so far away from alpha we can prune them too
    // assumes already the move itself is futile on above test
    // and we dont prune Hash moves as they are not ordered and may have a capture value < later moves
    #[inline]
    pub fn can_prune_remaining_moves(&self, b: &Board, mt: MoveType, _n: &Node) -> bool {
        if mt == MoveType::Hash {
            return false;
        }
        // we might be pruning a low value capture, with a high value promo later in the move list
        let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
        if (b.pawns() & b.us() & Bitboard::home_half(b.color_them()) & near_promos).any() {
            return false;
        }
        self.futility.prune_remaining
    }

    pub fn can_prune_all_moves(
        &self,
        b: &Board,
        measure: FutilityMeasure,
        node: &Node,
        eval: &Eval,
    ) -> Option<Score> {
        if !self.futility.beta_enabled {
            return None;
        }
        let phase = b.phase(&eval.phaser);
        let maximum_opp_piece =
            if let Some((piece, _)) = b.most_valuable_piece_except_king(b.them()) {
                eval.mb.piece_weights[piece]
            } else {
                Weight::zero() // all they have is a king
            };

        let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
        let promo_value =
            if (b.pawns() & b.us() & Bitboard::home_half(b.color_them()) & near_promos).any() {
                eval.mb.piece_weights[Piece::Queen] - eval.mb.piece_weights[Piece::Pawn]
            } else {
                Weight::zero() // no promos possible
            };
        let gain = cmp::max(
            maximum_opp_piece.interpolate(phase) as i32,
            promo_value.interpolate(phase) as i32,
        );

        // fail low pruning
        let est_score = measure.eval + measure.margin + Score::from_cp(gain);
        if est_score <= node.alpha {
            return Some(est_score);
        }
        None
    }

    // if node.depth == 2 {
    //     // move at depth=1 (by opponent) promo
    //     let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
    //     if (b.pawns() & b.them() & Bitboard::home_half(b.color_us()) & near_promos).any() {
    //         if self.avoid_promos {
    //             return None;
    //         } else if self.promo_margin {
    //             promo_value = eval.mb.material_weights[Piece::Queen].s() as i32 - eval.mb.material_weights[Piece::Pawn].s() as i32
    //             // gain is only for 1st move. We need to take into account opponents possible promotion
    //         }
    //     }

    // }
    // let mut opponents_gain = 0;
    // if node.depth == 2 {
    //     let our_piece_value = if let Some((piece, _)) = b.most_valuable_piece(b.us()) {
    //         eval.mb.material_weights[piece].s() as i32
    //     } else {
    //         0 // all we have is a king
    //     };
    //     // opponents_gain = cmp::max(our_piece_value, promo_value);
    // }

    // let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
    // if node.depth > 1 && (b.pawns() & b.them() & Bitboard::home_half(b.color_us()) & near_promos).any() {
    //     return None;
    // }

    // reverse futility
    //
    // thinking straight from HGM post on deep futiltiy pruning
    //
    // so if at d=1 we are going to prune because
    //      eval + margin + mv.gain <= node.alpha
    // then
    //     setting M = max_value(opponent piece on board at d1)
    //
    // implies
    //     mv.gain <= M for any move
    //
    // so
    //     eval + margin + mv.gain <= eval + margin + M
    //
    // and we will certainly prune the child node if
    //
    //     eval + margin + M < node.alpha
    //
    //  or looking from the parent node at d=2 (negamax reverses with alpha = -beta)
    //
    //     eval_at_d1 - margin_d1 - M > node_d2.beta    (mult by -1)
    //
    // but eval_at_d1 >= static_eval_at_d2 + gain(mv_at_d2) - margin_d2
    //
    //  so if static_eval_at_d2 + gain(mv_at_d2) - margin_d2 > node_d2.beta + M(at d1)+ margin_at_d1
    //  then we can prune
    //
    // We are ignoring the phase difference between d1 and d2 (a queen could be captured),
    // so this has to be taken into account using the margin2 (which should be bigger than margin 1)
    // and since our margin is cumulative, we have margin 2 strictly > 2 * margin 1
    //
    // pub fn reverse_futility(&self, b: &Board, n: &Node) -> Option<Score> {
    //     if self.beta_enabled {
    //         let piece_value;
    //         if let Some((piece, _)) = b.most_valuable_piece(b.us()) {
    //             piece_value = Score::from_cp(eval.mb.material_weights[piece].s() as i32);
    //         } else {
    //             piece_value = Score::zero();
    //         }
    //         let est_score = measure.eval - measure.margin + Score::from_cp(gain) - piece_value;
    //         if est_score >= node.beta {
    //             return Some(est_score);
    //         }
    //     }
    // }
}

impl fmt::Display for Futility {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::catalog::*;
    //use crate::comms::uci::*;
    // use crate::eval::eval::*;

    #[test]
    fn test_futility() {
        let nmp = Futility::default();
        println!("{:#?}", nmp);
        println!("{}", nmp);
    }
}
