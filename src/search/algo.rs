use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::eval::{Scorable, SimpleScorer};
use crate::eval::score::Score;
use crate::globals::counts;
use crate::log_debug;
use crate::movelist::Move;
use crate::tags::Tag;
use crate::movelist::MoveList;
use crate::pvtable::PvTable;
use crate::position::Position;
use crate::search::move_orderer::MoveOrderer;
use crate::search::quiescence::Quiescence;
use crate::search::searchprogress::SearchProgress;
use crate::search::searchstats::SearchStats;
use crate::search::taskcontrol::TaskControl;
use crate::search::node::Node;
use crate::repetition::Repetition;
use crate::search::timecontrol::TimeControl;
use crate::search::move_time_estimator::MoveTimeEstimator;
use crate::search::iterative_deepening::IterativeDeepening;
use crate::tt::{TranspositionTable, Entry, NodeType};
use crate::types::{MAX_PLY, Ply};
use std::fmt;
use std::ops::Range;
use std::thread;




#[derive(Clone, Default)]
pub struct Algo {
    pub board: Board,
    pub max_depth: Ply,
    pub minmax: bool,
    iterative_deepening: bool,
    pub ids: IterativeDeepening,
    pub eval: SimpleScorer,
    pub task_control: TaskControl<SearchProgress>,
    pub quiescence: Quiescence,
    pub search_stats: SearchStats,

    pub range: Range<Ply>,
    pub pv_table: PvTable,
    pub current_best: Option<Move>,
    //pub score: Score,
    pub mte: MoveTimeEstimator,
    pub move_orderer: MoveOrderer,
    pub repetition: Repetition,
    pub tt: TranspositionTable,

    child_thread: AlgoThreadHandle,

    clock_checks: u64,
    pub current_variation: MoveList,
}

/// builder methods
impl Algo {
    pub fn new() -> Algo {
        let mut algo = Algo::default();
        algo.iterative_deepening = true;
        algo
    }

    pub fn set_qsearch(&mut self, enable: bool) -> &mut Self {
        self.quiescence.enabled = enable;
        self
    }

    pub fn set_iterative_deepening(&mut self, enable: bool) -> &mut Self {
        self.iterative_deepening = enable;
        self
    }

    pub fn set_minmax(&mut self, minmax: bool) -> &mut Self {
        self.minmax = minmax;
        self
    }

    pub fn set_eval(&mut self, eval: SimpleScorer) -> &mut Self {
        self.eval = eval;
        self
    }

    pub fn set_timing_method(&mut self, tm: TimeControl) -> &mut Self {
        self.mte.time_control = tm;
        self
    }

    pub fn set_callback(&mut self, callback: impl Fn(&SearchProgress) + Send + Sync + 'static) -> &mut Self {
        self.task_control.register_callback(callback);
        self
    }

    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

impl Configurable for Algo {
    fn settings(&self, c: &mut Config) {
        c.set("algo.minmax", "type check default false");
        c.set("algo.ids", "type check default true");
        self.eval.settings(c);
        self.mte.settings(c);
        self.move_orderer.settings(c);
        self.quiescence.settings(c);
        self.ids.settings(c);
        self.repetition.settings(c);
        self.tt.settings(c);
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("algo.configure with {}", c);
        self.minmax = c.bool("algo.minmax").unwrap_or(self.minmax);
        self.iterative_deepening = c.bool("algo.ids").unwrap_or(self.iterative_deepening);
        self.eval.configure(c);
        self.move_orderer.configure(c);
        self.mte.configure(c);
        self.quiescence.configure(c);
        self.ids.configure(c);
        self.repetition.configure(c);
        self.tt.configure(c);
    }
}

impl fmt::Debug for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Algo")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("board", &self.board)
            .field("current_best", &self.current_best)
            //.field("pv", &self.pv)
            .field("depth", &self.max_depth)
            .field("minmax", &self.minmax)
            .field("eval", &self.eval)
            .field("iterative_deepening", &self.iterative_deepening)
            .field("move_orderer", &self.move_orderer)
            .field("mte", &self.mte)
            .field("depth", &self.max_depth)
            .field("range", &self.range)
            .field("search_stats", &self.search_stats)
            .field("quiescence", &self.quiescence)
            .field("ids", &self.ids)
            .field("repetition", &self.repetition)
            .field("tt", &self.tt)
            .finish()
    }
}

