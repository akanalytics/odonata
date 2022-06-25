use crate::board::Board;
use crate::cache::tt2::TranspositionTable2;
use crate::clock::Clock;
use crate::eval::eval::Eval;
use crate::eval::recognizer::Recognizer;
use crate::eval::score::Score;
use crate::infra::component::{Component, State};
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::position::Position;
use crate::other::pvtable::PvTable;
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
use crate::search::razor::Razor;
use crate::search::restrictions::Restrictions;
use crate::search::search_progress::SearchProgress;
use crate::search::taskcontrol::TaskControl;
use crate::search::timecontrol::TimeControl;
use crate::piece::Ply;
use crate::variation::Variation;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::counter_move::CounterMove;
use super::lmp::Lmp;
use super::mate_distance::MateDistance;
use super::node::Event;
use super::qs::Qs;
use super::reverse_futility::ReverseFutility;
use super::search_explainer::Explainer;
use super::search_progress::SearchProgressMode;
use super::search_results::SearchResults;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Algo {
    pub show_refutations: bool,
    pub analyse_mode: bool, // tries to find full PV etc
    pub qsearch_disabled: bool,
    pub debug: bool,
    pub show_metrics_on_exit: bool, 

    pub ids: IterativeDeepening,
    pub eval: Eval,
    pub nmp: NullMovePruning,
    pub futility: Futility,
    pub rev_fut: ReverseFutility,
    pub mate_dist: MateDistance,

    pub pvs: Pvs,
    pub ext: Extensions,
    pub lmr: Lmr,
    pub mte: MoveTimeEstimator,
    pub move_orderer: MoveOrderer,

    pub repetition: Repetition,
    pub tt: TranspositionTable2,
    pub killers: Killers,
    pub history: HistoryHeuristic,
    pub explainer: Explainer,

    pub restrictions: Restrictions,
    pub razor: Razor,
    pub recognizer: Recognizer,
    pub aspiration: Aspiration,
    pub clock: Clock,

    pub progress: SearchProgress,
    pub controller: TaskControl<SearchProgress>,
    pub lmp: Lmp,
    pub qs: Qs,
    pub counter_move: CounterMove,

    #[serde(skip)]
    pub position: Position,


    #[serde(skip)]
    pub results: SearchResults,

    #[serde(skip)]
    pub board: Board,
    #[serde(skip)]
    pub max_depth: Ply,

    #[serde(skip)]
    pub pv_table: PvTable,

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

    pub fn set_eval(&mut self, eval: Eval) -> &mut Self {
        self.eval = eval;
        self
    }

    pub fn set_timing_method(&mut self, tm: TimeControl) -> &mut Self {
        self.mte.time_control = tm;
        self
    }

    pub fn set_callback(
        &mut self,
        callback: impl Fn(&SearchProgress) + Send + Sync + 'static,
    ) -> &mut Self {
        self.controller.register_callback(callback);
        self
    }
}

impl Component for Algo {
    fn new_iter(&mut self) {}

    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => self.new_game(),
            SetPosition => self.new_position(),
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => self.new_iter(),
        }

        self.ids.set_state(s);
        self.eval.set_state(s);
        self.nmp.set_state(s);
        self.futility.set_state(s);
        self.rev_fut.set_state(s);
        self.mate_dist.set_state(s);

        self.pvs.set_state(s);
        self.ext.set_state(s);
        self.lmr.set_state(s);
        self.mte.set_state(s);
        self.move_orderer.set_state(s);

        self.repetition.set_state(s);
        self.tt.set_state(s);
        self.killers.set_state(s);
        self.history.set_state(s);
        self.explainer.set_state(s);

        self.restrictions.set_state(s);
        self.razor.set_state(s);
        self.recognizer.set_state(s);
        self.aspiration.set_state(s);
        self.clock.set_state(s);

        self.progress.set_state(s);
        self.controller.set_state(s);
        self.lmp.set_state(s);
        self.counter_move.set_state(s);
        self.qs.set_state(s);
    }

    fn new_game(&mut self) {
        self.clock_checks = 0;
        self.pv_table = PvTable::default();
        self.current_variation = Variation::new();
        self.max_depth = 0;
    }

    fn new_position(&mut self) {
        self.clock_checks = 0;
        self.pv_table = PvTable::default();
        self.current_variation = Variation::new();
        self.max_depth = 0;
    }
}

