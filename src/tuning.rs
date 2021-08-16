use crate::eval::score::Score;
use crate::position::Position;
use crate::search::algo::Engine;
use crate::search::node::Node;
use crate::utils::Formatter;
use crate::{debug, logger::LogInit};
use crate::tags::Tag;


#[derive(Clone, Default, Debug)]
pub struct Tuning {
    positions: Vec<Position>,
    engine: Engine,        
}




impl Tuning {


    pub fn new() -> Self {
        Tuning::default()
    }

    pub fn calc_white_win_prob_from_pos(&self, pos: &Position) -> f32 {
        if let Tag::Comment(_n, s) = pos.tag(Tag::C9) {
            return match s.as_str() {
                "1/2-1/2" => 0.5,
                "1-0" => 1.0,
                "0-1" => 0.0,
                _ => unreachable!()
            }
        }
        panic!("Unable to find result comment c9 in {}", pos);
    }

    pub fn calculate_average_diffs(&self) -> f32 {
        let mut total_diff_squared = 0.0;
        for pos in &self.positions {

            // estimate result by looking at centipawn evaluation
            let w_score = pos.board().color_us().chooser_wb(1, -1) * pos.board().eval(&self.engine.algo.eval, &Node::root(0));
            let win_prob_estimate = w_score.win_probability();

            let win_prob_actual = self.calc_white_win_prob_from_pos(pos);

            let diff = win_prob_estimate - win_prob_actual;
            total_diff_squared += diff * diff;

            debug!("{:>4} {:>4} {:>4}   {}", 
                w_score, 
                Formatter::format_decimal(2,win_prob_estimate), 
                Formatter::format_decimal(2, win_prob_actual), 
                Formatter::format_decimal(2, diff*diff) );
        }

        // return average
        total_diff_squared / self.positions.len() as f32
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::types::Piece;
    use crate::eval::weight::Weight;





    #[test]
    fn test_tuning() {
        info!("Starting...");
        let mut tuning = Tuning::default();
        tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled.epd").unwrap();
        //tuning.positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled-small.epd").unwrap();
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");
        for n in (-100..140).step_by(10) {
            let value = n;
            tuning.engine.algo.eval.mb.enabled = false;
            tuning.engine.algo.eval.pawn_isolated = Weight::new(0, value); 
            let diffs = tuning.calculate_average_diffs();
            println!("{}, {}", value, diffs);
        }
    }
}