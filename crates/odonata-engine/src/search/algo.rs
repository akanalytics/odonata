use super::{
    counter_move::CounterMove, engine::ThreadedSearch, lmp::Lmp, mate_distance::MateDistance,
    qs::Qs, reverse_futility::ReverseFutility, search_explainer::Explainer,
};
use crate::{
    book::opening_book::OpeningBook,
    cache::tt2::TranspositionTable2,
    engine::Engine,
    eval::{recognizer::Recognizer, Eval},
    search::{
        aspiration::Aspiration, controller::Controller, extensions::Extensions, futility::Futility,
        history_heuristic::HistoryHeuristic, iterative_deepening::IterativeDeepening,
        killers::Killers, lmr::Lmr, move_orderer::MoveOrderer,
        move_time_estimator::MoveTimeEstimator, nmp::NullMovePruning, pvs::Pvs, razor::Razor,
        restrictions::Restrictions, search_results::SearchResults, trail::Trail,
    },
};
use indexmap::map::IndexMap;
use odonata_base::{
    boards::{Board, Position, Repetition},
    clock::Clock,
    domain::{
        info::{Info, InfoKind},
        node::Event,
        score::Score,
        timecontrol::TimeControl,
    },
    epd::Epd,
    infra::{
        component::{Component, State},
        metric::Metrics,
        utils::UciString,
        value::{Aggregate, Stats},
    },
    mv::Move,
    piece::Ply,
    prelude::*,
    trace::logger::LoggingSystem,
    variation::Variation,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    io::stdout,
    ops::Deref,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub ids:       IterativeDeepening,
    pub nmp:       NullMovePruning,
    pub futility:  Futility,
    pub rev_fut:   ReverseFutility,
    pub mate_dist: MateDistance,

    pub pvs:          Pvs,
    pub ext:          Extensions,
    pub lmr:          Lmr,
    pub mte:          MoveTimeEstimator,
    pub move_orderer: MoveOrderer,

    pub repetition: Repetition,
    pub tt:         TranspositionTable2,
    pub killers:    Killers,
    pub history:    HistoryHeuristic,
    pub explainer:  Explainer,

    pub razor:      Razor,
    pub recognizer: Recognizer,
    pub aspiration: Aspiration,
    pub clock:      Clock,

    pub controller:   Controller,
    pub lmp:          Lmp,
    pub qs:           Qs,
    pub counter_move: CounterMove,
    pub opening_book: OpeningBook,
    pub thread_count: u32,
    pub engine_name:  String,

    #[serde(skip)]
    pub eval: Eval,

    #[serde(skip)]
    pub position: Position,

    #[serde(skip)]
    pub results: SearchResults,

    #[serde(skip)]
    pub game_metrics: Arc<Mutex<Stats>>,

    #[serde(skip)]
    pub board:     Board,
    #[serde(skip)]
    pub max_depth: Ply,

    #[serde(skip)]
    pub clock_checks: u64,

    #[serde(skip)]
    pub current_variation: Variation,

    #[serde(skip)]
    pub restrictions: Restrictions,
}

pub struct Query {
    pub board:      Board,
    pub moves:      Variation,
    pub tc:         TimeControl,
    pub root_moves: Vec<Move>,
}

impl Engine for Search {
    fn name(&self) -> String {
        self.engine_name.clone()
    }

    fn set_name(&mut self, name: String) {
        self.engine_name = name;
    }

    fn static_eval(&mut self, pos: Epd) -> anyhow::Result<Score> {
        self.qsearch(pos)?
            .score()
            .ok_or_else(|| anyhow::anyhow!("failed to get score"))
    }

    fn qsearch(&mut self, pos: Epd) -> anyhow::Result<SearchResults> {
        self.search_with_options(pos, TimeControl::Depth(0), SearchOptions::none())
    }

