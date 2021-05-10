use crate::config::{Config, Configurable};
use crate::position::Position;
use crate::search::algo::Algo;
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

        let mut exam = Exam {
            name: String::from(name),
            positions,
            algo: Algo::new().set_timing_method(TimeControl::NodeCount(1_000_000)).clone(),
            ..Exam::default()
        };
        exam.algo.configure(&c);
        // exam.algo.set_callback(Uci::uci_info);
        for (i, pos) in exam.positions.iter().enumerate() {
            exam.out_of += 1;
            exam.algo.new_game();
            //exam.algo.tt.aging = false;
            // exam.algo.eval.cache.clear();
            // exam.algo.eval.rook_open_file = 0;
            //exam.algo.eval.cache.enabled = true;
            exam.algo.search(pos.board());
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
        assert!(bk.score >= 8);
        println!("{}", bk.algo);
    }

    #[test]
    #[ignore]
    fn test_exam_wac() {
        let wac = Exam::take_exam("win at chess", Catalog::win_at_chess());
        assert!(wac.score >= 117);
        println!("{}", wac.algo);
    }
}