impl fmt::Display for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "starting pos     : {}", self.board.to_fen())?;
        //writeln!(f, "pv               : {}", self.pv())?;
        writeln!(f, "bm               : {}", self.bm())?;
        writeln!(f, "score            : {}", self.score())?;
        writeln!(f, "depth            : {}", self.max_depth)?;
        writeln!(f, "range            : {:?}", self.range)?;
        writeln!(f, "current_best     : {}", self.current_best.unwrap_or(Move::new_null()))?;
        writeln!(f, "minmax           : {}", self.minmax)?;
        writeln!(f, "iter deepening   : {}", self.iterative_deepening)?;
        writeln!(f, "clock_checks     : {}", self.clock_checks)?;
        write!(f, "\n[task control]\n{}", self.task_control)?;
        write!(f, "\n[move orderer]\n{}", self.move_orderer)?;
        write!(f, "\n[move time estimator]\n{}", self.mte)?;
        write!(f, "\n[quiescence]\n{}", self.quiescence)?;
        write!(f, "\n[eval]\n{}", self.eval)?;
        write!(f, "\n[iterative deepening]\n{}", self.ids)?;
        write!(f, "\n[repetition]\n{}", self.repetition)?;
        write!(f, "\n[tt]\n{}", self.tt)?;
        write!(f, "\n[stats]\n{}", self.search_stats)?;
        write!(f, "\n[global counts]\n{}", counts::GLOBAL_COUNTS)?;
        write!(f, "\n[pvtable]\n{}", self.pv_table)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct AlgoThreadHandle(Option<thread::JoinHandle<Algo>>);

impl Clone for AlgoThreadHandle {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl Algo {
    pub fn search_async(&mut self, board: &Board) {
        const FOUR_MB: usize = 4 * 1024 * 1024;
        let name = String::from("search");
        let builder = thread::Builder::new().name(name).stack_size(FOUR_MB);
        self.task_control.set_running();
        let mut algo = self.clone();
        let board = board.clone();
        self.child_thread = AlgoThreadHandle(
            Some(
                builder.spawn(move || { algo.search(&board); algo}).unwrap()
            ));
    }


    pub fn algo_description(&self) -> String {
        format!(
            "{algo} depth:{depth}",
            algo = if self.minmax { "minmax" } else { "alphabeta" },
            depth = self.max_depth
        )
    }

    #[inline]
    pub fn set_iteration_depth(&mut self, max_depth: Ply) {
        self.max_depth = max_depth;
    }

    #[inline]
    pub fn search_stats(&self) -> &SearchStats {
        &self.search_stats
    }

    pub fn results(&self) -> Position {
        let mut pos = Position::from_board(self.board.clone());
        pos.set(&Tag::BestMove(self.bm()));
        pos.set(&Tag::Pv(self.pv().clone()));
        pos.set(&Tag::CentipawnEvaluation(self.score()));
        pos.set(&Tag::AnalysisCountDepth(self.search_stats().depth()));
        pos
    }

    pub fn bm(&self) -> Move {
        if self.pv().len() > 0 { 
            self.pv()[0]
        } else {
            Move::NULL_MOVE
        }
    }

    pub fn score(&self) -> Score {
        self.search_stats().score
    }

    pub fn pv(&self) -> &MoveList {
        self.search_stats().pv()
    }

    pub fn search_async_stop(&mut self) {
        self.task_control.cancel();
        self.search_stats.user_cancelled = true;
        let handle = self.child_thread.0.take();
        if let Some(handle) = handle {
            // wait for thread to cancel
            let algo = handle.join().unwrap();
            self.search_stats = algo.search_stats;
            self.pv_table = algo.pv_table;
        }
    }

