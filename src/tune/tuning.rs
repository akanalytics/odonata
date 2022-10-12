use std::io::Write;

use crate::eval::calc::Calc;
use crate::eval::eval::Attr;
use crate::eval::eval::Feature;
use crate::infra::utils::{win_probability_from_cp_and_k, sigmoid};
use crate::eval::scorer::ExplainScore;
use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::infra::metric::Metrics;
use crate::other::outcome::Outcome;
use crate::position::Position;
use crate::search::engine::AsyncEngine;
use crate::search::node::Timing;
use crate::tags::Tag;
use anyhow::Result;
use bitflags::_core::sync::atomic::AtomicU32;
use bitflags::_core::sync::atomic::Ordering;
use itertools::Itertools;
use rayon::prelude::IndexedParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
// use rayon::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RegressionType {
    LinearOnCp,
    LogisticOnOutcome,
    LogisticOnCp,
    CrossEntropy,
    CumulativeLogisticLink,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Sigmoid {
    Exponential,
    WinProb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Method {
    New,
    Sparse,
    Dense,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tuning {
    pub regression_type: RegressionType,
    pub sigmoid: Sigmoid,
    pub method: Method,
    pub search_depth: i32,
    pub ignore_certain_endgames: bool,
    pub max_eval: Option<f32>,
    pub ignore_likely_endgames: bool,
    pub multi_threading_min_positions: usize,
    pub threads: usize,
    pub ignore_draws: bool,
    pub consolidate: bool,
    pub logistic_steepness_k: Weight,

    #[serde(skip)]
    pub explains: Vec<ExplainScore>,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            regression_type: RegressionType::LogisticOnOutcome,
            sigmoid: Sigmoid::WinProb,
            method: Method::New,
            search_depth: -1,
            ignore_certain_endgames: true,
            ignore_likely_endgames: true,
            max_eval: None,
            multi_threading_min_positions: 20000,
            threads: 32,
            logistic_steepness_k: Weight::from_i32(4, 4),
            explains: Default::default(),
            ignore_draws: false,
            consolidate: false,
        }
    }
}

impl Component for Tuning {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl fmt::Debug for Tuning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Tuning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !f.alternate() {
            writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        }
        // else {
        //     for (_s, name, v) in &self.filter_values(self.feature_vec(0), |s| s.starts_with("")) {
        //         writeln!(f, "{fen} {name} = {v}", fen = self.fens()[0])?;
        //     }
        // }
        Ok(())
    }
}

// fn model_and_accum(eng: &Engine, b: &Board, _p: Phase, scorer: &mut impl ScorerBase) {
//     // let mut model = Model::from_board(b, p, Switches::ALL_SCORING);
//     // model.csv = eng.tuner.sparse;
//     // let mut scorer = ExplainScorer::new(String::new());
//     // eng.algo.eval.predict(&model, scorer);
//     Calc::score(scorer, b, &eng.algo.eval, &eng.algo.eval.phaser)
// }

impl Tuning {
    pub fn new() -> Self {
        Tuning::default()
    }

    pub fn ncols(&self) -> usize {
        2 * Feature::all().len()
    }

    pub fn nrows(&self) -> usize {
        self.explains.len()
    }

    /// free the memory associated with prior tuning runs
    pub fn clear(&mut self) {
        // self.feature_matrix.feature_vectors = Vec::new();
        // self.feature_matrix.feature_names = Vec::new();
        self.explains = Vec::new();
    }

    pub fn fens(&self) -> Vec<String> {
        self.explains
            .iter()
            .map(|ExplainScore { fen, .. }| fen.clone())
            .collect_vec()
    }

    pub fn feature_names(&self) -> Vec<String> {
        let s = Feature::all().into_iter().map(|f| f.name() + ".s");
        let e = Feature::all().into_iter().map(|f| f.name() + ".e");
        s.interleave(e).collect_vec()
    }

    pub fn sparse_feature_vec(&self, idx: usize) -> Vec<(usize, Feature, f32)> {
        let ex = &self.explains[idx];
        let s_w = Weight::from_f32(1., 0.).interpolate(ex.phase);
        let e_w = Weight::from_f32(0., 1.).interpolate(ex.phase);

        let s = ex.values().map(|(f, idx, i)| (2 * idx, f, i as f32 * s_w));
        let e = ex
            .values()
            .map(|(f, idx, i)| (2 * idx + 1, f, i as f32 * e_w));
        s.interleave(e).collect_vec()
    }

