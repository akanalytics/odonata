use crate::board::Board;
use crate::cache::tt2::TranspositionTable2;
use crate::infra::parsed_config::{Component};
use crate::eval::eval::SimpleScorer;
use crate::eval::score::Score;
use crate::globals::counts;
use crate::mv::Move;
use crate::position::Position;
use crate::pvtable::PvTable;
use crate::repetition::Repetition;
use crate::search::extensions::Extensions;
use crate::search::node::Node;
use crate::search::lmr::Lmr;
use crate::search::futility::Futility;
use crate::search::iterative_deepening::IterativeDeepening;
use crate::search::killers::Killers;
use crate::search::move_orderer::MoveOrderer;
use crate::search::move_time_estimator::MoveTimeEstimator;
use crate::search::nmp::NullMovePruning;
use crate::search::pvs::Pvs;
use crate::search::qsearch::QSearch;
use crate::search::search_results::SearchResults;
use crate::search::searchstats::SearchStats;
use crate::search::restrictions::Restrictions;
use crate::search::razor::Razor;
use crate::eval::recognizer::Recognizer;
use crate::search::taskcontrol::TaskControl;
use crate::search::timecontrol::TimeControl;
use crate::search::history_heuristic::HistoryHeuristic;
use crate::types::Ply;
use crate::variation::Variation;
use std::fmt;
use serde::{Deserialize, Serialize};


use super::search_explainer::SearchExplainer;
use super::search_results::SearchResultsMode;


#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Algo {
    pub minmax: bool,
    pub show_refutations: bool, 
    pub analyse_mode: bool, // tries to find full PV etc

    pub ids: IterativeDeepening,
    pub eval: SimpleScorer,
    pub qsearch: QSearch,
    pub nmp: NullMovePruning,
    pub futility: Futility,

    pub pvs: Pvs,
    pub extensions: Extensions,
    pub lmr: Lmr,
    pub mte: MoveTimeEstimator,
    pub move_orderer: MoveOrderer,

    pub repetition: Repetition,
    pub tt: TranspositionTable2,
    pub killers: Killers,
    pub history: HistoryHeuristic,
    pub explainer: SearchExplainer,

    pub restrictions: Restrictions,
    pub razor: Razor,
    pub recognizer: Recognizer,


    #[serde(skip)]
    pub results: SearchResults,
    #[serde(skip)]
    pub position: Position,
    #[serde(skip)]
    pub board: Board,
    #[serde(skip)]
    pub max_depth: Ply,

    #[serde(skip)]
    pub task_control: TaskControl<SearchResults>,
    #[serde(skip)]
    pub stats: SearchStats,
    #[serde(skip)]
    pub pv_table: PvTable,
    #[serde(skip)]
    pub current_best: Option<Move>,
    //pub score: Score,

    // child_thread: AlgoThreadHandle,

    #[serde(skip)]
    clock_checks: u64,
    #[serde(skip)]
    pub current_variation: Variation,
}

/// builder methods
impl Algo {
    pub fn new() -> Algo {
        Algo::default()
    }

    pub fn set_eval(&mut self, eval: SimpleScorer) -> &mut Self {
        self.eval = eval;
        self
    }

    pub fn set_timing_method(&mut self, tm: TimeControl) -> &mut Self {
        self.mte.time_control = tm;
        self
    }

    pub fn set_callback(&mut self, callback: impl Fn(&SearchResults) + Send + Sync + 'static) -> &mut Self {
        self.task_control.register_callback(callback);
        self
    }

    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

impl Component for Algo {

