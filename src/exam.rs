use crate::config::{Config, Component};
use crate::position::Position;
use crate::search::algo::Algo;
use crate::search::timecontrol::TimeControl;
use std::fmt;
use std::env;


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
        let mut c = Config::new();
        let mut algo = Algo::new();
        // algo.set_timing_method(TimeControl::Depth(5));
        algo.set_timing_method(TimeControl::NodeCount(1_000_000));
        // Prints each argument on a separate line
        for arg in env::vars() {
            if arg.0.starts_with("ODONATA") {
                if let Some(combo) = arg.1.split_once("=") {
                    let (key,value) = combo;
                    c.set(key, value);
                }
            }
        }   
        println!("using config\n{}", c);
        algo.configure(&c);


        let mut exam = Exam {
            name: String::from(name),
            positions,
            algo: algo.clone(),
            //algo: algo.set_timing_method(TimeControl::NodeCount(1_000_000)).clone(),
            ..Exam::default()
        };

        // exam.algo.set_callback(Uci::uci_info);
        for (i, pos) in exam.positions.iter().enumerate() {
            exam.out_of += 1;
            exam.algo.new_game();
            exam.algo.search(pos.board());
            if pos.bm().ok().unwrap().contains(&exam.algo.bm()) {
                exam.score += 1;
            }
            println!("#{} score {}   {}", i, exam, pos.board().to_fen())
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
        assert!(ex.score >= 5);
        println!("{}", ex.algo);
    }

    #[test]
    #[ignore]
    fn test_exam_iq() {
        let ex = Exam::take_exam("IQ(182)", Catalog::iq());
        assert!(ex.score >= 8);
        println!("{}", ex.algo);
    }

    #[test]
    #[ignore]
    fn test_exam_wac() {
        let ex = Exam::take_exam("win at chess", Catalog::win_at_chess());
        assert!(ex.score >= 117);
        println!("{}", ex.algo);
    }

    #[test]
    #[ignore]
    fn test_wac_259() {
        let pos = &Catalog::win_at_chess()[258]; // WAC.259 as start at #1
        let mut algo = Algo::new();
        algo.set_timing_method(TimeControl::Depth(5));
        algo.search(pos.board());
        println!("{}", algo);
    }

}
