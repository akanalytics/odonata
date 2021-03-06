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

    // FIXME! awful structure
    pub fn perfts() -> Vec<(Board, Vec<u32>)> {
        let mut vec = Vec::new();
        vec.push((BoardBuf::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap().as_board(), vec!{1, 20, 400, 8902, 197281, 4865609}));
        vec.push((BoardBuf::parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap().as_board(), vec!{1, 48, 2039, 97862, 4085603, 119060324}));
        vec.push((BoardBuf::parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap().as_board(), vec!{1, 14, 191, 2812, 43238, 674624, 11030083}));
        vec.push((BoardBuf::parse_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap().as_board(), vec!{1, 6, 264, 9467, 422333, 15833292}));
        vec.push((BoardBuf::parse_fen("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1").unwrap().as_board(), vec!{1, 6, 264, 9467, 422333, 15833292}));
        vec.push((BoardBuf::parse_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap().as_board(), vec!{1, 44, 1486, 62379, 2103487, 89941194}));
        vec.push((BoardBuf::parse_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10").unwrap().as_board(), vec!{1, 46, 2079, 89890, 3894594}));
        vec.push((BoardBuf::parse_fen("r1b2rk1/2p2ppp/p7/1p6/3P3q/1BP3bP/PP3QP1/RNB1R1K1 w - - 1 0").unwrap().as_board(), vec!{1, 40, 1334, 50182, 1807137}));
        vec
    }
}
