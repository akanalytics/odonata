use crate::cache::tt2::TranspositionTable2;
use crate::infra::parsed_config::{Component, ParsedConfig};
use crate::position::Position;
use crate::search::algo::Algo;
use crate::search::timecontrol::TimeControl;
use crate::stat::Stat;
use crate::utils::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Engine {
    pub shared_tt: bool,
    pub thread_count: u32,
    pub config_filename: String,
    #[serde(flatten)]
    pub algo: Algo,

    #[serde(skip)]
    pub engine_init_time: Duration,
    #[serde(skip)]
    pub search_init_time: Duration,
    #[serde(skip)]
    threads: Vec<JoinHandle<Algo>>,
}

const DEFAULT_CONFIG_FILE: &'static str = "config.toml";

impl Default for Engine {
    fn default() -> Self {
        Engine {
            config_filename: DEFAULT_CONFIG_FILE.to_string(),
            shared_tt: true,
            algo: Algo::default(),
            engine_init_time: Instant::now().elapsed(),
            search_init_time: Duration::default(),
            thread_count: 1,
            threads: vec![],
        }
    }
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Self {
            config_filename: self.config_filename.clone(),
            threads: Vec::new(),
            algo: self.algo.clone(),
            ..*self
        }
    }
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "config filename  : {}", self.config_filename)?;
        writeln!(f, "threads          : {}", self.thread_count)?;
        writeln!(f, "shared tt        : {}", self.shared_tt)?;
        writeln!(
            f,
            "engine init time : {}",
            Formatter::format_duration(self.engine_init_time)
        )?;
        writeln!(
            f,
            "search init time : {}",
            Formatter::format_duration(self.search_init_time)
        )?;
        write!(f, "\n[algo]\n{}", self.algo)
    }
}

impl Component for Engine {
    fn settings(&self, c: &mut ParsedConfig) {
        c.set(
            "Threads",
            &format!("type spin default {} min 1 max 16", self.thread_count),
        );
        c.set(
            "ParsedConfig file",
            &format!("type string default {}", self.config_filename),
        );
        self.algo.settings(c);
    }
    fn configure(&mut self, c: &ParsedConfig) {
        info!("engine.configure");
        self.thread_count = c.int("Threads").unwrap_or(self.thread_count.into()) as u32;
        self.config_filename = c
            .string("ParsedConfig file")
            .unwrap_or(self.config_filename.clone());
        if self.config_filename != DEFAULT_CONFIG_FILE {
            if let Ok(config) = &ParsedConfig::read_from_file(&self.config_filename) {
                // FIXME: HOW to handle file error?
                self.algo.configure(config);
                return;
            }
        }
        self.algo.configure(c);
    }

    // clears evaluation and transposition caches as well as repetition counts
    fn new_game(&mut self) {
        self.threads.clear();
        self.algo.new_game();
    }

    fn new_position(&mut self) {
        self.threads.clear();
        self.algo.new_position();
    }
}

use crate::infra::resources::RESOURCE_DIR;
use figment::providers::Env;
use figment::providers::{Format, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Error, Figment, Metadata, Profile, Provider};

impl Provider for Engine {
    fn metadata(&self) -> Metadata {
        Metadata::named("Engine default config")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        figment::providers::Serialized::defaults(self).data()
    }

    fn profile(&self) -> Option<Profile> {
        // Optionally, a profile that's selected by default.
        None
    }
}

impl Engine {
    pub fn new() -> Self {
        let toml = RESOURCE_DIR
            .get_file("figment.toml")
            .unwrap()
            .contents_utf8()
            .unwrap();

        let toml = Toml::string(toml);
        let mut engine = Self::default();
        engine.configure(&ParsedConfig::global());
        let engine: Engine = Figment::new().merge(engine).merge(toml).extract().unwrap();
        engine
    }

