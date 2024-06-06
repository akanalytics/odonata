use std::fmt::{self, Debug};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use indexmap::map::IndexMap;
use odonata_base::boards::{Position, Repetition};
use odonata_base::clock::Clock;
use odonata_base::domain::info::{Info, InfoKind};
use odonata_base::domain::node::Event;
use odonata_base::domain::staticeval::StaticEval;
use odonata_base::epd::Epd;
use odonata_base::infra::component::{Component, State};
use odonata_base::infra::metric::Metrics;
use odonata_base::infra::utils::UciString;
use odonata_base::infra::value::{Aggregate, Stats};
use odonata_base::prelude::*;
use odonata_base::trace::logger::LoggingSystem;
use regex::Regex;

use super::counter_move::CounterMove;
use super::lmp::Lmp;
use super::mate_distance::MateDistance;
use super::qs::Qs;
use super::reverse_futility::ReverseFutility;
use crate::book::opening_book::OpeningBook;
use crate::cache::tt2::TranspositionTable2;
use crate::eval::Eval;
use crate::eval::recognizer::Recognizer;
use crate::search::aspiration::Aspiration;
use crate::search::controller::Controller;
use crate::search::extensions::Extensions;
use crate::search::futility::Futility;
use crate::search::history_heuristic::HistoryHeuristic;
use crate::search::iterative_deepening::IterativeDeepening;
use crate::search::killers::Killers;
use crate::search::lmr::Lmr;
use crate::search::move_orderer::MoveOrderer;
use crate::search::move_time_estimator::MoveTimeEstimator;
use crate::search::nmp::NullMovePruning;
use crate::search::pvs::Pvs;
use crate::search::razor::Razor;
use crate::search::restrictions::Restrictions;
use crate::search::search_results::Response;
use crate::search::trail::Trail;

#[derive(Clone)]
pub struct Callback(pub Arc<dyn Fn(&Info) + Send + Sync>);

impl Debug for Callback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Callback").finish()
    }
}

fn do_nothing(_: &Info) {}

impl Default for Callback {
    fn default() -> Self {
        Self(Arc::new(do_nothing))
    }
}

#[derive(Clone, Default, Debug)]
pub struct Search {
    pub ids:          IterativeDeepening,
    pub nmp:          NullMovePruning,
    pub futility:     Futility,
    pub rev_fut:      ReverseFutility,
    pub mate_dist:    MateDistance,
    pub pvs:          Pvs,
    pub ext:          Extensions,
    pub lmr:          Lmr,
    pub mte:          MoveTimeEstimator,
    pub move_orderer: MoveOrderer,
    pub repetition:   Repetition,
    pub tt:           TranspositionTable2,
    pub killers:      Killers,
    pub history:      HistoryHeuristic,
    pub razor:        Razor,
    pub recognizer:   Recognizer,
    pub aspiration:   Aspiration,
    pub clock:        Clock,
    pub controller:   Controller,
    pub lmp:          Lmp,
    pub qs:           Qs,
    pub counter_move: CounterMove,
    pub opening_book: OpeningBook,

    pub callback:          Callback,
    pub eval:              Eval,
    pub position:          Position,
    pub response:          Response,
    pub game_metrics:      Arc<Mutex<Stats>>,
    pub board:             Board,
    pub max_depth:         Ply,
    pub clock_checks:      u64,
    pub current_variation: Variation,
    pub restrictions:      Restrictions,
}

pub struct Query {
    pub board:      Board,
    pub moves:      Variation,
    pub tc:         TimeControl,
    pub root_moves: Vec<Move>,
}

