use std::io::Write;

use crate::eval::model::ExplainScorer;
use crate::eval::model::Model;
use crate::eval::model::ModelScore;
use crate::eval::model::ReportLine;
use crate::eval::switches::Switches;
use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::position::Position;
use crate::search::engine::Engine;
use crate::tags::Tag;
use anyhow::{bail, Result};
use rayon::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tuning {
    pub ignore_known_outcomes: bool,
    pub multi_threading_min_positions: usize,
    pub ignore_draws: bool,
    pub logistic_steepness_k: Weight,

    #[serde(skip)]
    pub models_and_outcomes: Vec<(Model, f32)>,

    #[serde(skip)]
    pub boards: Vec<Position>,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            ignore_known_outcomes: true,
            multi_threading_min_positions: 20000,
            models_and_outcomes: Default::default(),
            boards: Default::default(),
            logistic_steepness_k: Weight::from_i32(4, 4),
            ignore_draws: false,
        }
    }
}

impl Component for Tuning {
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl Tuning {
    pub fn new() -> Self {
        Tuning::default()
    }

    pub fn clear(&mut self) {
        self.models_and_outcomes.clear();
        self.boards.clear();
    }

    pub fn upload_positions(&mut self, positions: &[Position]) -> usize {
        for pos in positions {
            if self.ignore_known_outcomes && pos.board().outcome().is_game_over() {
                trace!("Discarding drawn/checkmate position {}", pos);
                continue;
            }
            let model = Model::from_board(pos.board(), Switches::ALL_SCORING);
            let (outcome, outcome_str) = self.calc_player_win_prob_from_pos(pos);
            if self.ignore_draws && outcome_str == "1/2-1/2" {
                continue;
            }
            self.models_and_outcomes.push((model, outcome));
            self.boards.push(pos.clone());
        }
        self.models_and_outcomes.len()
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

    pub fn write_model<W: Write>(&self, engine: &Engine, writer: &mut W) -> Result<i32> {
        let eval = &engine.algo.eval;
        let mut line_count = 0;
        for (model, outcome) in self.models_and_outcomes.iter() {
            // if outcome > &0.25 && outcome < &0.75 {
            //     continue;
            // }
            let phase = model.mat.phase(&eval.phaser);
            let mut w_score = ExplainScorer::new(phase);
            eval.predict(model, &mut w_score);
            if line_count == 0 {
                #[allow(clippy::write_literal)]
                writeln!(writer, "{}{}, {}, {}", w_score.as_csv(ReportLine::Header), "phase", "outcome", "fen")?;
            }
            writeln!(writer, "{}{}, {}, {}", w_score.as_csv(ReportLine::Body), phase, outcome, model.multiboard.to_fen())?;
            line_count += 1;
        }
        writer.flush()?;
        Ok(line_count)
    }

    pub fn calculate_mean_square_error(&self, engine: &Engine) -> Result<f32> {
        let eval = &engine.algo.eval;
        let logistic_steepness_k = self.logistic_steepness_k; // so that closure does not capture engine/tuner

        let closure = |pair: &(Model, f32)| {
            // estimate result by looking at centipawn evaluation
            let (model, outcome) = pair;
            let phase = model.mat.phase(&eval.phaser);
            let mut w_score = ModelScore::new(phase);
            eval.predict(model, &mut w_score);
            // let score = w_score.as_f32() / (2.0 + (phase as f32 - 50.0) / 50.0);
            // let score = w_score.as_f32();
            let k = logistic_steepness_k.interpolate(phase) as f32;
            let win_prob_estimate = w_score.as_score().win_probability_using_k(k);
            let win_prob_actual = *outcome;
            let diff = win_prob_estimate - win_prob_actual;
            diff * diff
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
    fn test_tuning() {
        info!("Starting...");
        let mut engine = Engine::new();
        engine
            .tuner
            .upload_positions(&Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap());
        //tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap();
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");

        for n in (-100..140).step_by(10) {
            let value = n;
            engine.algo.eval.mb.enabled = false;
            engine.algo.eval.pawn_isolated = Weight::from_i32(0, value);
            let diffs = engine.tuner.calculate_mean_square_error(&engine).unwrap();
            println!("{}, {}", value, diffs);
        }
    }

    #[test]
    fn test_quick_tuning() {
        info!("Starting quick tuning...");
        let mut tuning = Tuning::new();
        tuning.upload_positions(&Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap());

        let engine = Engine::new();
        let diffs = tuning.calculate_mean_square_error(&engine).unwrap();
        assert!(diffs > 0.0);
    }
}