    // clears evaluation and transposition caches as well as repetition counts
    fn new_game(&mut self) {
        self.stats = SearchStats::new();
        self.clock_checks = 0;
        self.pv_table = PvTable::default();
        self.results = SearchResults::default();
        self.current_best = None;
        self.current_variation = Variation::new();
        self.task_control = TaskControl::default();
        self.max_depth = 0;


        self.ids.new_game();
        self.eval.new_game();
        self.qsearch.new_game();
        self.nmp.new_game();
        self.futility.new_game();

        self.pvs.new_game();
        self.extensions.new_game();
        self.lmr.new_game();
        self.mte.new_game();
        self.move_orderer.new_game();

        self.repetition.new_game();
        self.tt.new_game();
        self.killers.new_game();
        self.history.new_game();
        self.explainer.new_game();

        self.restrictions.new_game();
        self.razor.new_game();
        self.recognizer.new_game();
    }

    fn new_position(&mut self) {
        self.task_control = TaskControl::default();
        self.task_control.set_running();
        self.stats = SearchStats::new();
        self.clock_checks = 0;
        self.pv_table = PvTable::default();
        self.results = SearchResults::default();
        self.current_best = None;
        self.current_variation = Variation::new();
        self.max_depth = 0;

        self.ids.new_position();
        self.eval.new_position();
        self.qsearch.new_position();
        self.nmp.new_position();
        self.futility.new_position();

        self.pvs.new_position();
        self.extensions.new_position();
        self.lmr.new_position();
        self.mte.new_position();
        self.move_orderer.new_position();

        self.repetition.new_position();
        self.tt.new_position();
        self.killers.new_position();
        self.history.new_position();
        self.explainer.new_position();

        self.restrictions.new_position();
        self.razor.new_position();
        self.recognizer.new_position();
    }
}

impl fmt::Debug for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Algo")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("board", &self.board)
            .field("current_best", &self.current_best)
            .field("analyse_mode", &self.analyse_mode)
            //.field("pv", &self.pv)
            .field("depth", &self.max_depth)
            .field("minmax", &self.minmax)
            .field("search_stats", &self.stats)
            .field("depth", &self.max_depth)

            .field("ids", &self.ids)
            .field("eval", &self.eval)
            .field("qsearch", &self.qsearch)
            .field("nmp", &self.nmp)
            .field("futility", &self.futility)

            .field("pvs", &self.pvs)
            .field("extensions", &self.extensions)
            .field("lmr", &self.lmr)
            .field("mte", &self.mte)
            .field("move_orderer", &self.move_orderer)

            .field("repetition", &self.repetition)
            .field("tt", &self.tt)
            .field("killers", &self.killers)
            .field("history", &self.history)
            .field("explainer", &self.explainer)

            .field("restrictions", &self.restrictions)
            .field("razor", &self.razor)
            .field("recognizer", &self.recognizer)
            .finish()
    }
}

impl fmt::Display for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "search position  : {}", self.position)?;
        writeln!(f, "starting board   : {}", self.board.to_fen())?;
        writeln!(f, "material         : {}", self.board.material())?;
        writeln!(f, "phase            : {} %", self.board.phase(&self.eval.phaser))?;
        writeln!(f, "static eval      : {}", self.board.eval(&self.eval, &Node::root(0)))?;
        writeln!(f, "bm               : {}", self.results.bm())?;
        writeln!(f, "score            : {}", self.score())?;
        writeln!(f, "analyse mode     : {}", self.analyse_mode)?;
        writeln!(f, "depth            : {}", self.max_depth)?;
        writeln!(
            f,
            "current_best     : {}",
            self.current_best.unwrap_or(Move::new_null())
        )?;
        writeln!(f, "results          : {}", self.results_as_position())?;
        writeln!(f, "minmax           : {}", self.minmax)?;
        writeln!(f, "clock_checks     : {}", self.clock_checks)?;
        write!(f, "\n[task control]\n{}", self.task_control)?;
        write!(f, "\n[move orderer]\n{}", self.move_orderer)?;
        write!(f, "\n[move time estimator]\n{}", self.mte)?;
        write!(f, "\n[nmp]\n{}", self.nmp)?;
        write!(f, "\n[futility]\n{}", self.futility)?;
        write!(f, "\n[pvs]\n{}", self.pvs)?;
        write!(f, "\n[extensions]\n{}", self.extensions)?;
        write!(f, "\n[reductions]\n{}", self.lmr)?;
        write!(f, "\n[qsearch]\n{}", self.qsearch)?;
        write!(f, "\n[eval]\n{}", self.eval)?;
        write!(f, "\n[repetition]\n{}", self.repetition)?;
        write!(f, "\n[tt]\n{}", self.tt)?;
        writeln!(f, "tt nodes")?;
        self.tt.fmt_nodes(f, &self.board)?;
        write!(f, "\n[killers]\n{}", self.killers)?;
        write!(f, "\n[history]\n{}", self.history)?;
        write!(f, "\n[stats]\n{}", self.stats)?;
        write!(f, "\n[iterative deepening]\n{}", self.ids)?;
        write!(f, "\n[global counts]\n{}", counts::GLOBAL_COUNTS)?;
        write!(f, "\n[pvtable]\n{}", self.pv_table)?;
        write!(f, "\n[explainer]\n{}", self.explainer)?;
        write!(f, "\n[restrictions]\n{}", self.restrictions)?;
        write!(f, "\n[razor]\n{}", self.razor)?;
        write!(f, "\n[recognizer]\n{}", self.recognizer)?;
        Ok(())
    }
}

