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
use crate::types::MoveType;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use treeline::Tree;
use fmt::{Debug,Display};

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
    vars: Vec<Variation>,

    #[serde(skip)]
    tree: Arc<Mutex<SearchTree<Move>>>,
}


// inspired by https://crates.io/crates/treeline (License MIT)


#[derive(Debug, Default, Clone)]
struct SearchTree<E: Clone + Debug + Default + Display> {
    root: E,
    leaves: Vec<SearchTree<E>>,
}

impl<D:Clone + Debug + Default + Display> SearchTree<D> {
    fn display_leaves(f: &mut fmt::Formatter, leaves: &[SearchTree<D>], spaces: Vec<bool>) -> fmt::Result {
        for (i, leaf) in leaves.iter().enumerate() {
            let last = i >= leaves.len() - 1;
            // print single line
            for s in &spaces {
                if *s {
                    write!(f, "    ")?;
                } else {
                    write!(f, "|   ")?;
                }
            }
            if last {
                writeln!(f, "└── {}", leaf.root)?;
            } else {
                writeln!(f, "├── {}", leaf.root)?;
            }

            // recurse
            if !leaf.leaves.is_empty() {
                let mut clone = spaces.clone();
                clone.push(last);
                Self::display_leaves(f, &leaf.leaves, clone)?;
            }
        }
        write!(f, "")
    }
}

impl<D: Clone + Debug + Default + Display> Display for SearchTree<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.root)?;
        Self::display_leaves(f, &self.leaves, Vec::new())
    }
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
                self.vars.clear();
            }
            StartSearch => {}
            EndSearch => {
                if self.enabled {
                    self.write_explanation().unwrap();
                }
            }
            StartDepthIteration(_) => {}
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
            vars: Vec::new(),
            tree: Default::default(),
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

    fn write_explanation(&mut self) -> Result<()> {
        let mut writer: BufWriter<File> = self.open_file()?;

        writer.flush()?;
        Ok(())
    }

    pub fn open_file(&mut self) -> Result<BufWriter<File>> {
        let filename = format!("explain.csv"); // SEARCH_COUNTER.load(Ordering::SeqCst));
        let f = File::create(&filename).with_context(|| format!("Failed to open file {}", &filename))?;
        let writer = BufWriter::new(f);
        println!("*****Opened {}", filename);
        Ok(writer)
    }

    #[inline]
    pub fn start(&mut self, n: &Node, current: &Variation) {
        if self.enabled {
            self.is_explaining = self.enabled
                && n.depth >= self.min_depth
                && (self
                    .vars
                    .iter()
                    .any(|v| current.starts_with(v) && current.len() <= v.len() + self.max_additional_ply as usize)
                    || self.vars.is_empty());
            if self.is_explaining {
                // println!("Explaining {}", self.variation);
            }
        }
    }

    #[inline]
    pub fn stop(&mut self) {
        self.is_explaining = false;
    }
}

fn header(_n: &Node, var: &Variation) -> String {
    let strings: Vec<String> = var.iter().map(Move::to_string).collect();
    format!("{:<26}", strings.join("."))
}

impl Algo {
    #[inline]
    pub fn explain_futility(&mut self, mv: &Move, move_type: MoveType, estimated: Score, n: &Node) {
        if !self.explainer.enabled || !self.explainer.is_explaining {
            return;
        }

        // if let Some(w) = &self.explainer.writer {
        //     writeln!(
        //         w.lock().unwrap(),
        //         "{} futile move {} of type {} scores an estimated {} against {}",
        //         header(n, self.var()),
        //         mv,
        //         move_type,
        //         estimated,
        //         n.alpha
        //     )
        //     .unwrap();
        // }
    }

    #[inline]
    pub fn explain_move(&self, mv: &Move, child_score: Score, cat: Event, n: &Node) {
        if !self.explainer.enabled || !self.explainer.is_explaining {
            return;
        }

        let (text, bound) = match child_score {
            d if d >= n.beta => ("beta cutoff", n.beta),
            d if d > n.alpha => ("raised alpha", n.alpha),
            _ => ("failed low", n.alpha),
        };
        // if let Some(w) = &self.explainer.writer {
        //     writeln!(
        //         w.lock().unwrap(),
        //         "{} move {} scored {} and {} {} cat {}",
        //         header(n, self.var()),
        //         mv,
        //         child_score,
        //         text,
        //         bound,
        //         cat
        //     )
        //     .unwrap();
        // }
    }

    #[inline]
    pub fn explain_nmp(&self, child_score: Score, n: &Node) {
        if !self.explainer.enabled || !self.explainer.is_explaining {
            return;
        }
        // if let Some(w) = &self.explainer.writer {
        //     writeln!(
        //         w.lock().unwrap(),
        //         "{} null move scored {} and cutoff beta at {}",
        //         header(n, self.var()),
        //         child_score,
        //         n.beta
        //     )
        //     .unwrap();
        // }
    }

    #[inline]
    pub fn explain_node(&self, bm: &Move, nt: NodeType, score: Score, n: &Node, pv: &Variation) {
        if !self.explainer.enabled || !self.explainer.is_explaining {
            return;
        }
        // if let Some(w) = &self.explainer.writer {
        //     writeln!(
        //         w.lock().unwrap(),
        //         "{} best move {} scored {} at node type {} pv {}",
        //         header(n, self.var()),
        //         bm,
        //         score,
        //         nt,
        //         pv
        //     )
        //     .unwrap();
        // }
        let tree = self.explainer.tree.lock();
    }
}

#[cfg(test)]
mod tests {
    use crate::test_log::test;
    use crate::{
        search::{engine::Engine, timecontrol::TimeControl},
        Position,
    };

    #[test]
    fn test_explainer() {
        let mut eng = Engine::new();
        let pos = Position::parse_epd("r1b1k2r/1p3p1p/p2p4/6B1/1q1np3/2Q5/PPP1BPPP/1R2K2R w Kkq - 1 15").unwrap();
        let _var = pos.board().parse_san_variation("").unwrap();
        // eng.algo.explainer.add_variation_to_explain(var);

        // let var = pos.board().parse_san_variation("Qxc3").unwrap();
        // eng.algo.explainer.add_variation_to_explain(var);

        eng.set_position(pos);
        eng.algo.set_timing_method(TimeControl::Depth(2));
        eng.search();
        // warn!("{}", eng);
    }
}
