use crate::{
    cache::tt2::TranspositionTable2, engine::Engine, eval::Eval, search::algo::Search,
    version::Version,
};
use anyhow::{anyhow, Result};
use indexmap::map::IndexMap;
use odonata_base::{
    domain::info::Info,
    epd::Epd,
    infra::{
        component::{Component, State},
        config::Config,
        utils::{DurationFormatter, UciString},
        value::Stats,
    },
    prelude::*,
};
use std::{
    collections::HashMap,
    fmt,
    fmt::Debug,
    panic,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use super::search_results::SearchResults;

#[derive(Default)]
pub struct ThreadedSearch {
    pub search:        Search,
    thread_handles:    Vec<JoinHandle<Result<Search>>>,
    settings:          HashMap<String, String>,
    options:           Vec<(String, String)>,
    progress_callback: Option<Arc<Mutex<dyn Fn(&Info) + Send + Sync>>>,
}

const DEFAULT_CONFIG_FILE: &str = "config.toml";

impl Debug for ThreadedSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadedSearch")
            .field("thread_count", &self.search.thread_count)
            .field("thread_handles", &self.thread_handles)
            .field("settings", &self.settings)
            .field("options", &self.options)
            .field("progress_callback", &match self.progress_callback {
                Some(..) => "set",
                _ => "unset",
            })
            .finish()
    }
}

impl Engine for ThreadedSearch {
    fn name(&self) -> String {
        self.search.name()
    }

    fn set_name(&mut self, name: String) {
        self.search.set_name(name);
    }

    fn static_eval(&mut self, pos: Epd) -> anyhow::Result<Score> {
        self.search_with_options(pos, TimeControl::Depth(0), SearchOptions::none())?
            .score()
            .ok_or_else(|| anyhow::anyhow!("failed to get score"))
    }

    fn search_with_options(
        &mut self,
        pos: Epd,
        tc: TimeControl,
        opts: SearchOptions,
    ) -> anyhow::Result<SearchResults> {
        debug!(target: "eng", "-> search on {n}", n = self.name());
        debug!(target: "eng", "-> search on {b} {tc}", b = pos.board());
        self.search
            .controller
            .register_callback(|i| debug!(target: "eng", "<- info {i}"));

        if let TimeControl::DefaultTime = tc {
            let suggested_depth = pos.int("acd").ok_or(anyhow!(
                "tc default specified but position has no depth (acd): {pos}"
            ))? as i32;
            self.search
                .set_timing_method(TimeControl::Depth(suggested_depth));
        } else {
            self.search.set_timing_method(tc);
        }
        self.set_position(pos.clone());
        self.search.restrictions.search_moves = opts.root_moves;
        self.search_sync();
        self.search.results.pos = Some(pos);

        debug!(target: "eng", " <- results {res}", res = self.search.results);
        debug!(target: "eng", " <- game metrics {}", self.search.game_metrics.lock().unwrap());

        Ok(self.search.results.clone())
    }

    fn options(&self) -> IndexMap<String, String> {
        let mut map = IndexMap::new();
        let tc = format!("spin default {} min 1 max 16", self.search.thread_count);
        map.insert("Ponder", "check default false");
        map.insert("Threads", &tc);
        let s = format!("string default {}", UciString::to_uci(""));
        map.insert("Init", &s);

        let mut map: IndexMap<String, String> = map
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        map.extend(self.search.options());
        map
    }

