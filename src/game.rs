use crate::board::makemove::MoveMaker;
use crate::board::boardbuf::BoardBuf;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::movelist::MoveValidator;
use crate::movelist::{Move, MoveList};
use crate::outcome::GameEnd;
use crate::outcome::Outcome;
use crate::types::Color;
use crate::tags::Tags;
use crate::search::algo::Algo;
// use crate::comms::uci::Uci;
use std::fmt;
use std::time;

#[derive(Clone, Debug)]
pub struct Game {
    starting_pos: Board,
    tags: Tags,
    moves: MoveList,
    outcome: Outcome,
}



impl Game {
    pub fn set_starting_pos(&mut self, board: &Board) -> Game {
        self.starting_pos = board.clone();
        self.starting_pos = board.clone();
        self.clone()
    }
    // pub const MOVE_SORTS: &'static [&'static str] = &["Natural", "PV from Prior Iteration", "MVV/LVA"];
}

impl Default for Game {
    fn default() -> Self {
        Game { 
            starting_pos: Board::default(),
            tags: Tags::default(),
            moves: MoveList::default(),
            outcome: Outcome::InProgress,

        }
    }
}


// [Event "GRENKE Chess Classic 2019"]
// [Site "Karlsruhe/Baden Baden GER"]
// [Date "2019.04.20"]
// [Round "1.2"]
// [White "Svidler, Peter"]
// [Black "Caruana, Fabiano"]
// [Result "1/2-1/2"]
// [WhiteTitle "GM"]
// [BlackTitle "GM"]
// [WhiteElo "2735"]
// [BlackElo "2819"]
// [ECO "B33"]
// [Opening "Sicilian"]
// [Variation "Pelikan (Lasker/Sveshnikov) variation"]
// [WhiteFideId "4102142"]
// [BlackFideId "2020009"]
// [EventDate "2019.04.20"]
// [WhiteACPL "252"]
// [BlackACPL "141"]
impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        
        // seven tag roster
        // writeln!(f, "[Event \"{}\"]", self.white.name())?;
        // writeln!(f, "[Site \"{}\"]", self.white.name())?;
        // writeln!(f, "[Date \"{}\"]", self.white.name())?;
        // // writeln!(f, "[Time \"{}\"]", self.white.name())?;
        // writeln!(f, "[Round \"{}\"]", self.white.name())?;
        // writeln!(f, "[White \"{}\"]", self.white.name())?;
        // writeln!(f, "[Black \"{}\"]", self.black.name())?;
        // writeln!(f, "[Result \"{}\"]", self.outcome().as_pgn())?;

        // // optional tag pairs
        // writeln!(f, "[Annotator \"{}\"]", self.outcome().as_pgn())?;
        // writeln!(f, "[PlyCount \"{}\"]", self.outcome().as_pgn())?;
        // writeln!(f, "[Time \"{}\"]", self.white.name())?;
        // writeln!(f, "[Termination \"{}\"]", self.white.name())?;
        // writeln!(f, "[Mode \"{}\"]", self.white.name())?;
        // if self.starting_pos != Catalog::starting_position() {
        //     writeln!(f, "[FEN \"{}\"]", self.starting_pos.to_fen())?;
        //     writeln!(f, "[SetUp \"1\"]")?;
        // }
        // writeln!(f, "{} {}", self.starting_pos.to_san_moves(&self.moves), self.board.outcome().as_pgn())?;
        Ok(())
    }
}
