use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::movelist::MoveValidator;
use crate::movelist::{Move, MoveList};
use crate::outcome::GameEnd;
use crate::outcome::Outcome;
use crate::search::algo::Algo;
use crate::search::clock::TimingMethod;
use std::fmt;
use std::time;

trait Player {
    fn name(&self) -> String;
    fn choose_move(&mut self, board: &Board) -> Move;
}

impl Player for Algo {
    fn name(&self) -> String {
        self.algo_description()
    }

    fn choose_move(&mut self, board: &Board) -> Move {
        self.search(board.clone());
        self.pv.extract_pv()[0]
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    white: Algo,
    black: Algo,
    starting_pos: Board,
    starting_time: time::Instant,
    ending_time: Option<time::Instant>,
    board: Board,
    moves: MoveList,
}

impl Game {
    pub fn new(white: Algo, black: Algo) -> Game {
        Game {
            white,
            black,
            board: Board::default(),
            starting_pos: Board::default(),
            starting_time: time::Instant::now(),
            ending_time: None,
            moves: MoveList::default(),
        }
    }

    pub fn set_board(&mut self, board: Board) -> Game {
        self.starting_pos = board.clone();
        self.board = board;
        self.clone()
    }

    pub fn play_move(&mut self) -> Move {
        if !self.board.outcome().is_game_over() {
            let player = self.board.color_us().chooser_wb(&mut self.white, &mut self.black);
            let mv = player.choose_move(&self.board);
            self.moves.push(mv);
            println!("{}. {}", self.board.fullmove_counter(), self.board.to_san(&mv));
            self.board = self.board.make_move(&mv);
            return mv;
        }
        Move::new_null()
    }

    pub fn play(&mut self) {
        while !self.board.outcome().is_game_over() {
            self.play_move();
        }
        self.ending_time = Some(time::Instant::now());
    }

    pub fn outcome(&self) -> Outcome {
        self.board.outcome()
    }

    pub fn moves(&self) -> &MoveList {
        &self.moves
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //        writeln!(f, "[Date \"{}\"]", self.white.name())?;
        //        writeln!(f, "[Time \"{}\"]", self.white.name())?;
        writeln!(f, "[White \"{}\"]", self.white.name())?;
        writeln!(f, "[Black \"{}\"]", self.black.name())?;
        writeln!(f, "[Result \"{}\"]", self.outcome().to_pgn())?;
        if self.starting_pos != Catalog::starting_position() {
            writeln!(f, "[FEN \"{}\"]", self.starting_pos.to_fen())?;
            writeln!(f, "[SetUp \"1\"]")?;
        }
        writeln!(f, "{} {}", self.starting_pos.to_san_moves(&self.moves), self.board.outcome().to_pgn())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::*;

    #[test]
    #[ignore]
    fn test_game() {
        let eval_w = SimpleScorer::new().set_position(true);
        let search_w = Algo::new().set_timing_method(TimingMethod::Depth(5)).set_eval(eval_w);
        let white = search_w;

        let eval_b = SimpleScorer::new().set_position(false);
        let search_b = Algo::new().set_timing_method(TimingMethod::Depth(5)).set_eval(eval_b);
        let black = search_b;

        let board = Catalog::starting_position();
        let mut game = Game::new(white, black).set_board(board);
        game.play();
        println!("{}", game);
    }
}
