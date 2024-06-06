use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::{fmt, panic};

use anyhow::anyhow;
use indexmap::map::IndexMap;
use odonata_base::domain::info::Info;
use odonata_base::epd::Epd;
use odonata_base::infra::component::{Component, State};
use odonata_base::infra::utils::{DurationFormatter, UciString};
use odonata_base::infra::value::Stats;
use odonata_base::infra::version::Version;
use odonata_base::prelude::*;

use super::algo::Callback;
use super::search_results::Response;
use crate::cache::tt2::TranspositionTable2;
use crate::search::algo::Search;

#[derive(Debug)]
pub struct ThreadedSearch {
    pub search:       Search,
    pub thread_count: u32,
    engine_name:      String,
    thread_handles:   Vec<JoinHandle<Result<Search>>>,
    settings:         HashMap<String, String>,
    options:          Vec<(String, String)>,
}

impl Default for ThreadedSearch {
    fn default() -> Self {
        Self {
            search:         Search::default(),
            thread_count:   0,
            engine_name:    Version::name_and_version(),
            settings:       HashMap::new(),
            thread_handles: Vec::new(),
            options:        Vec::new(),
        }
    }
}

impl ThreadedSearch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threads(thread_count: u32) -> Self {
        Self {
            thread_count,
            ..Self::default()
        }
    }

    pub fn configure(&mut self, settings: HashMap<String, String>) -> Result<()> {
        for (k, v) in settings.iter() {
            let modified = self.search.set(Param::new(k, v))?;
            if !modified {
                anyhow::bail!("setting {k} = {v} failed as no matching keys");
            }
        }
        Ok(())
    }

    pub fn name(&self) -> String {
        self.engine_name.clone()
    }

    pub fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            search:         self.search.clone(),
            thread_count:   self.thread_count,
            engine_name:    self.engine_name.clone(),
            thread_handles: vec![], // dont clone running threads
            settings:       self.settings.clone(),
            options:        self.options.clone(),
        })
    }

    pub fn set_name(&mut self, name: String) {
        self.engine_name = name;
    }

    pub fn search(&mut self, pos: Epd, tc: TimeControl) -> Result<Response> {
        self.search_with_options(pos, tc, SearchOptions::none())
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        if let Some(features) = self.options().get("Features") {
            let features = features
                .trim_start()
                .trim_start_matches("string")
                .trim_start()
                .trim_start_matches("default")
                .trim();
            features.replace(['[', ']'], ",").split(',').any(|w| w == feature)
        } else {
            false
        }
    }

    pub fn qsearch(&mut self, pos: Epd) -> anyhow::Result<Response> {
        self.search_with_options(pos, TimeControl::Depth(0), SearchOptions::none())
    }

    pub fn static_eval(&mut self, pos: Epd) -> anyhow::Result<Score> {
        self.search_with_options(pos, TimeControl::Depth(0), SearchOptions::none())?
            .score()
            .ok_or_else(|| anyhow::anyhow!("failed to get score"))
    }

    pub fn search_with_options(&mut self, epd: Epd, tc: TimeControl, opts: SearchOptions) -> anyhow::Result<Response> {
        debug!(target: "eng", "-> search on {n}", n = self.name());
        debug!(target: "eng", "-> search on {b} {tc}", b = epd.board());
        self.search
            .controller
            .register_callback(|i| debug!(target: "eng", "<- info {i}"));

        if let TimeControl::DefaultTime = tc {
            let suggested_depth = epd
                .int("acd")
                .ok_or(anyhow!("tc default specified but position has no depth (acd): {epd}"))?
                as i32;
            self.search.set_timing_method(TimeControl::Depth(suggested_depth));
        } else {
            self.search.set_timing_method(tc);
        }
        self.set_position(epd.clone());
        self.search.restrictions.search_moves = opts.root_moves;
        self.search_sync();

        debug!(target: "eng", " <- results {res}", res = self.search.response);
        debug!(target: "eng", " <- game metrics {}", self.search.game_metrics.lock().unwrap());

        Ok(self.search.response.clone())
    }

    pub fn options(&self) -> IndexMap<String, String> {
        let mut map = IndexMap::new();
        let tc = format!("spin default {} min 1 max 16", self.thread_count);
        map.insert("Ponder", "check default false");
        map.insert("Threads", &tc);
        let s = format!("string default {}", UciString::to_uci(""));
        map.insert("Init", &s);

        let mut map: IndexMap<String, String> = map.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        map.extend(self.search.options());
        map
    }

    pub fn set_option(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        debug!(target: "eng", "-> trying threaded-search set option('{name}' = '{value}')");
        match name {
            "Init" => {
                let Some((key, value)) = value.split_once('=') else {
                    return Err(anyhow!("unable to split Init '{value}' into key=value"));
                };
                let modified = self.search.set(Param::new(key, value))?;
                anyhow::ensure!(modified, "unable to set {key} = {value}"); // goes to UI
            }
            "Threads" => self.thread_count = value.parse()?,

            _ => {
                if self.search.options().contains_key(name) {
                    self.options.push((name.to_string(), value.to_string()));
                    self.search.set_option(name, value)?;
                } else {
                    anyhow::bail!("ThreadedSearch does not support set option '{name}'");
                }
            }
        };
        Ok(())
    }

    pub fn start_game(&mut self) -> anyhow::Result<()> {
        self.set_state(State::NewGame);
        Ok(())
    }

    pub fn metrics(&mut self, filter: &str) -> anyhow::Result<Stats> {
        // TODO! sum metrics accross threads
        self.search.metrics(filter)
    }
}