//#[derive(Debug, Default)]
// struct AlgoThreadHandle(Option<thread::JoinHandle<Algo>>);

// impl Clone for AlgoThreadHandle {
//     fn clone(&self) -> Self {
//         Self(None)
//     }
// }

impl Algo {
    pub fn var(&self) -> &Variation {
        &self.current_variation
    }

    pub fn report_progress(&self) {
        if self.stats.iteration().all_nodes() % 5_000_000 == 0 && self.stats.iteration().all_nodes() != 0 {
            let sp = SearchResults::with_report_progress(&self);
            self.task_control.invoke_callback(&sp);
        }
    }

    pub fn report_refutation(&self, ply: Ply) {
        if self.show_refutations && ply < 4 {
            let sp = SearchResults { 
                    pv: Some(self.pv_table.extract_pv_for(ply).clone()),   
                    mode: SearchResultsMode::Refutation,
                    .. SearchResults::default()
            };

            self.task_control.invoke_callback(&sp);
        }
    }

    pub fn set_position(&mut self, pos: Position) -> &mut Self {
        self.new_position();
        self.repetition.push_position(&pos);
        self.board = pos.supplied_variation().apply_to(pos.board());
        self.position = pos;
        self
    }

    pub fn search(&mut self) {
        self.search_iteratively();
        debug!("\n\n\n=====Search completed=====\n{}", self);
    }

    // pub fn search_async(&mut self, board: &Board) {
    //     self.task_control.set_running();
    //     const FOUR_MB: usize = 4 * 1024 * 1024;
    //     let name = String::from("search");
    //     let builder = thread::Builder::new().name(name).stack_size(FOUR_MB);
    //     self.board = board.clone();
    //     let mut algo = self.clone();
    //     // destroy/release this threads copy of the tt.
    //     // self.tt.destroy();
    //     self.child_thread = AlgoThreadHandle(Some(
    //         builder
    //             .spawn(move || {
    //                 algo.search_iteratively();
    //                 algo
    //             })
    //             .unwrap(),
    //     ));
    // }

    #[inline]
    pub fn search_stats(&self) -> &SearchStats {
        &self.stats
    }

    pub fn results_as_position(&self) -> Position {
        self.results.to_pos()
    }

    // pub fn bm(&self) -> Move {
    //     if self.pv().len() > 0 {
    //         self.pv()[0]
    //     } else {
    //         Move::NULL_MOVE
    //     }
    // }


    pub fn score(&self) -> Score {
        self.search_stats().score
    }

    pub fn pv(&self) -> &Variation {
        self.search_stats().pv()
    }

    pub fn ponder_hit(&mut self) {
        self.mte.set_shared_ponder(false);
        self.stats.restart_clocks();
    }