    #[inline]
    pub fn time_up_or_cancelled(&mut self, ply: Ply, force_check: bool) -> bool {
        self.clock_checks += 1;

        // never cancel on ply=1, this way we always have a best move, and we detect mates
        if self.max_depth == 1 {
            return false;
        }

        if self.task_control.is_cancelled() {
            return true;
        }

        // only do this every 128th call to avoid expensive time computation
        if !force_check && self.clock_checks % 128 != 0 {
            return false;
        }

        let time_up = self.mte.is_time_up(ply, self.search_stats());
        if time_up {
            self.search_stats.completed = false;
            self.task_control.cancel();
        }
        time_up
    }

    #[inline]
    pub fn is_leaf(&self, ply: Ply) -> bool {
        ply == self.max_depth
    }


    pub fn alphabeta(&mut self, node: &mut Node) {
        self.search_stats.reset_keeping_pv();
        self.pv_table = PvTable::new(MAX_PLY as usize);
        self.tt.clear();

        if 1==0 {
            self.alphabeta_recursive(node);
            self.search_stats.score = node.score;
        } else {
            self.search_stats.score = self.alphabeta_recursive2(
            node.board,
            node.ply,
            node.alpha,
            node.beta,
            &Move::NULL_MOVE );
        }

        // self.search_stats.alpha = node.alpha;
        // self.search_stats.beta = node.beta;
        self.search_stats.record_time_actual_and_completion_status(self.max_depth, !self.task_control.is_cancelled(), self.pv_table.extract_pv());
    }

    
    pub fn alphabeta_recursive(&mut self, node: &mut Node) {
        debug_assert!(self.max_depth > 0);
        if self.search_stats.total().nodes() % 1000000 == 0 && self.search_stats.total().nodes() != 0 {
            let sp = SearchProgress::from_search_stats(&self.search_stats());
            self.task_control.invoke_callback(&sp);
        }

        if self.time_up_or_cancelled(node.ply, false) {
            return;
        }

        if node.board.repetition_count() >= 2 {
            node.score = node.board.eval(&self.eval);
            self.search_stats.inc_leaf_nodes(node.ply);
            return;
        }

        if self.is_leaf(node.ply) {
            self.quiescence_search(node);
            return;
        }
        self.search_stats.inc_interior_nodes(node.ply);

        // // FIXME!!!!
        // if self.max_depth > self.search_stats.selective_depth() {
        //     let sp = SearchProgress::from_search_stats(&self.search_stats());
        //     self.task_control.invoke_callback(&sp);
        // }
        // bailing here means the score is +/- inf and wont be used
        // FIXME!

        let mut moves = node.board.legal_moves();
        if moves.is_empty() {
            node.score = node.board.eval(&self.eval);
            self.search_stats.inc_leaf_nodes(node.ply);
            return;
        }

        self.order_moves(node.ply, &mut moves);

        let original_score = node.score;
        for (_i, mv) in moves.iter().enumerate() {
            let mut child_board = node.board.make_move(mv);
            self.repetition.push(&mv, &child_board);
            child_board.set_repetition_count(self.repetition.count(&child_board));
            let mut child = node.new_child(mv, &mut child_board);
            debug_assert!(child.alpha < child.beta || self.minmax);
            self.current_variation.set_last_move(child.ply, mv);

            self.alphabeta_recursive(&mut child);

            node.board.undo_move(mv);
            self.repetition.pop();
            let is_cut = self.process_child(&mv, node, &child);
            if is_cut {
                self.search_stats.inc_cuts(node.ply);
                break;
                // let entry = Entry {
                //     hash: node.board.hash(),`
                //     score: node.score,
                //     ply: node.ply,
                //     entry_type: NodeType::LowerBound,
                //     best_move: Move::NULL_MOVE,
                // };
                // self.tt.insert(entry);
                // break;
            }
        }
 
        self.current_variation.set_last_move(node.ply, &Move::new_null());
    }

