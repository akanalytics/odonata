use crate::board::Board;
// use crate::Bitboard;
// use crate::board::Board;
use crate::{bound::NodeType, piece::Ply};
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
use crate::domain::{SearchTree, TreeNode, Game, search_results::SearchResultsWithExplanation};
use crate::piece::MoveType;
use anyhow::{Context, Result};
use fmt::Debug;
use fslock::LockFile;
use serde::{Deserialize, Serialize};
use std::{fmt, io};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};

// static SEARCH_COUNTER: AtomicU32 = AtomicU32::new(0);
// SEARCH_COUNTER.fetch_add(1, Ordering::SeqCst);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Explainer {
    pub explainer_enabled: bool,
    pub show_pv_eval: bool,
    pub show_metrics_on_exit: bool,
    pub export_games: bool,
    pub log_dir: String,
    pub debug: bool,


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

    #[serde(skip)]
    board: Board,
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
                if self.explainer_enabled {
                    // if self.why_not.is_some() {
                    //     self.print_whynot_explanation();
                    // }
                    if !self.vars.is_empty() {
                        self.write_explanation().unwrap();
                    }
                }
            }
            StartDepthIteration(iter) => {
                self.iter = iter;
                self.tree = None;
            }
            Shutdown => {}
        }
    }
}

impl Default for Explainer {
    fn default() -> Self {
        Explainer {
            explainer_enabled: false,
            show_pv_eval: false,
            show_metrics_on_exit: false,
            export_games: false,
            log_dir: String::new(),
            debug: false,            
            board: Default::default(),
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
        writeln!(f, "{}", toml::to_string_pretty(&self).unwrap())?;
        Ok(())
    }
}

impl Explainer {
    pub fn add_variation_to_explain(&mut self, var: Variation) {
        if self.explainer_enabled {
            self.vars.push(var);
        }
    }

    pub fn set_board(&mut self, b: Board) {
        self.board = b;
    }

    pub fn why_not(&mut self, var: Variation) {
        if self.explainer_enabled {
            self.why_not = Some(var);
        }
    }

    pub fn show_pv_eval(&self, results: &SearchResultsWithExplanation) {
        if self.show_pv_eval {
        println!("Search results\n{results}");
        }
    }    

    pub fn export_game(&self, g: &Game) -> Result<()> {
        if self.export_games {
            let mut lock_file = LockFile::open(&format!("{}/game.lock", self.log_dir))?;
            lock_file.lock()?;
            let mut w = self.game_writer(g.game_id)?;
            writeln!(w, "# begin")?;
            g.export(&mut w)?;
            writeln!(w, "# end")?;
            lock_file.unlock()?;
        }
        Ok(())
    }

    pub fn game_writer(&self, _game_id: usize) -> Result<Box<dyn Write>> {
        if !self.export_games {
            Ok(Box::new(io::sink()))
        } else {
            // let filename = format!("{}/game-{:06}.csv", self.log_dir, game_id);
            // let _dt = Local::now().format("%Y-%m-%d-%H-%M-%S.%.3f").to_string();
            // let dt = "1"; //
            let filename = format!("{}/games.txt", self.log_dir);
            info!("Opening file {} for game export", filename);
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&filename)
                .with_context(|| format!("Failed to open file {}", &filename))?;
            let writer = BufWriter::new(f);
            Ok(Box::new(writer))
        }
    }



    fn write_explanation(&mut self) -> Result<()> {
        // if its been "started" write a file
        if self.tree.is_some() {
            let mut writer: BufWriter<File> = self.open_file()?;
            let tree = self.tree.take().unwrap();
            if self.why_not.is_some() {
                writeln!(writer, "Why not... ")?;
            }
            writeln!(
                writer,
                "Initial position {}",
                tree.initial_position.to_fen()
            )?;
            writeln!(writer, "{}", tree)?;
            writer.flush()?;
        }
        Ok(())
    }

