
// use crate::Bitboard;
// use crate::board::Board;
use crate::bound::NodeType;
// use crate::eval::weight::Weight;
// use crate::search::node::Node;
use crate::search::algo::Algo;
use crate::mv::Move;
use crate::variation::Variation;
use crate::eval::score::Score;
// use crate::eval::switches::Switches;
// use crate::eval::eval::SimpleScorer;
use crate::config::{Config, Component};
// use crate::{debug, logger::LogInit};
use crate::types::{MoveType};
use std::{fmt};






#[derive(Clone, Debug)]
pub struct SearchExplainer {
    pub enabled: bool,
    pub is_explaining: bool,


    pub vars: Vec<Variation>,
}

impl Component for SearchExplainer {
    fn settings(&self, c: &mut Config) {
        c.set("explainer.enabled", &format!("type check default {}", self.enabled));
    }
    fn configure(&mut self, c: &Config) {
        debug!("explainer.configure");
        self.enabled = c.bool("explainer.enabled").unwrap_or(self.enabled);
    }

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {
    }
}

impl Default for SearchExplainer {
    fn default() -> Self {
        SearchExplainer {
            enabled: false,
            is_explaining: false,
            vars: Vec::new(),
        }
    }
}



impl fmt::Display for SearchExplainer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}




impl SearchExplainer {
    pub fn add_variation_to_explain(&mut self, var: Variation) {
        if self.enabled {
            self.vars.push(var);
        }
    }

    #[inline]
    pub fn start(&mut self, current: &Variation) {
        if self.enabled {
            self.is_explaining = self.enabled && self.vars.iter().any(|v| current.starts_with(v));
            if self.is_explaining {
                // warn!("Explaining {}", self.variation);
            }
        }
    }

    #[inline]
    pub fn stop(&mut self) {
        self.is_explaining = false;
    }

}

impl Algo {
    #[inline]
    pub fn explain_futility(&mut self, mv: &Move, move_type: MoveType, estimated: Score, alpha: Score) {
        if self.explainer.enabled && self.explainer.is_explaining {
            warn!("Var {} - futile move {} of type {} scores an estimated {} against {}", self.var(), mv, move_type, estimated, alpha)
        }
    }

    #[inline]
    pub fn explain_raised_alpha(&self, mv: &Move, child_score: Score, alpha: Score) {
        if self.explainer.enabled && self.explainer.is_explaining {
            warn!("Var {} move {} scored {} and raised alpha from {}", self.var(), mv, child_score, alpha);
        }
    }

    #[inline]
    pub fn explain_nmp(&self, child_score: Score, beta: Score) {
        if self.explainer.enabled && self.explainer.is_explaining {
            warn!("Var {} null move scored {} and cutoff beta at {}", self.var(), child_score, beta);
        }
    }

    #[inline]
    pub fn explain_node(&self, bm: &Move, nt: NodeType, score: Score, pv: &Variation) {
        if self.explainer.enabled && self.explainer.is_explaining {
            warn!(">>> {} best move {} scored {} at node type {} pv {}", self.var(), bm, score, nt, pv);
        }
    }

}




#[cfg(test)]
mod tests {
    use crate::{BoardBuf, Position, position, search::{engine::Engine, timecontrol::TimeControl}};

    use super::*;
    use crate::test_env_log::test;
    // use crate::catalog::*;
    //use crate::comms::uci::*;
    // use crate::eval::eval::*;

    #[test]
    fn test_explainer() {

        let mut eng = Engine::new();
        let pos = Position::parse_epd("r1b1k2r/1p3p1p/p2p4/6B1/1q1np3/2Q5/PPP1BPPP/1R2K2R w Kkq - 1 15  ").unwrap();
        let var = pos.board().parse_san_variation("").unwrap();
        eng.algo.explainer.add_variation_to_explain(var);

        // let var = pos.board().parse_san_variation("Qxc3").unwrap();
        // eng.algo.explainer.add_variation_to_explain(var);

        eng.set_position(pos);
        eng.algo.set_timing_method(TimeControl::Depth(2));
        eng.search();
        // warn!("{}", eng);
    }
}