    pub fn configment(&self, key: &str, value: &str) -> Result<Self> {
        let engine: Engine = Figment::new()
            .merge(self)
            .merge(Toml::string(&format!("{} = {}", key, value)))
            .extract()?;
        Ok(engine)
    }

    pub fn set_position(&mut self, pos: Position) {
        self.algo.set_position(pos);
    }

    pub fn ponder_hit(&mut self) {
        self.algo.mte.set_shared_ponder(false);
        self.algo.search_stats.restart_clocks();
    }

    pub fn search(&mut self) {
        self.search_start();
        self.wait();
    }

    pub fn search_start(&mut self) {
        debug!("resize?? {}", self.algo.tt.requires_resize());
        let t = Instant::now();
        for i in 0..self.thread_count {
            let builder = thread::Builder::new().name(format!("S{}", i)).stack_size(800_000);
            let mut algo = self.algo.clone();
            if !self.shared_tt {
                algo.tt = TranspositionTable2::new_with_mb(self.algo.tt.mb);
                warn!("tt not shared accross threads");
                algo.tt.enabled = self.algo.tt.enabled;
            }
            algo.move_orderer.thread = i;

            if i >= 1 {
                algo.max_depth += 8;
                algo.task_control.progress_callback = None;
                algo.set_timing_method(TimeControl::Infinite);
            }
            if i == 1 {
                algo.ids.step_size = 2;
            }
            if i == 2 {
                algo.ids.step_size = 3;
            }
            if i == 3 {
                algo.ids.step_size = 2;
                algo.ids.start_ply = 2;
            }
            let cl = move || {
                Stat::set_this_thread_index(i as usize);
                algo.search_iteratively();
                algo
            };
            self.threads.push(builder.spawn(cl).unwrap());
        }
        self.search_init_time = t.elapsed();
    }

    pub fn search_stop(&mut self) {
        self.algo.task_control.cancel();
        self.algo.search_stats.user_cancelled = true;
        self.wait();
    }

