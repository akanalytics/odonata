use crate::cache::tt2::TranspositionTable2;
use crate::infra::component::{Component, State, FEATURE};
use crate::infra::metric::Metrics;
use crate::infra::resources::RESOURCE_DIR;
use crate::mv::Move;
use crate::position::Position;
use crate::search::algo::Algo;
use crate::search::timecontrol::TimeControl;
use crate::trace::stat::Stat;
use crate::tune::Tuning;
use crate::infra::utils::Formatting;
use anyhow::{anyhow, Context, Result};
use figment::providers::{Env, Format, Toml};
use figment::value::{Dict, Map};
use figment::{Error, Figment, Metadata, Profile, Provider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::{fmt, mem, panic};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Engine {
    pub shared_tt: bool,
    pub thread_count: u32,
    pub config_filename: String,
    pub feature: bool,

    #[serde(flatten)]
    pub algo: Algo,

    pub tuner: Tuning,


    #[serde(skip)]
    pub engine_init_time: Duration,

    #[serde(skip)]
    pub search_init_time: Duration,

    #[serde(skip)]
    threads: Vec<JoinHandle<Result<Algo>>>,
}

const DEFAULT_CONFIG_FILE: &str = "config.toml";

impl Default for Engine {
    fn default() -> Self {
        Engine {
            config_filename: DEFAULT_CONFIG_FILE.to_string(),
            shared_tt: true,
            feature: false,
            tuner: Tuning::default(),
            algo: Algo::default(),
            engine_init_time: Instant::now().elapsed(),
            search_init_time: Duration::default(),
            thread_count: 1,
            threads: vec![],
        }
    }
}

// impl Clone for Engine {
//     fn clone(&self) -> Self {
//         Self {
//             config_filename: self.config_filename.clone(),
//             threads: Vec::new(),
//             tuner: self.tuner,  //mv
//             algo: self.algo.clone(),
//             ..*self
//         }
//     }
// }

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "config filename  : {}", self.config_filename)?;
        writeln!(f, "threads          : {}", self.thread_count)?;
        writeln!(f, "shared tt        : {}", self.shared_tt)?;
        writeln!(f, "feature          : {}", self.feature)?;
        writeln!(f, "tuner            : {}", self.tuner)?;
        writeln!(
            f,
            "engine init time : {}",
            Formatting::duration(self.engine_init_time)
        )?;
        writeln!(
            f,
            "search init time : {}",
            Formatting::duration(self.search_init_time)
        )?;
        write!(f, "\n[algo]\n{}", self.algo)
    }
}

