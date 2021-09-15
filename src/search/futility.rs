
use crate::Bitboard;
use crate::board::Board;
use crate::search::node::Node;
use crate::mv::Move;
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::eval::eval::SimpleScorer;
use crate::config::{Config, Component};
// use crate::{debug, logger::LogInit};
use crate::types::{Piece, MoveType, Ply};
use std::fmt;






#[derive(Clone, Debug)]
pub struct Futility {
    pub enabled: bool,
    pub beta_enabled: bool,
    pub eval_position: bool,
    pub prune_remaining: bool,
    pub avoid_checks: bool, 
    pub avoid_promos: bool, 
    pub max_depth: Ply,
    pub max_depth_captures: Ply,
    pub margin1: i32,
    pub margin2: i32,
    pub margin3: i32,
    pub eval_switches: Switches,
}

impl Component for Futility {
    fn settings(&self, c: &mut Config) {
        c.set("futility.enabled", &format!("type check default {}", self.enabled));
        c.set("futility.beta.enabled", &format!("type check default {}", self.beta_enabled));
        c.set("futility.eval.position", &format!("type check default {}", self.eval_position));
        c.set("futility.avoid.checks", &format!("type check default {}", self.avoid_checks));
        c.set("futility.avoid.promos", &format!("type check default {}", self.avoid_promos));
        c.set("futility.prune.remaining", &format!("type check default {}", self.prune_remaining));
        c.set("futility.max.depth",  &format!("type spin min 0 max 100 default {}", self.max_depth));
        c.set("futility.max.depth.captures",  &format!("type spin min 0 max 100 default {}", self.max_depth_captures));
        c.set("futility.margin1",  &format!("type spin min 0 max 9999 default {}", self.margin1));
        c.set("futility.margin2",  &format!("type spin min 0 max 9999 default {}", self.margin2));
        c.set("futility.margin3",  &format!("type spin min 0 max 9999 default {}", self.margin3));
    }
    fn configure(&mut self, c: &Config) {
        debug!("futility.configure");
        self.enabled = c.bool("futility.enabled").unwrap_or(self.enabled);
        self.beta_enabled = c.bool("futility.beta.enabled").unwrap_or(self.beta_enabled);
        self.eval_position = c.bool("futility.eval.position").unwrap_or(self.eval_position);
        self.prune_remaining = c.bool("futility.prune.remaining").unwrap_or(self.prune_remaining);
        self.avoid_checks = c.bool("futility.avoid.checks").unwrap_or(self.avoid_checks);
        self.avoid_promos = c.bool("futility.avoid.promos").unwrap_or(self.avoid_promos);
        self.max_depth = c.int("futility.max.depth").unwrap_or(self.max_depth as i64) as Ply;
        self.max_depth_captures = c.int("futility.max.depth.captures").unwrap_or(self.max_depth_captures as i64) as Ply;
        self.margin1 = c.int("futility.margin1").unwrap_or(self.margin1 as i64) as i32;
        self.margin2 = c.int("futility.margin2").unwrap_or(self.margin2 as i64) as i32;
        self.margin3 = c.int("futility.margin3").unwrap_or(self.margin3 as i64) as i32;
        self.eval_switches = Switches::MATERIAL | if self.eval_position { Switches::POSITION } else { Switches::NONE };

    }
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
    }
}

impl Default for Futility {
    fn default() -> Self {
        Futility {
            enabled: true,
            beta_enabled: false,
            eval_position: true,
            prune_remaining: true,
            avoid_checks: false,
            avoid_promos: true,  
            eval_switches: Switches::MATERIAL | Switches::POSITION,
            max_depth: 2, // not sure > 2 really makes sense
            max_depth_captures: 1, 
            margin1: 100,
            margin2: 250,
            margin3: 1500,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FutilityMeasure {
    eval: Score,
    margin: Score,
}

impl Futility {
    pub fn can_prune_at_node(&self, b: &Board, node: &Node, eval: &SimpleScorer) -> Option<FutilityMeasure> {
        if !self.enabled 
            ||
            node.ply == 0   // dont prune at root node
            ||
            node.depth > self.max_depth // dont prune too far away from leaf nodes
            ||
            node.alpha.is_mate()  // dont prune if either alpha or beta is a mate score
            || 
            node.beta.is_mate() 
            ||
            (self.avoid_checks && b.is_in_check(b.color_us())) {
            return None;
        }
        let score = b.eval_some(eval, self.eval_switches);

        // safety margin depends on how far away we are from leaf node
        let margin = match node.depth {
            1 => self.margin1,
            2 => self.margin2,
            3 => self.margin3,
            _ => self.margin1 + self.margin2 + self.margin3,
        };

        // if the score + a configured margin is less than alpha we can consider pruning at this node
        Some(FutilityMeasure {
            eval: score,
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
    pub fn can_prune_move(&self, mv: &Move, b: &Board, measure: FutilityMeasure, node: &Node, eval: &SimpleScorer) -> Option<Score> {
        // if mv.is_null() || b.will_check_them(mv) {
        //     return None
        // } 

        // position wise, passed pawn promos make a huge impact so exclude them
        if self.avoid_promos && mv.mover_piece() == Piece::Pawn && mv.to().rank_index_as_white(b.color_us()) >= 6 {
            return None;
        }

        let near_promos = Bitboard::RANKS_27; // .or(Bitboard::RANKS_36);
        if node.depth > 1 && (b.pawns() & b.them() & Bitboard::home_half(b.color_us()) & near_promos).any() {
            return None;
        } 

        // not a capture or promo => gain = 0
        let gain = eval.eval_move_material(&mv);

        // fail low pruning
        if node.depth <= self.max_depth_captures || !mv.is_capture() {
            let est_score = measure.eval + measure.margin + Score::from_cp(gain);
            if est_score <= node.alpha {
                return Some(est_score);
            }
        }
        // }

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
        if node.depth <= 2 && self.beta_enabled {
            let piece_value;
            if let Some((piece, _)) = b.most_valuable_piece(b.us()) {
                piece_value = Score::from_cp(eval.mb.material_weights[piece].s() as i32);
            } else {
                piece_value = Score::zero();
            }
            let est_score = measure.eval - measure.margin + Score::from_cp(gain) - piece_value;
            if est_score >= node.beta {
                return Some(est_score);
            }
        }
        None
    }

    // at depth = 1, assuming captures sorted by MVVLVA, then once one falls below futility, 
    // then the rest will, and quiet
    // moves will also be so far away from alpha we can prune them too
    // assumes already the move itself is futile on above test 
    // and we dont prune Hash moves as they are not ordered and may have a capture value < later moves
    pub fn can_prune_remaining_moves(&self, mt: MoveType, n: &Node) -> bool {
        if mt == MoveType::Hash || n.depth > 1 {
            return false;
        }
        self.prune_remaining
    }

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