    pub fn feature_vec(&self, idx: usize) -> Vec<f32> {
        let mut vec = vec![0.; self.ncols()];
        self.sparse_feature_vec(idx)
            .iter()
            .for_each(|(i, _f, x)| vec[*i] = *x);
        vec
    }

    pub fn upload_positions(eng: &mut AsyncEngine, positions: Vec<Position>) -> Result<usize> {
        let t = Metrics::timing_start();
        let draws = AtomicU32::new(0);
        let likely = AtomicU32::new(0);
        let certain = AtomicU32::new(0);
        let max_evals = AtomicU32::new(0);
        use rayon::iter::ParallelIterator;
        let weights_vec = eng.algo.eval.weights_vector();
        eng.tuner.explains = positions
            .par_iter()
            .enumerate()
            .filter_map(|(_i, pos)| {
                if eng.tuner.ignore_certain_endgames && pos.board().outcome().is_game_over() {
                    trace!("Discarding drawn/checkmate position {}", pos);
                    certain.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                let (_outcome, outcome_str) = eng.tuner.calc_player_win_prob_from_pos(pos);
                let o = Outcome::try_from_pgn(&outcome_str).unwrap();
                if eng.tuner.ignore_draws && outcome_str == "1/2-1/2" {
                    draws.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                let ph = eng.algo.eval.phaser.phase(&pos.board().material());
                let mut explain = ExplainScore::new(ph, 1., pos.board().to_fen());
                Calc::new(&pos.board()).score(&mut explain, pos.board());
                if eng.tuner.ignore_likely_endgames
                    && explain.value(Feature::Discrete(Attr::WinBonus)) != 0
                {
                    likely.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                if let Some(max_eval) = eng.tuner.max_eval {
                    explain.set_weights(weights_vec.clone()); ///////////
                    if explain.total().interpolate(ph).abs() > max_eval {
                        max_evals.fetch_add(1, Ordering::Relaxed);
                        return None;
                    }
                }
                explain.set_outcome(o);
                // eng.algo.eval.predict(&model, &mut w_scorer);
                // let _consolidate = eng.tuner.consolidate;
                explain.discard_balanced_features();
                Some(explain)
            })
            .collect();
        Metrics::profile(t, Timing::TimimgTunerUploadPositions);
        println!(
            "Loaded {} positions ignoring draws {draws}, likely {likely}, certain {certain} and max evals {max_evals}",
            positions.len(),
            draws = draws.load(Ordering::Relaxed),
            likely = likely.load(Ordering::Relaxed),
            certain = certain.load(Ordering::Relaxed),
            max_evals = max_evals.load(Ordering::Relaxed),
        );
        Ok(positions.len())
    }

    pub fn calc_player_win_prob_from_pos(&self, pos: &Position) -> (f32, String) {
        if let Tag::Comment(_n, s) = pos.tag(Tag::C9) {
            let prob = match s.as_str() {
                "1/2-1/2" => 0.5,
                "1-0" => 1.0,
                "0-1" => 0.0,
                prob => panic!("Unexpected match outcome {}", prob),
            };
            return (pos.board().color_us().chooser_wb(prob, prob), s.clone());
        }
        panic!("Unable to find result comment c9 in {}", pos);
    }

    pub fn write_training_data<W: Write>(eng: &mut AsyncEngine, writer: &mut W) -> Result<i32> {
        ExplainScore::write_csv(eng.tuner.explains.iter(), writer)
    }

    pub fn calculate_mean_square_error(&self, eng: &AsyncEngine) -> Result<f32> {
        let eval = &eng.algo.eval;
        let logistic_steepness_k = self.logistic_steepness_k; // so that closure does not capture engine/tuner
                                                              // let mut scorer = ExplainScorer::new(String::new(), true);
                                                              // let board = Catalog::starting_board();
                                                              // model_and_accum(eng, &board, Phase(0), &mut scorer);
                                                              // let weight_vector = scorer.weights_vector();
        let eval_weight_vector = eval.weights_vector();
        // trace!("Weights = {}", weight_vector);
        let regression_type = eng.tuner.regression_type;
        // let mut diff_squared: f32 = 0.0;

        let closure_es = |pair: (usize, &ExplainScore)| {
            let (i, es) = pair;
            // let fv = pair;
            let mut w_score = es.dot_product(&eval_weight_vector).interpolate(es.phase);
            w_score = w_score.clamp(-600.0, 600.0);
            // if w_score > 10000.0 || w_score < -10000.0 {
            //     error!("w_score:{w_score}\n{}\n\n{eval_weight_vector}", es);
            // }
            let k = logistic_steepness_k.interpolate(es.phase) as f32;
            let win_prob_estimate = match self.sigmoid {
                Sigmoid::WinProb => win_probability_from_cp_and_k(w_score, k),
                // Sigmoid::Exponential if w_score > 300.0 => 1.0,
                // Sigmoid::Exponential if w_score < -300.0 => 0.0,
                Sigmoid::Exponential => sigmoid(w_score / 100.0),
            };
            let win_prob_actual = match es.outcome {
                Outcome::WinWhite => 1.0,
                Outcome::WinBlack => 0.0,
                Outcome::DrawRule50 => 0.5,
                _ => unreachable!(),
            };
            let cost = match regression_type {
                RegressionType::LogisticOnOutcome => {
                    let diff = win_prob_estimate - win_prob_actual;
                    diff * diff
                }
                // p is prob of belonging to class 1, 1-p prob of class 0
                // J(x)  = Sum     yi * log(pi) + (1-yi)*log(1-pi)
                // white win:  win_prob_est close to zero => penalize with large positive cost
                RegressionType::CrossEntropy => match es.outcome {
                    Outcome::WinWhite => -f32::ln(win_prob_estimate),
                    Outcome::WinBlack => -f32::ln(1.0 - win_prob_estimate),
                    Outcome::DrawRule50 => 0.0,
                    _ => 0.0,
                },
                RegressionType::CumulativeLogisticLink => match es.outcome {
                    Outcome::WinWhite => -f32::ln(win_prob_estimate),
                    Outcome::WinBlack => -f32::ln(1.0 - win_prob_estimate),
                    Outcome::DrawRule50 => -f32::ln(1.0 - f32::abs(win_prob_estimate - 0.5)),
                    _ => 0.0,
                },
                _ => unreachable!(),
            };
            if cost.is_infinite() || cost.is_nan() {
                error!(
                    "Sparse : i:{i} est:{win_prob_estimate} act:{win_prob_actual} score:{w_score} {}% cost:{cost} {}",
                    es.phase, es.fen
                );
            }
            if cost.is_infinite() || cost.is_nan() {
                panic!("Cost function NaN or infinite")
            } else {
                cost
            }
        };

        // if eng.tuner.method == Method::New {
        let l = self.explains.len();
        info!(
            "Calculating mse (new) on {} positions using single thread",
            l
        );
        let total_diff_squared: f32 = self.explains.iter().enumerate().map(closure_es).sum();
        let mse = total_diff_squared / l as f32;
        info!("Calculated (new) mse as {}", mse);
        Ok(mse)
        // } else {
        //     let total_diff_squared: f32 = match self.feature_matrix.feature_vectors.len() {
        //         0 => bail!("No (sparse) tuning positions loaded or remain after filtering"),
        //         l if l < self.multi_threading_min_positions => {
        //             info!("Calculating mse (sparse) on {} positions using single thread", l);
        //             self.feature_matrix.feature_vectors.iter().enumerate().map(closure_fv).sum()
        //         }
        //         l => {
        //             info!("Calculating mse (sparse) on {} positions using multi thread", l);
        //             self.calc_sparse(closure_fv)
        //         }
        //     };

        //     mse = total_diff_squared / self.feature_matrix.feature_vectors.len() as f32;
        //     info!("Calculated mse as {}", mse);
        //     return Ok(mse);
        // }
    }

    // fn calc_sparse(&self, f: impl Copy + Sync + Send + Fn((usize, &FeatureVector)) -> f32) -> f32 {
    //     info!("Calculating mse (sparse) on positions using several threads");
    //     // use rayon on larger sized files
    //     // self.feature_matrix.feature_vectors.par_iter().enumerate().map(closure).sum()
    //     let v = &self.feature_matrix.feature_vectors;
    //     if self.threads == 0 {
    //         panic!("At least one thread required.");
    //     }
    //     if self.threads > v.len() {
    //         panic!("More threads than items in vector.");
    //     }
    //     if v.len() == 0 {
    //         return 0.0;
    //     }

    //     // divide round up
    //     let items_per_thread = (v.len() - 1) / self.threads + 1;

    //     let arc = Arc::new(v);
    //     // let mut threads = Vec::with_capacity(nb_threads);

    //     // // this channel will be use to send values (partial sums) for threads
    //     // let (sender, receiver) = mpsc::channel::<T>();
    //     let thread_sum = std::sync::Mutex::new(0.0_f32);
    //     let vec = (0..self.threads).collect_vec();
    //     let slice = &vec[..];
    //     rayon::scope(|s| {
    //         for i in slice {
    //             s.spawn(|_s| {
    //                 let data = arc.clone();
    //                 let from = *i * items_per_thread;
    //                 let to = std::cmp::min(from + items_per_thread, data.len());
    //                 let mut sum = 0.0;
    //                 for v in &data[from..to] {
    //                     sum += f((0, v));
    //                 }
    //                 let mut ts = thread_sum.lock().unwrap();
    //                 *ts = *ts + sum;
    //             });
    //         }
    //     });
    //     let x = *thread_sum.lock().unwrap();
    //     x
    // }
}
// let mut sum = 0.0;
// for t in threads {
//     sum += t.join().expect("panic in worker thread");
// }
// sum

// parallel_fold(self.feature_matrix.feature_vectors.clone(), 0.0, closure, 20)

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufWriter, time::Instant};

    use super::*;
    use crate::eval::eval::Attr;
    use crate::infra::utils::{read_file, Formatting};
    use crate::{eval::weight::Weight, infra::profiler::Profiler};
    use anyhow::Context;
    use test_log::test;

    #[test]
    fn tuning_serde_test() {
        let tuner = Tuning::new();
        let text = toml::to_string(&tuner).unwrap();
        info!("{}", text);
    }

    #[test]
    fn test_tuning_load() {
        let mut eng = AsyncEngine::new();
        let file = "../odonata-extras/epd/quiet-labeled-small.epd";
        let _count = Tuning::upload_positions(
            &mut eng,
            Position::parse_many_epd(read_file(file).unwrap()).unwrap(),
        )
        .unwrap();

        println!("{len}", len = eng.tuner.feature_names().len());
        eng.tuner
            .sparse_feature_vec(0)
            .iter()
            .for_each(|x| println!("{x:?}"));

        println!("{vec11} 1.5", vec11 = eng.tuner.feature_vec(0)[11]);
        println!("{fv}", fv = eng.tuner.feature_vec(0).iter().join(":"));
        println!("{tuning:#}", tuning = eng.tuner);
    }

    #[ignore]
    #[test]
    fn test_tuning_mse() {
        info!("Starting...");
        let mut eng = AsyncEngine::new();
        eng.tuner.multi_threading_min_positions = 10000000;

        eng.tuner.regression_type = RegressionType::CrossEntropy;
        eng.tuner.method = Method::Sparse;

        let file = "../odonata-extras/epd/quiet-labeled-small.epd";
        let _count = Tuning::upload_positions(
            &mut eng,
            Position::parse_many_epd(read_file(file).unwrap()).unwrap(),
        )
        .unwrap();

        //tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap();
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");

        let start = Instant::now();
        let mut iters = 0;
        for n in (-120..120).step_by(1) {
            let value = n;
            eng.algo.eval.mb.enabled = false;
            eng
                .algo
                .eval
                .set_weight(Attr::PawnIsolated.into(), Weight::from_i32(0, value));
            iters += 1;
            let diffs = eng.tuner.calculate_mean_square_error(&eng).unwrap();
            println!("{}, {}", value, diffs);
        }
        let time = Instant::now() - start;
        println!(
            "Time {} for {} iters, {} per iter.",
            Formatting::duration(time),
            iters,
            Formatting::duration(time / iters)
        );
    }

    #[ignore]
    #[test]
    fn test_tuning_csv() {
        info!("Starting...");
        let mut eng = AsyncEngine::new();
        eng.tuner.method = Method::New;
        let file = "../odonata-extras/epd/quiet-labeled-small.epd";
        let _count = Tuning::upload_positions(
            &mut eng,
            Position::parse_many_epd(read_file(file).unwrap()).unwrap(),
        )
        .unwrap();
        //tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap();
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");

        let filename = "/tmp/test_training_data.csv";
        let f = File::create(&filename)
            .with_context(|| format!("Failed to open file {}", &filename))
            .unwrap();
        let mut f = BufWriter::new(f);
        let line_count = Tuning::write_training_data(&mut eng, &mut f).unwrap();
        println!(" lines proicessed: {line_count}");
    }

    #[test]
    fn bench_mse() {

        let mut eng1 = AsyncEngine::new();
        eng1.tuner.multi_threading_min_positions = 3_000_000;
        eng1.tuner.method = Method::New;
        let file = "../odonata-extras/epd/quiet-labeled-small.epd";
        let _count = Tuning::upload_positions(
            &mut eng1,
            Position::parse_many_epd(read_file(file).unwrap()).unwrap(),
        )
        .unwrap();
        let mut prof1 = Profiler::new("mse new".into());
        prof1.start();
        let diffs1 = eng1.tuner.calculate_mean_square_error(&eng1).unwrap();
        prof1.stop();
        println!("MSE {diffs1}");
    }
}