impl Component for Engine {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame | SetPosition => self.threads.clear(),
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => {}
            Shutdown => {}
        }
        self.algo.set_state(s);
        self.tuner.set_state(s);
    }

    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

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
        // use backtrace::Backtrace;
        // panic::set_hook(Box::new(|panic_info| {
        //     error!("panic occured: {:?}", panic_info.payload().downcast_ref::<String>().unwrap());
        //     error!("{:?}", Backtrace::new());
        //     std::process::exit(1);
        //     // panic!("Panic!!!!")
        // }));

        let toml = RESOURCE_DIR
            .get_file("config.toml")
            .unwrap()
            .contents_utf8()
            .unwrap();

        let toml = Toml::string(toml);
        // let _engine = Self::default();
        // engine.configure(&ParsedConfig::global());
        // let mut engine: Engine = Engine::default();
        let mut eng: Engine = Figment::new()
            .merge(toml)
            //     .merge(Env::prefixed("odonata_var_").split("__"))
            .extract()
            .unwrap();
        eng.configment_many(HashMap::new()).unwrap();
        eng
    }

    pub fn configment(&mut self, key: &str, value: &str) -> Result<()> {
        let mut kvs = HashMap::new();
        kvs.insert(key.to_string(), value.to_string());
        self.configment_many(kvs)
    }

    pub fn configment_many(&mut self, map: HashMap<String, String>) -> Result<()> {
        let mut fig = Figment::new()
            .merge(&*self)
            .merge(Env::prefixed("odonata_var_").split("__"));

        for (k, v) in map.iter() {
            fig = fig.merge(Toml::string(&format!("{} = {}", k, v)));
        }
        let engine: Engine = fig.extract().context(format!("error in {:?}", map))?;
        let mut tuner = Tuning::default();
        // mem::swap(&mut tuner.feature_matrix, &mut self.tuner.feature_matrix);
        mem::swap(&mut tuner.explains, &mut self.tuner.explains);
        // mem::swap(&mut tuner.models_and_outcomes, &mut self.tuner.models_and_outcomes);
        // mem::swap(&mut tuner.boards, &mut self.tuner.boards);
        // mem::swap(&mut tuner.model, &mut self.tuner.model);
        *self = Engine {
            tuner: Tuning {
                // boards: tuner.boards,
                // models_and_outcomes: tuner.models_and_outcomes,
                // feature_matrix: tuner.feature_matrix,
                explains: tuner.explains,
                // model: tuner.model,
                ..engine.tuner
            },
            ..engine
        };
        FEATURE.store(engine.feature, Ordering::SeqCst);        
        // self.algo.eval.populate_feature_weights();
        Ok(())
    }

    pub fn set_position(&mut self, pos: Position) {
        self.algo.set_position(pos);
    }

    pub fn ponder_hit(&mut self) {
        self.algo.mte.set_shared_ponder(false);
        self.algo.clock.restart_elapsed_search_clock();
    }

    pub fn search(&mut self) {
        self.search_start();
        self.wait();
    }

    pub fn search_start(&mut self) {
        // debug!("resize?? {}", self.algo.tt.requires_resize());
        let t = Instant::now();
        for i in 0..self.thread_count {
            let builder = thread::Builder::new()
                .name(format!("S{}", i))
                .stack_size(1_000_000);
            let mut algo = self.algo.clone();
            if !self.shared_tt {
                algo.tt = TranspositionTable2::default();
                warn!("tt not shared accross threads");
                algo.tt.enabled = self.algo.tt.enabled;
            }
            algo.set_thread_index(i);
            algo.move_orderer.thread = i;

            if i == 0 {
                algo.controller.set_running();

            }
            if i >= 1 {
                algo.max_depth += 8;
                algo.controller.progress_callback = None;
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
                let result = panic::catch_unwind(|| {
                    Stat::set_this_thread_index(i as usize);
                    algo.search();
                    Metrics::flush_thread_local();
                    algo
                });
                if let Err(ref error) = result {
                    if let Some(e) = error.downcast_ref::<anyhow::Error>() {
                        error!("Thrae panic returned {:?}", e);
                        error!("Backtrace {:?}", e.source());
                    }
                }
                result.map_err(|e| anyhow!("Anyhow {:?}", e))
            };
            self.threads.push(builder.spawn(cl).unwrap());
        }
        self.search_init_time = t.elapsed();
    }

    pub fn search_stop(&mut self) {
        self.algo.controller.cancel();
        self.wait();
    }

    pub fn wait(&mut self) {
        let mut nodes = 0;
        for (i, t) in self.threads.drain(..).enumerate() {
            let algo = t.join().unwrap();
            let algo = match algo {
                Ok(algo) => algo,
                Err(e) => {
                    warn!("Thread returned {:?}", e); // t.thread().name().unwrap(),
                    panic!("Thread returned {:?}", e);
                }
            };
            debug!(
                "thread {:>3} {:>5} {:>8} {:>10} {:>10} {:>10}   {:<48}",
                i, // thread::current().name().unwrap(),
                algo.results
                    .best_move()
                    .unwrap_or(Move::NULL_MOVE)
                    .to_string(),
                algo.score().to_string(),
                algo.clock.cumul_nodes_this_thread(),
                algo.clock.cumul_knps_this_thread(),
                Formatting::duration(algo.clock.elapsed_search().0),
                algo.pv().to_string(),
            );
            // knps += algo.search_stats.cumulative_knps();
            nodes += algo.clock.cumul_nodes_all_threads();
            if i == 0 {
                self.algo = algo;
                // self.algo.results = algo.results().clone();
                self.algo.controller.cancel();
            }
        }
        let knps = self.algo.clock.cumul_knps_all_threads();
        debug!(
            "{:>3} {:>5} {:>8}        {:>10}      {:>5}     {:5}   {:>48}",
            "", "", "", "---------", "-----", "", "",
        );
        debug!(
            "{:>3} {:>5} {:>8}   nodes{:>10} knps {:>5} (avg knps {})",
            "",
            "",
            "",
            nodes,
            knps,
            knps as u32 / self.thread_count,
        );
        // crate::globals::counts::LEGAL_MOVE_COUNT.print_all()        ;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::UciServer;
    use crate::infra::black_box;
    use crate::infra::utils::Formatting;
    use std::time;
    use test_log::test;

    #[test]
    fn engine_serde_test() {
        let engine1 = Engine::new();
        let text1 = toml::Value::try_from(&engine1).unwrap();
        println!("toml\n{:#?}", text1);

        toml::to_string(&engine1.algo.qs).unwrap();
        let text1 = toml::to_string(&engine1).unwrap();
        eprintln!("toml\n{:?}", text1);


        let engine2: Engine = toml::from_str(&text1).unwrap();
        let _text2 = toml::to_string(&engine2).unwrap();
        // assert_eq!(text1, text2);

        let engine3 = Engine::new();
        let text3 = toml::to_string(&engine3).unwrap();
        eprintln!("toml\n{}", text3);
    }

    #[test]
    fn engine_init_test() {
        let mut engine = Engine::new();
        assert_eq!(engine.algo.eval.quantum, 1);
        eprintln!("{}", toml::to_string(&engine).unwrap());
        engine.configment("eval.quantum", "1").unwrap();
        eprintln!("{}", engine);
        assert_eq!(engine.algo.eval.quantum, 1);
        engine.configment("eval.quantum", "2").unwrap();
        assert_eq!(engine.algo.eval.quantum, 2);
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
                    Formatting::duration(time::Instant::now() - start)
                );
                // println!("\ntt\n{}", eng.algo.tt);
            }
        }
    }

    #[test]
    fn example_search() {
        let pos = Position::parse_epd(
            "k7/8/8/4b3/8/4p3/8/K6N w - - 0 23"
            // "2rqr2k/pp4bp/2np1pp1/5b2/2BP4/2N1B3/PP3PPP/2RQR1K1 w - - 10 17 acd:15;",
        )
        .unwrap();
        
        let mut engine = Engine::new();
        engine.set_position(pos);
        engine.algo.set_timing_method(TimeControl::Depth(15));
        // use crate::Color;
        // engine.algo.set_timing_method(TimeControl::RemainingTime {
        //     our_color: Color::White,
        //     wtime: Duration::from_secs(15),
        //     btime: Duration::from_secs(15),
        //     winc: Duration::ZERO,
        //     binc: Duration::ZERO,
        //     movestogo: 20,
        // });
        engine.algo.set_callback(UciServer::uci_info);
        engine.algo.search();
        println!("{}", engine);
    }

    #[ignore]
    #[test]
    fn profile_search() {
        let positions = Catalog::example_game();
        let mut engine = Engine::new();
        for _ in 0..1 {
            for pos in &positions {
                engine.set_position(pos.clone());
                engine
                    .algo
                    .set_timing_method(TimeControl::SearchTime(Duration::from_millis(30)));
                engine.algo.search();
                let results = engine.algo.results_as_position();
                black_box(results);
                // println!("{}", results);
            }
        }
    }

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
