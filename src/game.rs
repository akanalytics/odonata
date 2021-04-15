use crate::board::makemove::MoveMaker;
use crate::board::boardbuf::BoardBuf;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::movelist::MoveValidator;
use crate::movelist::{Move, MoveList};
use crate::outcome::GameEnd;
use crate::outcome::Outcome;
use crate::types::Color;
use crate::search::algo::Algo;
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
        let bm = self.pv()[0];
        // println!("{:#}", self.score);
        if bm == Move::new_null() {
            println!("{:#}", self);
        }
        bm
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
    outcome: Outcome,
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
            outcome: Outcome::InProgress,
        }
    }

    pub fn set_board(&mut self, board: Board) -> Game {
        self.starting_pos = board.clone();
        self.board = board;
        self.clone()
    }

    pub fn play_move(&mut self) -> Move {
        if !self.board.outcome().is_game_over() {
            if let Err(e) = self.board.validate() {
                println!("Error: {}", e);
                println!("{:#}\n\nmoves: {}", self.board, self.moves());
                panic!("Error on board");
            };

            let player = self.board.color_us().chooser_wb(&mut self.white, &mut self.black);
            let mv = player.choose_move(&self.board);
            self.moves.push(mv);
            
            // FIXME
            if 1 == 0 {
                println!(
                    "{}.{} {}   {}",
                    self.board.fullmove_counter(),
                    if self.board.color_us() == Color::Black { ".. " } else { "" },
                    self.board.to_san(&mv),
                    self.board.to_fen()
                );
            }
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
        writeln!(f, "[Result \"{}\"]", self.outcome().as_pgn())?;
        if self.starting_pos != Catalog::starting_position() {
            writeln!(f, "[FEN \"{}\"]", self.starting_pos.to_fen())?;
            writeln!(f, "[SetUp \"1\"]")?;
        }
        writeln!(f, "{} {}", self.starting_pos.to_san_moves(&self.moves), self.board.outcome().as_pgn())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::*;
    use crate::search::timecontrol::*;
    use std::time::Duration;
    use crate::types::{CastlingRights,ScoreWdl};


    #[test]
    #[ignore]
    fn test_game() {
        let eval_w = SimpleScorer::new().set_position(true);
        let search_w = Algo::new().set_timing_method(TimeControl::Depth(2)).set_eval(eval_w);
        let white = search_w;

        let eval_b = SimpleScorer::new().set_position(false);
        let search_b = Algo::new().set_timing_method(TimeControl::Depth(2)).set_eval(eval_b);
        let black = search_b;

        let board = Catalog::starting_position();
        let mut game = Game::new(white, black).set_board(board);
        game.play();
        println!("{}", game);
    }

    // use crate::comms::uci::Uci;

    #[test]
    #[ignore]
    fn test_competition() {
        let tc = TimeControl::from_remaining_time(Duration::from_millis(200));
        //let tc = TimeControl::Depth(3);
        let mut white = Algo::new().set_timing_method(tc);
        let mut black = Algo::new().set_timing_method(tc);
        // white.set_callback(Uci::uci_info);

        white.quiescence.enabled = true;
        white.move_orderer.mvv_lva = true;
        black.move_orderer.mvv_lva = true;
        black.move_orderer.prior_pv = true;
        black.move_orderer.prior_bm = false;
        
        let mut score = ScoreWdl::default();
        for id in 0..960 {
            let mut b = *Catalog::chess960(id).board();
            b.set_castling(CastlingRights::NONE);
            let mut game1 = Game::new(white.clone(), black.clone()).set_board(b);
            let mut game2 = Game::new(black.clone(), white.clone()).set_board(b);
            game1.play();
            // println!("\n{}", game1);
            game2.play();
            // println!("\n{}", game2);
            score += game1.outcome().as_wdl() + game2.outcome().reversed().as_wdl();
            println!("game: {} score {}", id+1, score);
        }
        println!("score {}", score);
    }

    #[test]
    fn test_bug1() {
        let b = Board::parse_fen("1rk2qRr/8/B3P3/B4QN1/P4p2/2K1PP1P/P7/R2N4 b - - 0 38").unwrap();
        let tc = TimeControl::MoveTime(Duration::from_secs(3));
        let mut white = Algo::new().set_timing_method(tc);
        let mut black = Algo::new().set_timing_method(tc);
        white.move_orderer.mvv_lva = true;
        black.move_orderer.mvv_lva = false;
        black.search(b);
    }
}