    #[inline]
    pub fn process_child(&mut self, mv: &Move, parent: &mut Node, child: &Node) -> bool {
        if Node::is_maximizing(parent.board) {
            if child.score > parent.score {
                parent.score = child.score;
            }
            if child.score > parent.alpha {
                parent.alpha = child.score;
                self.pv_table.set(child.ply, mv);
                self.pv_table.propagate_from(child.ply);
                self.search_stats.inc_improvements(parent.ply);
                if parent.is_root() {
                    let sp = SearchProgress::from_search_stats(&self.search_stats());
                    self.task_control.invoke_callback(&sp);
                }
            }
        } else {
            if child.score < parent.score {
                parent.score = child.score;
            }
            if child.score < parent.beta {
                parent.beta = child.score;
                self.pv_table.set(child.ply, mv);
                self.pv_table.propagate_from(child.ply);
                self.search_stats.inc_improvements(parent.ply);
                if parent.is_root() {
                    let sp = SearchProgress::from_search_stats(&self.search_stats());
                    self.task_control.invoke_callback(&sp);
                }
            }
        }
        parent.alpha >= parent.beta && !self.minmax  
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::*;
    use crate::types::*;
    use crate::comms::uci::Uci;
    use crate::eval::eval::*;
    use crate::movelist::MoveValidator;
    use std::time;

    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[test]
    fn test_node() {
        // init();
        let board = Catalog::starting_position();
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(3)).set_minmax(true).set_eval(eval).set_qsearch(false).build();
        search.search(&board);
        assert_eq!(search.search_stats().total().nodes(), 1 + 20 + 400 + 8902 /* + 197_281 */);
        assert_eq!(search.search_stats().branching_factor().round() as u64, 21);

        let board = Catalog::starting_position();
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(4)).set_minmax(false).set_eval(eval).build();
        search.move_orderer.enabled = false;
        search.search(&board);
        println!("{}", search);
        assert_eq!(search.search_stats().total().nodes(), 1833);  // qsearch
        // assert_eq!(search.search_stats().total().nodes(), 1757);
        assert_eq!(search.search_stats().branching_factor().round() as u64, 1);
    }

    #[test]
    fn test_display_algo() {
        let algo = Algo::new().set_timing_method(TimeControl::Depth(1)).set_minmax(false).build();
        println!("{}", algo);
        println!("{:?}", algo);
        println!("{:#?}", algo);
    }