impl fmt::Debug for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Algo")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("board", &self.board)
            .field("analyse_mode", &self.analyse_mode)
            //.field("pv", &self.pv)
            .field("debug", &self.debug)
            .field("depth", &self.max_depth)
            .field("ids", &self.ids)
            .field("eval", &self.eval)
            .field("nmp", &self.nmp)
            .field("futility", &self.futility)
            .field("rev_fut", &self.rev_fut)
            .field("mate_dist", &self.mate_dist)
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
            .field("counter_move", &self.counter_move)
            .field("qs", &self.qs)
            .field("clock", &self.clock)
            .field("results", &self.progress)
            .finish()
    }
}

impl fmt::Display for Algo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "search position  : {}", self.position)?;
        writeln!(f, "starting board   : {}", self.board.to_fen())?;
        writeln!(f, "time control     : {}", self.mte.time_control)?;
        writeln!(f, "material         : {}", self.board.material())?;
        writeln!(
            f,
            "phase            : {} %",
            self.board.phase(&self.eval.phaser).0
        )?;
        writeln!(
            f,
            "static eval      : {}",
            self.board.eval_with_outcome(&self.eval, &Node::root(0))
        )?;
        // writeln!(f, "bm               : {}", self.results.bm())?;
        writeln!(f, "debug            : {}", self.debug)?;
        writeln!(f, "score            : {}", self.score())?;
        writeln!(f, "analyse mode     : {}", self.analyse_mode)?;
        writeln!(f, "qsearch          : {}", self.qsearch_disabled)?;
        writeln!(f, "depth            : {}", self.max_depth)?;
        writeln!(f, "results          : {}", self.results_as_position())?;
        writeln!(f, ".\n.\n[controller]\n{}", self.controller)?;
        writeln!(f, ".\n.\n[move orderer]\n{}", self.move_orderer)?;
        writeln!(f, ".\n.\n[move time estimator]\n{}", self.mte)?;
        writeln!(f, ".\n.\n[nmp]\n{}", self.nmp)?;
        writeln!(f, ".\n.\n[futility]\n{}", self.futility)?;
        writeln!(f, ".\n.\n[rev_fut]\n{}", self.rev_fut)?;
        writeln!(f, ".\n.\n[mate_dist]\n{}", self.mate_dist)?;
        writeln!(f, ".\n.\n[pvs]\n{}", self.pvs)?;
        writeln!(f, ".\n.\n[extensions]\n{}", self.ext)?;
        writeln!(f, ".\n.\n[reductions]\n{}", self.lmr)?;
        writeln!(f, ".\n.\n[eval]\n{}", self.eval)?;
        writeln!(f, ".\n.\n[repetition]\n{}", self.repetition)?;
        writeln!(f, ".\n.\n[tt]\n{}", self.tt)?;
        writeln!(f, "tt nodes")?;
        self.tt.fmt_nodes(f, &self.board)?;
        writeln!(f, ".\n.\n[killers]\n{}", self.killers)?;
        writeln!(f, ".\n.\n[history]\n{}", self.history)?;
        writeln!(f, ".\n.\n[iterative deepening]\n{}", self.ids)?;
        writeln!(f, ".\n.\n[pvtable]\n{}", self.pv_table)?;
        writeln!(f, ".\n.\n[explainer]\n{}", self.explainer)?;

        writeln!(f, ".\n.\n[restrictions]\n{}", self.restrictions)?;
        writeln!(f, ".\n.\n[razor]\n{}", self.razor)?;
        writeln!(f, ".\n.\n[recognizer]\n{}", self.recognizer)?;
        writeln!(f, ".\n.\n[aspiration]\n{:}", self.aspiration)?;
        writeln!(f, ".\n.\n[clock]\n{:}", self.clock)?;
        writeln!(f, ".\n.\n[counter_move]\n{:}", self.counter_move)?;
        writeln!(f, ".\n.\n[qs]\n{:}", self.qs)?;

        // writeln!(f, ".\n.\n[results]\n{}", self.progress)?;
        write!(f, "\n[results]\n{}", self.results)?;
        writeln!(f, ".\n.\n[metrics]\n{}", Metrics::to_string())?;
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
        if self.clock.cumul_nodes_this_thread() % 5_000_000 == 0
            && self.clock.cumul_nodes_this_thread() != 0
        {
            let sp = SearchProgress::with_report_progress(self);
            self.controller.invoke_callback(&sp);
        }
    }

    pub fn report_refutation(&self, ply: Ply) {
        if self.show_refutations && ply < 4 {
            let sp = SearchProgress {
                pv: self.pv_table.extract_pv_for(ply),
                mode: SearchProgressMode::Refutation,
                ..SearchProgress::default()
            };

            self.controller.invoke_callback(&sp);
        }
    }

    pub fn set_position(&mut self, pos: Position) -> &mut Self {
        self.set_state(State::SetPosition);
        self.explainer.set_board(pos.board().clone());
        self.repetition.push_position(&pos);
        self.board = pos.supplied_variation().apply_to(pos.board());
        self.tt.rewrite_pv(pos.board());
        self.position = pos;
        self
    }

    pub fn search(&mut self) {
        {
            // profile_method!(search);
            // hprof::profiler().disable();
            // let _g = hprof::enter("search");

            self.set_state(State::StartSearch);
            self.search_iteratively();
            self.set_state(State::EndSearch);
        }
        // if firestorm::enabled() {
        //     firestorm::save("./flames/")
        //         .map_err(|e| e.to_string())
        //         .unwrap();
        // }
        // hprof::profiler().print_timing();
    }

    pub fn results_as_position(&self) -> Position {
        self.results.to_position()
    }

    pub fn score(&self) -> Score {
        self.results.score()
    }

    pub fn best_move(&self) -> Move {
        self.results.best_move().unwrap_or(Move::NULL_MOVE)
    }

    pub fn pv(&self) -> &Variation {
        &self.results.pv()
    }


    #[inline]
    pub fn time_up_or_cancelled(&mut self, ply: Ply, force_check: bool) -> (bool, Event) {
        self.clock_checks += 1;

        // never cancel on ply=1, this way we always have a best move, and we detect mates
        if self.max_depth == 1 {
            return (false, Event::Unknown);
        }

        if self.controller.is_cancelled() {
            return (true, Event::UserCancelled);
        }

        let time_up = self.mte.is_time_up(ply, &self.clock, force_check);
        if time_up {
            // self.stats.completed = false;
            // self.stats.set_score(-Score::INFINITY, Event::SearchTimeUp);
            self.controller.cancel();
        }
        (time_up, Event::SearchTimeUp)
    }

    pub fn clear_move(&mut self, ply: Ply) {
        self.pv_table.set(ply, &Move::NULL_MOVE, true);
    }

    pub fn record_move(&mut self, ply: Ply, mv: &Move) {
        self.pv_table.set(ply, mv, false);
        self.pv_table.propagate_from(ply + 1);
    }

    pub fn record_truncated_move(&mut self, ply: Ply, mv: &Move) {
        self.pv_table.set(ply, mv, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::Uci;
    use crate::eval::eval::*;
    use crate::piece::*;
    use anyhow::*;
    use test_log::test;
    use toml;

    #[test]
    fn serde_algo_test() -> Result<()> {
        info!("toml\n{}", toml::to_string(&Algo::default())?);
        Ok(())
    }

    #[test]
    fn test_display_algo() {
        let mut algo = Algo::new();
        algo.set_timing_method(TimeControl::Depth(1));
        println!("{}", algo);
        println!("{:?}", algo);
        println!("{:#?}", algo);
    }

    #[test]
    fn test_black_opening() {
        let mut board = Catalog::starting_board();
        board.set_turn(Color::Black);
        let mut search = Algo::new();
        search.set_timing_method(TimeControl::Depth(1));
        search.move_orderer.enabled = false;
        search.set_position(Position::from_board(board));
        search.search();
        assert_eq!(search.best_move().uci(), "d7d5");
    }

    #[test]
    #[ignore]
    fn jons_chess_problem() {
        let pos =
            Position::parse_epd("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1")
                .unwrap();
        println!("{}", pos);
        let mut search = Algo::new();
        let eval = Eval::new();
        search
            .set_timing_method(TimeControl::Depth(9))
            .set_eval(eval);
        search.set_position(pos);
        search.search();
        println!("{}", search);
    }

    #[test]
    fn bug05() {
        let pos = Position::parse_epd("8/8/3N4/4B3/6p1/5k1p/4n2P/7K b - - 75 93 ").unwrap();
        let mut search = Algo::new();
        search
            .set_timing_method(TimeControl::Depth(8))
            .set_callback(Uci::uci_info);
        search.set_position(pos);
        search.search();
        println!("{}", search);
    }

    #[test]
    fn bug06() -> Result<()> {
        // 11.Qd3       b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11   acd 4; bm d1d3; ce 60; pv "d1d3 c6b4 d3d1";
        // 11... Nb4    b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2NQ4/PPP1BPPP/B3R1KR b - - 2 11   acd 4; bm c6b4; ce 30; pv "c6b4 d3d1 b4c6";
        let mut search = Algo::new();
        let pos06 =
            Position::parse_epd("b1q1r1kr/ppppbppp/2nnp3/4N3/3P4/2N1P3/PPP2PPP/BQ2RBKR w - - 2 6")?;
        let pos07 =
            Position::parse_epd("b2qr1kr/ppppbppp/2nnp3/4N3/3P4/2NBP3/PPP2PPP/BQ2R1KR w - - 4 7")?;
        let pos08 =
            Position::parse_epd("b2qr1kr/pppp1ppp/2nnpb2/4N3/3P4/2NBP3/PPP2PPP/B2QR1KR w - - 6 8")?;
        let pos09 =
            Position::parse_epd("b2qr1kr/ppppbppp/2nnp3/8/3P2N1/2NBP3/PPP2PPP/B2QR1KR w - - 8 9")?;
        let pos10 = Position::parse_epd(
            "b2qr1kr/pppp1ppp/2nnp3/6b1/3P2N1/2N1P3/PPP1BPPP/B2QR1KR w - - 10 10",
        )?;
        let pos11 = Position::parse_epd(
            "b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11",
        )?;
        let pos12 = Position::parse_epd(
            "b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12",
        )?;
        search
            .set_position(pos06)
            .set_timing_method(TimeControl::Depth(3))
            .search();
        search
            .set_position(pos07)
            .set_timing_method(TimeControl::Depth(3))
            .search();
        search
            .set_position(pos08)
            .set_timing_method(TimeControl::Depth(3))
            .search();
        search
            .set_position(pos09)
            .set_timing_method(TimeControl::Depth(3))
            .search();
        search
            .set_position(pos10)
            .set_timing_method(TimeControl::Depth(3))
            .search();
        search
            .set_position(pos11)
            .set_timing_method(TimeControl::Depth(3))
            .search();
        search
            .set_position(pos12)
            .set_timing_method(TimeControl::Depth(3))
            .search();
        println!("{}", search);
        Ok(())
    }

    #[test]
    fn bug07() {
        let pos = Position::parse_epd("8/4R3/8/8/8/3K4/1k6/8 b - - 18 10").unwrap();
        let mut search = Algo::new();
        search
            .set_timing_method(TimeControl::Depth(12))
            .set_callback(Uci::uci_info);
        search.set_position(pos);
        search.search();
        println!("{}", search);
    }

    #[test]
    #[ignore]
    fn test_truncated_pv() {
        let mut algo = Algo::new();
        //             .set_timing_method(TimeControl::from_move_time_millis(1000))
        algo.set_timing_method(TimeControl::Depth(7));
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