    pub fn wait(&mut self) {
        let mut nodes = 0;
        for (i, t) in self.threads.drain(..).enumerate() {
            let algo = t.join().unwrap();
            debug!("Thread returned {}", algo); // t.thread().name().unwrap(),
            info!(
                "thread {:>3} {:>5} {:>8} {:>10} {:>10} {:>10}   {:<48}",
                i, // thread::current().name().unwrap(),
                algo.bm().to_string(),
                algo.score().to_string(),
                algo.search_stats.cumulative().all_nodes(),
                algo.search_stats.cumulative_knps(),
                Formatter::format_duration(algo.search_stats.cumulative().elapsed),
                algo.pv().to_string(),
            );
            // knps += algo.search_stats.cumulative_knps();
            nodes += algo.search_stats.cumulative().all_nodes();
            if i == 0 {
                self.algo = algo;
                // self.algo.results = algo.results().clone();
                self.algo.task_control.cancel();
            }
        }
        let knps = self.algo.search_stats().all_threads_cumulative_knps();
        info!(
            "{:>3} {:>5} {:>8}        {:>10}      {:>5}     {:5}   {:>48}",
            "", "", "", "---------", "-----", "", "",
        );
        info!(
            "{:>3} {:>5} {:>8}   nodes{:>10} knps {:>5} (avg knps {})",
            "",
            "",
            "",
            nodes,
            knps,
            knps as u32 / self.thread_count,
        );
        debug!("\n\n\n=====Search completed=====\n{}", self);
        // crate::globals::counts::LEGAL_MOVE_COUNT.print_all()        ;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::*;
    use crate::clock::Clock;
    use crate::comms::uci::Uci;
    use crate::eval::eval::*;
    use crate::eval::score::*;
    use crate::types::*;
    use crate::utils::Formatter;
    use std::time;
    use test_env_log;

    #[test]
    fn engine_serde_test() {
        eprintln!("toml\n{}", toml::to_string(&Engine::default()).unwrap());
    }

    #[test]
    fn engine_init_test() {
        let mut engine = Engine::new();
        assert_eq!(engine.algo.eval.position, true);
        eprintln!("{}", engine);
        engine = engine.configment("eval.position", "false").unwrap();
        eprintln!("{}", engine);
        assert_eq!(engine.algo.eval.position, false);
        assert_eq!(engine.algo.eval.safety, true);
        engine = engine.configment("eval.safety", "true").unwrap();
        assert_eq!(engine.algo.eval.safety, true);
        assert!(engine.configment("eval1.safety", "true").is_err());
    }

    #[test]
    #[ignore]
    fn test_threading() {
        for &i in [1, 2, 3, 4, 8, 16, 32].iter() {
            for &shared in &[true] {
                let mut eng = Engine::new();
                eng.algo.set_timing_method(TimeControl::Depth(7));
                eng.algo.tt.enabled = true;
                eng.shared_tt = shared;
                eng.thread_count = i;

                let b = Catalog::test_position().board().clone();
                let start = time::Instant::now();
                eng.algo.board = b;
                eng.search();
                println!(
                    "Time with {} threads (shared:{}): {}\n\n\n",
                    i,
                    shared,
                    Formatter::format_duration(time::Instant::now() - start)
                );
                // println!("\ntt\n{}", eng.algo.tt);
            }
        }
    }

    #[test]
    #[ignore]
    fn example_search() {
        let pos = Catalog::test_position();
        let mut engine = Engine::new();
        engine.set_position(pos);
        engine.algo.set_timing_method(TimeControl::Depth(8));
        engine.algo.set_callback(Uci::uci_info);
        engine.search();
        println!("{}", engine);
    }

    #[test]
    fn test_mate_in_2_ids() {
        for &id in &[true, false] {
            let position = Catalog::mate_in_2()[0].clone();
            let eval = SimpleScorer::new().set_position(false);
            let mut engine = Engine::new();
            engine
                .algo
                .set_timing_method(TimeControl::Depth(3))
                .set_eval(eval)
                .set_callback(Uci::uci_info)
                .build();
            engine.set_position(position.clone());
            assert_eq!(engine.algo.repetition.prior_positions(), 1);
            engine.search();
            println!("{}", engine);
            if id {
                assert!(
                    engine.algo.search_stats().total().all_nodes() < 22500,
                    "nodes {} > 22500",
                    engine.algo.search_stats().total().all_nodes()
                ); // with piece mob

            // previous
            // assert_eq!(engine.algo.search_stats().total().nodes(), 3456); // with pawn promo
            // assert_eq!(engine.algo.search_stats().total().nodes(), 3885); // with gen qsearch
            // with sq q qsearch
            // assert_eq!(engine.algo.search_stats().total().nodes(), 2108);  // with ordering pv + mvvlva
            // assert_eq!(engine.algo.search_stats().total().nodes(), 3560);
            // assert_eq!(engine.algo.search_stats().total().nodes(), 6553);  // with ordering pv
            // assert_eq!(engine.algo.search_stats().total().nodes(), 6740);
            } else {
                // assert!(engine.algo.search_stats().total().nodes() < 5232); // with piece mob

                // previous
                // assert_eq!(engine.algo.search_stats().total().nodes(), 3456); // with pawn promos
                // assert_eq!(engine.algo.search_stats().total().nodes(), 3885); // with sq qsearch
                // assert_eq!(engine.algo.search_stats().total().nodes(), 2200); // with sq qsearch
                // assert_eq!(engine.algo.search_stats().total().nodes(), 2108); // with  mvvlva
                //assert_eq!(engine.algo.search_stats().total().nodes(), 7749); // no ids no mvvlva
            }
            assert_eq!(engine.algo.pv_table.extract_pv(), position.pv().unwrap());
            assert_eq!(engine.algo.score(), Score::white_win(3));
            assert_eq!(engine.algo.repetition.prior_positions(), 1);
            println!("{}", engine.algo.results());
        }
    }
}