impl Configurable for Search {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.max_depth.set(p.get("max_depth"))?;
        self.ids.set(p.get("ids"))?;
        self.lmr.set(p.get("lmr"))?;
        self.nmp.set(p.get("nmp"))?;
        self.controller.set(p.get("controller"))?;
        self.lmp.set(p.get("lmp"))?;
        self.killers.set(p.get("killers"))?;
        self.eval.set(p.get("eval"))?;
        self.tt.set(p.get("tt"))?;
        self.razor.set(p.get("razor"))?;
        self.mate_dist.set(p.get("mate_dist"))?;
        self.opening_book.set(p.get("opening_book"))?;
        self.aspiration.set(p.get("aspiration"))?;
        self.ext.set(p.get("ext"))?;
        self.pvs.set(p.get("pvs"))?;
        self.futility.set(p.get("futility"))?;
        self.rev_fut.set(p.get("rev_fut"))?;
        self.history.set(p.get("history"))?;
        self.counter_move.set(p.get("counter_move"))?;
        self.repetition.set(p.get("repetition"))?;
        self.clock.set(p.get("clock"))?;
        self.mte.set(p.get("mte"))?;
        self.qs.set(p.get("qs"))?;
        self.move_orderer.set(p.get("move_orderer"))?;
        Ok(p.is_modified())
    }
}

impl Search {
    /// builder methods
    pub fn new() -> Self {
        Search::default()
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
        self.callback = Callback(Arc::new(callback));
        // self.controller.register_callback(callback);
        self
    }

    // pub fn name(&self) -> String {
    //     self.engine_name.clone()
    // }

    // pub fn set_name(&mut self, name: String) {
    //     self.engine_name = name;
    // }

    pub fn static_eval(&mut self, pos: Epd) -> anyhow::Result<Score> {
        self.qsearch(pos)?
            .score()
            .ok_or_else(|| anyhow::anyhow!("failed to get score"))
    }

    pub fn qsearch(&mut self, pos: Epd) -> anyhow::Result<Response> {
        self.search_with_options(pos, TimeControl::Depth(0), SearchOptions::none())
    }

    pub fn search_with_options(&mut self, epd: Epd, tc: TimeControl, opts: SearchOptions) -> anyhow::Result<Response> {
        info!(target: "eng", "-> search on {b} {tc}", b = epd.board());
        self.controller
            .register_callback(|i| info!(target: "eng", "<- info {i}"));
        self.controller.set_running();
        let t = if let TimeControl::DefaultTime = tc {
            let suggested_depth =
                epd.int("acd")
                    .context(format!("tc=default specified but EPD has no depth (acd): {epd}"))? as i32;
            TimeControl::Depth(suggested_depth)
        } else {
            tc
        };
        self.set_timing_method(t.clone());
        self.set_position(epd.clone());
        self.restrictions.search_moves = opts.root_moves;
        let mut pos = Position::from_played_moves(epd.setup_board(), epd.played().clone());
        self.run_search(&mut pos);
        info!(target: "eng", " <- results {res}", res = self.response);
        Ok(self.response.clone())
    }