impl fmt::Display for ThreadedSearch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:#?}")
    }
}

impl ThreadedSearch {
    fn fmt_metrics(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n[algo]\n{}", self.search)?;
        Ok(())
    }

    pub fn display_metrics(&self) -> impl fmt::Display + '_ {
        Displayable(|f| self.fmt_metrics(f))
    }
}

impl Component for ThreadedSearch {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame | SetPosition => self.thread_handles.clear(),
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => {}
            Shutdown => {}
        }
        self.search.set_state(s);
    }

    fn new_game(&mut self) {
        self.set_state(State::NewGame)
    }

    fn new_position(&mut self) {
        self.set_state(State::SetPosition)
    }
}

impl ThreadedSearch {
    pub fn show_config(&self) -> Result<String> {
        let eng_cfg = format!("{self:#?}");
        Ok(format!("{eng_cfg}\n"))
    }

    pub fn set_position(&mut self, pos: Epd) {
        self.search.set_position(pos);
    }

    pub fn ponder_hit(&mut self) {
        self.search.mte.set_shared_ponder(false);
        self.search.clock.restart_elapsed_search_clock();
    }

    fn search_sync(&mut self) {
        // zero threads means in-process on main thread (don't spawn)
        if self.thread_count == 0 {
            self.search.controller.progress_callback = self.search.callback.clone();
            self.search.controller.set_running();
            let mut p = self.search.position.clone();
            self.search.run_search(&mut p);
        } else {
            self.search_start();
            self.wait();
        }
    }

    pub fn set_callback(&mut self, callback: impl Fn(&Info) + Send + Sync + 'static) {
        self.search.callback = Callback(Arc::new(callback));
    }

    pub fn search_start(&mut self) {
        for i in 0..self.thread_count {
            let builder = thread::Builder::new()
                .name(format!("S{}-{i}", self.name()))
                .stack_size(1_000_000);
            let mut search = self.search.clone();
            if !self.search.tt.shared {
                search.tt = TranspositionTable2::default();
                warn!("tt not shared accross threads");
                search.tt.enabled = self.search.tt.enabled;
            }
            search.clock.thread_index = i;
            search.move_orderer.thread = i;

            if i == 0 {
                search.controller.progress_callback = self.search.callback.clone();
                search.controller.set_running();
            }
            if i >= 1 {
                search.max_depth += 8;
                search.controller.progress_callback = Callback::default();
                search.set_timing_method(TimeControl::Infinite);
            }
            if i == 1 {
                search.ids.step_size = 2;
            }
            if i == 2 {
                search.ids.step_size = 3;
            }
            if i == 3 {
                search.ids.step_size = 2;
                search.ids.start_ply = 2;
            }
            let cl = move || {
                let mut p = search.position.clone();
                search.run_search(&mut p);
                Ok(search)
            };
            self.thread_handles.push(builder.spawn(cl).unwrap());
            trace!(target: "thread", "spawning thread {i} of {}", self.thread_count);
        }
    }