    fn search_with_options(
        &mut self,
        epd: Epd,
        tc: TimeControl,
        opts: SearchOptions,
    ) -> anyhow::Result<SearchResults> {
        info!(target: "eng","-> search on {n}", n = self.name());
        info!(target: "eng", "-> search on {b} {tc}", b = epd.board());
        self.controller
            .register_callback(|i| info!(target: "eng", "<- info {i}"));
        self.controller.set_running();
        let t = if let TimeControl::DefaultTime = tc {
            let suggested_depth = epd.int("acd").context(format!(
                "tc=default specified but EPD has no depth (acd): {epd}"
            ))? as i32;
            TimeControl::Depth(suggested_depth)
        } else {
            tc
        };
        self.set_timing_method(t.clone());
        self.set_position(epd.clone());
        self.restrictions.search_moves = opts.root_moves;
        let mut pos = Position::from_played_moves(epd.setup_board(), epd.played().clone());
        self.run_search(&mut pos);
        info!(target: "eng", " <- results {res}", res = self.results);
        Ok(self.results.clone())
    }

    fn options(&self) -> IndexMap<String, String> {
        let mut map: IndexMap<&str, &str> = IndexMap::new();
        let mb = format!("spin default {} min 0 max 4000", self.tt.mb);
        let mpv = format!("spin default {} min 1 max 64", self.controller.multi_pv);
        let book_file = format!(
            "string default {}",
            UciString::to_uci(&self.opening_book.book_file)
        );
        let opponent = format!("string default {}", UciString::to_uci(""));
        let debug_log_file = format!("string default {}", UciString::to_uci(""));
        let tc = format!("spin default {} min 1 max 16", self.thread_count);

        map.insert("Hash", &mb);
        map.insert("MultiPV", &mpv);
        map.insert("Debug_Log_File", &debug_log_file);
        map.insert("Ponder", "check default false");
        map.insert("Threads", &tc);
        map.insert("UCI_AnalyseMode", "check default false");
        map.insert("Analyse_Mode", "check default false");
        map.insert("UCI_Opponent", &opponent);
        // map.insert("nodestime", "");
        map.insert("OwnBook", "check default false");
        map.insert("Book File", &book_file);
        map.insert("Best Book Move", "check default true");
        map.insert("Clear Hash", "button");
        if Metrics::metrics_enabled() {
            map.insert(
                "Features",
                "string default [perft,eval,board,d,metrics,compiler]",
            );
        } else {
            map.insert("Features", "string default [perft,eval,board,d,compiler]");
        }
        // map

        map.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn set_option(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        match name {
            "Features" => {}
            "Clear Hash" => self.set_state(State::NewGame),
            "Hash" => self.tt.mb = value.parse()?,
            "MultiPV" => self.controller.multi_pv = value.parse()?,
            "Debug_Log_File" => LoggingSystem::instance()?.set_log_filename(value, &self.name())?,
            "UCI_AnalyseMode" => self.controller.analyse_mode = value.parse()?,
            "Analyse_Mode" => self.controller.analyse_mode = value.parse()?,
            "UCI_Opponent" => {}
            "nodestime" => self.mte.set_nodestime(value.parse()?),
            "Threads" => self.thread_count = value.parse()?,
            "Ponder" => {} // pondering determined by "go ponder", so no variable to track
            "OwnBook" => {
                self.opening_book.own_book = value.parse()?;
                self.opening_book.reload()?;
            }
            "Book File" => {
                self.opening_book.book_file = UciString::parse_uci(value);
                self.opening_book.reload()?;
            }
            "Best Book Move" => self.opening_book.best_book_line = value.parse()?,
            _ => anyhow::bail!("Algo does not support set option '{name}'"),
        }
        trace!(target: "eng", "-> set option '{name}' = '{value}'");

        Ok(())
    }

    fn start_game(&mut self) -> anyhow::Result<()> {
        self.set_state(State::NewGame);
        Ok(())
    }

    // fn general_command(&mut self, filter: &str, wait_for: &str) -> anyhow::Result<String> {
    //     let metrics = self.results.metrics.unwrap_or_default();
    //     Ok(metrics.snapshot().filter_rows(filter).to_string())
    // }

    fn metrics(&mut self, filter: &str) -> anyhow::Result<Stats> {
        let stats = self
            .game_metrics
            .lock()
            .unwrap()
            .clone()
            .filter(&Regex::new(filter)?);
        trace!(target: "eng", "-> metrics {filter} = '{stats}'");
        Ok(stats)
    }
}

/// builder methods
impl Search {
    pub fn new() -> Search {
        let mut algo = ThreadedSearch::new().search;
        algo.engine_name = String::from("odonata");
        algo
    }