    pub fn options(&self) -> IndexMap<String, String> {
        let mut map: IndexMap<&str, &str> = IndexMap::new();
        let mb = format!("spin default {} min 0 max 4000", self.tt.mb);
        let mpv = format!("spin default {} min 1 max 64", self.controller.multi_pv);
        let book_file = format!(
            "string default {}",
            UciString::to_uci(&self.opening_book.book_file.to_string_lossy())
        );
        let opponent = format!("string default {}", UciString::to_uci(""));
        let debug_log_file = format!("string default {}", UciString::to_uci(""));

        map.insert("Hash", &mb);
        map.insert("MultiPV", &mpv);
        map.insert("Debug_Log_File", &debug_log_file);
        map.insert("Ponder", "check default false");
        map.insert("UCI_AnalyseMode", "check default false");
        map.insert("Analyse_Mode", "check default false");
        map.insert("UCI_Opponent", &opponent);
        // map.insert("nodestime", "");
        map.insert("OwnBook", "check default false");
        map.insert("Book File", &book_file);
        map.insert("Best Book Move", "check default true");
        map.insert("Clear Hash", "button");
        if Metrics::metrics_enabled() {
            map.insert("Features", "string default [perft,eval,board,d,metrics,compiler]");
        } else {
            map.insert("Features", "string default [perft,eval,board,d,compiler]");
        }
        // map

        map.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    pub fn set_option(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        trace!(target: "eng", "-> trying... search.set_option('{key}' = '{value}')");
        match key {
            "Features" => {}
            "Clear Hash" => self.set_state(State::NewGame),
            "Hash" => self.tt.mb = value.parse()?,
            "MultiPV" => self.controller.multi_pv = value.parse()?,
            "Debug_Log_File" => LoggingSystem::parse(value)?.apply()?,
            "UCI_AnalyseMode" => self.controller.analyse_mode = value.parse()?,
            "Analyse_Mode" => self.controller.analyse_mode = value.parse()?,
            "UCI_Opponent" => {}
            "nodestime" => self.mte.set_nodestime(value.parse()?),
            "Ponder" => {} // pondering determined by "go ponder", so no variable to track
            "OwnBook" => {
                self.opening_book.own_book = value.parse()?;
                self.opening_book.reload()?;
            }
            "Book File" => {
                self.opening_book.book_file = PathBuf::from(UciString::parse_uci(value));
                self.opening_book.reload()?;
            }
            "Best Book Move" => self.opening_book.best_book_line = value.parse()?,
            _ => anyhow::bail!("search does not support set option '{key}'"),
        }

        Ok(())
    }

    pub fn start_game(&mut self) -> anyhow::Result<()> {
        self.set_state(State::NewGame);
        Ok(())
    }

    // fn general_command(&mut self, filter: &str, wait_for: &str) -> anyhow::Result<String> {
    //     let metrics = self.results.metrics.unwrap_or_default();
    //     Ok(metrics.snapshot().filter_rows(filter).to_string())
    // }

    pub fn metrics(&mut self, filter: &str) -> anyhow::Result<Stats> {
        let stats = self.game_metrics.lock().unwrap().clone().filter(&Regex::new(filter)?);
        trace!(target: "eng", "-> metrics {filter} = '{stats}'");
        Ok(stats)
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
                debug!(target = "eng", "new game");
                self.eval.new_game();
                self.response = Default::default();
                self.current_variation = Default::default();
                self.position = Default::default();
                self.board = Board::new_empty();
                self.game_metrics = Arc::default();
                self.clock_checks = 0;
                self.max_depth = 0;
                self.restrictions = Restrictions::default();
            }

            SetPosition => {
                self.clock_checks = 0;
                self.current_variation = Variation::new();
                self.max_depth = 0;
            }

            StartSearch => {}
            EndSearch => {
                // if self.explainer.show_pv_eval {
                //     self.response
                //         .write_explanation(stdout(), &self.eval, self.position.clone())
                //         .unwrap();
                // }
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
        // self.explainer.set_state(s);

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
        self.set_state(State::NewGame);
    }

    fn new_position(&mut self) {
        self.set_state(State::SetPosition);
    }
}

impl fmt::Display for Search {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{self:#?}")
    }
}
//         if !f.alternate() {
//             write!(f, "Algo")?;
//             return Ok(());
//         }
//         writeln!(f, "search position  : {:?}", self.position)?;
//         writeln!(f, "starting board   : {}", self.board.to_fen())?;
//         writeln!(f, "time control     : {}", self.mte.time_control())?;
//         writeln!(f, "material         : {}", self.board.material())?;
//         writeln!(f, "static eval      : {}", self.eval.static_eval(&self.position))?;
//         // writeln!(f, "bm               : {}", self.results.bm())?;
//         writeln!(f, "score            : {}", self.score())?;
//         writeln!(f, "depth            : {}", self.max_depth)?;
//         writeln!(f, ".\n.\n[controller]\n{}", self.controller)?;
//         writeln!(f, ".\n.\n[move orderer]\n{}", self.move_orderer)?;
//         writeln!(f, ".\n.\n[move time estimator]\n{}", self.mte)?;
//         writeln!(f, ".\n.\n[nmp]\n{}", self.nmp)?;
//         writeln!(f, ".\n.\n[futility]\n{}", self.futility)?;
//         writeln!(f, ".\n.\n[rev_fut]\n{}", self.rev_fut)?;
//         writeln!(f, ".\n.\n[mate_dist]\n{}", self.mate_dist)?;
//         writeln!(f, ".\n.\n[pvs]\n{}", self.pvs)?;
//         writeln!(f, ".\n.\n[extensions]\n{}", self.ext)?;
//         writeln!(f, ".\n.\n[reductions]\n{}", self.lmr)?;
//         writeln!(f, ".\n.\n[eval]\n{:?}", self.eval)?;
//         writeln!(f, ".\n.\n[repetition]\n{}", self.repetition)?;
//         writeln!(f, ".\n.\n[tt]\n{}", self.tt)?;
//         writeln!(f, "tt pv extract")?;
//         self.tt.fmt_extract_pv_and_score(f, &self.board)?;
//         writeln!(f, ".\n.\n[killers]\n{}", self.killers)?;
//         writeln!(f, ".\n.\n[history]\n{}", self.history)?;
//         writeln!(f, ".\n.\n[iterative deepening]\n{}", self.ids)?;
//         // writeln!(f, ".\n.\n[explainer]\n{}", self.explainer)?;

//         writeln!(f, ".\n.\n[restrictions]\n{}", self.restrictions)?;
//         writeln!(f, ".\n.\n[razor]\n{}", self.razor)?;
//         writeln!(f, ".\n.\n[recognizer]\n{}", self.recognizer)?;
//         writeln!(f, ".\n.\n[aspiration]\n{:}", self.aspiration)?;
//         writeln!(f, ".\n.\n[clock]\n{:}", self.clock)?;
//         writeln!(f, ".\n.\n[counter_move]\n{:}", self.counter_move)?;
//         writeln!(f, ".\n.\n[qs]\n{:}", self.qs)?;
//         writeln!(f, ".\n.\n[opening_book]\n{:?}", self.opening_book)?;

//         write!(f, "\n[results]\n{}", self.response)?;
//         // writeln!(f, ".\n.\n[metrics]\n{}", Metrics::to_string())?;
//         Ok(())
//     }
// }

impl Search {
    pub fn var(&self) -> &Variation {
        &self.current_variation
    }