    fn set_option(&mut self, name: &str, value: &str) -> anyhow::Result<()> {
        debug!(target: "eng", "-> set option '{name}' = '{value}'");
        match name {
            "Init" => {
                let Some((name, value)) = value.split_once('=') else {
                    return Err(anyhow!("unable to split '{value}' into key=value"));
                };
                self.settings.insert(name.to_string(), value.to_string());
                self.reconfigure()?
            }

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

    fn start_game(&mut self) -> anyhow::Result<()> {
        self.set_state(State::NewGame);
        Ok(())
    }

    fn metrics(&mut self, filter: &str) -> anyhow::Result<Stats> {
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

    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl ThreadedSearch {
    pub fn configure(settings: HashMap<String, String>) -> Result<Self> {
        Ok(Self {
            search: Self::configure_search(&settings)?,
            settings,
            ..Self::default()
        })
    }

    pub fn configure_search(settings: &HashMap<String, String>) -> Result<Search> {
        let (eval, non_eval) = settings
            .clone()
            .into_iter()
            .partition(|e| e.0.starts_with("eval"));
        let mut search: Search = Config::new()
            .resource("config.toml")
            .props(non_eval)
            .env_var_props("ODONATA")
            .allow_override_files()
            .deserialize()?;
        search.set_name(Version::name_and_version());
        search.eval = Eval::configure(eval)?;
        Ok(search)
    }

    fn reconfigure(&mut self) -> Result<()> {
        self.search = Self::configure_search(&self.settings)?;

        // we drain the options and replay them: set_option re-captures them one at a time
        let options = self.options.drain(..).collect_vec();
        for (k, v) in &options {
            self.set_option(k, v)?;
        }
        Ok(())
    }

    pub fn show_config(&self) -> Result<String> {
        let eng_cfg = format!("{self:#?}");
        let algo_cfg = toml::to_string(&self.search).context("toml::to_string")?;
        let eval_cfg = toml::to_string(&self.search.eval).context("toml::to_string")?;
        Ok(format!(
            "{eng_cfg}\n[algo]\n{algo_cfg}\n[eval]\n{eval_cfg}\n"
        ))
    }

    pub fn new() -> Self {
        Self::configure(HashMap::new()).expect("unable to configure engine")
    }

    pub fn set_position(&mut self, pos: Epd) {
        self.search.set_position(pos);
    }

    pub fn ponder_hit(&mut self) {
        self.search.mte.set_shared_ponder(false);
        self.search.clock.restart_elapsed_search_clock();
    }

    pub fn search_sync(&mut self) {
        // zero threads means in-process on main thread (don't spawn)
        if self.search.thread_count == 0 {
            self.search.controller.progress_callback = self.progress_callback.clone();
            self.search.controller.set_running();
            let mut p = self.search.position.clone();
            self.search.run_search(&mut p);
        } else {
            self.search_start();
            self.wait();
        }
    }

    pub fn set_callback(&mut self, callback: impl Fn(&Info) + Send + Sync + 'static) {
        self.progress_callback = Some(Arc::new(Mutex::new(callback)));
    }

    pub fn search_start(&mut self) {
        for i in 0..self.search.thread_count {
            let builder = thread::Builder::new()
                .name(format!("S{}", i))
                .stack_size(1_000_000);
            let mut search = self.search.clone();
            if !self.search.tt.shared {
                search.tt = TranspositionTable2::default();
                warn!("tt not shared accross threads");
                search.tt.enabled = self.search.tt.enabled;
            }
            search.clock.set_thread_index(i);
            search.move_orderer.thread = i;

            if i == 0 {
                search.controller.progress_callback = self.progress_callback.clone();
                search.controller.set_running();
            }
            if i >= 1 {
                search.max_depth += 8;
                search.controller.progress_callback = None;
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
            trace!(target: "thread", "spawning thread {i} of {}", self.search.thread_count);
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
                search.results.supplied_move().unwrap_or_default().to_string(),
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
            knps_all_threads as u32 / self.search.thread_count,
        );
        // crate::globals::counts::LEGAL_MOVE_COUNT.print_all()        ;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comms::uci_server::UciServer;
    use odonata_base::{catalog::*, infra::utils::Formatting};
    use std::{
        hint::black_box,
        io::stdout,
        time::{self, Duration},
    };
    use test_log::test;

    #[test]
    fn engine_serde_test() {
        let engine1 = ThreadedSearch::new();
        let text1 = toml::Value::try_from(&engine1.search).unwrap();
        println!("toml\n{:#?}", text1);

        toml::to_string(&engine1.search.qs).unwrap();
        let text1 = toml::to_string(&engine1.search).unwrap();
        println!("toml\n{:?}", text1);

        let mut engine2 = ThreadedSearch::new();
        engine2.search = toml::from_str(&text1).unwrap();
        let _text2 = toml::to_string(&engine2.search).unwrap();
        // assert_eq!(text1, text2);

        let engine3 = ThreadedSearch::new();
        let text3 = toml::to_string(&engine3.search).unwrap();
        println!("toml\n{}", text3);
    }

    #[test]
    fn engine_init_test() {
        let engine = ThreadedSearch::new();
        let hce = match &engine.search.eval {
            Eval::Hce(hce) => hce,
            _ => unreachable!(),
        };
        assert_eq!(hce.quantum, 1);
        // println!("{}", toml::to_string(&engine).unwrap());

        let settings = [
            ("eval.quantum".into(), "300".into()),
            ("controller.multi_pv".into(), "6".into()),
        ];
        let engine = ThreadedSearch::configure(settings.into()).unwrap();
        let hce = match &engine.search.eval {
            Eval::Hce(hce) => hce,
            _ => unreachable!(),
        };

        // engine.configment("eval.quantum", "1").unwrap();
        info!("{}", engine);
        assert_eq!(hce.quantum, 300);
        assert_eq!(engine.search.controller.multi_pv, 6);
        // engine.configment("eval.quantum", "2").unwrap();
        // assert_eq!(engine.algo.eval.quantum, 2);
    }

    #[test]
    #[ignore]
    fn test_threading() {
        for &i in [1, 2, 3, 4, 8, 16, 32].iter() {
            let mut eng = ThreadedSearch::new();
            eng.search.set_timing_method(TimeControl::Depth(7));
            eng.search.tt.enabled = true;
            eng.search.thread_count = i;

            let b = Catalog::test_position().board().clone();
            let start = time::Instant::now();
            eng.search.board = b;
            eng.search_sync();
            println!(
                "Time with {i} threads: {}\n\n\n",
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
        assert_eq!(res.supplied_move().unwrap().to_uci(), "b1c2");
    }

    #[ignore]
    #[test]
    fn profile_search() {
        let positions = Catalog::example_game();
        let mut engine = ThreadedSearch::new();
        for _ in 0..1 {
            for pos in &positions {
                let results = engine
                    .search(
                        pos.clone(),
                        TimeControl::SearchTime(Duration::from_millis(30)),
                    )
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
        engine.set_position(pos);
        engine.search.set_timing_method(TimeControl::Depth(8));
        // engine.algo.set_callback(Uci::uci_info);
        engine.search_sync();

        engine
            .search
            .results
            .write_explanation(
                stdout(),
                &engine.search.eval,
                engine.search.position.clone(),
            )
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
        println!("{}", res.to_epd());
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