    pub fn set_timing_method(&mut self, tm: TimeControl) -> &mut Self {
        // match tm.is_time_sensitive() {
        //     true => self.controller.analyse_mode = false,
        //     false => self.controller.analyse_mode = true,
        // };
        self.mte.set_time_control(tm);
        self
    }

    pub fn set_callback(&mut self, callback: impl Fn(&Info) + Send + Sync + 'static) -> &mut Self {
        self.controller.register_callback(callback);
        self
    }
}

impl Component for Search {
    fn new_iter(&mut self) {}

    fn set_state(&mut self, s: State) {
        use State::*;

        // clock first
        self.clock.set_state(s);

        match s {
            NewGame => {
                self.new_game();
                self.eval.new_game();
            }

            SetPosition => self.new_position(),
            StartSearch => {}
            EndSearch => {
                if self.explainer.show_pv_eval {
                    self.results
                        .write_explanation(stdout(), &self.eval, self.position.clone())
                        .unwrap();
                }
            }
            StartDepthIteration(_) => self.new_iter(),
            Shutdown => {}
        }

        self.ids.set_state(s);
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

        self.razor.set_state(s);
        self.recognizer.set_state(s);
        self.aspiration.set_state(s);

        self.controller.set_state(s);
        self.lmp.set_state(s);
        self.counter_move.set_state(s);
        self.qs.set_state(s);
        self.opening_book.set_state(s);
    }

    fn new_game(&mut self) {
        debug!(target = "eng", "new game");
        self.game_metrics = Arc::default();
    }

    fn new_position(&mut self) {
        self.clock_checks = 0;
        self.current_variation = Variation::new();
        self.max_depth = 0;
    }
}

impl fmt::Debug for Search {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Algo")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("board", &self.board)
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
            .finish()
    }
}

impl fmt::Display for Search {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !f.alternate() {
            write!(f, "Algo")?;
            return Ok(());
        }
        writeln!(f, "search position  : {:?}", self.position)?;
        writeln!(f, "starting board   : {}", self.board.to_fen())?;
        writeln!(f, "time control     : {}", self.mte.time_control())?;
        writeln!(f, "material         : {}", self.board.material())?;
        writeln!(
            f,
            "static eval      : {}",
            self.eval.static_eval(&self.position)
        )?;
        // writeln!(f, "bm               : {}", self.results.bm())?;
        writeln!(f, "score            : {}", self.score())?;
        writeln!(f, "depth            : {}", self.max_depth)?;
        writeln!(f, "threads          : {}", self.thread_count)?;
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
        writeln!(f, ".\n.\n[eval]\n{:?}", self.eval.deref())?;
        writeln!(f, ".\n.\n[repetition]\n{}", self.repetition)?;
        writeln!(f, ".\n.\n[tt]\n{}", self.tt)?;
        writeln!(f, "tt pv extract")?;
        self.tt.fmt_extract_pv_and_score(f, &self.board)?;
        writeln!(f, ".\n.\n[killers]\n{}", self.killers)?;
        writeln!(f, ".\n.\n[history]\n{}", self.history)?;
        writeln!(f, ".\n.\n[iterative deepening]\n{}", self.ids)?;
        writeln!(f, ".\n.\n[explainer]\n{}", self.explainer)?;

