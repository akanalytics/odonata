use crate::Board;
// use crate::Bitboard;
// use crate::board::Board;
use crate::{bound::NodeType, types::Ply};
// use crate::eval::weight::Weight;
// use crate::search::node::Node;
use crate::eval::score::Score;
use crate::mv::Move;
use crate::search::algo::Algo;
use crate::variation::Variation;
// use crate::eval::switches::Switches;
// use crate::eval::eval::SimpleScorer;
use crate::infra::component::{Component, State};
// use crate::{debug, logger::LogInit};
use super::node::{Event, Node};
use crate::domain::tree::{SearchTree, SearchTreeWeight};
use crate::types::MoveType;
use anyhow::{Context, Result};
use fmt::Debug;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};

// static SEARCH_COUNTER: AtomicU32 = AtomicU32::new(0);
// SEARCH_COUNTER.fetch_add(1, Ordering::SeqCst);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Explainer {
    pub enabled: bool,
    min_depth: Ply,
    max_additional_ply: Ply,
    is_explaining: bool,

    #[serde(skip)]
    iter: i32,

    #[serde(skip)]
    vars: Vec<Variation>,

    #[serde(skip)]
    why_not: Option<Variation>,

    #[serde(skip)]
    tree: Option<SearchTree>,
}

impl Component for Explainer {
    fn new_iter(&mut self) {}

    fn new_position(&mut self) {}

    fn new_game(&mut self) {}

    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame | SetPosition => {
                self.is_explaining = false;
                // self.vars.clear();
            }
            StartSearch => {}
            EndSearch => {
                if self.enabled {
                    self.write_explanation().unwrap();
                }
            }
            StartDepthIteration(iter) => {
                self.iter = iter;
                self.tree = None;
            }
        }
    }
}

impl Default for Explainer {
    fn default() -> Self {
        Explainer {
            enabled: false,
            is_explaining: false,
            min_depth: 0,
            max_additional_ply: 4,
            iter: 0,
            vars: vec![Variation::new()],
            why_not: None, 
            tree: None,
        }
    }
}

impl fmt::Display for Explainer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}

impl Explainer {
    pub fn add_variation_to_explain(&mut self, var: Variation) {
        if self.enabled {
            self.vars.push(var);
        }
    }

    pub fn why_not(&mut self, var: Variation) {
        if self.enabled {
            self.why_not = Some(var);
        }
    }

    fn write_explanation(&mut self) -> Result<()> {
        // if its been "started" write a file
        if self.tree.is_some() {
            let mut writer: BufWriter<File> = self.open_file()?;
            let tree = self.tree.take().unwrap();
            writeln!(writer, "Initial position {}", tree.initial_position.to_fen())?;
            writeln!(writer, "{}", tree.tree)?;
            writer.flush()?;
        }
        Ok(())
    }

    pub fn open_file(&mut self) -> Result<BufWriter<File>> {
        let filename = format!("explain-{:02}.csv", self.iter); // SEARCH_COUNTER.load(Ordering::SeqCst));
        println!("Opening file {} for explainer", filename);
        let f = File::create(&filename).with_context(|| format!("Failed to open file {}", &filename))?;
        let writer = BufWriter::new(f);
        Ok(writer)
    }

    /// if explaining this variation (ie this node's var is an extension of one of the variations configured,
    /// but not beyond max_additional_ply),
    /// then return a SearchTreeWeight to be populated, else None
    #[inline]
    pub fn explaining(&mut self, n: &Node, var: &Variation) -> Option<&mut SearchTreeWeight> {
        if !self.enabled
            || n.depth < self.min_depth {
                return None;
            }
        if self
                .vars
                .iter()
                // .inspect(|x| println!("about to check var: {}", x))
                .any(|v| var.starts_with(v) && var.len() <= v.len() + self.max_additional_ply as usize)
        {
            if self.tree.is_none() {
                let tree = SearchTree::new(Board::default());
                self.tree = Some(tree);
            }
            if let Some(tree) = &mut self.tree {
                return Some(tree.get_or_insert(var));
            };
        }
        None
    }

    #[inline]
    pub fn stop(&mut self) {
        self.is_explaining = false;
    }
}

// fn header(_n: &Node, var: &Variation) -> String {
//     let strings: Vec<String> = var.iter().map(Move::to_string).collect();
//     format!("{:<26}", strings.join("."))
// }

impl Algo {
    #[inline]
    pub fn explain_futility(&mut self, mv: Move, _move_type: MoveType, estimated: Score, n: &Node, e: Event) {
        if self.explainer.enabled {
            if let Some(w) = self.explainer.explaining(n, &self.current_variation.append(mv)) {
                w.score = estimated;
                w.node = *n;
                w.event = e;
            }
        }
    }

    #[inline]
    pub fn explain_move(&mut self, mv: Move, child_score: Score, cat: Event, n: &Node) {
        if self.explainer.enabled {
            if let Some(w) = self.explainer.explaining(n, &self.current_variation.append(mv)) {
                // let bound = match child_score {
                //     d if d >= n.beta => Event::NodeInteriorCut,
                //     d if d > n.alpha => Event::NodeInteriorPv,
                //     _ => Event::NodeInteriorAll,
                // };
                w.score = child_score;
                w.node = *n;
                w.event = cat;
            }
        }
    }

    #[inline]
    pub fn explain_nmp(&mut self, child_score: Score, n: &Node) {
        if self.explainer.enabled {
            if let Some(w) = self.explainer.explaining(n, &self.current_variation) {
                w.score = child_score;
                w.node = *n;
                w.event = Event::PruneNullMovePrune;
            }
        }
    }

    #[inline]
    pub fn explain_node(&mut self, bm: Move, nt: NodeType, score: Score, n: &Node, _pv: &Variation) {
        if self.explainer.enabled {
            if let Some(w) = self.explainer.explaining(n, &self.current_variation) {
                w.score = score;
                w.node = *n;
                w.nt = nt;
            }
            if nt == NodeType::ExactPv {
                if let Some(w) = self.explainer.explaining(n, &self.current_variation.append(bm)) {
                    w.is_best_move = true;
                    w.nt = nt;
                    debug!("{}: {} setting best move to {}", self.explainer.iter, &self.current_variation, bm);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::Catalog;
    use crate::search::{engine::Engine, timecontrol::TimeControl};
    use crate::test_log::test;

    #[test]
    fn test_search_explainer() {
        let mut eng = Engine::new();
        eng.algo.explainer.enabled = true;
        assert_eq!(eng.algo.explainer.vars.len(), 1);

        let pos = Catalog::starting_position();
        // let pos = Position::parse_epd("r1b1k2r/1p3p1p/p2p4/6B1/1q1np3/2Q5/PPP1BPPP/1R2K2R w Kkq - 1 15").unwrap();
        // let var = pos.board().parse_san_variation("").unwrap();
        // eng.algo.explainer.add_variation_to_explain(var);

        let v1 = pos.board().parse_san_variation("b2b4").unwrap();
        eng.algo.explainer.add_variation_to_explain(v1.clone());
        assert_eq!(eng.algo.explainer.vars.len(), 2);

        assert!(eng.algo.explainer.vars.iter().any(|v| v1.starts_with(v)));

        eng.set_position(pos);
        eng.algo.set_timing_method(TimeControl::Depth(6));

        eng.search();
        println!("{:?}", eng.algo.explainer);
        // warn!("{}", eng);
    }
}
