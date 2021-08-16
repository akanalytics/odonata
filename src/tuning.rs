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
        0.0
        //panic!("Unable to find result comment c9 in {}", pos);
    }

    pub fn calc_white_win_prob_from_cp(&self, ce: Score) -> f32 {
        ce.win_probability()
    }

    pub fn calculate_average_diffs(&self) -> f32 {
        let mut total_diff = 0.0;
        for pos in &self.positions {
            let w_score = pos.board().color_us().chooser_wb(1, -1) * pos.board().eval(&self.engine.algo.eval, &Node::root(0));
            let win_prob_cp = self.calc_white_win_prob_from_cp(w_score);
            let win_prob_pos = self.calc_white_win_prob_from_pos(pos);
            let diff = win_prob_cp - win_prob_pos;
            total_diff += diff * diff;
            debug!("{:>4} {:>4} {:>4}   {}", 
                w_score, 
                Formatter::format_decimal(2,win_prob_cp), 
                Formatter::format_decimal(2, win_prob_pos), 
                Formatter::format_decimal(2, diff*diff) );
        }
        total_diff / self.positions.len() as f32
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
        // tuning.positions = Position::parse_epd_file("../odonata-extras/epd/com15.epd")?;
        // tuning.positions = Catalog::bratko_kopec();
        println!("Loaded\n");
        for n in 0..30 {
            let value = 300 + n*15;
            tuning.engine.algo.eval.mb.enabled = false;
            tuning.engine.algo.eval.mb.material_weights[Piece::Rook] = Weight::new(value, value); 
            let diffs = tuning.calculate_average_diffs();
            println!("#{} value = {} diff = {}", n, value, diffs);
        }
    }
}