        writeln!(f, ".\n.\n[restrictions]\n{}", self.restrictions)?;
        writeln!(f, ".\n.\n[razor]\n{}", self.razor)?;
        writeln!(f, ".\n.\n[recognizer]\n{}", self.recognizer)?;
        writeln!(f, ".\n.\n[aspiration]\n{:}", self.aspiration)?;
        writeln!(f, ".\n.\n[clock]\n{:}", self.clock)?;
        writeln!(f, ".\n.\n[counter_move]\n{:}", self.counter_move)?;
        writeln!(f, ".\n.\n[qs]\n{:}", self.qs)?;
        writeln!(f, ".\n.\n[opening_book]\n{:?}", self.opening_book)?;

        write!(f, "\n[results]\n{}", self.results)?;
        // writeln!(f, ".\n.\n[metrics]\n{}", Metrics::to_string())?;
        Ok(())
    }
}

impl Search {
    pub fn var(&self) -> &Variation {
        &self.current_variation
    }

    pub fn report_progress(&self) {
        if self.clock.cumul_nodes_this_thread() % 1_000_000 == 0
            && self.clock.cumul_nodes_this_thread() != 0
        {
            let info = Info {
                kind: InfoKind::NodeCounts,
                nodes: Some(self.clock.cumul_nodes_all_threads()),
                nodes_thread: Some(self.clock.cumul_nodes_this_thread()),
                nps: Some(self.clock.cumul_knps_all_threads() * 1000),
                hashfull_per_mille: Some(self.tt.hashfull_per_mille()),
                time_millis: Some(self.clock.elapsed_search().time.as_millis() as u64),
                ..Default::default()
            };

            self.controller.invoke_callback(&info);
        }
    }

    pub fn report_refutation(&self, ply: Ply) {
        if self.controller.show_refutations && ply < 4 {
            // let sp = Info {
            //     kind: InfoKind::Refutation,
            //     pv: Some(self.pv_table.extract_pv_for(ply).to_inner()),
            //     ..Info::default()
            // };

            // self.controller.invoke_callback(&sp);
        }
    }

    pub fn set_position(&mut self, epd: Epd) -> &mut Self {
        self.set_state(State::SetPosition);
        self.explainer.set_board(epd.board().clone());
        self.repetition.capture_all_prior_positions(&epd);
        self.board = epd.board();
        self.position = Position::from_played_moves(epd.setup_board(), epd.played().clone());
        self
    }

    pub fn run_search(&mut self, pos: &mut Position) {
        {
            let mut trail = Trail::new(self.board.clone());
            self.set_state(State::StartSearch);
            self.search_iteratively(pos, &mut trail);
            self.set_state(State::EndSearch);
            // Metrics::flush_thread_local();
            let search_stats = Metrics::take_last_iter().to_stats();
            let mut gm = self.game_metrics.lock().unwrap();
            debug!(target: "metrics", "[seach metrics]\n{search_stats:#?}\n[game metrics]{gm:#?}" );
            gm.aggregate(search_stats);
        }
    }

    // pub fn results_as_position(&self) -> Epd {
    //     self.to_epd()
    //     // let epd = Epd::from_board(self.position.board().clone());
    //     // self.results.to_position(epd, &SearchResults::TAGS)
    // }

    pub fn score(&self) -> Score {
        self.results.score().unwrap_or_default()
    }

    pub fn best_move(&self) -> Move {
        self.results.supplied_move().unwrap_or_default()
    }

    pub fn pv(&self) -> Variation {
        self.results.pv()
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

        let time_up = self.mte.is_time_up(ply, &mut self.clock, force_check);
        if time_up {
            // self.stats.completed = false;
            // self.stats.set_score(-Score::INFINITY, Event::SearchTimeUp);
            self.controller.cancel();
        }
        (time_up, Event::SearchTimeUp)
    }