    // pub fn search_async_stop(&mut self) -> bool {
    //     self.task_control.cancel();
    //     self.search_stats.user_cancelled = true;
    //     let handle = self.child_thread.0.take();
    //     if let Some(handle) = handle {
    //         // wait for thread to cancel
    //         let algo = handle.join().unwrap();
    //         *self = algo;
    //         return false;
    //     } else {
    //         return true;
    //         // self.tt = algo.tt.clone();
    //         // self.search_stats = algo.search_stats;
    //         // self.pv_table = algo.pv_table;
    //     }
    // }

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
            self.stats.completed = false;
            self.task_control.cancel();
        }
        time_up
    }

    // pub fn node_all(&mut self, b: &Board, n: &Node, mv: &Move, score: Score) -> Score {
    //     // self.tt.store(b.hash(), TtNode{ score, draft: n.depth, node_type: NodeType::All, bm: Move::NULL_MOVE} );
    //     score
    // }

    // pub fn node_cut(&mut self, b: &Board, n: &Node, mv: &Move, s: Score) -> Score {
    //     // self.search_stats.inc_cuts(n.ply);
    //     // self.killers.store(n.ply, &mv);
    //     s
    // }

    // pub fn node_exact(&mut self, b: &Board, n: &Node, mv: &Move, s: Score)  -> Score {
    //     self.record_new_pv(b, n.ply, mv, false);
    //     s
    // }

    // pub fn node_leaf(&mut self, b: &Board, n: &Node, mv: &Move, s: Score) -> Score {
    //     self.record_new_pv(b, n.ply, mv, true);
    //     // self.search_stats.inc_leaf_nodes(n.ply);
    //     s
    // }
    pub fn clear_move(&mut self, ply: Ply) {
        self.pv_table.set(ply, &Move::NULL_MOVE, true);
    }

    pub fn record_move(&mut self, ply: Ply, mv: &Move) {
        self.pv_table.set(ply, &mv, false);
        self.pv_table.propagate_from(ply + 1);
    }

    pub fn record_truncated_move(&mut self, ply: Ply, mv: &Move) {
        self.pv_table.set(ply, &mv, true);
    }

    // pub fn record_new_pv(&mut self, board: &Board, ply: Ply, mv: &Move, terminal_move: bool) {
    //     debug_assert!(mv.is_null() || board.is_pseudo_legal_move(mv) && board.is_legal_move(&mv), "{} on {}\n{:?}", mv, board, mv);
    //     self.pv_table.set(ply + 1, mv, terminal_move);
    //     // debug_assert!(board.is_legal_variation(&self.pv_table.extract_pv_for(ply+1)), "mv {} on {}\n{:?}\nvar: {}", mv, board, mv,self.pv_table.extract_pv_for(ply+1) );
    //     self.search_stats.inc_improvements(ply);
    //     if ply == 0 {
    //         let sp = SearchProgress::from_stats(&self.search_stats(), self.board.color_us());
    //         self.task_control.invoke_callback(&sp);
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::*;
    use crate::comms::uci::Uci;
    use crate::eval::eval::*;
    use crate::types::*;
    use crate::utils::Formatting;
    use toml;
    use anyhow::*;
    use test_env_log::test;

    #[test]
    fn serde_algo_test() -> Result<()> {
        info!("toml\n{}", toml::to_string(&Algo::default())?);
        Ok(())
    }

    #[test]
    fn test_minmax() {
        let pos = Catalog::starting_position();
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new();
        search.set_position(pos);
        search.qsearch.enabled = false;
        search.futility.alpha_enabled = false;
        search.futility.beta_enabled = false;
        search.nmp.enabled = false;
        search.tt.enabled = false;
        search.minmax = true;
        search.set_eval(eval);
        search.set_timing_method(TimeControl::Depth(3));
        search.search();
        println!("{}", search);
        assert_eq!(
            search.search_stats().iteration().regular_nodes(),
            1 + 20 + 400 + 8902 /* + 197_281 */
        );
        assert_eq!(search.search_stats().branching_factor().round() as u64, 22);
    }

    #[test]
    fn test_node() {
        let mut eval = SimpleScorer::new().set_position(false);
        eval.mobility = false;
        let mut search = Algo::new()
            .set_eval(eval)
            .build();
        search.move_orderer.enabled = false;
        search.set_position(Catalog::starting_position());
        search.set_timing_method(TimeControl::Depth(4));
        search.search();
        println!("{}", search);
        assert_eq!(search.search_stats().iteration().all_nodes(), 1941); // null move pruning
                                                                // assert_eq!(search.search_stats().total().nodes(), 1468);
                                                                // assert_eq!(search.search_stats().total().nodes(), 1516); // rejigged pawn PST
                                                                // previous
                                                                // assert_eq!(search.search_stats().total().nodes(), 1326); // piece mob (disabled)
                                                                // assert_eq!(search.search_stats().total().nodes(), 1404); // pawn promo
                                                                // assert_eq!(search.search_stats().total().nodes(), 1480); // gen qsearch
                                                                // assert_eq!(search.search_stats().total().nodes(), 1642); added tt
                                                                // assert_eq!(search.search_stats().total().nodes(), 1833); qsearch sq
                                                                // assert_eq!(search.search_stats().total().nodes(), 1757);
        assert_eq!(
            Formatting::format_decimal(2, search.search_stats().branching_factor()), "12.80"
        );
    }

    #[test]
    fn test_display_algo() {
        let algo = Algo::new().set_timing_method(TimeControl::Depth(1)).build();
        println!("{}", algo);
        println!("{:?}", algo);
        println!("{:#?}", algo);
    }

    #[test]
    fn test_black_opening() {
        let mut board = Catalog::starting_board();
        board.set_turn(Color::Black);
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(1)).build();
        search.move_orderer.enabled = false;
        search.set_position(Position::from_board(board));
        search.search();
        println!("{}", search);
        assert_eq!(search.pv()[0].uci(), "g8f6");
    }


    // #[test]
    // fn test_async_sleep() {
    //     let position = Catalog::mate_in_2()[0].clone();
    //     let mut algo = Algo::new().set_timing_method(TimeControl::Depth(3)).build();
    //     algo.search_async(position.board());
    //     let millis = time::Duration::from_millis(5500);
    //     thread::sleep(millis);

    //     algo.search_async_stop();
    //     println!("{}\n\nasync....", algo);
    //     let nodes = algo.search_stats().total().nodes();

    //     // with gen qsearch
    //     assert!(nodes < 6000); // piece mob

    //     // previous
    //     // assert_eq!(nodes, 4586); // pawn promo
    //     // assert_eq!(nodes, 5096);  // gen qsearch
    //     // assert_eq!(nodes, 5197);  // wrong halfmove counts in mate score
    //     // assert_eq!(nodes, 2274); // with sq based qsearch
    //     // assert_eq!(nodes, 2274); // from 2248 (due to iterator ordering on bits)
    //     // assert_eq!(nodes, 66234);
    //     assert_eq!(algo.pv_table.extract_pv().uci(), position.pv().unwrap().uci());
    //     assert_eq!(algo.score(), Score::white_win(3));

    //     // search again using the tt
    //     algo.search_async(position.board());
    //     let millis = time::Duration::from_millis(150);
    //     thread::sleep(millis);
    //     algo.search_async_stop();
    //     println!("{}\n\nasync #2....", algo);
    // }

    // #[test]
    // fn test_mate_in_2_async_stopped() {
    //     let position = Catalog::mate_in_2()[0].clone();
    //     let mut algo2 = Algo::new()
    //         .set_timing_method(TimeControl::Depth(3))
    //         .build();
    //     algo2.minmax = true;
    //     let closure = |sp: &SearchProgress| println!("nps {}", sp.time_millis.unwrap_or_default());
    //     algo2.set_callback(closure);
    //     algo2.search_async(position.board());
    //     let millis = time::Duration::from_millis(200);
    //     thread::sleep(millis);
    //     algo2.search_async_stop();
    //     println!("{}", algo2);
    //     // println!("after stop clock:\n{}", algo.clock);
    //     let nodes = algo2.search_stats().total().nodes();
    //     assert!(nodes > 10 && nodes < 66234);
    // }

    #[test]
    #[ignore]
    fn jons_chess_problem() {
        let pos = Position::parse_epd("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1").unwrap();
        println!("{}", pos);
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new()
            .set_timing_method(TimeControl::Depth(9))
            .set_eval(eval)
            .build(); //9
        search.set_position(pos);
        search.search();
        println!("{}", search);
    }


    #[test]
    fn bug05() {
        let pos = Position::parse_epd("8/8/3N4/4B3/6p1/5k1p/4n2P/7K b - - 75 93 ").unwrap();
        let mut search = Algo::new()
            .set_timing_method(TimeControl::Depth(8))
            .set_callback(Uci::uci_info)
            .build();
        search.set_position(pos);
        search.search();
        println!("{}", search);
    }

    #[test]
    fn bug06() -> Result<()> {
        // 11.Qd3       b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11   acd 4; bm d1d3; ce 60; pv "d1d3 c6b4 d3d1";
        // 11... Nb4    b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2NQ4/PPP1BPPP/B3R1KR b - - 2 11   acd 4; bm c6b4; ce 30; pv "c6b4 d3d1 b4c6";
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(3)).build();
        let pos06 = Position::parse_epd("b1q1r1kr/ppppbppp/2nnp3/4N3/3P4/2N1P3/PPP2PPP/BQ2RBKR w - - 2 6")?;
        let pos07 = Position::parse_epd("b2qr1kr/ppppbppp/2nnp3/4N3/3P4/2NBP3/PPP2PPP/BQ2R1KR w - - 4 7")?;
        let pos08 = Position::parse_epd("b2qr1kr/pppp1ppp/2nnpb2/4N3/3P4/2NBP3/PPP2PPP/B2QR1KR w - - 6 8")?;
        let pos09 = Position::parse_epd("b2qr1kr/ppppbppp/2nnp3/8/3P2N1/2NBP3/PPP2PPP/B2QR1KR w - - 8 9")?;
        let pos10 =
            Position::parse_epd("b2qr1kr/pppp1ppp/2nnp3/6b1/3P2N1/2N1P3/PPP1BPPP/B2QR1KR w - - 10 10")?;
        let pos11 = Position::parse_epd("b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11")?;
        let pos12 = Position::parse_epd("b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12")?;
        search.set_position(pos06).search();
        search.set_position(pos07).search();
        search.set_position(pos08).search();
        search.set_position(pos09).search();
        search.set_position(pos10).search();
        search.set_position(pos11).search();
        search.set_position(pos12).search();
        println!("{}", search);
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_truncated_pv() {
        let mut algo = Algo::new()
            //             .set_timing_method(TimeControl::from_move_time_millis(1000))
            .set_timing_method(TimeControl::Depth(7))
            .build();
        // algo.repetition.avoid_tt_on_repeats = false;
        // algo.tt.min_ply = 2;
        let positions = Catalog::win_at_chess();
        for p in positions {
            algo.new_game();
            algo.tt.allow_truncated_pv = true;
            algo.set_position(p.clone()).search();
            let pv1 = algo.results_as_position().pv().unwrap();
            algo.tt.current_age -= 1;
            println!("{:<40} - {}", pv1.uci(), algo.results_as_position());

            algo.tt.allow_truncated_pv = true;
            algo.set_position(p.clone()).search();
            let pv2 = algo.results_as_position().pv().unwrap();
            println!("{:<40} - {}", pv2.uci(), algo.results_as_position());

            algo.tt.allow_truncated_pv = false;
            algo.set_position(p.clone()).search();
            let pv3 = algo.results_as_position().pv().unwrap();
            println!("{:<40} - {}\n", pv3.uci(), algo.results_as_position());

            //assert_eq!(pv1, pv2, "{}", p );
        }
    }
}
