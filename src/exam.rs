use crate::config::{Component, Config};
// use crate::mv::Move;
use crate::position::Position;
use crate::search::algo::Engine;
use crate::search::timecontrol::TimeControl;
use std::env;
use std::fmt;

#[derive(Clone, Default, Debug)]
pub struct Exam {
    positions: Vec<Position>,
    engine: Engine,
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
        let mut c = Config::new();
        let mut engine = Engine::new();
        engine.thread_count = 1;
        // algo.set_timing_method(TimeControl::Depth(5));
        engine.algo.set_timing_method(TimeControl::NodeCount(1_000_000));
        // Prints each argument on a separate line
        for arg in env::vars() {
            if arg.0.starts_with("ODONATA") {
                if let Some(combo) = arg.1.split_once("=") {
                    let (key, value) = combo;
                    c.set(key, value);
                }
            }
        }
        println!("using config\n{}", c);
        engine.configure(&c);

        let mut exam = Exam {
            name: String::from(name),
            positions,
            engine: engine,
            //algo: algo.set_timing_method(TimeControl::NodeCount(1_000_000)).clone(),
            ..Exam::default()
        };

        // exam.algo.set_callback(Uci::uci_info);
        for (i, pos) in exam.positions.iter().enumerate() {
            exam.out_of += 1;
            exam.engine.new_game();
            exam.engine.new_search();
            exam.engine.algo.board = pos.board().clone();
            exam.engine.search();
            if pos
                .bm()
                .ok()
                .unwrap()
                .iter()
                .map(|m| m.uci())
                .collect::<String>()
                .contains(&exam.engine.algo.results().bm().unwrap().uci())
            {
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
        assert!(ex.score >= 5);
        println!("{}", ex.engine.algo);
    }

    #[test]
    #[ignore]
    fn test_exam_iq() {
        let ex = Exam::take_exam("IQ(182)", Catalog::iq());
        assert!(ex.score >= 8);
        println!("{}", ex.engine.algo);
    }

    #[test]
    #[ignore]
    fn test_exam_wac() {
        let ex = Exam::take_exam("win at chess", Catalog::win_at_chess());
        assert!(ex.score >= 117);
        println!("{}", ex.engine.algo);
    }

    #[test]
    #[ignore]
    fn test_wac_259() {
        let pos = &Catalog::win_at_chess()[258]; // WAC.259 as start at #1
        let mut engine = Engine::new();
        engine.algo.set_timing_method(TimeControl::Depth(5));
        engine.algo.board = pos.board().clone();
        engine.search();
        println!("{}", engine.algo);
    }
}
