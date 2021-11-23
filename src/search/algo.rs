use crate::board::Board;
use crate::cache::tt2::TranspositionTable2;
use crate::clock::Clock;
use crate::eval::eval::SimpleScorer;
use crate::eval::recognizer::Recognizer;
use crate::eval::score::Score;
use crate::globals::counts;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::position::Position;
use crate::pvtable::PvTable;
use crate::repetition::Repetition;
use crate::search::aspiration::Aspiration;
use crate::search::extensions::Extensions;
use crate::search::futility::Futility;
use crate::search::history_heuristic::HistoryHeuristic;
use crate::search::iterative_deepening::IterativeDeepening;
use crate::search::killers::Killers;
use crate::search::lmr::Lmr;
use crate::search::move_orderer::MoveOrderer;
use crate::search::move_time_estimator::MoveTimeEstimator;
use crate::search::nmp::NullMovePruning;
use crate::search::node::Node;
use crate::search::pvs::Pvs;
use crate::search::qsearch::QSearch;
use crate::search::razor::Razor;
use crate::search::restrictions::Restrictions;
use crate::search::search_results::SearchResults;
use crate::search::searchstats::SearchStats;
use crate::search::taskcontrol::TaskControl;
use crate::search::timecontrol::TimeControl;
use crate::trace::counts::Counts;
use crate::types::Ply;
use crate::variation::Variation;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::Event;
use super::search_explainer::SearchExplainer;
use super::search_results::SearchResultsMode;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
    pub ext: Extensions,
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
    pub aspiration: Aspiration,
    pub clock: Clock,

    pub counts: Counts,

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
    fn new_iter(&mut self) {
        self.counts.new_iter();
        self.restrictions.new_iter();
        self.clock.new_iter();
    }

    fn new_game(&mut self) {
        self.stats = SearchStats::new();
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
        self.ext.new_game();
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
        self.aspiration.new_game();
        self.clock.new_game();

        self.counts.new_game();
    }

    fn new_position(&mut self) {
        self.task_control = TaskControl::default();
        self.task_control.set_running();
        self.stats = SearchStats::new();
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
        self.ext.new_position();
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
        self.aspiration.new_position();
        self.clock.new_position();

        self.counts.new_position();
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
            .field("extensions", &self.ext)
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
            .field("aspiration", &self.aspiration)
            .field("clock", &self.clock)
            .field("counts", &self.counts)
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
        writeln!(f, "current_best     : {}", self.current_best.unwrap_or(Move::new_null()))?;
        writeln!(f, "results          : {}", self.results_as_position())?;
        writeln!(f, "minmax           : {}", self.minmax)?;
        writeln!(f, "clock_checks     : {}", self.clock_checks)?;
        write!(f, "\n[task control]\n{}", self.task_control)?;
        write!(f, "\n[move orderer]\n{}", self.move_orderer)?;
        write!(f, "\n[move time estimator]\n{}", self.mte)?;
        write!(f, "\n[nmp]\n{}", self.nmp)?;
        write!(f, "\n[futility]\n{}", self.futility)?;
        write!(f, "\n[pvs]\n{}", self.pvs)?;
        write!(f, "\n[extensions]\n{}", self.ext)?;
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
        write!(f, "\n[aspiration]\n{:}", self.aspiration)?;
        write!(f, "\n[clock]\n{:}", self.clock)?;

        write!(f, "\n[counts]\n{}", self.counts)?;
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
                ..SearchResults::default()
            };

            self.task_control.invoke_callback(&sp);
        }
    }

    pub fn set_position(&mut self, pos: Position) -> &mut Self {
        self.new_position();
        self.repetition.push_position(&pos);
        self.board = pos.supplied_variation().apply_to(pos.board());
        self.tt.rewrite_pv(pos.board());
        self.position = pos;
        self
    }

    pub fn search(&mut self) {
        self.search_iteratively();
        debug!("\n\n\n=====Search completed=====\n{}", self);
    }

    #[inline]
    pub fn search_stats(&self) -> &SearchStats {
        &self.stats
    }

    pub fn results_as_position(&self) -> Position {
        self.results.to_pos()
    }

    pub fn score(&self) -> Score {
        self.search_stats().score()
    }

    pub fn pv(&self) -> &Variation {
        self.search_stats().pv()
    }

    pub fn ponder_hit(&mut self) {
        self.mte.set_shared_ponder(false);
        self.stats.restart_clocks();
    }

    #[inline]
    pub fn time_up_or_cancelled(&mut self, ply: Ply, force_check: bool) -> bool {
        // never cancel on ply=1, this way we always have a best move, and we detect mates
        if self.max_depth == 1 {
            return false;
        }

        if self.task_control.is_cancelled() {
            return true;
        }


        let time_up = self.mte.is_time_up(ply, &self.clock, force_check);
        if time_up {
            self.stats.completed = false;
            self.stats.set_score(-Score::INFINITY, Event::Cancelled);
            self.task_control.cancel();
        }
        time_up
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::*;
    use crate::comms::uci::Uci;
    use crate::eval::eval::*;
    use crate::types::*;
    use anyhow::*;
    use test_env_log::test;
    use toml;

    #[test]
    fn serde_algo_test() -> Result<()> {
        info!("toml\n{}", toml::to_string(&Algo::default())?);
        Ok(())
    }

    #[test]
    fn test_minmax() {
        let pos = Catalog::starting_position();
        let eval = SimpleScorer::new().set_position(false);
        let mut algo = Algo::new();
        algo.set_position(pos);
        algo.qsearch.enabled = false;
        algo.ids.enabled = false;
        algo.futility.alpha_enabled = false;
        algo.futility.beta_enabled = false;
        algo.nmp.enabled = false;
        algo.tt.enabled = false;
        algo.pvs.enabled = false;
        algo.lmr.enabled = false;
        algo.ext.enabled = false;
        algo.minmax = true;
        algo.set_eval(eval);
        algo.set_timing_method(TimeControl::Depth(3));
        algo.search();
        println!("{}", algo);
        assert_eq!(algo.clock.cumul_nodes(), algo.clock.cumul_nodes_all_threads());
        assert_eq!(algo.clock.cumul_nodes(), 1 + 20 + 400 + 8902);
        assert_eq!(
            algo.counts.cumul(Event::NodeInterior) + algo.counts.cumul(Event::DerivedLeaf),
            1 + 20 + 400 + 8902 /* + 197_281 */
        );
        assert_eq!(algo.counts.cumul(Event::PercentBranchingFactor), 2114);
    }

    #[test]
    fn test_node() {
        let mut eval = SimpleScorer::new().set_position(false);
        eval.mobility = false;
        let mut algo = Algo::new().set_eval(eval).build();
        algo.move_orderer.enabled = false;
        algo.set_position(Catalog::starting_position());
        algo.set_timing_method(TimeControl::Depth(4));
        algo.search();
        println!("{}", algo.counts);
        assert_eq!(algo.clock.cumul_nodes(), 2385);
        assert_eq!(algo.counts.cumul(Event::DerivedLeaf), 2106);
        assert_eq!(algo.counts.cumul(Event::NodeInterior), 249);
        assert_eq!(algo.counts.cumul(Event::PercentBranchingFactor), 845);
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
        assert_eq!(search.pv()[0].uci(), "d7d5");
    }

    #[test]
    #[ignore]
    fn jons_chess_problem() {
        let pos = Position::parse_epd("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1").unwrap();
        println!("{}", pos);
        let eval = SimpleScorer::new().set_position(false);
        let mut search = Algo::new().set_timing_method(TimeControl::Depth(9)).set_eval(eval).build(); //9
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
        let mut search = Algo::new();
        let pos06 = Position::parse_epd("b1q1r1kr/ppppbppp/2nnp3/4N3/3P4/2N1P3/PPP2PPP/BQ2RBKR w - - 2 6")?;
        let pos07 = Position::parse_epd("b2qr1kr/ppppbppp/2nnp3/4N3/3P4/2NBP3/PPP2PPP/BQ2R1KR w - - 4 7")?;
        let pos08 = Position::parse_epd("b2qr1kr/pppp1ppp/2nnpb2/4N3/3P4/2NBP3/PPP2PPP/B2QR1KR w - - 6 8")?;
        let pos09 = Position::parse_epd("b2qr1kr/ppppbppp/2nnp3/8/3P2N1/2NBP3/PPP2PPP/B2QR1KR w - - 8 9")?;
        let pos10 = Position::parse_epd("b2qr1kr/pppp1ppp/2nnp3/6b1/3P2N1/2N1P3/PPP1BPPP/B2QR1KR w - - 10 10")?;
        let pos11 = Position::parse_epd("b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11")?;
        let pos12 = Position::parse_epd("b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12")?;
        search.set_position(pos06).set_timing_method(TimeControl::Depth(3)).search();
        search.set_position(pos07).set_timing_method(TimeControl::Depth(3)).search();
        search.set_position(pos08).set_timing_method(TimeControl::Depth(3)).search();
        search.set_position(pos09).set_timing_method(TimeControl::Depth(3)).search();
        search.set_position(pos10).set_timing_method(TimeControl::Depth(3)).search();
        search.set_position(pos11).set_timing_method(TimeControl::Depth(3)).search();
        search.set_position(pos12).set_timing_method(TimeControl::Depth(3)).search();
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
