use std::fmt;

use odonata_base::domain::node::Node;
use odonata_base::domain::staticeval::StaticEval;
use odonata_base::infra::component::Component;
use odonata_base::infra::metric::{Event, Metrics};
use odonata_base::piece::MoveType;
use odonata_base::prelude::*;
use strum_macros::EnumString;

use super::algo::Search;

// 5+0.04 => 165/1600
// 10+0.08 => 82/1189 (3000 sims)

#[derive(Clone, Debug, EnumString)]
pub enum MoveEvalMethod {
    Classical,
    Weighted,
    See,
}

#[derive(Clone, Debug)]
pub struct Futility {
    discoverer:           bool,
    first_move:           bool,
    giving_check:         bool,
    in_check:             bool,
    margin_ply:           i32,
    margin_qs:            i32,
    max_depth_captures:   Ply,
    max_depth:            Ply,
    max_pawn_rank:        u8,
    min_pieces_depth:     Ply,
    min_pieces:           i32,
    move_eval_method:     MoveEvalMethod,
    move_types_forbidden: MoveType,
    prune_alpha_mate:     bool,
    prune_beta_mate:      bool,
    prune_extensions:     bool,
    prune_fw_node:        bool,
    prune_remaining:      bool,
    pub alpha_enabled:    bool,
    pub beta_enabled:     bool,
    pub margin1:          i32,
    pub margin2:          i32,
    pub margin3:          i32,
    pub margin4:          i32,
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
            alpha_enabled:        true,
            beta_enabled:         false,
            discoverer:           false,
            first_move:           false,
            giving_check:         false,
            in_check:             false,
            margin_ply:           100,
            margin_qs:            0,
            margin1:              65,
            margin2:              200,
            margin3:              825,
            margin4:              1000,
            max_depth_captures:   2,
            max_depth:            20, // not sure > 2 really makes sense
            max_pawn_rank:        5,
            min_pieces_depth:     1,
            min_pieces:           0,
            move_eval_method:     MoveEvalMethod::Classical,
            move_types_forbidden: MoveType::empty(), // MoveType::Hash | MoveType::Killer, // HK
            prune_alpha_mate:     false,
            prune_beta_mate:      true,
            prune_extensions:     false,
            prune_fw_node:        true,
            prune_remaining:      false,
        }
    }
}

impl Configurable for Futility {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.alpha_enabled.set(p.get("alpha_enabled"))?;
        self.beta_enabled.set(p.get("beta_enabled"))?;
        self.discoverer.set(p.get("discoverer"))?;
        self.first_move.set(p.get("first_move"))?;
        self.giving_check.set(p.get("giving_check"))?;
        self.in_check.set(p.get("in_check"))?;
        self.margin_ply.set(p.get("margin_ply"))?;
        self.margin_qs.set(p.get("margin_qs"))?;
        self.margin1.set(p.get("margin1"))?;
        self.margin2.set(p.get("margin2"))?;
        self.margin3.set(p.get("margin3"))?;
        self.margin4.set(p.get("margin4"))?;
        self.max_depth_captures.set(p.get("max_depth_captures"))?;
        self.max_depth.set(p.get("max_depth"))?;
        self.max_pawn_rank.set(p.get("max_pawn_rank"))?;
        self.min_pieces_depth.set(p.get("min_pieces_depth"))?;
        self.min_pieces.set(p.get("min_pieces"))?;
        self.move_eval_method.set(p.get("move_eval_method"))?;
        Configurable::set(&mut self.move_types_forbidden, p.get("move_types_forbidden"))?;
        self.prune_alpha_mate.set(p.get("prune_alpha_mate"))?;
        self.prune_beta_mate.set(p.get("prune_beta_mate"))?;
        self.prune_extensions.set(p.get("prune_extensions"))?;
        self.prune_fw_node.set(p.get("prune_fw_node"))?;
        self.prune_remaining.set(p.get("prune_remaining"))?;
        Ok(p.is_modified())
    }
}

