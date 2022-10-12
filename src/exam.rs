use crate::infra::component::Component;
// use crate::mv::Move;
use crate::position::Position;
use crate::search::engine::AsyncEngine;
use crate::search::timecontrol::TimeControl;
use std::fmt;

#[derive(Default, Debug)]
pub struct Exam {
    positions: Vec<Position>,
    engine: AsyncEngine,
    name: String,
    score: u32,
    out_of: u32,
}

impl fmt::Display for Exam {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {}/{} ({:.02}%)",
            self.name,
            self.score,
            self.out_of,
            self.percentage()
        )
    }
}

impl Exam {
    pub fn percentage(&self) -> f32 {
        100.0 * self.score as f32 / self.out_of as f32
    }

    pub fn take_exam(name: &str, positions: Vec<Position>) -> Exam {
        let mut engine = AsyncEngine::new();
        // algo.set_timing_method(TimeControl::Depth(5));
        engine
            .algo
            .set_timing_method(TimeControl::NodeCount(1_000_000));
        // engine.algo.set_timing_method(TimeControl::from_move_time_millis(1900));

        let mut exam = Exam {
            name: String::from(name),
            positions,
            engine,
            //algo: algo.set_timing_method(TimeControl::NodeCount(1_000_000)).clone(),
            ..Exam::default()
        };

        // exam.algo.set_callback(Uci::uci_info);
        for (i, pos) in exam.positions.iter().enumerate() {
            exam.out_of += 1;
            exam.engine.new_game();
            exam.engine.new_position();
            exam.engine.algo.board = pos.board().clone();
            exam.engine.search();

            let correct = pos
                .bm()
                .ok()
                .unwrap()
                .contains(&exam.engine.algo.results_as_position().sm().unwrap());
            if correct {
                exam.score += 1;
            }
            println!("#{:<2} score {}   {}", i, exam, pos.board().to_fen())
        }
        exam
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    #[ignore]
    fn test_exam_bk() {
        let ex = Exam::take_exam("bratko kopec", Catalog::bratko_kopec());
        assert!(ex.score >= 1);
        // println!("{}", ex.engine.algo);
    }

    #[test]
    #[ignore]
    fn test_exam_iq() {
        let ex = Exam::take_exam("IQ(182)", Catalog::iq());
        assert!(ex.score >= 8);
        // println!("{}", ex.engine.algo);
    }

    #[test]
    #[ignore]
    fn test_exam_wac() {
        let ex = Exam::take_exam("win at chess", Catalog::win_at_chess());
        assert!(ex.score >= 117);
        // println!("{}", ex.engine.algo);
    }

    #[test]
    #[ignore]
    fn test_wac_259() {
        let pos = &Catalog::win_at_chess()[258]; // WAC.259 as start at #1
        let mut engine = AsyncEngine::new();
        engine.algo.set_timing_method(TimeControl::Depth(5));
        engine.algo.board = pos.board().clone();
        engine.search();
        println!("{}", engine.algo);
    }
}
