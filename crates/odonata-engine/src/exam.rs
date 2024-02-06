use odonata_base::infra::component::Component;
// use crate::mv::Move;
use crate::{engine::Engine, search::engine::ThreadedSearch};
use odonata_base::{domain::timecontrol::TimeControl, epd::Epd};
use std::fmt;

#[derive(Default, Debug)]
pub struct Exam {
    positions: Vec<Epd>,
    engine:    ThreadedSearch,
    name:      String,
    score:     u32,
    out_of:    u32,
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

    pub fn take_exam(name: &str, positions: Vec<Epd>) -> Exam {
        let engine = ThreadedSearch::new();

        let mut exam = Exam {
            name: String::from(name),
            positions,
            engine,
            ..Exam::default()
        };

        // exam.algo.set_callback(Uci::uci_info);
        for (i, epd) in exam.positions.iter().enumerate() {
            exam.out_of += 1;
            exam.engine.new_game();
            exam.engine.new_position();
            let res = exam
                .engine
                .search(epd.clone(), TimeControl::NodeCount(1_000_000))
                .unwrap();

            let correct = epd.moves("bm").unwrap().contains(&res.supplied_move);
            if correct {
                exam.score += 1;
            }
            println!("#{:<2} score {}   {}", i, exam, epd.board().to_fen())
        }
        exam
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odonata_base::catalog::Catalog;

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
        let mut engine = ThreadedSearch::new();
        engine.search.set_timing_method(TimeControl::Depth(5));
        engine.search.board = pos.board().clone();
        engine.search_sync();
        println!("{}", engine.search);
    }
}
