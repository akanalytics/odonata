use std::io::Write;

use crate::eval::calc::Calc;
use crate::eval::feature::FeatureMatrix;
use crate::eval::feature::FeatureVector;
use crate::eval::score::Score;
use crate::eval::scorer::ExplainScore;
use crate::eval::scorer::ScorerBase;
use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::outcome::Outcome;
use crate::phaser::Phase;
use crate::position::Position;
use crate::search::engine::Engine;
use crate::tags::Tag;
use crate::Board;
use anyhow::Result;
use itertools::Itertools;
// use rayon::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

use std::sync::Arc;

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
pub enum Method {
    New,
    Sparse,
    Dense,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tuning {
    pub regression_type: RegressionType,
    pub method: Method,
    pub search_depth: i32,
    pub ignore_known_outcomes: bool,
    pub ignore_endgames: bool,
    pub multi_threading_min_positions: usize,
    pub threads: usize,
    pub ignore_draws: bool,
    pub consolidate: bool,
    pub logistic_steepness_k: Weight,

    #[serde(skip)]
    pub feature_matrix: FeatureMatrix,

    #[serde(skip)]
    pub explains: Vec<ExplainScore>,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            regression_type: RegressionType::LogisticOnOutcome,
            method: Method::New,
            search_depth: -1,
            ignore_known_outcomes: true,
            ignore_endgames: true,
            multi_threading_min_positions: 20000,
            threads: 32,
            logistic_steepness_k: Weight::from_i32(4, 4),
            feature_matrix: Default::default(),
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
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for Tuning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

fn model_and_accum(eng: &Engine, b: &Board, _p: Phase, scorer: &mut impl ScorerBase) {
    // let mut model = Model::from_board(b, p, Switches::ALL_SCORING);
    // model.csv = eng.tuner.sparse;
    // let mut scorer = ExplainScorer::new(String::new());
    // eng.algo.eval.predict(&model, scorer);
    Calc::score(scorer, b, &eng.algo.eval, &eng.algo.eval.phaser)
}

impl Tuning {
    pub fn new() -> Self {
        Tuning::default()
    }

    pub fn clear(&mut self) {
        // self.models_and_outcomes.clear();
        self.feature_matrix.feature_vectors.clear();
    }

    pub fn upload_positions(eng: &mut Engine, positions: Vec<Position>) -> Result<usize> {
        for (_i, pos) in positions.iter().enumerate() {
            if eng.tuner.ignore_known_outcomes && pos.board().outcome().is_game_over() {
                trace!("Discarding drawn/checkmate position {}", pos);
                continue;
            }
            // if eng.tuner.ignore_endgames
            //     && (model.endgame.try_winner().is_some() || model.endgame.is_likely_draw() || model.endgame.is_immediately_declared_draw())
            // {
            //     trace!("Discarding known endgame position {}", pos);
            //     continue;
            // }
            let (_outcome, outcome_str) = eng.tuner.calc_player_win_prob_from_pos(pos);
            let o = Outcome::try_from_pgn(&outcome_str)?;
            if eng.tuner.ignore_draws && outcome_str == "1/2-1/2" {
                continue;
            }
            let ph = eng.algo.eval.phaser.phase(&pos.board().material());
            let mut explain = ExplainScore::new(ph, pos.board().to_fen());
            Calc::score(&mut explain, pos.board(), &eng.algo.eval, &eng.algo.eval.phaser);
            explain.set_outcome(o);
            // eng.algo.eval.predict(&model, &mut w_scorer);
            // let _consolidate = eng.tuner.consolidate;
            explain.discard_balanced_features();
            eng.tuner.explains.push(explain);
        }
        info!("Loaded {} positions", positions.len());
        Ok(positions.len())
    }

    pub fn calc_player_win_prob_from_pos(&self, pos: &Position) -> (f32, String) {
        if let Tag::Comment(_n, s) = pos.tag(Tag::C9) {
            let prob = match s.as_str() {
                "1/2-1/2" => 0.5,
                "1-0" => 1.0,
                "0-1" => 0.0,
                _ => unreachable!(),
            };
            return (pos.board().color_us().chooser_wb(prob, prob), s.clone());
        }
        panic!("Unable to find result comment c9 in {}", pos);
    }

    pub fn write_training_data<W: Write>(eng: &mut Engine, writer: &mut W) -> Result<i32> {
        ExplainScore::write_csv(eng.tuner.explains.iter(), writer)
    }

    pub fn calculate_mean_square_error(&self, eng: &Engine) -> Result<f32> {
        let eval = &eng.algo.eval;
        let logistic_steepness_k = self.logistic_steepness_k; // so that closure does not capture engine/tuner
        let mse: f32;
        // let mut scorer = ExplainScorer::new(String::new(), true);
        // let board = Catalog::starting_board();
        // model_and_accum(eng, &board, Phase(0), &mut scorer);
        // let weight_vector = scorer.weights_vector();
        let eval_weight_vector = eval.weights_vector();
        // trace!("Weights = {}", weight_vector);
        let regression_type = eng.tuner.regression_type;
        // let mut diff_squared: f32 = 0.0;

        // let closure_fv = |pair: (usize, &FeatureVector)| {
        //     let (i, fv) = pair;
        //     // let fv = pair;
        //     let w_score = self.feature_matrix.dot_product(&fv, &weight_vector);
        //     let k = logistic_steepness_k.interpolate(fv.phase) as f32;
        //     let win_prob_estimate = Score::win_probability_from_cp_and_k(w_score, k);
        //     let win_prob_actual = match fv.outcome {
        //         Outcome::WinWhite => 1.0,
        //         Outcome::WinBlack => 0.0,
        //         Outcome::DrawRule50 => 0.5,
        //         _ => unreachable!(),
        //     };
        //     let cost = match regression_type {
        //         RegressionType::LogisticOnOutcome => {
        //             let diff = win_prob_estimate - win_prob_actual;
        //             diff * diff
        //         }
        //         RegressionType::CrossEntropy => match fv.outcome {
        //             Outcome::WinWhite => -f32::ln(win_prob_estimate),
        //             Outcome::WinBlack => -f32::ln(1.0 - win_prob_estimate),
        //             Outcome::DrawRule50 | _ => 0.0,
        //         },
        //         RegressionType::CumulativeLogisticLink => match fv.outcome {
        //             Outcome::WinWhite => -f32::ln(win_prob_estimate),
        //             Outcome::WinBlack => -f32::ln(1.0 - win_prob_estimate),
        //             Outcome::DrawRule50 => -f32::ln(1.0 - f32::abs(win_prob_estimate - 0.5)),
        //             _ => 0.0,
        //         },
        //         _ => unreachable!(),
        //     };
        //     if cost.is_infinite() || cost.is_nan() {
        //         debug!(
        //             "Sparse : {} {} {} {} {} {} {}",
        //             i, win_prob_estimate, win_prob_actual, w_score, fv.phase, cost, fv.fen
        //         );
        //     }
        //     if cost.is_infinite() || cost.is_nan() {
        //         0.0
        //     } else {
        //         cost
        //     }
        // };

        let closure_es = |pair: (usize, &ExplainScore)| {
            let (i, es) = pair;
            // let fv = pair;
            let w_score = es.dot_product(&eval_weight_vector).interpolate(es.phase);

            let k = logistic_steepness_k.interpolate(es.phase) as f32;
            let win_prob_estimate = Score::win_probability_from_cp_and_k(w_score, k);
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
                RegressionType::CrossEntropy => match es.outcome {
                    Outcome::WinWhite => -f32::ln(win_prob_estimate),
                    Outcome::WinBlack => -f32::ln(1.0 - win_prob_estimate),
                    Outcome::DrawRule50 | _ => 0.0,
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
                debug!(
                    "Sparse : {} {} {} {} {} {} {}",
                    i, win_prob_estimate, win_prob_actual, w_score, es.phase, cost, es.fen
                );
            }
            if cost.is_infinite() || cost.is_nan() {
                0.0
            } else {
                cost
            }
        };

        // if eng.tuner.method == Method::New {
        let l = self.explains.len();
        info!("Calculating mse (new) on {} positions using single thread", l);
        let total_diff_squared: f32 = self.explains.iter().enumerate().map(closure_es).sum();
        mse = total_diff_squared / l as f32;
        info!("Calculated (new) mse as {}", mse);
        return Ok(mse);
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

    fn calc_sparse(&self, f: impl Copy + Sync + Send + Fn((usize, &FeatureVector)) -> f32) -> f32 {
        info!("Calculating mse (sparse) on positions using several threads");
        // use rayon on larger sized files
        // self.feature_matrix.feature_vectors.par_iter().enumerate().map(closure).sum()
        let v = &self.feature_matrix.feature_vectors;
        if self.threads == 0 {
            panic!("At least one thread required.");
        }
        if self.threads > v.len() {
            panic!("More threads than items in vector.");
        }
        if v.len() == 0 {
            return 0.0;
        }

        // divide round up
        let items_per_thread = (v.len() - 1) / self.threads + 1;

        let arc = Arc::new(v);
        // let mut threads = Vec::with_capacity(nb_threads);

        // // this channel will be use to send values (partial sums) for threads
        // let (sender, receiver) = mpsc::channel::<T>();
        let thread_sum = std::sync::Mutex::new(0.0_f32);
        let vec = (0..self.threads).collect_vec();
        let slice = &vec[..];
        rayon::scope(|s| {
            for i in slice {
                s.spawn(|_s| {
                    let data = arc.clone();
                    let from = *i * items_per_thread;
                    let to = std::cmp::min(from + items_per_thread, data.len());
                    let mut sum = 0.0;
                    for v in &data[from..to] {
                        sum += f((0, v));
                    }
                    let mut ts = thread_sum.lock().unwrap();
                    *ts = *ts + sum;
                });
            }
        });
        let x = *thread_sum.lock().unwrap();
        x
    }
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
    use crate::utils::Formatting;
    use crate::{eval::weight::Weight, infra::profiler::Profiler};
    use anyhow::Context;
    use test_log::test;

    #[test]
    fn tuning_serde_test() {
        let tuner = Tuning::new();
        let text = toml::to_string(&tuner).unwrap();
        info!("{}", text);
    }

    #[ignore]
    #[test]
    fn test_tuning_mse() {
        info!("Starting...");
        let mut engine = Engine::new();
        engine.tuner.multi_threading_min_positions = 10000000;

        engine.tuner.regression_type = RegressionType::CrossEntropy;
        engine.tuner.method = Method::Sparse;
        Tuning::upload_positions(
            &mut engine,
            Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-combo.epd").unwrap(),
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
            engine.algo.eval.mb.enabled = false;
            engine.algo.eval.set_weight(Attr::PawnIsolated.into(), Weight::from_i32(0, value));
            iters += 1;
            let diffs = engine.tuner.calculate_mean_square_error(&engine).unwrap();
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
        let mut eng = Engine::new();
        eng.tuner.method = Method::New;
        Tuning::upload_positions(
            &mut eng,
            Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap(),
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
        let file = "../odonata-extras/epd/quiet-labeled-small.epd";

        let mut eng1 = Engine::new();
        eng1.tuner.multi_threading_min_positions = 3_000_000;
        eng1.tuner.method = Method::New;
        Tuning::upload_positions(&mut eng1, Position::parse_epd_file(file).unwrap()).unwrap();
        let mut prof1 = Profiler::new("mse new".into());
        prof1.start();
        let diffs1 = eng1.tuner.calculate_mean_square_error(&eng1).unwrap();
        prof1.stop();
        println!("MSE {diffs1}");
    }
}