    pub fn search_stop(&mut self) {
        self.search.controller.cancel();
        self.wait();
    }

    pub fn wait(&mut self) {
        let mut nodes_all_threads = 0;
        debug!(target: "thread",
        "thread {:>3} {:>5} {:>8} {:>10} {:>10} {:>10} {:>10}   {:<48}",
        "id", // thread::current().name().unwrap(),
        "bm",
        "cp",
        "nodes/thrd",
        "nps/thrd",
        "nps/all",
        "ms",
        "pv");

        for (i, t) in self.thread_handles.drain(..).enumerate() {
            let search = t.join().unwrap();
            let search = match search {
                Ok(algo) => algo,
                Err(e) => {
                    warn!("Thread returned {:?}", e); // t.thread().name().unwrap(),
                    panic!("Thread returned {:?}", e);
                }
            };
            debug!(target: "thread",
                "thread {:>3} {:>5} {:>8} {:>10} {:>10} {:>10} {:>10}   {:<48}",
                search.clock.thread_index, // thread::current().name().unwrap(),
                search.response.supplied_move().unwrap_or_default().to_string(),
                search.score().to_string(),
                search.clock.cumul_nodes_this_thread(),
                search.clock.cumul_knps_this_thread(),
                search.clock.cumul_knps_all_threads(),
                search.clock.elapsed_search().time.human(),
                search.pv().to_string(),
            );
            // knps += algo.search_stats.cumulative_knps();
            nodes_all_threads += search.clock.cumul_nodes_all_threads();
            if i == 0 {
                self.search = search;
                // self.algo.results = algo.results().clone();
                self.search.controller.cancel();
            }
        }
        let knps_all_threads = self.search.clock.cumul_knps_all_threads();
        trace!(target: "thread", "{:#?}",self.search.clock.state);
        debug!(target: "thread",
            "{:>3} {:>5} {:>8}        {:>10}      {:>5}     {:5}   {:>48}",
            "", "", "", "---------", "-----", "", "",
        );
        debug!(target: "thread",
            "{:>3} {:>5} {:>8}   nodes{:>10} knps {:>5} (avg knps {})",
            "",
            "",
            "",
            nodes_all_threads,
            knps_all_threads,
            knps_all_threads as u32 / self.thread_count,
        );
        // crate::globals::counts::LEGAL_MOVE_COUNT.print_all()        ;
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::io::stdout;
    use std::time::{self, Duration};

    use odonata_base::catalog::*;
    use odonata_base::infra::metric::MetricsRegistry;
    use odonata_base::infra::utils::Formatting;
    use pretty_assertions::assert_eq;
    use test_log::test;

    use super::*;
    use crate::comms::uci_server::UciServer;

    #[test]
    fn engine_new_game_test() {
        let mut eng1 = ThreadedSearch::new();
        let mut eng2 = ThreadedSearch::new();
        let d1 = format!("{eng1:#?}");
        let d2 = format!("{eng2:#?}");
        assert_eq!(d1, d2);
        assert_eq!(eng1.search.to_string(), eng2.search.to_string());
        eng2.search(Epd::starting_pos(), TimeControl::NodeCount(1000)).unwrap();
        eng2.new_game();
        eng1.new_game(); // force tt resize
        assert_eq!(eng1.search.to_string(), eng2.search.to_string());
        assert_eq!(eng1.to_string(), eng2.to_string());
    }

    #[test]
    fn engine_init_test() {
        let engine = ThreadedSearch::new();
        assert_eq!(engine.search.eval.hce.quantum, 1);
        // println!("{}", toml::to_string(&engine).unwrap());

        let settings = [
            ("eval.hce.quantum".into(), "300".into()),
            ("controller.multi_pv".into(), "6".into()),
        ]
        .into();
        let mut engine = ThreadedSearch::new();
        engine.configure(settings).unwrap();

        info!("{}", engine);
        assert_eq!(engine.search.eval.hce.quantum, 300);
        assert_eq!(engine.search.controller.multi_pv, 6);
        // engine.configment("eval.quantum", "2").unwrap();
        // assert_eq!(engine.algo.eval.quantum, 2);
    }

    #[test]
    fn test_threading() {
        for i in [1, 2, 3, 4, 8, 16, 32].into_iter() {
            let mut eng = ThreadedSearch::with_threads(i);

            let epd = Catalog::test_position();
            let start = time::Instant::now();
            eng.search(epd, TimeControl::Depth(7)).unwrap();
            println!(
                "Time with {i} threads: {}",
                Formatting::duration(time::Instant::now() - start)
            );
            // println!("\ntt\n{}", eng.algo.tt);
        }
    }

    #[test]
    fn example_search() {
        let epd = Epd::parse_epd("k7/8/8/4b3/8/4p3/8/1K5N w - - 0 23").unwrap();
        let mut engine = ThreadedSearch::new();
        engine.search.set_callback(UciServer::uci_info);
        let res = engine.search(epd, TimeControl::Depth(13)).unwrap();
        println!("{res}");
        let mvs = ["b1c2", "b1c1"];
        assert!(mvs.contains(&res.supplied_move().unwrap().to_uci().as_str()));
    }

    #[ignore]
    #[test]
    fn profile_search() {
        let positions = Catalog::example_game();
        let mut engine = ThreadedSearch::new();
        for _ in 0..1 {
            for pos in &positions {
                let results = engine
                    .search(pos.clone(), TimeControl::SearchTime(Duration::from_millis(30)))
                    .unwrap();
                black_box(results);
                // println!("{}", results);
            }
        }
    }

    #[test]
    fn test_explain_results() {
        let pos = Catalog::test_position();
        let mut engine = ThreadedSearch::new();
        // engine.algo.set_callback(Uci::uci_info);
        let res = engine.search(pos, TimeControl::Depth(8)).unwrap();

        res.write_explanation(stdout(), &engine.search.eval, engine.search.position.clone())
            .unwrap()
    }

    #[test]
    fn test_rep_bug1() {
        let s = concat!(
            "e2e4 b8c6 b1c3 e7e5 g1f3 g8f6 d2d4 e5d4 f3d4 f8b4 c1g5 d8e7 f2f3 e8g8 ",
            "d4c6 b4c3 b2c3 d7c6 d1d2 h7h6 g5e3 c8e6 f1d3 e7a3 e3f4 a3a5 c3c4 a5d2 e1d2 f8c8 d2c1 ",
            "f6d7 c1b1 c6c5 f4e3 d7e5 f3f4 e5d3 c2d3 c8d8 b1c2 b7b6 h1d1 e6g4 d1d2 d8e8 a1e1 a8d8 ",
            "f4f5 a7a6 h2h3 g4f5 e4f5 e8e5 d2f2 d8d6 f2f3 b6b5 c4b5 a6b5 e1c1 d6d8 c1h1 d8d7 h1d1 ",
            "d7d5 g2g4 e5e8 c2b1 d5e5 e3f2 e8a8 f3e3 e5e3 f2e3 c5c4 e3f4 a8a3 d3c4 b5c4 f4c7 a3h3 ",
            "d1c1 h6h5 c1c4 h3h1 b1b2 h1h4 c7e5 h4g4 c4c8 g8h7 c8c7 h7g8 c7c8"
        );
        let b = Catalog::starting_board();
        let mut eng = ThreadedSearch::new();
        eng.search.set_callback(UciServer::uci_info);
        eng.search.repetition.new_game();
        let played = b.parse_uci_variation(s).unwrap();
        let epd = Epd::from_var(b, played);
        // for mv in var.moves() {
        //     eng.search.repetition.push_move(mv, &b);
        //     b = b.make_move(mv);
        // }
        // eng.search.set_position(Epd::from_board(b));
        // // eng.algo.explainer.enabled = true;
        // eng.algo.explainer.add_variation_to_explain(Variation::new());
        let res = eng.search(epd, TimeControl::Depth(5)).unwrap();
        println!("{}", res.to_results_epd());
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
        let mvs = ["e7e5", "c7c5", "g8f6", "d7d5"];
        let sm = sr.unwrap().supplied_move().unwrap().to_uci();
        assert!(mvs.contains(&sm.as_str()), "{sm}");
    }

    #[test]
    fn test_algo_as_engine() {
        let board = Catalog::starting_board();
        let mut eng = ThreadedSearch::new();
        assert_eq!(eng.options().contains_key("MultiPV"), true);
        assert!(eng.set_option("UCI_AnalyseMode", "False").is_err());
        assert!(eng.set_option("UCI_AnalyseMode", "true").is_ok());
        assert!(eng.set_option("UCI_AnalyseMode", "false").is_ok());
        let tc = TimeControl::Depth(1);
        let sr = eng.search(Epd::from_board(board), tc).unwrap();
        let moves = ["e2e4", "g1f3", "g1h3", "f2f3", "b1c3", "d2d3", "b1a3", "d2d4", "a2a3"];
        let bm = sr.supplied_move().unwrap().to_uci();
        assert!(moves.contains(&bm.as_str()), "{bm}");
    }

    #[test]
    fn jons_chess_problem() {
        let pos = Epd::parse_epd("2r2k2/5pp1/3p1b1p/2qPpP2/1p2B2P/pP3P2/2P1R3/2KRQ3 b - - 0 1").unwrap();
        let mut search = ThreadedSearch::new();
        search.search.set_callback(UciServer::uci_info);
        let sr = search.search(pos.clone(), TimeControl::Depth(12)).unwrap();
        println!("{}", search.search.eval);
        println!("{}", sr.to_results_epd());
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
        let pos06 = Epd::parse_epd("b1q1r1kr/ppppbppp/2nnp3/4N3/3P4/2N1P3/PPP2PPP/BQ2RBKR w - - 2 6")?;
        let pos07 = Epd::parse_epd("b2qr1kr/ppppbppp/2nnp3/4N3/3P4/2NBP3/PPP2PPP/BQ2R1KR w - - 4 7")?;
        let pos08 = Epd::parse_epd("b2qr1kr/pppp1ppp/2nnpb2/4N3/3P4/2NBP3/PPP2PPP/B2QR1KR w - - 6 8")?;
        let pos09 = Epd::parse_epd("b2qr1kr/ppppbppp/2nnp3/8/3P2N1/2NBP3/PPP2PPP/B2QR1KR w - - 8 9")?;
        let pos10 = Epd::parse_epd("b2qr1kr/pppp1ppp/2nnp3/6b1/3P2N1/2N1P3/PPP1BPPP/B2QR1KR w - - 10 10")?;
        let pos11 = Epd::parse_epd("b3r1kr/ppppqppp/2nnp3/6b1/3PP1N1/2N5/PPP1BPPP/B2QR1KR w - - 1 11")?;
        let pos12 = Epd::parse_epd("b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12")?;
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
            let res = eng.search(p.clone(), tc).unwrap();
            let pv1 = res.pv();
            eng.search.tt.current_age -= 1;
            println!("{:<40} - {}", pv1.to_uci(), res.to_results_epd());

            let tc = TimeControl::Depth(7);
            eng.search.tt.allow_truncated_pv = true;
            let res = eng.search(p.clone(), tc).unwrap();
            let pv2 = res.pv();
            println!("{:<40} - {}", pv2.to_uci(), res.to_results_epd());

            let tc = TimeControl::Depth(7);
            eng.search.tt.allow_truncated_pv = false;
            let res = eng.search(p.clone(), tc).unwrap();
            let pv3 = res.pv();
            println!("{:<40} - {}\n", pv3.to_uci(), res.to_results_epd());

            // assert_eq!(pv1, pv2, "{}", p );
        }
    }