    #[test]
    fn test_black_opening() {
        let mut board = Catalog::starting_position();
        board.set_turn(Color::Black);
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(1)).set_minmax(false).build();
        search.move_orderer.enabled = false;
        search.search(&board);
        println!("{}", search);
        assert_eq!(search.pv()[0].uci(), "d7d5");
    }

    #[test]
    fn test_all_mate_in_2() {
        let positions = Catalog::mate_in_2();
        for pos in positions {
            let mut search = Algo::new().set_timing_method(TimeControl::Depth(3)).set_callback(Uci::uci_info).build();
            search.search(pos.board());
            println!("{}", search);
            assert_eq!(
                pos.board().to_san_moves(&search.pv()),
                pos.board().to_san_moves(&pos.pv().unwrap()),
                "{}",
                pos.id().unwrap()
            );
            assert_eq!(
                search.pv().to_string(),
                pos.pv().unwrap().to_string(),
                "{}",
                pos.id().unwrap()
            );
            // FIXME assert_eq!(search.score.unwrap(), Score::WhiteWin { minus_ply: -3 });
        }
    }

    // fn test_all_mate_in_2_mock() {
    //     let positions = Catalog::mate_in_2();
    //     for pos in positions {
    //         let mut search = Algo::new().set_timing_method(TimeControl::Depth(3)).set_callback(Uci::uci_info);
    //         pos.search(eval);
    //         println!("{}", search);
    //         assert_eq!(search.pv_table.extract_pv().to_string(), pos.pv_table().unwrap().to_string(), "{}", pos.id().unwrap());
    //         // FIXME assert_eq!(search.score.unwrap(), Score::WhiteWin { minus_ply: -3 });
    //     }
    // }

    #[test]
    fn test_mate_in_2_ids() {
        for &id in &[true, false] {
            let position = Catalog::mate_in_2()[0].clone();
            let eval = SimpleScorer::new().set_position(false);
            let mut search = Algo::new()
                .set_timing_method(TimeControl::Depth(3))
                .set_minmax(false)
                .set_eval(eval)
                .set_iterative_deepening(id)
                .set_callback(Uci::uci_info).build();
            search.search(position.board());
            println!("{}", search);
            if id {
                assert_eq!(search.search_stats().total().nodes(), 2200);  // with sq q qsearch
                // assert_eq!(search.search_stats().total().nodes(), 2108);  // with ordering pv + mvvlva
                // assert_eq!(search.search_stats().total().nodes(), 3560); 
                // assert_eq!(search.search_stats().total().nodes(), 6553);  // with ordering pv
                // assert_eq!(search.search_stats().total().nodes(), 6740);
            } else {
                assert_eq!(search.search_stats().total().nodes(), 2200); // with sq qsearch
                // assert_eq!(search.search_stats().total().nodes(), 2108); // with  mvvlva
                //assert_eq!(search.search_stats().total().nodes(), 7749); // no ids no mvvlva
            }
            assert_eq!(search.pv_table.extract_pv(), position.pv().unwrap());
            assert_eq!(search.score(), Score::WhiteWin { minus_ply: -3 });
            println!("{}", search.results());
        }
    }

    #[test]
    fn test_mate_in_2_async() {
        let position = Catalog::mate_in_2()[0].clone();
        let mut algo = Algo::new().set_timing_method(TimeControl::Depth(3)).set_minmax(true).build();
        algo.search(position.board());
        let nodes = algo.search_stats().total().nodes();
        let millis = time::Duration::from_millis(20);
        thread::sleep(millis);

        assert_eq!(nodes, 77221);  // with sq based qsearch
        // assert_eq!(nodes, 66234);
        assert_eq!(algo.pv_table.extract_pv(), position.pv().unwrap());
        assert_eq!(algo.score(), Score::WhiteWin { minus_ply: -3 });
        println!("{}\n\nasync....", algo);
    }

    #[test]
    fn test_mate_in_2_async_stopped() {
        let position = Catalog::mate_in_2()[0].clone();
        let mut algo2 = Algo::new().set_timing_method(TimeControl::Depth(3)).set_minmax(true).build();
        let closure = |sp: &SearchProgress| println!("nps {}", sp.time_millis.unwrap_or_default());
        algo2.set_callback(closure);
        algo2.search_async(position.board());
        let millis = time::Duration::from_millis(200);
        thread::sleep(millis);
        algo2.search_async_stop();
        println!("{}", algo2);
        // println!("after stop clock:\n{}", algo.clock);
        let nodes = algo2.search_stats().total().nodes();
        assert!(nodes > 10 && nodes < 66234);
    }

    #[test]
    #[ignore]
    fn test_mate_in_3_sync() -> Result<(), String> {
        let position = Catalog::mate_in_3()[0].clone();
        let expected_pv = position.pv()?;
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(5)).set_minmax(false).build();
        search.search(position.board());
        let san = position.board().to_san_moves(&search.pv_table.extract_pv()).replace("\n", " ");
        println!("{}", search);
        assert_eq!(san, position.board().to_san_moves(&expected_pv).replace("\n", " "));
        assert_eq!(search.pv_table.extract_pv(), expected_pv);
        assert_eq!(search.score(), Score::WhiteWin { minus_ply: -4 });
        Ok(())
    }

    #[test]
    #[ignore]
    fn jons_chess_problem() {
        init();
        let board = Board::parse_fen("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1")
            .unwrap()
            .as_board();
        println!("{}", board);
        let eval = SimpleScorer::new().set_position(false);
        let mut search =
            Algo::new().set_timing_method(TimeControl::Depth(9)).set_minmax(false).set_eval(eval).build(); //9
        search.search(&board);
        println!("{}", search);
    }

    #[test]
    fn debug_arena_issue() {
        let board =
            Board::parse_fen("r1bqkbnr/pppppppp/2n5/8/3P4/4P3/PPP2PPP/RNBQKBNR b KQkq - 0 2").unwrap();
        let time_control = TimeControl::RemainingTime {
            wtime: time::Duration::from_millis(141516),
            btime: time::Duration::from_millis(127990),
            winc: time::Duration::from_millis(12000),
            binc: time::Duration::from_millis(12000),
            movestogo: 0,
            our_color: Color::Black,
        };
        let mut search = Algo::new()
            .set_timing_method(time_control)
            .set_minmax(false)
            .set_iterative_deepening(true)
            .set_callback(Uci::uci_info).build();
        println!("{}", search);
        println!("{}", board);
        search.search(&board);
        println!("{}", search);
    }
}
