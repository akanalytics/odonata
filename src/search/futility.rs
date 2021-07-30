
use crate::board::Board;
use crate::search::node::Node;
use crate::mv::Move;
use crate::eval::score::Score;
use crate::eval::eval::SimpleScorer;
use crate::config::{Config, Component};
use crate::{debug, logger::LogInit};
use crate::types::Ply;
use std::fmt;






#[derive(Clone, Debug)]
pub struct Futility {
    pub enabled: bool,
    pub eval_position: bool,
    pub max_depth: Ply,
    pub margin1: i32,
    pub margin2: i32,
    pub margin3: i32,
}

impl Component for Futility {
    fn settings(&self, c: &mut Config) {
        c.set("futility.enabled", "type check default true");
        c.set("futility.eval.position", "type check default true");
        c.set("futility.max.depth",  &format!("type spin min 0 max 100 default {}", self.max_depth));
        c.set("futility.margin1",  &format!("type spin min 0 max 9999 default {}", self.margin1));
        c.set("futility.margin2",  &format!("type spin min 0 max 9999 default {}", self.margin2));
        c.set("futility.margin3",  &format!("type spin min 0 max 9999 default {}", self.margin3));
    }
    fn configure(&mut self, c: &Config) {
        debug!("futility.configure");
        self.enabled = c.bool("futility.enabled").unwrap_or(self.enabled);
        self.eval_position = c.bool("futility.eval.position").unwrap_or(self.eval_position);
        self.max_depth = c.int("futility.max.depth").unwrap_or(self.max_depth as i64) as Ply;
        self.margin1 = c.int("futility.margin1").unwrap_or(self.margin1 as i64) as i32;
        self.margin2 = c.int("futility.margin2").unwrap_or(self.margin2 as i64) as i32;
        self.margin3 = c.int("futility.margin3").unwrap_or(self.margin3 as i64) as i32;

    }
    fn new_game(&mut self) {
        self.new_search();
    }

    fn new_search(&mut self) {
    }
}

impl Default for Futility {
    fn default() -> Self {
        Futility {
            enabled: true,
            eval_position: true,
            max_depth: 2, // not sure > 2 really makes sense
            margin1: 100,
            margin2: 300,
            margin3: 1500,
        }
    }
}


impl Futility {
    pub fn can_prune_at_node(&self, b: &Board, node: &Node, eval: &SimpleScorer) -> bool {
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
            b.is_in_check(b.color_us()) {
            return false;
        }
        // use a static score of material and (optionally) the piece positions
        let mut score = b.eval_material(eval);
        if self.eval_position {
            score = score + b.eval_position(eval);
        }

        // safety margin depends on how far away we are from leaf node
        let margin = match node.depth {
            1 => self.margin1,
            2 => self.margin2,
            3 => self.margin3,
            _ => self.margin1 + self.margin2 + self.margin3,
        };

        // if the score + a configured margin is less than alpha we can consider pruning at this node
        // FIXME! put the scoring into the per move logic as alpha can change  
        if score + Score::from_cp(margin) <= node.alpha {
            return true
        }
        return false;

    }

    // for each move at a prunable node, see if its worth pruning
    // if this move isnt a capture then even if the opponents move IS a capture our score will still 
    // likely fall short of alpha
    // for futility at depth = 3, conceivably our second move could be a capture so risky to prune
    //
    // obviously even prunign at depth=2, this move could be a quite move that attacks a piece and means quiese
    // changes the score dramatically - so futility pruning at depth = 1/2 is not without downside
    //
    pub fn can_prune_move(&self, mv: &Move, b: &Board) -> bool {
        if mv.is_capture()
        ||
        mv.is_promo()
        ||
        mv.is_null()
        || 
        b.will_check_them(mv) {
            false
        } else {
            true 
        }

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
