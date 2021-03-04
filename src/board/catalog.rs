use crate::board::{Board, CastlingRights, Color, Piece};
use crate::board::boardbuf::{BoardBuf};
use crate::board::{MoveEnum, Move};
use crate::globals::constants::*;


pub struct Catalog;

impl Catalog {

    pub const STARTING_POSITION_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    pub fn starting_position() -> Board {
        BoardBuf::parse_fen(Self::STARTING_POSITION_FEN).unwrap().as_board()
    }
}
