use std::io::Write;

use crate::eval::model::ExplainScorer;
use crate::eval::model::Model;
use crate::eval::model::ModelScore;
use crate::eval::model::ReportLine;
use crate::eval::switches::Switches;
use crate::position::Position;
use crate::eval::score::Score;
use crate::search::engine::Engine;
use crate::tags::Tag;
use rayon::prelude::*;
use anyhow::Result;


#[derive(Clone, Default, Debug)]
pub struct Tuning {
    models_and_outcomes: Vec<(Model, f32)>,
    boards: Vec<Position>,
}




impl Tuning {


    pub fn new() -> Self {
        Tuning::default()
    }

    pub fn upload_positions(&mut self, positions: &[Position]) -> usize {
        for pos in positions {
            if pos.board().outcome().is_game_over() {
                trace!("Discarding drawn/checkmate position {}", pos);
                continue;
            }
            let model = Model::from_board(pos.board(), Switches::ALL_SCORING);
            let outcome = self.calc_player_win_prob_from_pos(pos);
            self.models_and_outcomes.push( (model, outcome) );
            self.boards.push(pos.clone());
        }
        self.models_and_outcomes.len()
    }


    pub fn calc_player_win_prob_from_pos(&self, pos: &Position) -> f32 {
        if let Tag::Comment(_n, s) = pos.tag(Tag::C9) {
            let prob = match s.as_str() {
                "1/2-1/2" => 0.5,
                "1-0" => 1.0,
                "0-1" => 0.0,
                _ => unreachable!()
            };
            return pos.board().color_us().chooser_wb(prob, prob);
        }
        panic!("Unable to find result comment c9 in {}", pos);
    }


    pub fn write_model<W: Write>(&self, engine: &Engine, writer: &mut W) -> Result<i32> {
        let eval = &engine.algo.eval;
        let mut line_count = 0;
        for (model, outcome) in self.models_and_outcomes.iter() {
            let phase = model.mat.phase(&eval.phaser);
            let mut w_score = ExplainScorer::new(phase);
            eval.predict(model, &mut w_score);
            if line_count == 0 {
                writeln!(writer, "{}{}", w_score.as_csv(ReportLine::Header), "outcome")?;
            }
            writeln!(writer, "{}{}", w_score.as_csv(ReportLine::Body), outcome)?;
            line_count += 1;
        }
        writer.flush()?;
        Ok(line_count)
    }


    pub fn calculate_mean_square_error(&self, engine: &Engine) -> f32 {

        let eval = &engine.algo.eval;
        let closure = |pair: &(Model,f32)| {
            // estimate result by looking at centipawn evaluation
            let (model,outcome) = pair;
            let phase = model.mat.phase(&eval.phaser);
            let mut w_score = ModelScore::new(phase);
            eval.predict(model, &mut w_score);
            let win_prob_estimate = Score::win_probability_of(w_score.as_f32());
            let win_prob_actual = *outcome;
            let diff = win_prob_estimate - win_prob_actual;
            diff * diff
        };

        let total_diff_squared: f32 = if self.models_and_outcomes.len() < 20000 {
            self.models_and_outcomes.iter().map(closure).sum()
        } else {
            // use rayon on larger sized files
            self.models_and_outcomes.par_iter().map(closure).sum()
        };

        // return average
        total_diff_squared / self.models_and_outcomes.len() as f32
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::weight::Weight;



    #[ignore]
    #[test]
    fn test_tuning() {
        info!("Starting...");
        let mut tuning = Tuning::new();
        tuning.upload_positions(&Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap());
        //tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap();
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");

        let mut engine = Engine::new();
        for n in (-100..140).step_by(10) {
            let value = n;
            engine.algo.eval.mb.enabled = false;
            engine.algo.eval.pawn_isolated = Weight::from_i32(0, value); 
            let diffs = tuning.calculate_mean_square_error(&engine);
            println!("{}, {}", value, diffs);
        }
    }


    #[test]
    fn test_quick_tuning() {
        info!("Starting quick tuning...");
        let mut tuning = Tuning::new();
        tuning.upload_positions(&Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap());

        let engine = Engine::new();
        let diffs = tuning.calculate_mean_square_error(&engine);
        assert!(diffs > 0.0);
    }
}