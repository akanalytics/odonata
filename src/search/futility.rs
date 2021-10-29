
use crate::Bitboard;
use crate::board::Board;
use crate::eval::weight::Weight;
use crate::search::node::Node;
use crate::mv::Move;
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::eval::eval::SimpleScorer;
use crate::infra::parsed_config::{Component};
// use crate::{debug, logger::LogInit};
use crate::types::{Piece, MoveType, MoveTypes, Ply};
use std::{cmp, fmt};
use serde::{Deserialize, Serialize};




// 5+0.04 => 165/1600
// 10+0.08 => 82/1189 (3000 sims) 


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Futility {
    pub alpha_enabled: bool,
    pub beta_enabled: bool,
    pub prune_remaining: bool,
    pub avoid_checks: bool, 
    pub avoid_promos: bool, 
    pub promo_margin: bool, 
    pub max_depth: Ply,
    pub max_depth_captures: Ply,
    pub margin1: i32,
    pub margin2: i32,
    pub margin3: i32,
    pub eval_switches: Switches,
    pub move_types_forbidden: MoveTypes,
}

impl Component for Futility {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
    }
}

impl Default for Futility {
    fn default() -> Self {
        Futility {
            alpha_enabled: true,
            beta_enabled: false,
            prune_remaining: false,
            avoid_checks: false,
            avoid_promos: false,  
            promo_margin: false,  
            max_depth: 2, // not sure > 2 really makes sense
            max_depth_captures: 2, 
            margin1: 100,
            margin2: 250,
            margin3: 1500,
            eval_switches: Switches::ALL_SCORING, 
            move_types_forbidden: MoveTypes::EMPTY // MoveType::Hash | MoveType::Killer, // HK 
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


impl Futility {
    pub fn can_prune_at_node(&self, b: &Board, node: &Node, eval: Score) -> Option<FutilityMeasure> {
        if (!self.alpha_enabled && !self.beta_enabled)
            ||
            node.ply == 0   // dont prune at root node
            ||
            // node.alpha + Score::from_cp(1) == node.beta // not in PVS
            // ||
            node.depth > self.max_depth // dont prune too far away from leaf nodes
            ||
            node.alpha.is_mate()  // dont prune if either alpha or beta is a mate score
            || 
            node.beta.is_mate() 
            ||
            (self.avoid_checks && b.is_in_check(b.color_us())) {
            return None;
        }

        // safety margin depends on how far away we are from leaf node
        let margin = match node.depth {
            1 => self.margin1,
            2 => self.margin2,
            3 => self.margin3,
            _ => self.margin1 + self.margin2 + self.margin3,
        };

        // if the score + a configured margin is less than alpha we can consider pruning at this node
        Some(FutilityMeasure {
            eval,
            margin: Score::from_cp(margin),
        })
    }

    // for each move at a prunable node, see if its worth pruning
    // if this move isnt a capture then even if the opponents move IS a capture our score will still 
    // likely fall short of alpha
    // for futility at depth = 3, conceivably our second move could be a capture so risky to prune
    //
    // obviously even prunign at depth=2, this move could be a quite move that attacks a piece and means quiese
    // changes the score dramatically - so futility pruning at depth = 1/2 is not without downside
    //
    pub fn can_prune_move(&self, mv: &Move, mt: MoveType, b: &Board, measure: FutilityMeasure, node: &Node, eval: &SimpleScorer) -> Option<Score> {
        if !self.alpha_enabled {
            return None;
        }
        if self.move_types_forbidden.contains(mt) {
            return None;
        }
        if !node.alpha.is_numeric() || node.alpha.is_mate() || node.beta.is_mate() {
            return None;
        }

        if self.avoid_checks || b.will_check_them(mv) {
            return None;
        } 

        // position wise, passed pawn promos make a huge impact so exclude them
        if self.avoid_promos && mv.mover_piece() == Piece::Pawn && mv.to().rank_index_as_white(b.color_us()) >= 6 {
            return None;
        } 

        if node.depth > self.max_depth_captures && mv.is_capture() {
            return None;
        }

        // not a capture or promo => gain = 0
        let gain = b.eval_move_material(eval, mv);

        // fail low pruning
        let est_score = measure.eval + measure.margin + gain;
        if est_score <= node.alpha {
            return Some(est_score);
        }
        None
    }

    // at depth = 1, assuming captures sorted by MVVLVA, then once one falls below futility, 
    // then the rest will, and quiet
    // moves will also be so far away from alpha we can prune them too
    // assumes already the move itself is futile on above test 
    // and we dont prune Hash moves as they are not ordered and may have a capture value < later moves
    pub fn can_prune_remaining_moves(&self, b: &Board, mt: MoveType, _n: &Node) -> bool {
        if mt == MoveType::Hash  {
            return false;
        }
        // we might be pruning a low value capture, with a high value promo later in the move list
        let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
        if (b.pawns() & b.us() & Bitboard::home_half(b.color_them()) & near_promos).any() {
            return false;
        }
        self.prune_remaining
    }


    pub fn can_prune_all_moves(&self, b: &Board, measure: FutilityMeasure, node: &Node, eval: &SimpleScorer) -> Option<Score> {

        if !self.beta_enabled {
            return None;
        }
        let phase = b.phase(&eval.phaser);
        let maximum_opp_piece = if let Some((piece, _)) = b.most_valuable_piece_except_king(b.them()) {
            eval.mb.piece_weights[piece]
        } else {
            Weight::zero() // all they have is a king
        };

        let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
        let promo_value = if (b.pawns() & b.us() & Bitboard::home_half(b.color_them()) & near_promos).any() {
            eval.mb.piece_weights[Piece::Queen] - eval.mb.piece_weights[Piece::Pawn]
        } else {
            Weight::zero() // no promos possible
        };
        let gain = cmp::max( maximum_opp_piece.interpolate(phase) as i32, promo_value.interpolate(phase) as i32);

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
