use std::io::Write;

use crate::Color;
use crate::eval::model::ExplainScorer;
use crate::eval::model::Model;
use crate::eval::model::ModelScore;
use crate::eval::model::ReportLine;
use crate::eval::score::Score;
use crate::eval::switches::Switches;
use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::phaser::Phase;
use crate::position::Position;
use crate::search::engine::Engine;
use crate::search::timecontrol::TimeControl;
use crate::tags::Tag;
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;


#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RegressionType {
    LinearOnCp,
    LogisticOnOutcome,
    LogisticOnCp,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tuning {
    pub regression_type: RegressionType,
    pub search_depth: i32,
    pub ignore_known_outcomes: bool,
    pub ignore_endgames: bool,
    pub multi_threading_min_positions: usize,
    pub ignore_draws: bool,
    pub logistic_steepness_k: Weight,
    pub consolidate: bool,

    #[serde(skip)]
    pub models_and_outcomes: Vec<(Model, f32)>,

    #[serde(skip)]
    pub boards: Vec<Position>,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            regression_type: RegressionType::LogisticOnOutcome,
            search_depth: -1,
            ignore_known_outcomes: true,
            ignore_endgames: true,
            multi_threading_min_positions: 20000,
            models_and_outcomes: Default::default(),
            boards: Default::default(),
            logistic_steepness_k: Weight::from_i32(4, 4),
            ignore_draws: false,
            consolidate: false,
        }
    }
}

impl Component for Tuning {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}


impl fmt::Display for Tuning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

impl Tuning {
    pub fn new() -> Self {
        Tuning::default()
    }

    pub fn clear(&mut self) {
        self.models_and_outcomes.clear();
        self.boards.clear();
    }

    pub fn upload_positions(&mut self, positions: &[Position]) -> Result<usize> {
        for pos in positions {
            if self.ignore_known_outcomes && pos.board().outcome().is_game_over() {
                trace!("Discarding drawn/checkmate position {}", pos);
                continue;
            }
            let model = Model::from_board(pos.board(), Switches::ALL_SCORING);

            if self.ignore_endgames
                && (model.endgame.try_winner().is_some() || model.endgame.is_likely_draw() || model.endgame.is_immediately_declared_draw())
            {
                trace!("Discarding known endgame position {}", pos);
                continue;
            }
            match self.regression_type {
                RegressionType::LogisticOnCp => {
                    if let Tag::Comment(_n, s) = pos.tag("c6") {
                        let mut w_score: f32 = s.parse()?;
                        // epd ce is from active placer
                        if pos.board().color_us() == Color::Black {
                            w_score = -w_score;
                        }
                        let k = self.logistic_steepness_k.interpolate(Phase(50)) as f32;
                        let win_prob_estimate = Score::from_f32(w_score).win_probability_using_k(k);
    
                        self.models_and_outcomes.push((model, win_prob_estimate));
                    }
                }
                RegressionType::LogisticOnOutcome => {
                    let (outcome, outcome_str) = self.calc_player_win_prob_from_pos(pos);
                    if self.ignore_draws && outcome_str == "1/2-1/2" {
                        continue;
                    }
                    self.models_and_outcomes.push((model, outcome));
                }
                RegressionType::LinearOnCp => {
                    if let Tag::Comment(_n, s) = pos.tag("c6") {
                        let mut outcome: f32 = s.parse()?;
                        // epd ce is from active placer
                        if pos.board().color_us() == Color::Black {
                            outcome = -outcome;
                        }
                        self.models_and_outcomes.push((model, outcome));
                    }
                }
            }
            self.boards.push(pos.clone());
        }
        Ok(self.models_and_outcomes.len())
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
        let mut line_count = 0;
        engine.algo.set_callback(|_| {}); // turn off uci_info output of doing zillions of searches
        for i in 0..engine.tuner.models_and_outcomes.len() {
            let result = Self::write_single_training_data(engine, writer, i)
                .with_context(|| format!("Failed on line {i} {}", engine.tuner.models_and_outcomes[i].0.multiboard.to_fen()));
            if let Err(e) = result {
                info!("write_training_data returns error");
                error!("Error in write_single_training_data {}", e);
                return Err(e);
            }                
            line_count += 1;
        }
        info!("write_training_data returns {line_count}");
        writer.flush()?;
        Ok(line_count)
    }

