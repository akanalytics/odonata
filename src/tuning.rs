use std::io::Write;

use crate::Board;
use crate::catalog::Catalog;
use crate::eval::feature::FeatureMatrix;
use crate::eval::feature::FeatureVector;
use crate::eval::score::Score;
use crate::eval::scorer::ExplainScorer;
// use crate::eval::scorer::ModelScore;
// use crate::eval::scorer::ReportLine;
use crate::eval::scorer::Scorer;
use crate::eval::scorer2::Scorer2;
use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::outcome::Outcome;
use crate::phaser::Phase;
use crate::position::Position;
use crate::search::engine::Engine;
use crate::tags::Tag;
use anyhow::{bail, Result};
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



#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tuning {
    pub regression_type: RegressionType,
    pub sparse: bool,
    pub search_depth: i32,
    pub ignore_known_outcomes: bool,
    pub ignore_endgames: bool,
    pub multi_threading_min_positions: usize,
    pub threads: usize,
    pub ignore_draws: bool,
    pub consolidate: bool,
    pub logistic_steepness_k: Weight,

    // #[serde(skip)]
    // pub models_and_outcomes: Vec<(Model, f32)>,

    #[serde(skip)]
    pub feature_matrix: FeatureMatrix,

    // #[serde(skip)]
    // pub boards: Vec<Position>,

    // #[serde(skip)]
    // pub model: Model,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            regression_type: RegressionType::LogisticOnOutcome,
            sparse: true,
            search_depth: -1,
            ignore_known_outcomes: true,
            ignore_endgames: true,
            multi_threading_min_positions: 20000,
            threads: 32,
            // models_and_outcomes: Default::default(),
            feature_matrix: Default::default(),
            // boards: Default::default(),
            logistic_steepness_k: Weight::from_i32(4, 4),
            ignore_draws: false,
            consolidate: false,
            // model: Model::default(),
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