    // #[test]
    // #[ignore]
    // fn test_rep_bug2() {
    //     let mut engine = ThreadedSearch::new();

    //     // engine
    //     //     .algo
    //     //     .eval
    //     //     .set_weight(Attr::ContemptPenalty.into(), Weight::from_i32(-30, -30));

    //     let epd3 = Catalog::draws()[3].clone();

    //     let board = epd3.board().make_moves_old(&epd3.var("sv").unwrap());
    //     engine.new_game();
    //     engine.set_position(epd3.clone());
    //     print!(
    //         "\n\npos3 = {}\nboard={}\nhash={}\nrep=\n{}\nRepeats{:?}\n",
    //         epd3,
    //         board,
    //         board.hash(),
    //         engine.search.repetition,
    //         engine.search.repetition.count_old(&board)
    //     );
    //     println!("rep = {:?}", engine.search.repetition);
    //     engine.search_sync();
    //     let res = engine.search.results_as_position();
    //     println!("res3: {}", res);

    //     let mut pos2 = epd3.clone();
    //     let mut var2 = epd3.var("sv").unwrap().clone();
    //     let len = var2.len();
    //     var2 = var2.take(len - 4);
    //     pos2.set_tag("sv", &var2.to_san(&pos2.board()));
    //     engine.new_game();
    //     engine.set_position(pos2.clone());
    //     print!(
    //         "\n\npos2 = {}\nboard={}\nhash={}\nrep=\n{}\nRepeats{:?}\n",
    //         pos2,
    //         board,
    //         board.hash(),
    //         engine.search.repetition,
    //         engine.search.repetition.count_old(&board)
    //     );
    //     println!("rep = {:?}", engine.search.repetition);
    //     engine.search_sync();
    //     let res = engine.search.results_as_position();
    //     println!("res2: {}", res);