    // pub fn clear_move(&mut self, ply: Ply) {
    //     self.pv_table.set(ply, &Move::NULL_MOVE, true);
    // }

    // pub fn record_move(&mut self, ply: Ply, mv: &Move) {
    //     self.pv_table.set(ply, mv, false);
    //     self.pv_table.propagate_from(ply + 1);
    // }

    // pub fn record_truncated_move(&mut self, ply: Ply, mv: &Move) {
    //     self.pv_table.set(ply, mv, true);
    // }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::*;
    use crate::comms::uci_server::UciServer;
    use anyhow::*;
    use odonata_base::{catalog::Catalog, infra::metric::MetricsRegistry};
    use test_log::test;
    use toml;

    #[test]
    fn serde_algo_test() -> Result<()> {
        info!("toml\n{}", toml::to_string(&Search::default())?);
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_display_algo() {
        let mut eng = ThreadedSearch::new();
        eng.search.set_timing_method(TimeControl::Depth(1));
        println!("{}", eng.search);
        println!("{:?}", eng.search);
        println!("{:#?}", eng.search);
    }

    #[test]
    fn test_black_opening() {
        let mut board = Catalog::starting_board();
        board.set_turn(Color::Black);
        let mut search = ThreadedSearch::new();
        search.search.move_orderer.enabled = false;
        let sr = search.search(Epd::from_board(board), TimeControl::Depth(1));
        assert_eq!(sr.unwrap().supplied_move().unwrap().to_uci(), "e7e5");
    }

    #[test]
    fn test_algo_as_engine() {
        let board = Catalog::starting_board();
        let mut eng = ThreadedSearch::new();
        assert_eq!(eng.options().contains_key("MultiPV"), true);
        assert!(eng.set_option("UCI_AnalyseMode", "False").is_err());
        assert!(eng.set_option("UCI_AnalyseMode", "true").is_ok());
        assert!(eng.set_option("UCI_AnalyseMode", "false").is_ok());
        let sr = eng.search(Epd::from_board(board), TimeControl::Depth(1));
        assert_eq!(sr.unwrap().supplied_move().unwrap().to_uci(), "e2e4");
    }

    #[test]
    fn jons_chess_problem() {
        let pos =
            Epd::parse_epd("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1").unwrap();
        let mut search = ThreadedSearch::new();
        let sr = search.search(pos.clone(), TimeControl::Depth(12)).unwrap();
        println!("{}", sr.to_epd());
        assert_eq!(
            sr.supplied_move().unwrap(),
            Move::parse_uci("f6h4", &pos.board()).unwrap()
        )
    }

    #[test]
    fn bug05() {
        let pos = Epd::parse_epd("8/8/3N4/4B3/6p1/5k1p/4n2P/7K b - - 75 93 ").unwrap();
        let mut eng = ThreadedSearch::new();
        eng.search(pos, TimeControl::Depth(8)).unwrap();
        println!("{}", eng);
    }

    #[test]
    fn bug06() -> Result<()> {
        // 11.Qd3       b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11   acd 4; bm d1d3; ce 60; pv "d1d3 c6b4 d3d1";
        // 11... Nb4    b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2NQ4/PPP1BPPP/B3R1KR b - - 2 11   acd 4; bm c6b4; ce 30; pv "c6b4 d3d1 b4c6";
        let mut eng = ThreadedSearch::new();
        let pos06 =
            Epd::parse_epd("b1q1r1kr/ppppbppp/2nnp3/4N3/3P4/2N1P3/PPP2PPP/BQ2RBKR w - - 2 6")?;
        let pos07 =
            Epd::parse_epd("b2qr1kr/ppppbppp/2nnp3/4N3/3P4/2NBP3/PPP2PPP/BQ2R1KR w - - 4 7")?;
        let pos08 =
            Epd::parse_epd("b2qr1kr/pppp1ppp/2nnpb2/4N3/3P4/2NBP3/PPP2PPP/B2QR1KR w - - 6 8")?;
        let pos09 =
            Epd::parse_epd("b2qr1kr/ppppbppp/2nnp3/8/3P2N1/2NBP3/PPP2PPP/B2QR1KR w - - 8 9")?;
        let pos10 =
            Epd::parse_epd("b2qr1kr/pppp1ppp/2nnp3/6b1/3P2N1/2N1P3/PPP1BPPP/B2QR1KR w - - 10 10")?;
        let pos11 =
            Epd::parse_epd("b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11")?;
        let pos12 =
            Epd::parse_epd("b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12")?;
        let _res = eng.search(pos06, TimeControl::Depth(3)).unwrap();
        let _res = eng.search(pos07, TimeControl::Depth(3)).unwrap();
        let _res = eng.search(pos08, TimeControl::Depth(3)).unwrap();
        let _res = eng.search(pos09, TimeControl::Depth(3)).unwrap();
        let _res = eng.search(pos10, TimeControl::Depth(3)).unwrap();
        let _res = eng.search(pos11, TimeControl::Depth(3)).unwrap();
        let _res = eng.search(pos12, TimeControl::Depth(3)).unwrap();
        println!("{}", eng);
        Ok(())
    }

    #[test]
    fn bug07() {
        let pos = Epd::parse_epd("8/4R3/8/8/8/3K4/1k6/8 b - - 18 10").unwrap();
        let mut eng = ThreadedSearch::new();
        eng.set_callback(UciServer::uci_info);
        eng.search(pos, TimeControl::Depth(12)).unwrap();
        println!("{}", eng);
    }

    #[test]
    fn test_search_metrics() {
        let mut eng = ThreadedSearch::new();
        let pos = Catalog::test_position();
        let _res = eng.search(pos, TimeControl::Depth(10));
        // let metrics = res.unwrap().metrics.unwrap().snapshot();
        let metrics = MetricsRegistry::snapshot_metrics();
        println!("metrics\n{metrics}");
        println!("metrics\n{s}", s = serde_json::to_string(&metrics).unwrap());
    }

    #[test]
    #[ignore]
    fn test_truncated_pv() {
        let mut eng = ThreadedSearch::new();
        //             .set_timing_method(TimeControl::from_move_time_millis(1000))
        // algo.repetition.avoid_tt_on_repeats = false;
        // algo.tt.min_ply = 2;
        let positions = Catalog::win_at_chess();
        for p in positions {
            let tc = TimeControl::Depth(7);
            eng.new_game();
            eng.search.tt.allow_truncated_pv = true;
            let res = eng.search.search(p.clone(), tc).unwrap();
            let pv1 = res.pv();
            eng.search.tt.current_age -= 1;
            println!("{:<40} - {}", pv1.to_uci(), res.to_epd());

            let tc = TimeControl::Depth(7);
            eng.search.tt.allow_truncated_pv = true;
            let res = eng.search.search(p.clone(), tc).unwrap();
            let pv2 = res.pv();
            println!("{:<40} - {}", pv2.to_uci(), res.to_epd());

            let tc = TimeControl::Depth(7);
            eng.search.tt.allow_truncated_pv = false;
            let res = eng.search.search(p.clone(), tc).unwrap();
            let pv3 = res.pv();
            println!("{:<40} - {}\n", pv3.to_uci(), res.to_epd());

            // assert_eq!(pv1, pv2, "{}", p );
        }
    }

    #[test]
    fn test_search_nnue() {
        let mut map = HashMap::new();
        map.insert("eval".to_string(), "nnue".to_string());
        let mut eng = ThreadedSearch::configure(map).unwrap();
        let res = eng
            .search(
                Epd::from_board(Board::starting_pos()),
                TimeControl::Depth(5),
            )
            .unwrap();
        assert_eq!(res.score().unwrap() > Score::from_cp(0), true);
    }
}