fn model_and_accum( eng: &Engine, b: &Board, _p: Phase, scorer: &mut impl Scorer){ 
    // let mut model = Model::from_board(b, p, Switches::ALL_SCORING);
    // model.csv = eng.tuner.sparse;
    // let mut scorer = ExplainScorer::new(String::new());
    // eng.algo.eval.predict(&model, scorer);
    Scorer2::score(scorer, b, &eng.algo.eval, &eng.algo.eval.phaser)
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
        // let mut eng.tuner.feature_matrix = FeatureMatrix::default();
        let board = Catalog::starting_board();
        let mut scorer = ExplainScorer::new(String::new(), true);
        model_and_accum( eng, &board, Phase(0), &mut scorer);
        eng.tuner.feature_matrix.feature_names = scorer.feature_names();

        for (_i, pos) in positions.iter().enumerate() {
            let ph = eng.algo.eval.phaser.phase(&pos.board().material());
            // let mut model = Model::from_board(pos.board(), ph, Switches::ALL_SCORING);
            // model.csv = eng.tuner.sparse;

            // set CSV flag so that feature weights get calculated
            
            // if i == 0 {
            //     eng.tuner.model = model.clone();
            //     let mut scorer = ExplainScorer::new(String::new());
            //     eng.algo.eval.predict(&model, &mut scorer);
            //     eng.tuner.feature_matrix.feature_names = scorer.feature_names();
            //     info!("Regression type {:?}", eng.tuner.regression_type);
            // }

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
            match (eng.tuner.regression_type, eng.tuner.sparse) {
                // (RegressionType::LogisticOnCp, _) => {
                //     if let Tag::Comment(_n, s) = pos.tag("c6") {
                //         let mut w_score: f32 = s.parse()?;
                //         // epd ce is from active placer
                //         if pos.board().color_us() == Color::Black {
                //             w_score = -w_score;
                //         }
                //         let k = eng.tuner.logistic_steepness_k.interpolate(Phase(50)) as f32;
                //         let win_prob_estimate = Score::from_f32(w_score).win_probability_using_k(k);

                //         eng.tuner.models_and_outcomes.push((model, win_prob_estimate));
                //     }
                // }
                // (RegressionType::LogisticOnOutcome,false) => {
                //     let (outcome, outcome_str) = eng.tuner.calc_player_win_prob_from_pos(pos);
                //     if eng.tuner.ignore_draws && outcome_str == "1/2-1/2" {
                //         continue;
                //     }
                //     eng.tuner.models_and_outcomes.push((model, outcome));
                // }
                (_, true) => {
                    let (_outcome, outcome_str) = eng.tuner.calc_player_win_prob_from_pos(pos);
                    let o = Outcome::try_from_pgn(&outcome_str)?;
                    if eng.tuner.ignore_draws && outcome_str == "1/2-1/2" {
                        continue;
                    }
                    let mut w_scorer = ExplainScorer::new(pos.board().to_fen(), true);
                    model_and_accum(eng, pos.board(), ph, &mut w_scorer);
                    // eng.algo.eval.predict(&model, &mut w_scorer);
                    // let _consolidate = eng.tuner.consolidate;
                    let fv = w_scorer.into_feature_vector(o);
                    eng.tuner.feature_matrix.feature_vectors.push(fv);
                }
                (_,_) => unreachable!("Unexpected regression type or dense")
                // (RegressionType::LinearOnCp, _) => {
                //     if let Tag::Comment(_n, s) = pos.tag("c6") {
                //         let mut outcome: f32 = s.parse()?;
                //         // epd ce is from active placer
                //         if pos.board().color_us() == Color::Black {
                //             outcome = -outcome;
                //         }
                //         eng.tuner.models_and_outcomes.push((model, outcome));
                //     }
                // }
            }
            // eng.tuner.boards.push(*pos);
        }
        //  eng.tuner.boards = positions;

        // let lines = if eng.tuner.sparse {
        info!("Loaded sparse lines");
        Ok(eng.tuner.feature_matrix.feature_vectors.len())
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


    pub fn write_training_data<W: Write>(engine: &mut Engine, writer: &mut W) -> Result<i32> {
        // if engine.tuner.sparse {
        engine.tuner.feature_matrix.write_csv(writer) 
        // } else {
        //     let mut line_count = 0;
        //     engine.algo.set_callback(|_| {}); // turn off uci_info output of doing zillions of searches
        //     for i in 0..engine.tuner.models_and_outcomes.len() {
        //         let result = Self::write_single_training_data(engine, writer, i)
        //             .with_context(|| format!("Failed on line {i} {}", engine.tuner.models_and_outcomes[i].0.board.to_fen()));
        //         if let Err(e) = result {
        //             info!("write_training_data returns error");
        //             error!("Error in write_single_training_data {}", e);
        //             return Err(e);
        //         }
        //         line_count += 1;
        //     }
        //     info!("write_training_data returns {line_count}");
        //     writer.flush()?;
        //     Ok(line_count)
        // }
    }
    // pub fn write_single_training_data<W: Write>(engine: &mut Engine, writer: &mut W, i: usize) -> Result<()> {
    //     if i % 500 == 0 {
    //         info!("Processed {i} positions");
    //     }
    //     let ce = if engine.tuner.search_depth > 0 {
    //         engine.new_position();
    //         engine.set_position(Position::from_board(engine.tuner.models_and_outcomes[i].0.board.clone()));
    //         engine.algo.set_timing_method(TimeControl::Depth(engine.tuner.search_depth));
    //         debug!("Searching using\n{engine}");
    //         engine.search();
    //         engine.algo.score().as_i16()
    //     } else {
    //         0
    //     };
    //     let (model, outcome) = &engine.tuner.models_and_outcomes[i];
    //     // if outcome > &0.25 && outcome < &0.75 {
    //     //     continue;
    //     // }
    //     let mut model = model.clone();
    //     model.csv = true;
    //     let fen = model.board.to_fen();
    //     let mut w_score = ExplainScorer::new(fen.clone());
    //     engine.algo.eval.predict(&model, &mut w_score);
    //     let consolidate = engine.tuner.consolidate;

    //     if i == 0 {
    //         #[allow(clippy::write_literal)]
    //         writeln!(
    //             writer,
    //             "{}{},{},{},{}",
    //             w_score.as_csv(ReportLine::Header, consolidate),
    //             "phase",
    //             "outcome",
    //             "ce",
    //             "fen"
    //         )?;
    //     }
    //     writeln!(
    //         writer,
    //         "{}{},{},{},{}",
    //         w_score.as_csv(ReportLine::Body, consolidate),
    //         model.phase.0,
    //         outcome,
    //         ce,
    //         fen,
    //     )?;
    //     Ok(())
    // }

    pub fn calculate_mean_square_error(&self, eng: &Engine) -> Result<f32> {
        let _eval = &eng.algo.eval;
        let logistic_steepness_k = self.logistic_steepness_k; // so that closure does not capture engine/tuner
        let mse: f32;
        let mut scorer = ExplainScorer::new(String::new(), true);
        let board = Catalog::starting_board();
        model_and_accum( eng, &board, Phase(0), &mut scorer);
        let weight_vector = scorer.weights_vector();
        trace!("Weights = {}", weight_vector);
        let regression_type = eng.tuner.regression_type;
        // let mut diff_squared: f32 = 0.0;

        let closure = |pair: (usize, &FeatureVector)| {
            let (i, fv) = pair;
            // let fv = pair;
            let w_score = self.feature_matrix.dot_product(&fv, &weight_vector);
            let k = logistic_steepness_k.interpolate(fv.phase) as f32;
            let win_prob_estimate = Score::win_probability_from_cp_and_k(w_score, k);
            let win_prob_actual = match fv.outcome {
                Outcome::WinWhite => 1.0,
                Outcome::WinBlack => 0.0,
                Outcome::DrawRule50 => 0.5,
                _ => unreachable!(),
            };
            let cost = match regression_type {
                RegressionType::LogisticOnOutcome => {
                    let diff = win_prob_estimate - win_prob_actual;
                    diff * diff
                },
                RegressionType::CrossEntropy => {
                    match fv.outcome {
                        Outcome::WinWhite => -f32::ln(win_prob_estimate),
                        Outcome::WinBlack => -f32::ln(1.0 - win_prob_estimate),
                        Outcome::DrawRule50 | _ => 0.0,

                    }
                }
                RegressionType::CumulativeLogisticLink => {
                    match fv.outcome {
                        Outcome::WinWhite => -f32::ln(win_prob_estimate),
                        Outcome::WinBlack => -f32::ln(1.0 - win_prob_estimate),
                        Outcome::DrawRule50 => -f32::ln( 1.0 - f32::abs(win_prob_estimate - 0.5)) ,
                        _ => 0.0,

                    }
                }
                _ => unreachable!(),
            };
            if cost.is_infinite() || cost.is_nan()  {
                debug!("Sparse : {} {} {} {} {} {} {}", i, win_prob_estimate, win_prob_actual, w_score, fv.phase, cost, fv.fen);
            }
            if cost.is_infinite() || cost.is_nan()  {
                0.0
            } else {
                cost
            }
        };

        let total_diff_squared: f32 = match self.feature_matrix.feature_vectors.len() {
            0 => bail!("No (sparse) tuning positions loaded or remain after filtering"),
            l if l < self.multi_threading_min_positions => {
                info!("Calculating mse (sparse) on {} positions using single thread", l);
                self.feature_matrix.feature_vectors.iter().enumerate().map(closure).sum()
            }
            l => {
                info!("Calculating mse (sparse) on {} positions using multi thread", l);
                self.calc_sparse(closure)
            }
        };

        mse = total_diff_squared / self.feature_matrix.feature_vectors.len() as f32;
        info!("Calculated mse as {}", mse);
        return Ok(mse);
    }

        // let closure = |pair: (usize, &(Model, f32))| {
        //     // estimate result by looking at centipawn evaluation
        //     let (i, (model, outcome)) = pair;
        //     let phase = model.mat.phase(&eval.phaser);
        //     // let mut w_score = ModelScore::new(phase, model.board.fifty_halfmove_clock());
        //     let mut w_score = ModelScore::new();
        //     eval.predict(model, &mut w_score);
        //     // let score = w_score.as_f32() / (2.0 + (phase as f32 - 50.0) / 50.0);
        //     // let score = w_score.as_f32();
        //     match self.regression_type {
        //         RegressionType::LogisticOnOutcome | RegressionType::LogisticOnCp => {
        //             let k = logistic_steepness_k.interpolate(phase) as f32;
        //             let win_prob_estimate = w_score.as_score().win_probability_using_k(k);
        //             let win_prob_actual = *outcome;
        //             let diff = win_prob_estimate - win_prob_actual;

        //             if i < 10 {
        //                 debug!(
        //                     "Regular: {} {} {} {}",
        //                     win_prob_estimate,
        //                     win_prob_actual,
        //                     w_score.as_score(),
        //                     phase.0
        //                 );
        //             }
        //             diff * diff
        //         }
        //         RegressionType::LinearOnCp => {
        //             let diff = w_score.as_score().as_i16() as f32 - outcome;
        //             diff * diff
        //         }
        //     }
        // };

        // let total_diff_squared: f32 = match self.models_and_outcomes.len() {
        //     0 => bail!("No tuning positions (non-sparse) loaded or remain after filtering"),
        //     l if l < self.multi_threading_min_positions => {
        //         info!("Calculating mse (non-sparse) on {} positions using single thread", l);
        //         self.models_and_outcomes.iter().enumerate().map(closure).sum()
        //     }
        //     l => {
        //         info!("Calculating mse (non-sparse) on {} positions using several threads", l);
        //         // use rayon on larger sized files
        //         self.models_and_outcomes.par_iter().enumerate().map(closure).sum()
        //     }
        // };

        // // return average
        // mse = total_diff_squared / self.models_and_outcomes.len() as f32;
        // info!("Calculated mse as {}", mse);
        // Ok(mse)

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
        engine.tuner.sparse = true;
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
            engine.algo.eval.pawn_isolated = Weight::from_i32(0, value);
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
        let mut engine = Engine::new();
        Tuning::upload_positions(
            &mut engine,
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
        let line_count = Tuning::write_training_data(&mut engine, &mut f).unwrap();
        println!(" lines proicessed: {line_count}");
    }

    #[test]
    fn bench_mse() {
        info!("Starting quick tuning...");
        let file = "../odonata-extras/epd/quiet-labeled-small.epd";

        // let mut eng1 = Engine::new();
        // eng1.tuner.sparse = false;
        // Tuning::upload_positions(&mut eng1, Position::parse_epd_file(file).unwrap()).unwrap();
        // let mut prof1 = Profiler::new("mse dense".into());
        // prof1.start();
        // let diffs1 = eng1.tuner.calculate_mean_square_error(&eng1).unwrap();
        // prof1.stop();

        let mut eng2 = Engine::new();
        eng2.tuner.sparse = true;
        Tuning::upload_positions(&mut eng2, Position::parse_epd_file(file).unwrap()).unwrap();
        let mut prof2 = Profiler::new("mse sparse".into());
        prof2.start();
        let diffs2 = eng2.tuner.calculate_mean_square_error(&eng2).unwrap();
        println!("{}", diffs2);
        prof2.stop();
        println!("{:#?}", eng2.tuner.feature_matrix);
        // println!("{:#?}", eng2.tuner.model);

        // compare calcs

        // let model1 = &eng1.tuner.models_and_outcomes[1].0;
        // let mut scorer1 = ExplainScorer::new();
        // eng1.algo.eval.predict(model1, &mut scorer1);
        // println!("Scorer1{}\n\n", scorer1);

        // let mut scorer2 = ExplainScorer::new();
        // let mut fm = FeatureMatrix::default();
        // eng2.algo.eval.predict(model1, &mut scorer2);
        // fm.feature_names = scorer2.feature_names();
        // let fv = scorer2.feature_vector(Outcome::DrawRule50);
        // fm.feature_vectors.push(fv);
        // println!("{}", fm);

        // println!("{:#?}", eng2.tuner.model);
        // println!("{:#?}", eng2.algo.eval);

        // assert!((diffs1 - diffs2).abs() < 0.00001);
    }
}