impl fmt::Display for Futility {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FutilityMeasure {
    eval:   Score,
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

impl Search {
    // #[inline]
    // pub fn standing_pat(&mut self, b: &Board, n: &mut Node, standing_pat: Score) -> Option<Score> {
    //     if !n.is_qs() {
    //         return None;
    //     }

    //     if n.depth == 0 {
    //         let outcome = b.outcome();
    //         if outcome.is_game_over() {
    //             if outcome.is_draw() {
    //                 Metrics::incr_node(&n, Event::StandingPatSuccess);
    //                 return Some(b.pov_score(self.eval.w_eval_draw(b, n)));
    //             }
    //             // depth 0: we have considered a full width move search to get here so a winning
    //             // result is valid. Beyond depth 0 it is not.
    //             if let Some(c) = outcome.winning_color() {
    //                 if c == b.color_us() {
    //                     Metrics::incr_node(&n, Event::StandingPatSuccess);
    //                     return Some(Score::we_win_in(n.ply));
    //                 } else {
    //                     Metrics::incr_node(&n, Event::StandingPatSuccess);
    //                     return Some(Score::we_lose_in(n.ply));
    //                 }
    //             }
    //         }
    //     }
    //     if standing_pat >= n.beta && !b.is_in_check(b.color_us()) {
    //         Metrics::incr_node(&n, Event::StandingPatSuccess);
    //         return Some(standing_pat);
    //     }

    //     if standing_pat > n.alpha && !b.is_in_check(b.color_us()) {
    //         n.alpha = standing_pat;
    //     }

    //     if self.qsearch_disabled {
    //         Metrics::incr_node(&n, Event::StandingPatSuccess);
    //         return Some(standing_pat);
    //     }
    //     None
    // }

    #[inline]
    pub fn can_futility_prune_at_node(&self, b: &Board, n: &Node) -> bool {
        if (!self.futility.alpha_enabled && !self.futility.beta_enabled) || n.ply == 0 {
            // dont prune at root node
            // node.alpha + Score::from_cp(1) == node.beta // not in PVS
            return false;
        }
        if n.depth > self.max_depth {
            // dont prune too far away from leaf nodes
            Metrics::incr_node(n, Event::FutilityDeclineMaxDepth);
            return false;
        }

        if (!self.futility.prune_alpha_mate && n.alpha.is_mate())
            || (!self.futility.prune_beta_mate && n.beta.is_mate())
        {
            Metrics::incr_node(n, Event::FutilityDeclineMateBound);
            return false;
        }
        if !self.futility.prune_fw_node && n.is_fw() {
            // VER:0.4.14
            Metrics::incr_node(n, Event::FutilityDeclineFwWindow);
            return false;
        }
        if !self.futility.in_check && b.is_in_check(b.color_us()) {
            Metrics::incr_node(n, Event::FutilityDeclineInCheck);
            return false;
        }
        if self.futility.min_pieces > 0
            && n.depth >= self.futility.min_pieces_depth
            && b.occupied().popcount() < self.futility.min_pieces
        {
            Metrics::incr_node(n, Event::FutilityDeclineMinPieces);
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

        Metrics::incr_node(n, Event::FutilityConsider);
        if !self.futility.prune_extensions && ext != 0 {
            Metrics::incr_node(n, Event::FutilityDeclineExtension);
            return None;
        }
        if self.futility.move_types_forbidden.contains(mt) {
            return None;
        }

        if !self.futility.first_move && mv_num <= 1 {
            Metrics::incr_node(n, Event::FutilityDeclineFirstMove);
            return None;
        }

        if !n.alpha.is_numeric() {
            Metrics::incr_node(n, Event::FutilityDeclineMateBound);
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
            Metrics::incr_node(n, Event::FutilityDeclineDiscoverer);
            return None;
        }

        // gives check a more precise and costly version of discoverers
        if !self.futility.giving_check && after.is_in_check(after.color_us()) {
            Metrics::incr_node(n, Event::FutilityDeclineGivesCheck);
            return None;
        }

        // position wise, passed pawn promos make a huge impact so exclude them
        if mv.mover_piece(before) == Piece::Pawn
            && mv.from().rank_number_as_white(before.color_us()) > self.futility.max_pawn_rank as usize
        {
            Metrics::incr_node(n, Event::FutilityDeclinePawnMaxRank);
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
            4 => self.futility.margin4,
            d => self.futility.margin4 + self.futility.margin_ply * (d - 4),
        });

        // not a capture or promo => gain = 0
        // let gain = before.eval_move_material(&self.eval, mv);
        let gain = self.eval_move_material(before, &self.eval, mv);
        // if est_score <= n.alpha
        //     && EndGame::from_board(after).likely_outcome(after) != LikelyOutcome::UnknownOutcome
        // {
        //     return None;
        // }

        // fail low pruning - even when we add a safety margin - is it still too low
        let est_score = eval.clamp_score() + margin + gain;
        if est_score <= n.alpha {
            let category = match n.depth {
                d if d <= 0 => Event::FutilityD0,
                1 => Event::FutilityD1,
                2 => Event::FutilityD2,
                _ => Event::FutilityD3,
            };
            // self.explain_futility(before, mv, mt, est_score, n, category);
            Metrics::incr_node(n, category);

            return Some(est_score);
        } else {
            Metrics::incr_node(n, Event::FutilityFail);
        }
        None
    }