    pub fn write_single_training_data<W: Write>(engine: &mut Engine, writer: &mut W, i: usize) -> Result<()> {
        if i % 500 == 0 {
            info!("Processed {i} positions");
        }
        let ce = if engine.tuner.search_depth > 0 {
            engine.new_position();
            engine.set_position(Position::from_board(engine.tuner.models_and_outcomes[i].0.multiboard.clone()));
            engine.algo.set_timing_method(TimeControl::Depth(engine.tuner.search_depth));
            debug!("Searching using\n{engine}");
            engine.search();
            engine.algo.score().as_i16()
        } else {
            0
        };
        let (model, outcome) = &engine.tuner.models_and_outcomes[i];
        // if outcome > &0.25 && outcome < &0.75 {
        //     continue;
        // }
        let mut model = model.clone();
        model.csv = true;
        let phase = model.mat.phase(&engine.algo.eval.phaser);
        let mut w_score = ExplainScorer::new(phase, model.multiboard.fifty_halfmove_clock());
        engine.algo.eval.predict(&model, &mut w_score);
        let consolidate = engine.tuner.consolidate;
        if i == 0 {
            #[allow(clippy::write_literal)]
            writeln!(
                writer,
                "{}{},{},{},{}",
                w_score.as_csv(ReportLine::Header, consolidate),
                "phase",
                "outcome",
                "ce",
                "fen"
            )?;
        }
        writeln!(
            writer,
            "{}{}, {}, {}, {}",
            w_score.as_csv(ReportLine::Body, consolidate),
            phase.0,
            outcome,
            ce,
            model.multiboard.to_fen()
        )?;
        Ok(())
    }

    pub fn calculate_mean_square_error(&self, engine: &Engine) -> Result<f32> {
        let eval = &engine.algo.eval;
        let logistic_steepness_k = self.logistic_steepness_k; // so that closure does not capture engine/tuner
        let closure = |pair: &(Model, f32)| {
            // estimate result by looking at centipawn evaluation
            let (model, outcome) = pair;
            let phase = model.mat.phase(&eval.phaser);
            let mut w_score = ModelScore::new(phase, model.multiboard.fifty_halfmove_clock());
            eval.predict(model, &mut w_score);
            // let score = w_score.as_f32() / (2.0 + (phase as f32 - 50.0) / 50.0);
            // let score = w_score.as_f32();
            match self.regression_type {
                RegressionType::LogisticOnOutcome | RegressionType::LogisticOnCp => {
                    let k = logistic_steepness_k.interpolate(phase) as f32;
                    let win_prob_estimate = w_score.as_score().win_probability_using_k(k);
                    let win_prob_actual = *outcome;
                    let diff = win_prob_estimate - win_prob_actual;
                    diff * diff
                }
                RegressionType::LinearOnCp => {
                        let diff = w_score.as_score().as_i16() as f32 - outcome;
                        diff * diff
                    }
                }
        };

        let total_diff_squared: f32 = match self.models_and_outcomes.len() {
            0 => bail!("No tuning positions loaded or remain after filtering"),
            l if l < self.multi_threading_min_positions => {
                info!("Calculating mse on {} positions using single thread", l);
                self.models_and_outcomes.iter().map(closure).sum()
            }
            l => {
                info!("Calculating mse on {} positions using several threads", l);
                // use rayon on larger sized files
                self.models_and_outcomes.par_iter().map(closure).sum()
            }
        };

        // return average
        let mse = total_diff_squared / self.models_and_outcomes.len() as f32;
        info!("Calculated mse as {}", mse);
        Ok(mse)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::BufWriter, fs::File};

    use super::*;
    use crate::eval::weight::Weight;
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
        engine
            .tuner
            .upload_positions(&Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-combo.epd").unwrap()).unwrap();
        //tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap();
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");

        for n in (-120..120).step_by(1) {
            let value = n;
            engine.algo.eval.mb.enabled = false;
            engine.algo.eval.pawn_isolated = Weight::from_i32(0, value);
            let diffs = engine.tuner.calculate_mean_square_error(&engine).unwrap();
            println!("{}, {}", value, diffs);
        }
    }

    #[ignore]
    #[test]
    fn test_tuning_csv() {
        info!("Starting...");
        let mut engine = Engine::new();
        engine
            .tuner
            .upload_positions(&Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap()).unwrap();
        //tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap();
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");

        let filename = "/tmp/test_training_data.csv";
        let f = File::create(&filename).with_context(|| format!("Failed to open file {}", &filename)).unwrap();
        let mut f = BufWriter::new(f);
        let line_count = Tuning::write_training_data(&mut engine, &mut f).unwrap();
        println!(" lines proicessed: {line_count}");
    }


    #[test]
    fn test_quick_tuning_mse() {
        info!("Starting quick tuning...");
        let mut tuning = Tuning::new();
        tuning.upload_positions(&Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap()).unwrap();

        let engine = Engine::new();
        let diffs = tuning.calculate_mean_square_error(&engine).unwrap();
        assert!(diffs > 0.0);
    }
}