    //     let mut pos1 = epd3.clone();
    //     let mut var1 = epd3.var("sv").unwrap().clone();
    //     let len = var1.len();
    //     var1 = var1.take(len - 8);
    //     pos1.set_tag("sv", &var1.to_san(&pos1.board()));
    //     engine.new_game();
    //     engine.set_position(pos1.clone());
    //     print!(
    //         "\n\npos1 = {}\nboard={}\nhash={}\nrep=\n{}\nRepeats{:?}\n",
    //         pos1,
    //         board,
    //         board.hash(),
    //         engine.search.repetition,
    //         engine.search.repetition.count_old(&board)
    //     );
    //     println!("rep = {:?}", engine.search.repetition);
    //     engine.search_sync();
    //     let res = engine.search.results_as_position();
    //     println!("res1: {}", res);
    // }

    // #[test]
    // fn test_mate_in_2_ids() {
    //     for &id in &[true, false] {
    //         let position = Catalog::mate_in_2()[0].clone();
    //         let mut engine = Engine::new();
    //         engine.set_position(position.clone());
    //         let eval = Eval::new();

    //         engine
    //             .algo
    //             .set_timing_method(TimeControl::Depth(3))
    //             .set_eval(eval)
    //             .set_callback(Uci::uci_info);
    //         assert_eq!(engine.algo.repetition.prior_positions(), 1);
    //         engine.search();
    //         info!("{}", engine);
    //         if id {
    //             assert!(
    //                 engine.algo.search_stats().iteration().all_nodes() < 22500,
    //                 "nodes {} > 22500",
    //                 engine.algo.search_stats().iteration().all_nodes()
    //             ); // with piece mob