    pub fn report_progress(&self) {
        if self.clock.cumul_nodes_this_thread() % 1_000_000 == 0 && self.clock.cumul_nodes_this_thread() != 0 {
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
        // self.explainer.set_board(epd.board().clone());
        self.repetition.capture_all_prior_positions(&epd);
        self.response.input = epd.clone();
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
        self.response.score().unwrap_or_default()
    }

    pub fn best_move(&self) -> Move {
        self.response.supplied_move().unwrap_or_default()
    }

    pub fn pv(&self) -> Variation {
        self.response.pv()
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

    use test_log::test;

    use super::*;

    #[test]
    fn test_search_config() {
        let mut search = Search::default();

        let param = Param::new("max_depth", "5");
        assert_eq!(search.max_depth, 0);
        search.set(param).unwrap();
        assert_eq!(search.max_depth, 5);

        let param = Param::new("nmp.enabled", "false");
        assert_eq!(search.nmp.enabled, true);
        search.set(param).unwrap();
        assert_eq!(search.nmp.enabled, false);

        let param = Param::new("lmr.enabled", "false");
        assert_eq!(search.lmr.cfg.enabled, true);
        search.set(param).unwrap();
        assert_eq!(search.lmr.cfg.enabled, false);
    }

    // #[test]
    // fn serde_algo_test() -> Result<()> {
    //     info!("toml\n{}", toml::to_string(&Search::default())?);
    //     Ok(())
    // }

    #[cfg(feature = "nnue")]
    #[test]
    fn test_search_nnue() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("eval".to_string(), "nnue".to_string());
        let mut eng = ThreadedSearch::configure(map).unwrap();
        let res = eng
            .search(Epd::from_board(Board::starting_pos()), TimeControl::Depth(5))
            .unwrap();
        assert_eq!(res.score().unwrap() > Score::from_cp(0), true);
    }
}