    fn eval_move_material(&self, before: &Board, eval: &dyn StaticEval, mv: Move) -> Score {
        match self.futility.move_eval_method {
            MoveEvalMethod::Classical => {
                let mut cp = 0;
                if let Some(cap) = mv.capture_piece(before) {
                    cp += cap.centipawns();
                }
                if let Some(promo) = mv.promo_piece() {
                    cp += promo.centipawns() - Piece::Pawn.centipawns();
                }
                Score::from_cp(cp)
            }
            MoveEvalMethod::Weighted => eval.move_material_eval(mv, before).pov_score(),
            MoveEvalMethod::See => {
                let mut cp = eval.eval_move_see(mv, before);
                if let Some(promo) = mv.promo_piece() {
                    cp = cp + Score::from_cp(promo.centipawns() - Piece::Pawn.centipawns());
                }
                cp
            }
        }
        // eval.move_material_eval(mv, before).pov_score()
        // let mut wt = Weight::zero();
        // if let Some(cap) = mv.capture_piece(before) {
        //     wt += eval.mb.piece_weights[cap];
        // }
        // if let Some(promo) = mv.promo_piece() {
        //     wt += eval.mb.piece_weights[promo] - eval.mb.piece_weights[Piece::Pawn];
        // }
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
}
// pub fn can_prune_all_moves(
//     &self,
//     b: &Board,
//     measure: FutilityMeasure,
//     node: &Node,
//     eval: &Eval,
// ) -> Option<Score> {
//     if !self.futility.beta_enabled {
//         return None;
//     }
//     let phase = b.phase(&eval.phaser);
//     let maximum_opp_piece =
//         if let Some((piece, _)) = b.most_valuable_piece_except_king(b.them()) {
//             eval.mb.piece_weights[piece]
//         } else {
//             Weight::zero() // all they have is a king
//         };

//     let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
//     let promo_value =
//         if (b.pawns() & b.us() & Bitboard::home_half(b.color_them()) & near_promos).any() {
//             eval.mb.piece_weights[Piece::Queen] - eval.mb.piece_weights[Piece::Pawn]
//         } else {
//             Weight::zero() // no promos possible
//         };
//     let gain = cmp::max(
//         maximum_opp_piece.interpolate(phase) as i32,
//         promo_value.interpolate(phase) as i32,
//     );

//     // fail low pruning
//     let est_score = measure.eval + measure.margin + Score::from_cp(gain);
//     if est_score <= node.alpha {
//         return Some(est_score);
//     }
//     None
// }

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

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::catalog::*;
    // use crate::comms::uci::*;
    // use crate::eval::eval::*;

    #[test]
    fn test_futility() {
        let nmp = Futility::default();
        println!("{:#?}", nmp);
        println!("{}", nmp);
    }
}
