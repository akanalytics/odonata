use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::outcome::Outcome;
use crate::search::Search;
use crate::movelist::{Move, MoveList};
use crate::outcome::GameEnd;
use std::fmt;




trait Player {
    fn name(&self) -> String;
    fn choose_move(&mut self, board: &Board) -> Move;
}

impl Player for Search {
    fn name(&self) -> String {
        format!("Search.max_depth({}).minmax({})", self.max_depth, self.minmax )
    }

    fn choose_move(&mut self, board: &Board) -> Move {
        self.search(board.clone());
        self.pv.extract_pv()[0]
    }
} 

#[derive(Debug, Clone)]
pub struct Game {
    white: Search,
    black: Search,
    board: Board,
    moves: MoveList,
}





impl Game {

    pub fn new(white: Search, black: Search) -> Game {
        Game { white, black, board: Board::default(), moves: MoveList::default() }
    }

    pub fn set_board(&mut self, board: Board) -> Game {
        self.board = board;
        self.clone()
    }

    pub fn play_move(&mut self) -> Move {
        if !self.board.outcome().is_game_over() {
            let player = self.board.color_us().chooser_wb(&mut self.white, &mut self.black);
            let mv = player.choose_move(&self.board);
            self.moves.push(mv);
            println!("Move: {}", mv);
            self.board = self.board.make_move(&mv);
            return mv;
        }
        return Move::new_null(); 
    }


    pub fn play(&mut self) {
        while !self.board.outcome().is_game_over() {
            self.play_move();
        }
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
        writeln!(f, "[White \"{}\"]", self.white.name())?;
        writeln!(f, "[Black \"{}\"]", self.black.name())?;
        writeln!(f, "[Result \"{}\"]", self.outcome())?;
        writeln!(f, "moves \"{}\"", self.moves)?;
        writeln!(f, "board\n \"{}\"", self.board)?;
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::eval::*;

    
    #[test]
    #[ignore]
    fn test_game() {
        let mut eval_w = SimpleScorer::default();
        eval_w.position = true;
        let search_w = Search::new().depth(3).eval(eval_w);
        let white = search_w;

        let mut eval_b = SimpleScorer::default();
        eval_b.position = false;
        let search_b = Search::new().depth(3).eval(eval_b);
        let black = search_b;

        let board = Catalog::starting_position();
        let mut game = Game::new(white, black).set_board(board);
        game.play();
        println!("{}", game);
    }
}