    // fn print_whynot_explanation(&mut self) {
    //     if self.tree.is_some() {
    //         let tree = self.tree.take().unwrap();
    //         println!("Why not....\nInitial position {}", tree.initial_position.to_fen());
    //         println!("{}", tree.tree);
    //     }
    // }

    pub fn open_file(&mut self) -> Result<BufWriter<File>> {
        let filename = format!("explain-{:02}.csv", self.iter); // SEARCH_COUNTER.load(Ordering::SeqCst));
        println!("Opening file {} for explainer", filename);
        let f = File::create(&filename)
            .with_context(|| format!("Failed to open file {}", &filename))?;
        let writer = BufWriter::new(f);
        Ok(writer)
    }

    /// if explaining this variation (ie this node's var is an extension of
    /// one of the variations configured,
    /// but not beyond max_additional_ply),
    /// then return a SearchTreeWeight to be populated, else None
    #[inline]
    pub fn explaining(&mut self, n: &Node, var: &Variation, e: Event) -> Option<&mut TreeNode> {
        if !self.explainer_enabled || n.depth < self.min_depth {
            return None;
        }

        debug_assert!(self.board.is_legal_variation(&var));

        // a. see if we are explaining why one variation not taken
        if let Some(ref why_not) = self.why_not {
            if why_not.starts_with(var) // if weve pruned part of the why_not
                && match e {
                    Event::FutilityD0
                    | Event::FutilityD1
                    | Event::FutilityD2
                    | Event::FutilityD3 => true,
                    Event::NmpSuccess => true,
                    Event::NodeInteriorCut => true,
                    _ => true,
                }
            {
                if self.tree.is_none() {
                    let tree = SearchTree::new(self.board.clone());
                    self.tree = Some(tree);
                }
                return Some(self.tree.as_mut().unwrap().get_or_insert(var));
            }

            // if var is same as why not and indicator = reject
            // or var is same as why_not except for last move and its a "beat"
        }

        // OR
        // b. see if we are explaining the whole tree
        if self.why_not.is_none()
            && 
            // current variation is an extension of one of the tracked variations
            // OR 
            // current variation is the start of a tracked variation
            (self
                .vars
                .iter()
                // .inspect(|x| println!("about to check var: {}", x))
                .any(|v| {
                    var.starts_with(v) && var.len() <= v.len() + self.max_additional_ply as usize
                })
    || 
                self
                .vars
                .iter()
                // .inspect(|x| println!("about to check var: {}", x))
                .any(|v| { v.starts_with(var) }))
        {
            if self.tree.is_none() {
                let tree = SearchTree::new(self.board.clone());
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
    pub fn explain_futility(
        &mut self,
        b: &Board,
        mv: Move,
        _move_type: MoveType,
        estimated: Score,
        n: &Node,
        e: Event,
    ) {
        if self.explainer.explainer_enabled {
            if let Some(w) = self
                .explainer
                .explaining(n, &self.current_variation.append(mv), e)
            {
                info!("Explain Futility {mv} in {n} on {}", self.current_variation);
                w.score = b.white_score(estimated);
                w.node = *n;
                w.event = e;
                w.cause = e;
            }
        }
    }

    #[inline]
    pub fn explain_move(
        &mut self,
        b: &Board,
        mv: Move,
        child_score: Score,
        e: Event,
        n: &Node,
        count: u32,
        ext: i32,
        red: i32,
    ) {
        if self.explainer.explainer_enabled {
            if let Some(w) = self
                .explainer
                .explaining(n, &self.current_variation.append(mv), e)
            {
                // let bound = match child_score {
                //     d if d >= n.beta => Event::NodeInteriorCut,
                //     d if d > n.alpha => Event::NodeInteriorPv,
                //     _ => Event::NodeInteriorAll,
                // };
                info!("Explain move {mv} in {n} on {}", self.current_variation);
                w.score = b.white_score(child_score);
                w.node = *n;
                w.event = e;
                w.count = count;
                w.ext = ext;
                w.red = red;
            }
        }
    }

    #[inline]
    pub fn explain_nmp(&mut self, b: &Board, child_score: Score, n: &Node) {
        if self.explainer.explainer_enabled {
            let e = Event::NmpSuccess;
            if let Some(w) = self.explainer.explaining(n, &self.current_variation, e) {
                info!(
                    "Explain null move prune in {n} on {}",
                    self.current_variation
                );
                w.score = b.white_score(child_score);
                w.node = *n;
                w.event = e;
                w.cause = Event::Unknown;
            }
        }
    }

    #[inline]
    pub fn explain_node(
        &mut self,
        b: &Board,
        bm: Move,
        nt: NodeType,
        score: Score,
        eval: Score,
        n: &Node,
        e: Event,
        pv: &Variation,
    ) {
        if self.explainer.explainer_enabled {
            if let Some(w) = self.explainer.explaining(n, &self.current_variation, e) {
                info!(
                    "Explain node in {n} with {nt} on {} with pv {pv}",
                    self.current_variation
                );
                w.score = b.white_score(score);
                w.node = *n;
                w.eval = b.white_score(eval);
                w.nt = nt;
            }
            if nt == NodeType::ExactPv {
                if let Some(w) = self
                    .explainer
                    .explaining(n, &self.current_variation.append(bm), e)
                {
                    w.is_best_move = true;
                    w.nt = nt;
                    debug!(
                        "{}: {} setting best move to {}",
                        self.explainer.iter, &self.current_variation, bm
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::search::{engine::ThreadedSearch, timecontrol::TimeControl};
    use crate::test_log::test;
    use crate::Position;

    #[test]
    fn test_search_explainer_tree() {
        let mut eng = ThreadedSearch::new();
        eng.algo.explainer.explainer_enabled = true;
        eng.algo.explainer.max_additional_ply = 6;
        assert_eq!(eng.algo.explainer.vars.len(), 1);

        eng.algo.set_timing_method(TimeControl::Depth(8));
        let pos = Position::parse_epd("r5k1/3bB1bp/1p1p2p1/3P4/4R3/qNp3PP/r4PB1/3QN1K1 w - - 1 31")
            .unwrap();
        // let pos = Position::parse_epd("r1b1k2r/1p3p1p/p2p4/6B1/1q1np3/2Q5/PPP1BPPP/1R2K2R w Kkq - 1 15").unwrap();
        // let var = pos.board().parse_san_variation("").unwrap();
        // eng.algo.explainer.add_variation_to_explain(var);

        let v1 = pos.board().parse_san_variation("h4 Rb2").unwrap();
        eng.set_position(pos);
        eng.algo.explainer.vars.clear();
        eng.algo.explainer.add_variation_to_explain(v1.clone());
        eng.algo.run_search();
        println!("{}", eng.algo.results_as_position());
    }
}

// #[test]
// fn test_search_explainer_whynot() {
//     let mut eng = Engine::new();
//     let pos = Position::parse_epd("r5k1/3bB1bp/1p1p2p1/3P4/4R3/qNp3PP/r4PB1/3QN1K1 w - - 1 31")
//         .unwrap();
//     let var = pos
//         .board()
//         // r##"Bf1 Rb2
//         // Qf3 Rxb3
//         // g4 c2
//         // Nxc2 Rxf3
//         // Nxa3 Rfxa3
//         // Bxd6
//         // "##,
//         .parse_san_variation(
//             // r##"Bf1 Rb2
//             // Qf3 Rxb3
//             // g4 c2
//             // Nxc2 Rxf3
//             // "##,
//             r##"Rc4"##,
//             //  Rb2
//             // Rc7 Qxb3
//             // Qxb3 Rxb3
//             // "##,
//         )
//         .unwrap();
//     eng.set_position(pos);
//     eng.algo.explainer.why_not = Some(var);
//     eng.algo.explainer.enabled = true;
//     eng.algo.tt.enabled = false;
//     eng.algo.set_callback(Uci::uci_info);
//     eng.algo.set_timing_method(TimeControl::Depth(9));
//     eng.search();
//     println!("{}", eng.algo.results_as_position());
