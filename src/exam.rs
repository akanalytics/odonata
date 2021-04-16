use crate::position::Position;
use crate::search::algo::Algo;
use crate::config::{Config, Configurable};
use crate::search::timecontrol::TimeControl;
use std::fmt;

#[derive(Clone, Default, Debug)]
pub struct Exam {
    positions: Vec<Position>,
    algo: Algo,
    name: String,
    score: u32,
    out_of: u32,
}

impl fmt::Display for Exam {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}/{} ({:.02}%)", self.name, self.score, self.out_of, self.percentage())
    }
}


impl Exam {
    pub fn percentage(&self) -> f32 {
        100.0 * self.score as f32 / self.out_of as f32
    }

    pub fn take_exam(name: &str, positions: Vec<Position>) -> Exam {
        let c = Config::new();
        //.set("move_orderer.prior_bm", "true").set("move_orderer.prior_pv", "false");
        
        let mut exam = Exam { 
            name: String::from(name),
            positions,
            algo: Algo::new().set_timing_method(TimeControl::NodeCount(1_000_000)),
            .. Exam::default()
        };
        exam.algo.configure(&c);
        // exam.algo.set_callback(Uci::uci_info);
        for (i, pos) in exam.positions.iter().enumerate() {
            exam.out_of += 1;
            exam.algo.search(pos.board().clone());
            // println!("Algo\n{}", exam.algo);
            // println!("Position {}", pos);
            if pos.bm().ok().unwrap().contains(&exam.algo.bm()) {
                exam.score += 1;
            }
            println!("#{} score {}", i, exam)
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
        let bk = Exam::take_exam("bratko kopec", Catalog::bratko_kopec());
        assert!(bk.score >= 2);
    }

    #[test]
    #[ignore]
    fn test_exam_wac() {
        let bk = Exam::take_exam("win at chess", Catalog::win_at_chess());
        assert!(bk.score >= 117);
    }
}