    //         // previous
    //         // assert_eq!(engine.algo.search_stats().total().nodes(), 3456); // with pawn promo
    //         // assert_eq!(engine.algo.search_stats().total().nodes(), 3885); // with gen qsearch
    //         // with sq q qsearch
    //         // assert_eq!(engine.algo.search_stats().total().nodes(), 2108);  // with ordering pv + mvvlva
    //         // assert_eq!(engine.algo.search_stats().total().nodes(), 3560);
    //         // assert_eq!(engine.algo.search_stats().total().nodes(), 6553);  // with ordering pv
    //         // assert_eq!(engine.algo.search_stats().total().nodes(), 6740);
    //         } else {
    //             // assert!(engine.algo.search_stats().total().nodes() < 5232); // with piece mob

    //             // previous
    //             // assert_eq!(engine.algo.search_stats().total().nodes(), 3456); // with pawn promos
    //             // assert_eq!(engine.algo.search_stats().total().nodes(), 3885); // with sq qsearch
    //             // assert_eq!(engine.algo.search_stats().total().nodes(), 2200); // with sq qsearch
    //             // assert_eq!(engine.algo.search_stats().total().nodes(), 2108); // with  mvvlva
    //             //assert_eq!(engine.algo.search_stats().total().nodes(), 7749); // no ids no mvvlva
    //         }
    //         assert_eq!(engine.algo.pv_table.extract_pv(), position.pv().unwrap());
    //         assert_eq!(engine.algo.score(), Score::we_win_in(3));
    //         assert_eq!(engine.algo.repetition.prior_positions(), 1);
    //         info!("{}", engine.algo.results_as_position());
    //     }
    // }
}
