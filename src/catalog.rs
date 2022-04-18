use crate::bitboard::bitboard::Bitboard;
use crate::bitboard::castling::CastlingRights;
use crate::board::boardbuf::BoardBuf;
use crate::board::Board;
use crate::globals::constants::*;
use crate::position::Position;
use crate::tags::Tag;
use serde::{Deserialize, Serialize};
use std::fmt;

pub struct Catalog;

// http://computer-chess.org/doku.php?id=computer_chess:wiki:download:epd_contents

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CatalogSuite {
    BratkoKopec,
    WinAtChess,
    Iq81,
    Tricky,
    Checkmate,
    EndGame,
    Pin,
    Recogs,
    Move,
    Quiesce,
    See,
    MateIn2,
    MateIn3,
    MateIn4,
    Ches960,
    Perft,
    Bench,
    ExampleGame,
}

impl fmt::Display for CatalogSuite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Catalog {
    pub const STARTING_POSITION_FEN: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    pub fn starting_board() -> Board {
        Board::parse_fen(Self::STARTING_POSITION_FEN).unwrap().as_board()
    }

    pub fn starting_position() -> Position {
        Position::from_board(Self::starting_board())
    }

    pub fn test_position() -> Position {
        let epd = r#"3r2k1/6p1/1p1q3p/1Q1prp1P/Pp6/1Pp1P3/2PnBPP1/3RR1K1 w - - 0 29 
        bm Qd3;
        id 'https://lichess.org/IieZFrPE#56'
        "#;
        Position::parse_epd(epd).unwrap()
    }

    pub fn example_game() -> Vec<Position> {
        let str = r#"
rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1 acd 9;
rnbqkbnr/ppppp1pp/8/5p2/8/5N2/PPPPPPPP/RNBQKB1R w KQkq f6 0 2 acd 8;
rnbqkbnr/ppppp1pp/8/5p2/8/4PN2/PPPP1PPP/RNBQKB1R b KQkq - 0 2 acd 8;
rnbqkbnr/ppppp2p/6p1/5p2/8/4PN2/PPPP1PPP/RNBQKB1R w KQkq - 0 3 acd 8;
rnbqkbnr/ppppp2p/6p1/1B3p2/8/4PN2/PPPP1PPP/RNBQK2R b KQkq - 1 3 acd 8;
rnbqkbnr/pp1pp2p/6p1/1Bp2p2/8/4PN2/PPPP1PPP/RNBQK2R w KQkq c6 0 4 acd 9;
rnbqkbnr/pp1pp2p/6p1/1Bp2p2/8/4PN2/PPPP1PPP/RNBQ1RK1 b kq - 1 4 acd 8;
rnb1kbnr/pp1pp2p/1q4p1/1Bp2p2/8/4PN2/PPPP1PPP/RNBQ1RK1 w kq - 2 5 acd 8;
rnb1kbnr/pp1pp2p/1q4p1/1Bp2p2/8/2N1PN2/PPPP1PPP/R1BQ1RK1 b kq - 3 5 acd 7;
rnb1kbnr/pp1p3p/1q2p1p1/1Bp2p2/8/2N1PN2/PPPP1PPP/R1BQ1RK1 w kq - 0 6 acd 8;
rnb1kbnr/pp1p3p/1q2p1p1/1Bp2p2/3P4/2N1PN2/PPP2PPP/R1BQ1RK1 b kq d3 0 6 acd 7;
rnb1kbnr/1p1p3p/pq2p1p1/1Bp2p2/3P4/2N1PN2/PPP2PPP/R1BQ1RK1 w kq - 0 7 acd 7;
rnb1kbnr/1p1p3p/pq2p1p1/2p2p2/3P4/2N1PN2/PPP1BPPP/R1BQ1RK1 b kq - 1 7 acd 9;
rnb1kbnr/1p5p/pq1pp1p1/2p2p2/3P4/2N1PN2/PPP1BPPP/R1BQ1RK1 w kq - 0 8 acd 7;
rnb1kbnr/1p5p/pq1pp1p1/2P2p2/8/2N1PN2/PPP1BPPP/R1BQ1RK1 b kq - 0 8 acd 8;
rnb1kbnr/1p5p/p2pp1p1/2q2p2/8/2N1PN2/PPP1BPPP/R1BQ1RK1 w kq - 0 9 acd 8;
rnb1kbnr/1p5p/p2pp1p1/2q2p2/4P3/2N2N2/PPP1BPPP/R1BQ1RK1 b kq - 0 9 acd 8;
r1b1kbnr/1p5p/p1npp1p1/2q2p2/4P3/2N2N2/PPP1BPPP/R1BQ1RK1 w kq - 1 10 acd 6;
r1b1kbnr/1p5p/p1npp1p1/2q2P2/8/2N2N2/PPP1BPPP/R1BQ1RK1 b kq - 0 10 acd 7;
r1b1kbnr/1p5p/p1npp1p1/5q2/8/2N2N2/PPP1BPPP/R1BQ1RK1 w kq - 0 11 acd 7;
r1b1kbnr/1p5p/p1npp1p1/5q2/8/2NB1N2/PPP2PPP/R1BQ1RK1 b kq - 1 11 acd 6;
r1b1kbnr/1p5p/p1npp1p1/q7/8/2NB1N2/PPP2PPP/R1BQ1RK1 w kq - 2 12 acd 7;
r1b1kbnr/1p5p/p1npp1p1/q7/8/2NB1N2/PPPB1PPP/R2Q1RK1 b kq - 3 12 acd 6;
r1b1k1nr/1p4bp/p1npp1p1/q7/8/2NB1N2/PPPB1PPP/R2Q1RK1 w kq - 4 13 acd 6;
r1b1k1nr/1p4bp/p1npp1p1/q7/4N3/3B1N2/PPPB1PPP/R2Q1RK1 b kq - 5 13 acd 6;
r1b1k1nr/1pq3bp/p1npp1p1/8/4N3/3B1N2/PPPB1PPP/R2Q1RK1 w kq - 6 14 acd 6;
r1b1k1nr/1pq3bp/p1npp1p1/8/4NB2/3B1N2/PPP2PPP/R2Q1RK1 b kq - 7 14 acd 5;
r1b1k1nr/1pq3bp/p1np2p1/4p3/4NB2/3B1N2/PPP2PPP/R2Q1RK1 w kq - 0 15 acd 5;
r1b1k1nr/1pq3bp/p1np2p1/4p1B1/4N3/3B1N2/PPP2PPP/R2Q1RK1 b kq - 1 15 acd 8;
r1b1k1nr/1pq3bp/p1n3p1/3pp1B1/4N3/3B1N2/PPP2PPP/R2Q1RK1 w kq - 0 16 acd 7;
r1b1k1nr/1pq3bp/p1n3p1/3pp1B1/8/2NB1N2/PPP2PPP/R2Q1RK1 b kq - 1 16 acd 6;
r3k1nr/1pq3bp/p1n1b1p1/3pp1B1/8/2NB1N2/PPP2PPP/R2Q1RK1 w kq - 2 17 acd 6;
r3k1nr/1pq3bp/p1n1b1p1/3pp3/7B/2NB1N2/PPP2PPP/R2Q1RK1 b kq - 3 17 acd 6;
r3k2r/1pq1n1bp/p1n1b1p1/3pp3/7B/2NB1N2/PPP2PPP/R2Q1RK1 w kq - 4 18 acd 6;
r3k2r/1pq1n1bp/p1n1b1p1/3pp1N1/7B/2NB4/PPP2PPP/R2Q1RK1 b kq - 5 18 acd 6;
r3k1br/1pq1n1bp/p1n3p1/3pp1N1/7B/2NB4/PPP2PPP/R2Q1RK1 w kq - 6 19 acd 7;
r3k1br/1pq1n1bp/p1n3p1/3pp1N1/8/2NB2B1/PPP2PPP/R2Q1RK1 b kq - 7 19 acd 6;
r3k1br/1pq1n1b1/p1n3pp/3pp1N1/8/2NB2B1/PPP2PPP/R2Q1RK1 w kq - 0 20 acd 7;
r3k1br/1pq1n1b1/p1n3pp/3pp3/8/2NB1NB1/PPP2PPP/R2Q1RK1 b kq - 1 20 acd 6;
r3k1br/1p2n1b1/pqn3pp/3pp3/8/2NB1NB1/PPP2PPP/R2Q1RK1 w kq - 2 21 acd 5;
r3k1br/1p2n1b1/pqn3pp/3pp3/N7/3B1NB1/PPP2PPP/R2Q1RK1 b kq - 3 21 acd 6;
r3k1br/1p2n1b1/p1n3pp/q2pp3/N7/3B1NB1/PPP2PPP/R2Q1RK1 w kq - 4 22 acd 7;
r3k1br/1p2n1b1/p1n3pp/q2pp3/N7/2PB1NB1/PP3PPP/R2Q1RK1 b kq - 0 22 acd 7;
r3k1br/1p2n1b1/p1n3pp/q2p4/N3p3/2PB1NB1/PP3PPP/R2Q1RK1 w kq - 0 23 acd 7;
r3k1br/1p2n1b1/p1n3pp/q2p4/NP2p3/2PB1NB1/P4PPP/R2Q1RK1 b kq b3 0 23 acd 8;
r2qk1br/1p2n1b1/p1n3pp/3p4/NP2p3/2PB1NB1/P4PPP/R2Q1RK1 w kq - 1 24 acd 5;
r2qk1br/1p2n1b1/p1n3pp/2Np4/1P2p3/2PB1NB1/P4PPP/R2Q1RK1 b kq - 2 24 acd 4;
r2qk1br/1p2n3/p1n3pp/2Np4/1P2p3/2bB1NB1/P4PPP/R2Q1RK1 w kq - 0 25 acd 7;
r2qk1br/1N2n3/p1n3pp/3p4/1P2p3/2bB1NB1/P4PPP/R2Q1RK1 b kq - 0 25 acd 4;
r3k1br/1N2n3/pqn3pp/3p4/1P2p3/2bB1NB1/P4PPP/R2Q1RK1 w kq - 1 26 acd 6;
r3k1br/4n3/pqnN2pp/3p4/1P2p3/2bB1NB1/P4PPP/R2Q1RK1 b kq - 2 26 acd 6;
r4kbr/4n3/pqnN2pp/3p4/1P2p3/2bB1NB1/P4PPP/R2Q1RK1 w - - 3 27 acd 5;
r4kbr/4n3/pqnN2pp/3p4/1P2p3/2bB1NB1/P4PPP/R1Q2RK1 b - - 4 27 acd 6;
r4kbr/4n3/pqnN1bpp/3p4/1P2p3/3B1NB1/P4PPP/R1Q2RK1 w - - 5 28 acd 5;
r4kbr/4n3/pqnN1bpp/1P1p4/4p3/3B1NB1/P4PPP/R1Q2RK1 b - - 0 28 acd 5;
r4kbr/4n3/1qnN1bpp/1p1p4/4p3/3B1NB1/P4PPP/R1Q2RK1 w - - 0 29 acd 5;
r4kbr/4n3/1qnN1bpp/1p1p4/4p3/3B1NB1/P4PPP/1RQ2RK1 b - - 1 29 acd 6;
r4kbr/4n3/1qnN1bpp/1p1p4/8/3p1NB1/P4PPP/1RQ2RK1 w - - 0 30 acd 6;
r4kbr/4n3/1qn2bpp/1N1p4/8/3p1NB1/P4PPP/1RQ2RK1 b - - 0 30 acd 5;
r4kbr/4n3/2n2bpp/qN1p4/8/3p1NB1/P4PPP/1RQ2RK1 w - - 1 31 acd 7;
r4kbr/4n3/2n2bpp/qN1p4/8/P2p1NB1/5PPP/1RQ2RK1 b - - 0 31 acd 6;
r4kbr/4n3/2n2b1p/qN1p2p1/8/P2p1NB1/5PPP/1RQ2RK1 w - - 0 32 acd 6;
r4kbr/4n3/2n2b1p/qN1p2p1/8/P2pQNB1/5PPP/1R3RK1 b - - 1 32 acd 7;
r5br/4n1k1/2n2b1p/qN1p2p1/8/P2pQNB1/5PPP/1R3RK1 w - - 2 33 acd 6;
r5br/4n1k1/2n2b1p/qN1p2p1/8/P2pQ1B1/3N1PPP/1R3RK1 b - - 3 33 acd 7;
r6r/4n1kb/2n2b1p/qN1p2p1/8/P2pQ1B1/3N1PPP/1R3RK1 w - - 4 34 acd 6;
r6r/2B1n1kb/2n2b1p/qN1p2p1/8/P2pQ3/3N1PPP/1R3RK1 b - - 5 34 acd 7;
r6r/2B1n1kb/2n2b1p/1N1p2p1/q7/P2pQ3/3N1PPP/1R3RK1 w - - 6 35 acd 7;
r6r/2B1n1kb/2n2b1p/1N1p2p1/q7/P2pQ3/3N1PPP/1RR3K1 b - - 7 35 acd 7;
r1r5/2B1n1kb/2n2b1p/1N1p2p1/q7/P2pQ3/3N1PPP/1RR3K1 w - - 8 36 acd 6;
r1r5/4n1kb/2nB1b1p/1N1p2p1/q7/P2pQ3/3N1PPP/1RR3K1 b - - 9 36 acd 6;
r1r5/4n1kb/2nB1b1p/1N1p2p1/6q1/P2pQ3/3N1PPP/1RR3K1 w - - 10 37 acd 5;
r1r5/2N1n1kb/2nB1b1p/3p2p1/6q1/P2pQ3/3N1PPP/1RR3K1 b - - 11 37 acd 6;
2r5/r1N1n1kb/2nB1b1p/3p2p1/6q1/P2pQ3/3N1PPP/1RR3K1 w - - 12 38 acd 5;
2r5/r1N1B1kb/2n2b1p/3p2p1/6q1/P2pQ3/3N1PPP/1RR3K1 b - - 0 38 acd 6;
2r5/2r1B1kb/2n2b1p/3p2p1/6q1/P2pQ3/3N1PPP/1RR3K1 w - - 0 39 acd 7;
2r5/2r3kb/2n2B1p/3p2p1/6q1/P2pQ3/3N1PPP/1RR3K1 b - - 0 39 acd 6;
2r5/2r4b/2n2k1p/3p2p1/6q1/P2pQ3/3N1PPP/1RR3K1 w - - 0 40 acd 5;
2r5/2r4b/2n2k1p/2Qp2p1/6q1/P2p4/3N1PPP/1RR3K1 b - - 1 40 acd 6;
2r5/2r4b/2n2k1p/2Qp2p1/5q2/P2p4/3N1PPP/1RR3K1 w - - 2 41 acd 5;
2r5/2r4b/2n2k1p/2Qp2p1/5q2/P2p4/3N1PPP/1R1R2K1 b - - 3 41 acd 6;
2r5/2r4b/5k1p/2Qp2p1/3n1q2/P2p4/3N1PPP/1R1R2K1 w - - 4 42 acd 7;
2r5/2r4b/5k1p/3Q2p1/3n1q2/P2p4/3N1PPP/1R1R2K1 b - - 0 42 acd 6;
2r5/2r4b/5k1p/3Q2p1/5q2/P2p4/3NnPPP/1R1R2K1 w - - 1 43 acd 9;
2r5/2r4b/5k1p/3Q2p1/5q2/P2p4/3NnPPP/1R1R3K b - - 2 43 acd 6;
2r5/2r4b/5k1p/3Q2p1/5q2/P1np4/3N1PPP/1R1R3K w - - 3 44 acd 7;
2r5/2r4b/5k1p/3Q2p1/5q2/P1np2P1/3N1P1P/1R1R3K b - - 0 44 acd 8;
2r5/2r4b/5k1p/3Q2p1/8/P1np2P1/3N1q1P/1R1R3K w - - 0 45 acd 8;
2r5/2r4b/3Q1k1p/6p1/8/P1np2P1/3N1q1P/1R1R3K b - - 1 45 acd 8;
2r5/2r3kb/3Q3p/6p1/8/P1np2P1/3N1q1P/1R1R3K w - - 2 46 acd 7;
2r5/2r3kb/7p/4Q1p1/8/P1np2P1/3N1q1P/1R1R3K b - - 3 46 acd 8;
2r3k1/2r4b/7p/4Q1p1/8/P1np2P1/3N1q1P/1R1R3K w - - 4 47 acd 7;
2r3k1/2r4b/4Q2p/6p1/8/P1np2P1/3N1q1P/1R1R3K b - - 5 47 acd 8;
2r4k/2r4b/4Q2p/6p1/8/P1np2P1/3N1q1P/1R1R3K w - - 6 48 acd 7;
2r4k/2r4b/7p/4Q1p1/8/P1np2P1/3N1q1P/1R1R3K b - - 7 48 acd 8;
2r4k/6rb/7p/4Q1p1/8/P1np2P1/3N1q1P/1R1R3K w - - 8 49 acd 6;
2r4k/6rb/7p/4Q1p1/8/P1np2P1/3N1q1P/2RR3K b - - 9 49 acd 9;
2r4k/6r1/7p/4Q1p1/4b3/P1np2P1/3N1q1P/2RR3K w - - 10 50 acd 8;
2r4k/6r1/7p/6p1/4Q3/P1np2P1/3N1q1P/2RR3K b - - 0 50 acd 9;
2r4k/6r1/7p/6p1/4n3/P2p2P1/3N1q1P/2RR3K w - - 0 51 acd 11;
2R4k/6r1/7p/6p1/4n3/P2p2P1/3N1q1P/3R3K b - - 0 51 acd 11;
2R5/6rk/7p/6p1/4n3/P2p2P1/3N1q1P/3R3K w - - 1 52 acd 10;
2R5/6rk/7p/6p1/4N3/P2p2P1/5q1P/3R3K b - - 0 52 acd 8;
2R5/6rk/7p/6p1/4N3/P2p1qP1/7P/3R3K w - - 1 53 acd 12;
2R5/6rk/7p/6p1/4N3/P2p1qP1/7P/3R2K1 b - - 2 53 acd 11;
2R5/6rk/7p/6p1/4N3/P2p2P1/7P/3q2K1 w - - 0 54 acd 10;
2R5/6rk/7p/6p1/4N3/P2p2P1/5K1P/3q4 b - - 1 54 acd 9;
2R5/6rk/7p/6p1/4N3/P2p2P1/4qK1P/8 w - - 2 55 acd 10;
2R5/6rk/7p/6p1/4N3/P2p2P1/4q2P/6K1 b - - 3 55 acd 9;
2R5/6rk/7p/6p1/4N3/P2p2P1/7P/4q1K1 w - - 4 56 acd 11;
2R5/6rk/7p/6p1/4N3/P2p2P1/6KP/4q3 b - - 5 56 acd 10;
2R5/6rk/7p/6p1/4q3/P2p2P1/6KP/8 w - - 0 57 acd 8;
2R5/6rk/7p/6p1/4q3/P2p2P1/7P/6K1 b - - 1 57 acd 9;
2R5/5r1k/7p/6p1/4q3/P2p2P1/7P/6K1 w - - 2 58 acd 8;
4R3/5r1k/7p/6p1/4q3/P2p2P1/7P/6K1 b - - 3 58 acd 7;
4q3/5r1k/7p/6p1/8/P2p2P1/7P/6K1 w - - 0 59 acd 6;
4q3/5r1k/7p/6p1/8/P2p2P1/6KP/8 b - - 1 59 acd 5;
8/5r1k/7p/6p1/4q3/P2p2P1/6KP/8 w - - 2 60 acd 4;
8/5r1k/7p/6p1/4q3/P2p2PK/7P/8 b - - 3 60 acd 3;
8/5r1k/7p/5qp1/8/P2p2PK/7P/8 w - - 4 61 acd 2;
8/5r1k/7p/5qp1/8/P2p2P1/6KP/8 b - - 5 61 acd 1;
"#;
        // final mate position
        // 8/5r1k/7p/6p1/8/P2p2P1/6KP/5q2 w - - 6 62

        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn white_starting_position() -> Board {
        // FIXME: set calls
        Board::parse_fen(Self::STARTING_POSITION_FEN)
            .unwrap()
            .set(Bitboard::RANK_7 | Bitboard::RANK_8, "................")
            .unwrap()
            .as_board()
    }

    pub fn black_starting_position() -> Board {
        // FIXME: set calls
        Board::parse_fen(Self::STARTING_POSITION_FEN)
            .unwrap()
            .set(Bitboard::RANK_1 | Bitboard::RANK_2, "................")
            .unwrap()
            .as_board()
    }

    pub fn positions(suite: CatalogSuite) -> Vec<Position> {
        match suite {
            CatalogSuite::BratkoKopec => Self::bratko_kopec(),
            CatalogSuite::WinAtChess => Self::win_at_chess(),
            CatalogSuite::Iq81 => Self::iq(),
            CatalogSuite::Tricky => Self::tricky(),
            CatalogSuite::Checkmate => Self::checkmates(),
            CatalogSuite::EndGame => Self::end_games(),
            CatalogSuite::Pin => Self::pins(),
            CatalogSuite::Recogs => Self::recogs(),
            CatalogSuite::Move => Self::moves(),
            CatalogSuite::Quiesce => Self::quiesce(),
            CatalogSuite::See => Self::see(),
            CatalogSuite::MateIn2 => Self::mate_in_2(),
            // CatalogSuite::MateIn3 => Self::mate_in_3(),
            CatalogSuite::MateIn4 => Self::mate_in_4(),
            // CatalogSuite::MateIn4 => Self::chess960(),
            // CatalogSuite::MateIn4 => Self::perft(),
            CatalogSuite::Bench => Self::bench(),
            CatalogSuite::ExampleGame => Self::example_game(),
            _ => Vec::new(),
        }
    }

    pub fn tricky() -> Vec<Position> {
        let str = r#"
3n3k/rb5b/1p5r/pPp5/PpPp1p2/3PpPp1/RN2P1P1/QBNR1BK1 w - - 0 1 id 'TR.01'; c0 'https://chess.stackexchange.com/questions/29530'; c1 'ce<0 as black can checkmate';
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn pawn_structure() -> Vec<Position> {
        let str = r#"
        k7/p7/P7/8/8/8/P7/K7 w - - 0 1 id 'PAWN.01'; c0 'isolated'; c1 '2'; c2 '1';
        k7/p7/P7/8/8/8/P7/K7 w - - 0 1 id 'PAWN.01'; c0 'passed';   c1 '0'; c2 '0';
        k7/p7/P7/8/8/8/P7/K7 w - - 0 1 id 'PAWN.01'; c0 'doubled';  c1 '1'; c2 '0';
        k7/p5p1/8/7P/8/3p4/P2P4/K7 w - - 0 1 id 'PAWN.02'; c0 'isolated'; c1 '3'; c2 '3';
        k7/p5p1/8/7P/8/3p4/P2P4/K7 w - - 0 1 id 'PAWN.02'; c0 'passed';   c1 '0'; c2 '0';
        k7/p5p1/8/7P/8/3p4/P2P4/K7 w - - 0 1 id 'PAWN.02'; c0 'doubled';  c1 '0'; c2 '0';
        k7/p5p1/8/7P/3P2p1/3p4/P2PP3/K7 w - - 0 1 id 'PAWN.03'; c0 'isolated'; c1 '2'; c2 '4';
        k7/p5p1/8/7P/3P2p1/3p4/P2PP3/K7 w - - 0 1 id 'PAWN.03'; c0 'passed';   c1 '1'; c2 '1';
        k7/p5p1/8/7P/3P2p1/3p4/P2PP3/K7 w - - 0 1 id 'PAWN.03'; c0 'doubled';  c1 '1'; c2 '1';
        2k5/ppp2ppp/8/4P3/P7/3p4/PP5P/K7 b - - 0 1 id 'PAWN.04'; c0 'isolated'; c1 '2'; c2 '0';
        2k5/ppp2ppp/8/4P3/P7/3p4/PP5P/K7 b - - 0 1 id 'PAWN.04'; c0 'passed';   c1 '0'; c2 '1';
        2k5/ppp2ppp/8/4P3/P7/3p4/PP5P/K7 b - - 0 1 id 'PAWN.04'; c0 'doubled';  c1 '1'; c2 '0';
        2k5/ppp2p1p/5p2/5p1P/3P3P/3P3P/P2P3P/K7 w - - 0 1 id 'PAWN.05'; c0 'isolated'; c1 '8'; c2 '4'; 
        2k5/ppp2p1p/5p2/5p1P/3P3P/3P3P/P2P3P/K7 w - - 0 1 id 'PAWN.05'; c0 'passed';   c1 '0'; c2 '3'; 
        2k5/ppp2p1p/5p2/5p1P/3P3P/3P3P/P2P3P/K7 w - - 0 1 id 'PAWN.05'; c0 'doubled';  c1 '5'; c2 '2'; 
        k7/p2p3p/3p3p/3p3p/5P1p/5P2/PPP2P1P/2K5 b - - 0 1 id 'PAWN.06'; c0 'isolated'; c1 '4'; c2 '8';
        k7/p2p3p/3p3p/3p3p/5P1p/5P2/PPP2P1P/2K5 b - - 0 1 id 'PAWN.06'; c0 'passed';   c1 '3'; c2 '0';
        k7/p2p3p/3p3p/3p3p/5P1p/5P2/PPP2P1P/2K5 b - - 0 1 id 'PAWN.06'; c0 'doubled';  c1 '2'; c2 '5';
        k7/p5p1/4P3/8/3P4/3p4/P2PP1p1/K7 w - - 0 1 id 'PAWN.07'; c0 'passed_r5';  c1 '0'; c2 '0';
        k7/p5p1/4P3/8/3P4/3p4/P2PP1p1/K7 w - - 0 1 id 'PAWN.07'; c0 'passed_r6';  c1 '1'; c2 '0';
        k7/p5p1/4P3/8/3P4/3p4/P2PP1p1/K7 w - - 0 1 id 'PAWN.07'; c0 'passed_r7';  c1 '0'; c2 '1';
        k7/3p4/PPp3P1/1p3P2/4P3/8/6pp/K7 w - - 0 1 id 'PAWN.08'; c0 'connected_r67';   c1 '1'; c2 '0';
        k7/3p4/PPp3P1/1p3P2/4P3/8/6pp/K7 w - - 0 1 id 'PAWN.08'; c0 'connected_r345';  c1 '1'; c2 '2';
        k7/3p4/PPp3P1/1p3P2/4P3/8/6pp/K7 w - - 0 1 id 'PAWN.08'; c0 'backward_half_open';  c1 '0'; c2 '0';
        k7/8/8/5p2/6p1/6Pp/7P/K7 w - - 0 1 id 'PAWN.09'; c0 'backward_half_open';  c1 '0'; c2 '1';
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn checkmates() -> Vec<Position> {
        let str = r#"
k7/Q7/K7/8/8/8/8/8 b - - 0 1
K7/7r/8/8/8/8/8/rr5k w - - 0 1
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn stalemates() -> [Board; 2] {
        [
            Board::parse_fen("k7/8/1Q6/8/8/8/8/7K b - - 0 1").unwrap().as_board(),
            Board::parse_fen("K7/7r/8/8/8/8/8/1r5k w - - 0 1").unwrap().as_board(),
        ]
    }

    pub fn end_games() -> Vec<Position> {
        // http://wtharvey.com/m8n2.txt
        let strs = &[
            // https://www.stmintz.com/ccc/index.php?id=14581
            "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1
            bm Kb1;
            dm 32;
            acd 23;
            id 'Lasker-Reichhelm Position Fine#70';",
            ];
        Position::parse_many_epd(strs).unwrap()
  }

    pub fn pins() -> Vec<Position> {
        let str = r#"
k6b/8/n4N2/8/Rr1K1P1b/2R5/8/q7 w - - 0 1 id "PIN.01"; c0 "Pins"; Sq c3 f6;
k2r4/8/3B4/3B4/3KQr2/5n2/8/8 w - - 0 1 id "PIN.02"; c0 "Pins"; Sq e4;
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn discovered_check() -> Vec<Position> {
        let str = r#"
        k2N1R1b/8/n4N2/8/Rr1K1P1b/2R5/5n2/q5b1 w - - 0 1 id "DC.01"; c0 "Discovered checks"; Sq f2;
        k2N1R1b/8/n4N2/8/Rr1K1P1b/2R5/5n2/q5b1 b - - 0 1 id "DC.01"; c0 "Discovered checks"; Sq d8;
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn recogs() -> Vec<Position> {
        let str = r#"
8/NN6/8/8/8/2K2nk1/4P3/8 w - - 0 1; id 'RECOG.01'; am e2f3; am exf3; c0 'white shouldnt take knight as recapture of pawn makes it KNN v k';
k7/1p6/3N4/8/8/8/6NB/K7 w - - 5 1; id 'RECOG.02'; bm Nxb7; c0 'white should take pawn to leave winning KBN v k';
k7/8/K1p5/8/3N4/8/6N1/7B w - - 5 1; id 'RECOG.03'; am Nxc6; bm Kb6; c0 'white shouldnt take pawn with knight as it triggers stalemate';
k1K5/8/8/2p5/8/6B1/5B2/8 w - - 0 1;  id 'RECOG.04'; bm Bxc5; c0 'white should force stalemate by capturing pawn on c5 as black could win';
6k1/7n/8/8/8/8/1N6/B5KR w - - 26 1; id 'RECOG.05'; bm Rxh7; acd 7; c0 'winning position despite king recapture';
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    // given in UCI format so that we dont need a working "legal moves" to parse
    pub fn moves() -> Vec<Position> {
        let str = concat!(r#"
8/8/p7/8/8/P7/8/8 w - - 0 1 id "LEGAL.01"; c0 "Pawn push (w)"; c1 "a3a4";
8/8/p7/8/8/P7/8/8 b - - 0 1 id "LEGAL.02"; c0 "Pawn push (b)"; c1 "a6a5";
8/p7/8/8/8/8/P7/8 w - - 0 1 id "LEGAL.03"; c0 "Pawn double push (w)"; c1 "a2a3 a2a4";
8/p7/8/8/8/8/P7/8 b - - 0 1 id "LEGAL.04"; c0 "Pawn double push (b)"; c1 "a7a6 a7a5";
8/8/8/8/8/1p6/B7/8 w - - 0 1 id "LEGAL.05"; c0 "Bishop"; c1 "a2b3 a2b1";
8/8/8/8/8/1p6/1Rp5/8 w - - 0 1 id "LEGAL.06"; c0 "Rook"; c1 "b2a2 b2b3 b2b1 b2c2";
8/8/8/8/8/1pp5/1Q1p4/8 w - - 0 1 id "LEGAL.07"; c0 "Queen"; c1 "b2a2 b2a1 b2a3 b2b3 b2b1 b2c3 b2c2 b2c1 b2d2";
k7/8/8/8/8/8/8/K7 w - - 0 1 id "LEGAL.08"; c0 "King"; c1 "a1a2 a1b1 a1b2";
8/8/8/8/8/8/2k5/K7 w - - 0 1 id "LEGAL.09"; c0 "King and king check"; c1 "a1a2";
k7/8/8/8/8/8/7r/K7 w - - 0 1 id "LEGAL.10"; c0 "King and rook check"; c1 "a1b1";
k7/8/8/8/8/8/r7/K7 w - - 0 1 id "LEGAL.11"; c0 "King and take rook checker"; c1 "a1a2 a1b1";
8/8/8/8/8/1k6/r7/K7 w - - 0 1 id "LEGAL.12"; c0 "King and cannot take rook checker"; c1 "a1b1";
4kr1r/8/8/3pP3/8/8/r7/6K1 w k d6 0 1 id "LEGAL.13"; c0 "Pawn ep"; c1 "e5d6 e5e6";
rk6/8/8/8/N7/1pp5/8/K7 w - - 0 1 id "LEGAL.20"; c0 "Pinned knight"; c1 "a1b1";
rk6/8/8/8/B7/1pp5/8/K7 w - - 0 1 id "LEGAL.21"; c0 "Pinned bishop"; c1 "a1b1";
rk6/8/8/8/8/Rpp5/8/K7 w - - 0 1 id "LEGAL.22"; c0 "Pinned rook"; c1 "a1b1 a3a2 a3a4 a3a5 a3a6 a3a7 a3a8";
rk6/8/8/8/8/Qpp5/8/K7 w - - 0 1 id "LEGAL.23"; c0 "Pinned queen"; c1 "a1b1 a3a2 a3a4 a3a5 a3a6 a3a7 a3a8";
rrk5/8/8/8/8/P7/8/K7 w - - 0 1 id "LEGAL.24"; c0 "Pinned pawn"; c1 "a1a2 a3a4";
rk6/8/8/8/2Q5/8/8/K6r w - - 0 1 id "LEGAL.30"; c0 "Double check"; c1 "a1b2";
rk6/8/8/8/2Q5/8/7r/K6r w - - 0 1 id "LEGAL.31"; c0 "Checkmate"; c1 "";
8/8/8/8/r7/1P6/7r/K1k5 w - - 0 1 id "LEGAL.40"; c0 "Pawn capture rook checker"; c1 "b3a4";
8/8/8/8/8/kn6/2P4r/K7 w - - 0 1 id "LEGAL.41"; c0 "Pawn capture knight checker"; c1 "a1b1 c2b3";
1k6/8/8/8/8/1n6/PP6/K7 w - - 0 1 id "LEGAL.42"; c0 "Pawn capture knight checker #2"; c1 "a1b1 a2b3";
r3k2r/p3bpp1/2q1p1b1/1ppPP1B1/7P/1n3NR1/PP2NP2/K1QR4 w kq - 1 2 id "LEGAL.42"; c0 "Pawn capture knight checker #3"; c1 "a2b3";
1r5k/8/7r/K1pP3r/7r/8/8/8 w - c6 0 1 id "LEGAL.50"; c0 "Pawn pinned ep"; c1 "d5d6";
1r5k/8/7r/K1pP1p1r/7r/8/8/8 w - c6 0 1 id "LEGAL.51"; c0 "Pawn not pinned ep"; c1 "d5d6 d5c6";
"#, 
"R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1 id 'LEGAL.60'; c0 'Max legal moves?'; c1 'a3a2 a3a4 a3a5 a3a6 a3a7 a3b2 a3b3 a3b4 \
a3c3 a3c5 a3d3 a3d6 a3e3 a3e7 a3f8 a8a4 a8a5 a8a6 a8a7 a8b8 a8c8 a8d8 a8e8 a8f8 a8g8 b1a2 b1c2 b1d3 b1e4 b1f5 b6a5 b6a6 b6a7 b6b2 b6b3 \
b6b4 b6b5 b6b7 b6b8 b6c5 b6c6 b6c7 b6d4 b6d6 b6d8 b6e3 b6e6 b6f2 b6f6 c1a2 c1b3 c1d3 c1e2 c4a2 c4a4 c4a6 c4b3 c4b4 c4b5 c4c2 c4c3 c4c5 \
c4c6 c4c7 c4c8 c4d3 c4d4 c4d5 c4e2 c4e4 c4e6 c4f4 c4f7 c4g4 c4g8 d1b2 d1c3 d1e3 d1f2 d2a5 d2b2 d2b4 d2c2 d2c3 d2d3 d2d4 d2d5 d2d6 d2e1 \
d2e2 d2e3 d2f2 d2f4 d2g2 d2g5 d2h2 d2h6 d7a4 d7a7 d7b5 d7b7 d7c6 d7c7 d7c8 d7d3 d7d4 d7d5 d7d6 d7d8 d7e6 d7e7 d7e8 d7f5 d7f7 d7g4 d7g7 \
d7h3 d7h7 e5a5 e5b2 e5b5 e5b8 e5c3 e5c5 e5c7 e5d4 e5d5 e5d6 e5e1 e5e2 e5e3 e5e4 e5e6 e5e7 e5e8 e5f4 e5f5 e5f6 e5g3 e5g5 e5g7 e5h2 e5h5 \
f1e1 f1e2 f1f2 f1g2 f3b3 f3b7 f3c3 f3c6 f3d3 f3d5 f3e2 f3e3 f3e4 f3f2 f3f4 f3f5 f3f6 f3f7 f3f8 f3g2 f3g3 f3g4 f3h1 f3h3 f3h5 g1c5 g1d4 \
g1e3 g1f2 g1h2 g6c2 g6c6 g6d3 g6d6 g6e4 g6e6 g6e8 g6f5 g6f6 g6f7 g6g2 g6g3 g6g4 g6g5 g6g7 g6g8 g6h5 g6h6 g6h7 h4d4 h4d8 h4e1 h4e4 h4e7 \
h4f2 h4f4 h4f6 h4g3 h4g4 h4g5 h4h1 h4h2 h4h3 h4h5 h4h6 h4h7 h8b8 h8c8 h8d8 h8e8 h8f8 h8g8 h8h5 h8h6 h8h7';");

        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn move_ordering() -> Vec<Position> {
        let str = r#"
r3k2r/1P6/1N3P2/2Pp4/3QP2Q/5B2/8/R3K2R w KQkq d6 0 1 id "MO.01"; c0 "Promos";
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    // FIXME! QS.10 - recaptures
    pub fn quiesce() -> Vec<Position> {
        let str = r#"
r4rk1/pp2qppp/2nbbn2/3Np3/2B1P3/P4N1P/1P1B1PP1/R2QR1K1 b - - 6 16 acd 0; am Nxd5; bm Qd8; pv; ce 0; id 'QS.00'; c0 'pawn fork'; c1 'https://lichess.org/q5XvkApk/black#31';
k7/p7/8/8/8/8/P7/K7 w - - 0 1 acd 0; pv; id QS.01; c0 'no captures';
k7/p7/8/8/8/8/Q7/K7 w - - 0 1 acd 0; pv; id QS.02; c0 'only bad captures';
2k5/p7/8/8/8/8/Q7/K7 w - - 0 1 acd 0; pv Qa7; id QS.03; c0 '1 good capture';
1k6/p7/4p3/8/8/8/Q7/K7 w - - 0 1 acd 0; pv Qe6; id QS.04; c0 '1 good capture 1 bad';
1k6/q7/8/8/8/8/Q7/K7 w - - 0 1 acd 0; pv; id QS.05; c0 'even exchange not made';
1kr1r3/8/8/8/8/8/r2K3P/3P4 w - - 0 1 acd 0; pv Kd3 Rxh2; id QS.06; c0 'in check then capture';
1kr1r3/8/8/8/8/5N2/r2K3P/8 w - - 0 1 acd 0; pv Kd3; id QS.07; c0 'in check then bad capture'; c1 'check evasion only (any sq good)';
k7/8/8/8/2p5/1b6/P7/K7 w - - 0 1 acd 0; pv axb3 cxb3; id QS.08; c0 'capture and recapture';
k7/8/8/8/p7/1p6/P1P5/K7 w - - 0 1 acd 0; pv cxb3; id QS.09; c0 'capture then bad 2 x recapture';
k7/p7/1p6/P1B5/8/8/8/K6R w - - 0 1 acd 0; pv axb6; id QS.10; c0 'capture then good 2 x recapture'; c1 'R means game not drawn pv axb6 axb6 Bxb6; ';
k7/p7/1p6/P1B5/8/8/8/K7 w - - 0 1 acd 0; pv; id QS.11; c0 'capture then bad recapture due to draw'; c1 'game a draw if white exchanges';
r2qkb1r/pp2nNpp/3p4/2pN2B1/2BnP3/3P4/PPP2PPP/R2bK2R b KQkq - 1 1 acd 0; pv ; c1 Kxf7 Rxd1 Nxc2+ Ke2; id QS.90; c0 'from mate in 2 - mates not checked in qsearch';
2b1k3/3r4/3b4/3p4/8/8/3Q4/R3K3 w - - 0 1 acd 0; id QS.91; pv Qxd5; c0 'qsearch fail!'; c1 'chess.stackexchange.com/questions/29602';
"#;
        // rnb1k2r/pp3ppp/4p3/q2p4/1bpPnB2/2N1PN2/PPPQ1PPP/2KR1B1R w kq - 1 9 "QS.01"; c0 "xray"; c1 "https://lichess.org/PqvjbdtB#16";
        // rnb1k2r/pp3ppp/4p3/3pB3/2pPn3/2P1PN2/q1P1QPPP/2KR1B1R b kq - 1 11 "QS.02"; c0 "bug?";
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn see() -> Vec<Position> {
        let str = r#"
7k/8/8/8/8/q7/8/R6K w - - 0 1 sm a1a3; ce 900; id 'SEE.00'; c0 '';
7k/8/8/8/1p6/q7/8/R6K w - - 0 1 sm a1a3; ce 400; id 'SEE.01'; c0 'Q=900 - R=500';
7k/8/8/8/1p6/q7/2N5/R6K w - - 0 1 sm a1a3; ce 500; id 'SEE.02'; c0 '+q+p -R = 900 - 500 + 100  = 500';
7k/8/8/8/1q6/p7/2N5/R6K w - - 0 1 sm a1a3; ce 100; id 'SEE.03'; c0 'RN v pq: +p  = +100 (retake by queen doesnt occur)';
1k5r/7r/8/8/R6p/8/8/K6R w - - 12 1 sm h1h4; ce -400; id 'SEE.04'; c0 'xray';
8/8/8/4pk2/5B2/8/8/K7 w - - 12 1 sm f4e5; ce -225; id 'SEE.05'; c0 'without xray onto king';
8/8/8/4pk2/5B2/8/7Q/K7 w - - 12 1 sm f4e5; ce 100; id 'SEE.06'; c0 'with xray onto king';
bb3rkr/pp2nppp/4pn2/2qp4/2P5/3RNN2/PP2PPPP/BBQ3KR w - - 0 8 sm c4d5; ce 0; id 'SEE.07'; c0 'shortcoming in SEE, as PxP exposes queen to capture';
k7/5n2/3p4/4p3/4K1N1/8/8/8 w - - 0 8 sm g4e5; ce -225; id 'SEE.08'; c0 'loses knight for pawn, as king cannot recapture because of check';
k7/5n2/8/4p3/4K1N1/8/8/8 w - - 0 8  sm g4e5; ce 100; id 'SEE.09'; c0 'king can capture as wont be in check';
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn draws() -> Vec<Position> {
        let str = r#"
6k1/6p1/8/6KQ/1r6/q2b4/8/8 w - - 2 1 bm Qe8+; pv Qe8+ Kh7 Qg6+ Kh8 Qe8+ Kh7; id DR.01; c0 'https://en.wikipedia.org/wiki/Perpetual_check'; c1 'Reinfeld 1958';   
5rk1/1rP3pp/p4n2/3Pp3/1P2Pq2/2Q4P/P5P1/R3R1K1 b - - 2 1 id DR.02; bm Rxc7; ce 0; pv Rxc7 Qxc7 Ng4 hxg4 Qf2+ Kh2 Qh4+ Kg1 Qf2+; c0 'https://en.wikipedia.org/wiki/Perpetual_check'; c1 'Unzicker versus Averbakh';
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 id DR.03; sv 1. d4 Nf6 2. c4 g6 3. Nc3 d5 4. cxd5 Nxd5 5. e4 Nxc3 6. bxc3 Bg7 7. Nf3 O-O 8. Bc4 Nc6 9. O-O Bg4 10. Ba3 Rc8 11. Bc5 Na5 12. Bb5 a6 13. Be2 Bxf3 14. Bxf3 Qd7 15. Bg4 f5 16. exf5 gxf5 17. Bf3 Rfe8 18. Bxb7 Nxb7 19. Qb3+ e6 20. Qxb7 a5 21. Rfb1 a4 22. Rb4 Ra8 23. Qf3 Rab8 24. Rc1 Rxb4 25. cxb4 Qd5 26. Qh5 Rc8 27. a3 Qb3 28. Qe2 Re8 29. Qb5 Rd8 30. Be7 Bh6 31. Re1 Qd5 32. Qe2 Re8 33. Bf6 Qd7 34. Be5 Bf8 35. Rc1 Bd6 36. Bxd6 cxd6 37. Qa6 f4 38. b5 f3 39. g3 Rb8 40. Rb1 e5 41. Qxa4 d5 42. Qb3 e4 43. b6 Kh8 44. Kh1 Qh3 45. Rg1 Qe6 46. Rb1 Qh3 47. Rg1 Qe6 48. Rb1 Qh3; 
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 id DR.04; sv 1. d4 Nf6 2. c4 g6 3. Nc3 d5 4. cxd5 Nxd5 5. e4 Nxc3 6. bxc3 Bg7 7. Nf3 O-O 8. Bc4 Nc6 9. O-O Bg4 10. Ba3 Rc8 11. Bc5 Na5 12. Bb5 a6 13. Be2 Bxf3 14. Bxf3 Qd7 15. Bg4 f5 16. exf5 gxf5 17. Bf3 Rfe8 18. Bxb7 Nxb7 19. Qb3+ e6 20. Qxb7 a5 21. Rfb1 a4 22. Rb4 Ra8 23. Qf3 Rab8 24. Rc1 Rxb4 25. cxb4 Qd5 26. Qh5 Rc8 27. a3 Qb3 28. Qe2 Re8 29. Qb5 Rd8 30. Be7 Bh6 31. Re1 Qd5 32. Qe2 Re8 33. Bf6 Qd7 34. Be5 Bf8 35. Rc1 Bd6 36. Bxd6 cxd6 37. Qa6 f4 38. b5 f3 39. g3 Rb8 40. Rb1 e5 41. Qxa4 d5 42. Qb3 e4 43. b6 Kh8 44. Kh1 Qh3 45. Rg1 Qe6 46. Rb1 Qh3 47. Rg1 Qe6; 
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    // TESTSUITE http://www.utzingerk.com/test.htm
    pub fn bratko_kopec() -> Vec<Position> {
        // source: "Sanny: Some test positions for you" google groups
        let str = r#"
1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - bm Qd1+; id "BK.01"; acd 7;
3r1k2/4npp1/1ppr3p/p6P/P2PPPP1/1NR5/5K2/2R5 w - - bm d5; id "BK.02"; acd 10;
2q1rr1k/3bbnnp/p2p1pp1/2pPp3/PpP1P1P1/1P2BNNP/2BQ1PRK/7R b - - bm f5; id "BK.03"; acd 8;
rnbqkb1r/p3pppp/1p6/2ppP3/3N4/2P5/PPP1QPPP/R1B1KB1R w KQkq - bm e6; id "BK.04"; acd 8;
r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - - bm Nd5 a4; id "BK.05"; acd 7;
2r3k1/pppR1pp1/4p3/4P1P1/5P2/1P4K1/P1P5/8 w - - bm g6; id "BK.06"; acd 13;
1nk1r1r1/pp2n1pp/4p3/q2pPp1N/b1pP1P2/B1P2R2/2P1B1PP/R2Q2K1 w - - bm Nf6; id "BK.07"; acd 8;
4b3/p3kp2/6p1/3pP2p/2pP1P2/4K1P1/P3N2P/8 w - - bm f5; id "BK.08"; acd 13;
2kr1bnr/pbpq4/2n1pp2/3p3p/3P1P1B/2N2N1Q/PPP3PP/2KR1B1R w - - bm f5; id "BK.09"; acd 8;
3rr1k1/pp3pp1/1qn2np1/8/3p4/PP1R1P2/2P1NQPP/R1B3K1 b - - bm Ne5; id "BK.10"; acd 7; 
2r1nrk1/p2q1ppp/bp1p4/n1pPp3/P1P1P3/2PBB1N1/4QPPP/R4RK1 w - - bm f4; id "BK.11"; acd 8;
r3r1k1/ppqb1ppp/8/4p1NQ/8/2P5/PP3PPP/R3R1K1 b - - bm Bf5; id "BK.12"; acd 8;
r2q1rk1/4bppp/p2p4/2pP4/3pP3/3Q4/PP1B1PPP/R3R1K1 w - - bm b4; id "BK.13"; acd 9;
rnb2r1k/pp2p2p/2pp2p1/q2P1p2/8/1Pb2NP1/PB2PPBP/R2Q1RK1 w - - bm Qd2 Qe1; id "BK.14"; acd 8;
2r3k1/1p2q1pp/2b1pr2/p1pp4/6Q1/1P1PP1R1/P1PN2PP/5RK1 w - - bm Qxg7+; id "BK.15"; acd 10;
r1bqkb1r/4npp1/p1p4p/1p1pP1B1/8/1B6/PPPN1PPP/R2Q1RK1 w kq - bm Ne4; id "BK.16"; acd 8;
r2q1rk1/1ppnbppp/p2p1nb1/3Pp3/2P1P1P1/2N2N1P/PPB1QP2/R1B2RK1 b - - bm h5; id "BK.17"; acd 9;
r1bq1rk1/pp2ppbp/2np2p1/2n5/P3PP2/N1P2N2/1PB3PP/R1B1QRK1 b - - bm Nb3; id "BK.18"; acd 7;
3rr3/2pq2pk/p2p1pnp/8/2QBPP2/1P6/P5PP/4RRK1 b - - bm Rxe4; id "BK.19"; acd 7;
r4k2/pb2bp1r/1p1qp2p/3pNp2/3P1P2/2N3P1/PPP1Q2P/2KRR3 w - - bm g4; id "BK.20"; acd 8;
3rn2k/ppb2rpp/2ppqp2/5N2/2P1P3/1P5Q/PB3PPP/3RR1K1 w - - bm Nh6; id "BK.21"; acd 7;
2r2rk1/1bqnbpp1/1p1ppn1p/pP6/N1P1P3/P2B1N1P/1B2QPP1/R2R2K1 b - - bm Bxe4; id "BK.22"; acd 7;
r1bqk2r/pp2bppp/2p5/3pP3/P2Q1P2/2N1B3/1PP3PP/R4RK1 b kq - bm f6; id "BK.23"; acd 9;
r2qnrnk/p2b2b1/1p1p2pp/2pPpp2/1PP1P3/PRNBB3/3QNPPP/5RK1 w - - bm f4; id "BK.24"; acd 8;
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    // https://www.madchess.net/ratings/position-tests/
    pub fn iq() -> Vec<Position> {
        let str = r#"
5rk1/1p6/q2P3p/2p2rp1/p1nbQ3/P1N3BP/1PR1B1P1/4K3 b - - bm Rf1+; id "5.IQ.918";
4r1k1/1pq2p1p/2p2npb/2N5/1PPQn1P1/7P/6B1/B2R2K1 b - - bm Ng3; id "5.IQ.921";
2q1r2k/5R1p/pp1B2pN/2p1P3/1n1b4/3P2Q1/1P4K1/8 w - - bm Qh4; id "5.IQ.931";
6rk/7p/Q2R1p2/1p2n3/4b3/1P4NP/P1P2PPK/2q5 b - - bm Qf1; id "5.IQ.934";
2k1r2r/ppn3pq/b7/5p2/5Qn1/2N1B1P1/1P3PB1/2RR2K1 w - - bm Qxc7+; id "5.IQ.936";
2k4r/pp3ppp/2p2n2/5PBb/P3P3/2Nr3P/1PB2bP1/R3R2K b - - bm Rxh3+; id "5.IQ.1009";
r5k1/pq2pn2/3pN1pp/n2P4/1rp1PQ2/1P3PP1/PN4K1/2R4R w - - bm Rxh6 Nd3; id "5.IQ.1010";
3r1r2/p4pk1/bp2p1P1/4P2p/Pq1N1P2/2p4Q/2P3P1/3RK2R w - - bm Qxh5; id "5.IQ.1014";
rr4k1/2q2ppp/p1b1p3/2b1pPPN/1p2P1R1/3B3Q/PPP4P/1K1R4 w - - bm Nf6+; id "5.IQ.1017";
2r1qrk1/3n3p/b3pPp1/4P3/1pp1nBN1/pP4PQ/P1P2PK1/3RR3 w - - bm Qxh7+; id "5.IQ.1021";
4rrk1/1b3p1p/pp2p1p1/2qnN1Pn/3N1P2/2P4R/PPB1Q2P/5RK1 w - - bm Rxh5; id "5.IQ.1026";
6k1/pp3p1p/3b3r/3p3q/P2n2p1/1P1Pp1P1/1N2P1QP/2R2RK1 b - - bm Bxg3; id "5.IQ.1030";
4r3/pbrq1p1k/1p3Bpp/3pPn2/5PQ1/P6R/1P4PP/1B3RK1 w - - bm Rh5 Qg5; id "5.IQ.1051";
r1r3k1/1p1b1nq1/6p1/pN1p4/P2Np2R/1P2P3/1Q3PP1/1K5R w - - bm Nd6; id "5.IQ.1065";
r2q1rk1/pb2Bppp/4p3/1p6/2pnP1n1/6N1/PPPN2PP/R1Q1RBK1 b - - bm Qb6; id "5.IQ.1072";
6k1/5qp1/pN5p/3pRp2/3QnP2/2r4P/r5P1/6RK b - - bm Qh5 Nf2+; id "5.IQ.1073";
2b2rk1/pp2R1bp/1qpp2p1/6N1/2P2PP1/3Q1B2/PP4PK/8 w - - bm Bd5+; id "5.IQ.1074";
r4qk1/1b3rpp/1pn1p3/p1b5/2P5/2NQ1N2/PP2BPPP/3R1RK1 b - - bm Ne5; id "5.IQ.1078";
5rk1/5pp1/3b4/1pp2qB1/4R2Q/1BPn4/1P3PPP/6K1 b - -  bm Bf4 Ra8; id "5.IQ.1091";
3r2k1/p1rq1pp1/1pR1p2p/8/Q2P4/4P3/P4PPP/2R3K1 b - - bm Rdc8; id "5.IQ.1095";
1q4r1/3Q1Npk/p6p/1p5N/8/7P/Pn3PP1/6K1 w - - bm Qf5+; id "5.IQ.1106";
5k2/1pp2b1p/2p5/r3PPR1/pr6/2N5/1PP4P/2KR4 w - - bm f6; id "5.IQ.1113";
6k1/6p1/7p/8/pb5N/2B1p2P/PP4P1/3K1b2 b - - bm e2+; id "5.IQ.1118";
r1bq1rk1/5pbp/pB1p1np1/3Pp3/4P3/2p1QP2/PP4PP/1KNR1B1R b - - bm Rb8; id "5.IQ.1120";
7r/8/3p1p1r/p1kP2p1/Pp2P1P1/1PpR3P/5R1K/8 b - - bm Re8; id "5.IQ.1121";
4R3/5Bq1/7p/5p2/1n3k1B/7P/6PK/8 w - - bm g3+; id "5.IQ.1128";
2r2rk1/4p2p/bq1P2p1/p4p2/3R1P1B/2N5/P2Qp1PP/6K1 b - - bm Rxc3; id "5.IQ.1135";
2q1n1kr/6b1/1r1p2B1/2p2pP1/2P1bP1N/1P1QB2P/6RK/1R6 w - - bm Qxe4; id "5.IQ.1142";
2r5/1p6/pq2p2p/3rN3/k2P2Q1/3R2P1/1P3PP1/1K6 b - - bm Qxb2+ Qd6; id "5.IQ.1157";
3r2k1/3rppb1/p1R1b1p1/qp1NP3/8/1P2QB2/P5PP/5RK1 w - - bm Rxe6; id "5.IQ.1190";
r1bqkn1r/p3bpp1/1p2pn1p/2pPN1B1/2P5/3B2N1/PP3PPP/R2QR1K1 w kq - bm Nf5 Nh5 dxe6; id "5.IQ.1200";
r3kb1r/2q2ppp/pn2p3/4P3/pPPN4/P3BQ2/6PP/R4RK1 w kq - bm Nxe6 Rf2 Rad1; id "5.IQ.1201";
1r5r/p2bbk1p/4pnp1/q1ppQ1B1/7P/2N3P1/PPP2PB1/1K1R3R w - - bm Nxd5; id "5.IQ.1206";
r1b1k1nr/2R1b1pp/1p2pp2/1N2p3/q3P3/3Q1N2/PP3PPP/3R2K1 w kq - bm b3 Nd6+; id "5.IQ.1218";
r4rk1/pp2bpp1/1qp1b2p/4B2Q/n7/3B4/PPP2PPP/3RR1K1 w - - bm Bxg7; id "5.IQ.1236";
r1b2k1r/1p4pp/p4B2/2bpN3/8/q2n4/P1P2PPP/1R1QR1K1 w - - bm Bxg7+ Qh5; id "5.IQ.1244";
1r3r1k/1b2bppp/p1qp4/5PP1/1p1B3Q/1PN2R2/1PP4P/5RK1 w - - bm Bxg7+; id "5.IQ.1245";
2r2rk1/1bq1bpp1/pn2p2p/1pn3P1/3NNP1Q/P6R/1PP3BP/R1B4K w - - bm Nf6+; id "5.IQ.1265";
1r2b3/pp1rqppk/3b3p/3Bp2Q/4P1N1/5R1P/PP3PP1/3R2K1 w - - bm Rf6; id "5.IQ.1272";
r1n1nrk1/p4p1p/1q4pQ/2p1pN2/1pB1P1P1/5P2/PPP4P/1K1R3R w - - bm Rd6 Rhg1; id "5.IQ.1276";
4Q3/p4pk1/2p3p1/8/3P1N2/4PBR1/q4P1K/r3r3 w - - bm Rxg6+ Qe5+; id "5.IQ.1281";
3rr1k1/5p2/1p1q2pQ/p2Pb3/1pPp3P/6P1/P4P2/1R1R1BK1 b - - bm Bxg3; id "5.IQ.1283";
2r1r1k1/5pbp/p5p1/3Pn3/3QP3/N1B5/PP4qP/1K1R3R b - - bm Rxc3; id "5.IQ.1289";
3r2k1/p2r1p1p/1p2p1p1/q4n2/3P4/PQ1R3P/1P2NPP1/3R2K1 b - - bm Nxd4; id "6.IQ.923";
5r1k/1p2qp1p/p4R1Q/P1p5/2Prp3/7P/1P4P1/5R1K w - - bm Re6; id "6.IQ.940";
5rk1/q5pp/4p3/r1bp1p2/1p1B1P2/1P1QP3/P4RPP/2R3K1 w - - bm Rxc5; id "6.IQ.943";
r1r3k1/1p1bnp1p/p5pQ/q5P1/3Bp3/1P4R1/1PP1KP1P/3R4 b - - bm Bg4+; id "6.IQ.946";
4k3/1bQ3p1/pp2P3/2nP1qp1/8/2P3P1/P6P/5BK1 w - - bm Bd3; id "6.IQ.951";
2rq1r2/pp1b1pkp/8/P2Q2P1/n1nN4/2P5/2P3BP/R1B2RK1 w - - bm Rxa4; id "6.IQ.954";
1k1r4/ppq2p2/8/2pPb1r1/2P1Q1B1/6P1/PP3PK1/R4R2 b - - bm f5; id "6.IQ.957";
8/5p1k/4p1b1/4P3/2Pq4/2Q2KP1/r3P3/2R2B2 b - - bm Ra3; id "6.IQ.962";
1k1r2r1/pp3p1p/B2q1n2/8/3Pb1p1/2Q5/PR3PPP/2B1R1K1 w - - bm Bf4; id "6.IQ.966";
r2qrn2/1p2nbkp/2p1pp2/p7/3PP1BN/2N4R/PP1Q1P2/2KR4 w - - bm Qh6+; id "6.IQ.976";
r2qk2r/1pp2ppp/p3pnn1/6N1/1B1P2P1/1Q2P2P/PP3PK1/R6R b kq - bm Nh4+; id "6.IQ.978";
7R/5Q2/6pp/4rp1k/P3P3/5Pb1/4KPq1/3R4 w - - bm Rxh6+; id "6.IQ.980";
rnb4r/p1pq1Pkp/1pn1p1p1/3p2N1/3P4/P1PB3Q/2P2PPP/R1B1K2R w KQ - bm Qh6+; id "6.IQ.981";
4rr1k/pQ4pp/8/7q/2B2BbP/P1P2nP1/5PK1/R6R b - - bm Rxf4; id "6.IQ.999";
1k6/p1p3Q1/2qp1P2/2p5/rp6/3P3P/1BP1rP2/2KR3R b - - bm Ra1+; id "6.IQ.1002";
4r3/1r2ppk1/p2p1np1/2qP4/PpP5/1P1Q4/2B1R1PP/5R1K w - - bm Rxf6; id "6.IQ.1018";
4r3/5pk1/2Q2n2/3B2pp/2b2q2/P6P/6P1/R2R2K1 b - - bm Ng4; id "6.IQ.1025";
6rk/6bp/p3p1r1/3p4/4p3/2P1N1Pq/PP2QP1P/4RRK1 b - - bm Bxc3 Bf8; id "6.IQ.1029";
r1r1q2k/pp3pbp/4p3/2npPp1Q/3P1P1N/2P2RR1/P1P3PP/6K1 w - - bm Rxg7; id "6.IQ.1038";
2R5/r3b1k1/p2p4/P1pPp2p/6q1/2P2N1r/4Q1P1/5RK1 w - - bm Nxe5; id "6.IQ.1042";
r1bn1r2/1pp3k1/p2p3p/6p1/2qNB3/2P2PP1/P1Q2KP1/1R5R w - - bm Rxh6; id "6.IQ.1052";
r4rk1/pp3pbp/3Qbnp1/q1p5/4P1P1/2N1BP2/PPP4P/2KR1B1R b - - bm Nxe4; id "6.IQ.1053";
r5k1/4p2p/pp1qNnp1/2pPb3/2P1p1P1/4B3/PP1Q2P1/5R1K w - - bm Bxc5; id "6.IQ.1059";
r7/1pqbr1pk/p1n4p/5p1P/2P1p3/2B3R1/PP1Q1PP1/1B2R1K1 w - - bm Rxe4; id "6.IQ.1063";
5Bk1/pr2pp1p/2b3pQ/2p1q3/8/2P4P/PP4P1/1B1Rb1K1 w - - bm Bg7; id "6.IQ.1067";
k1n1r3/p2R2bb/2p2qpp/4pp2/2P5/2B1Q1P1/PP3PBP/6K1 w - - bm Qc5 Bxe5; id "6.IQ.1085";
rr4k1/3nbp1p/p3p1p1/q2pPPP1/bp1B3R/1B2Q3/PPP4P/2K3R1 w - - bm fxe6; id "6.IQ.1086";
7k/p2rq2p/1p3R1P/2p5/2P2P1Q/8/6PK/8 w - - bm Qe1; id "6.IQ.1087";
b1R2nk1/5ppp/1p3n2/5N2/1b2p3/1P2BP2/q3BQPP/6K1 w - - bm Bc5; id "6.IQ.1089";
2b1r1k1/1r1n1pp1/q2p3p/1p1P4/1P1N1RPP/2Q1P3/5PBK/2R5 w - - bm Qxc8; id "6.IQ.1096";
8/6kp/8/5r2/1QP1q3/1P3pP1/P3rP1P/5RKB b - - bm Rxf2; id "6.IQ.1105";
7r/1pR5/p5k1/3p4/4n1np/8/PPP3PP/5R1K b - - bm Ng3+; id "6.IQ.1124";
8/kp6/4R3/1P2K3/8/3pp3/8/8 w - - bm Kd6; id "6.IQ.1125";
5r1k/p5bp/1p6/4P3/P2B4/3n2PB/1PR4K/8 w - - bm e6; id "6.IQ.1127";
1r1q1rk1/Nb1n1pbp/p5p1/3P4/PPB2p2/4B3/6PP/2RQ1RK1 w - - bm Nc6; id "6.IQ.1130";
r3k2r/1nqb1pp1/p3p3/1p1pP1P1/3R1P2/2N2Q2/PPP3B1/2KR4 w kq - bm Nxd5; id "6.IQ.1145";
1r2r1k1/p1p2p1p/6p1/8/8/4NBPb/qBQ2K1P/1R6 b - - bm Rxe3 Rb3; id "6.IQ.1146";
r7/1p3k1p/3RRp1P/1qr1p1p1/4P1P1/P3QP2/1P6/1K6 b - - bm Qf1+; id "6.IQ.1156";
rq2r1k1/pp2RNp1/1bp2n1p/8/1Q6/7P/PPB2PP1/5RK1 w - - bm Nxh6+; id "6.IQ.1195";
r2qk2r/1b1n1p2/p6n/1pp1p1Np/4P2P/2QBNP2/PPP5/2KR2R1 w kq - bm Bxb5 Bc4 Bf1; id "6.IQ.1210";
2r1r3/pbq1bpk1/1pn3p1/6Bp/4Q3/PB3N2/1P3PPP/R3R1K1 w - - bm Bxf7; id "6.IQ.1225";
4r3/p2qppkp/3p2b1/1p1N4/2r1P1RQ/2P4P/PP4P1/5RK1 w - - bm Rxg6+; id "6.IQ.1227";
8/1p4pk/6rp/p1q1pr2/2P1b3/P1Q1P3/1B2R1PP/2R3K1 b - - bm Bxg2; id "6.IQ.1239";
2r5/3nr1pk/p2qpp1p/1p5Q/3P1PN1/2P3RP/P5PK/4R3 w - - bm Nxh6; id "6.IQ.1260";
3rk1nr/1p3q2/p1n2P1p/P1ppp3/3b4/2N2QPB/1PP4P/R1B2R1K w k - bm Nxd5; id "7.IQ.917";
3r4/p2nrpkp/2B1p3/2P2pP1/3R3Q/q7/6PP/6RK w - - bm g6; id "7.IQ.927";
6k1/pp2q2r/2r1p3/1RPpPpp1/3P4/P5P1/3Q1PK1/7R w - - bm Rxb7; id "7.IQ.960";
b3k3/2q1b1p1/p3p2r/1pPpP2p/1P1N3P/6P1/P3Q3/1BR3K1 w - - bm Bg6+; id "7.IQ.984";
8/2bP1Q1p/6pk/p3p3/P1Bp4/1P1R2KP/5PP1/2q4r w - - bm Re3; id "7.IQ.985";
r3k2r/pppqnpbp/4p1p1/3nP1B1/Q2PN2P/5P2/PP2BP2/2R1K1R1 w - - bm Nd6+; id "7.IQ.990";
r1b2rk1/1p2qppp/2pb1n2/p3n1N1/2BR4/1PN1P3/PBQ2PPP/2K4R w - - bm Rxd6; id "7.IQ.998";
r1b2Q2/1p4pk/pB3pq1/4p3/3p4/6P1/PP2PP1P/2R3K1 b - - bm Bh3; id "7.IQ.1000";
r4rk1/5pp1/p2R1Bb1/1pq1Pp1p/7Q/1Bn5/P5PP/5R1K w - - bm Bxg7; id "7.IQ.1001";
r3r3/pp2bBkp/6p1/q3p2Q/4n3/2N2R2/PPP3PP/2KR4 w - - bm Bxg6 Qg4; id "7.IQ.1005";
r2q1rk1/1p1bbppp/p1n1p3/2p1P2Q/4N3/6P1/PPPB1PBP/3RR1K1 w - - bm Nf6+; id "7.IQ.1008";
r1b1rb2/1p3kpQ/p1p1p3/q1P1P3/3P4/4P1R1/PP4PP/R1B3K1 w - - bm Bd2; id "7.IQ.1013";
r1r2k2/1q1n1p2/3PpPp1/4P2p/pbpN3R/2N3K1/1P2Q2P/3R4 w - - bm Rxh5; id "7.IQ.1022";
r2qrnk1/1b2bppp/2p5/1p1pN1Pn/p2P1B2/3BP2P/PPQ2P2/2K3RR w - - bm Bxh7+; id "7.IQ.1034";
r4rk1/pppq2pp/1nbpp3/6B1/2PP2P1/3B3P/P1P2P2/1R1QR1K1 w - - bm d5; id "7.IQ.1036";
r3r1k1/pp1q1p2/6pp/3N4/3b4/1Q3PPb/PP3R1P/1B1R2K1 b - - bm Re2; id "7.IQ.1039";
4rrk1/1p3p1p/p5pb/3P4/2Pp3q/N6P/PPQ1BP1P/R3K2R b KQ - bm d3; id "7.IQ.1041";
2br1rk1/2qnQ2p/p4pp1/4p3/P1p1P1N1/2Pn1N1P/2B2PP1/1R1R1K2 w - - bm Rxd3; id "7.IQ.1056";
r7/pp2pkbp/2pq2p1/8/2P2N2/1P2R1PP/P3QPK1/8 w - - bm c5; id "7.IQ.1060";
5r1k/pb2r1bp/1p2B1p1/n7/4qNQ1/4B3/P4PPP/2RR2K1 w - - bm Rd4; id "7.IQ.1083";
b1n1r1k1/2q2p1p/p4Pp1/1pBP4/4B1P1/2P1Q3/P4K1P/4R3 w - - bm Qh6 Kg2; id "7.IQ.1097";
r1b2r1k/2q2pp1/2pb3p/p1n1NB2/1pP5/1P5P/PBQ2PP1/3RR1K1 w - - bm Rxd6; id "7.IQ.1101";
r4bk1/pbq2pp1/1pn1p2p/8/4N3/1P2P1P1/PB2QPBP/3R2K1 w - - bm Nf6+; id "7.IQ.1192";
r3k2r/1bqnBppp/p3p3/1p2P3/3N2Q1/3B4/PPP3PP/2KnR3 w kq - bm Nxe6; id "7.IQ.1199";
4k1r1/1q1r1p2/4pQp1/3pP1B1/2p3P1/1nP2BP1/1P5K/3R4 w - - bm Bxd5; id "7.IQ.1203";
r1b2k1r/p2n1pp1/1q2p3/2ppP1B1/1b1N1Q1P/2N5/PPP2P2/R3K1R1 w Q - bm Bd8; id "7.IQ.1212";
r4r2/1b4pk/p5pp/1pb1N3/4P3/2P1N1Pq/PPB1QP2/3R2K1 b - - bm Rxf2 Rad8; id "7.IQ.1219";
4r1k1/1p2qrpb/p1p4p/2Pp1p2/1Q1Rn3/PNN1P1P1/1P3PP1/3R2K1 b - - bm Nxf2; id "7.IQ.1221";
rn1q1rk1/1b2bppp/p3p3/1pn1P2Q/3N4/1BN1B3/PPP3PP/R4RK1 w - - bm Rxf7; id "7.IQ.1224";
r1bq1rk1/pp2ppbp/2p2np1/3p4/2PPn3/1PN3P1/PBQNPPBP/R3K2R b KQ - bm Nxf2; id "7.IQ.1229";
r3k3/2pb1p1p/p2p4/6rq/4P3/2P5/PP1Q1PP1/R3RNK1 b q - bm Rxg2+; id "7.IQ.1241";
r2q1rk1/pp1b1Rp1/4p2p/4PnNQ/2pP4/2P1B3/P5PP/5RK1 w - - bm R1xf5; id "7.IQ.1249";
r1b2rk1/pppn2bp/3p1qp1/3p2N1/3P4/3BB3/PPP2PPP/R2QR1K1 w - - bm Nxh7; id "7.IQ.1252";
4rb2/1p1q1ppk/p1n1p2p/2p1Pn2/3PNR2/2P2K2/P2QBP2/6R1 w - - bm Rxf5; id "7.IQ.1266";
4rr1k/pp4b1/4P2p/5p1R/3q4/3B2P1/PP1R4/1K5Q w - - bm Rxh6+; id "7.IQ.1271";
r1r1n3/5p1k/1n1P2pp/ppq1pN2/2p1P3/1PP1R2P/P1B2QPK/5R2 w - - bm Nxh6; id "7.IQ.1275";
r2q1r1k/p3pn1p/3p2pQ/1ppP1p1n/4P3/2N2P2/PPP1BP2/2K3RR w - - bm Qxg6; id "7.IQ.1280";
r4rk1/4ppbp/1q2bnp1/n1p4P/4P1P1/2NBBP2/PP1Q4/1K1R2NR b - - bm Bxa2+ Rfb8; id "7.IQ.1290";
r3rnk1/4qpp1/p5np/4pQ2/Pb2N3/1B5P/1P3PP1/R1BR2K1 w - - bm Bxh6; id "8.IQ.894";
1r2q1k1/1p6/pN2p1p1/P3p3/2PbQ3/1P5P/6P1/5R1K w - - bm Nd7; id "8.IQ.958";
r1b2rk1/p1q2ppp/1p2p3/4n3/3bP3/1B3N1P/PP2QPP1/R1B2RK1 b - - bm Ba6; id "8.IQ.963";
7r/p5k1/4pr2/4Np2/1p1Pn1p1/4P3/PP3PP1/2R2RK1 b - - bm Nc3; id "8.IQ.987";
5bk1/5p1p/Q5p1/1B4n1/8/2q1p3/6PP/3R2K1 b - - bm e2; id "8.IQ.1048";
6k1/1p3ppp/1PbNp3/3rP1q1/2Q5/8/5PPP/1R4K1 w - - bm Qxc6; id "8.IQ.1090";
3r4/5pkp/B5p1/PPn1p3/2P1P3/4qP1P/1Q4P1/4NK2 b - - bm Rd2 Rd1; id "8.IQ.1104";
6k1/p4pp1/Pp2r3/1QPq3p/8/6P1/2P2P1P/1R4K1 w - - bm cxb6; id "8.IQ.1122";
8/1p3b2/8/p1b1pN2/P1P1P1p1/1P6/6Bk/5K2 b - - bm b5; id "8.IQ.1137";
2r1r3/3qppkp/pp1p2p1/3PnP2/2PN4/1P5P/P2Q2P1/4RRK1 w - - bm Rxe5; id "8.IQ.1185";
1n2rk2/1qr1pp2/pp1p1npp/3P4/1PPN1P2/P2QR1P1/7P/4RBK1 w - - bm Ne6+; id "8.IQ.1186";
Q1b1r3/1p3pbk/1p3npp/7q/1BPpp1P1/1P5P/P3PPB1/R3R1K1 b - - bm Bxg4; id "8.IQ.1197";
r3kbnr/p4p2/1p2p2p/3qP1p1/3Nn1QB/8/PP2NPPP/R4RK1 w kq - bm Nxe6; id "8.IQ.1217";
r1bqr1k1/1p1nbppp/p1n1p3/3pP2Q/3N1P2/1BN1B3/PPP3PP/R4RK1 w - - bm Nxd5; id "8.IQ.1222";
2r1r1k1/5ppp/pq3b2/2pB1P2/2p2B2/5Q1P/Pn3PP1/2R1R1K1 w - - bm Bxf7+; id "8.IQ.1223";
r2r1bk1/1ppq2pp/2p2B2/p1n2N1b/P3P1R1/3PN3/2P3PP/R2Q2K1 w - - bm Ne7+; id "8.IQ.1233";
1nb2r1k/2r1qppp/p3p2n/1p1BP3/3N1Q2/2N5/PPP4P/2KR2R1 w - - bm Rxg7; id "8.IQ.1240";
2q3k1/3b1p2/1b1p3p/1p1Pp1p1/1P2Pn2/QN3P2/2r3PP/1R2BB1K b - - bm Rxg2; id "8.IQ.1242";
2r1k2r/3bbp1p/p3pp1Q/q2P4/1p3P1B/8/1PP3PP/1K1R1B1R b k - bm Rxc2; id "8.IQ.1288";
1rb2r2/1p1nppk1/2p1n1p1/p3P2p/2qP3R/2N3NP/PP1Q2B1/5RK1 w - - bm Bd5; id "9.IQ.926";
3r2k1/1p3pp1/1p6/7p/q2NP1P1/2Q2P1b/rP5P/2RR2K1 w - - bm Nf5; id "9.IQ.929";
2qr1r2/p2n1ppk/1p2p2p/5b2/2PP2NQ/1B4R1/P4PPP/R5K1 w - - bm Bc2; id "9.IQ.956";
5r1k/p5b1/1qp4p/1r2p3/4N1P1/2Pn2B1/QP2R1PP/R6K b - - bm Ra5; id "9.9IQ.959";
r4rk1/1nqb1pbn/6pB/pppPp2p/8/1PP2NNP/P1BQ2P1/R4RK1 w - - bm Nxh5; id "9.IQ.1020";
4r1k1/p4ppp/1p2q3/n2R4/2r1p3/Pn2B1P1/1Q2PPBP/3R2K1 w - - bm Re5; id "9.IQ.1099";
r1br2k1/1pq2ppp/p3pP2/8/1n3PP1/1NpB4/PPP2Q1P/2KR2R1 w - - bm Bxh7+; id "9.IQ.1112";
8/5p1p/6p1/3P4/1k3P2/pp1B2P1/7P/1K6 b - - bm Kc3; id "9.IQ.1115";
r4r1k/6pp/3p3b/1p1Npb2/3nB2q/2N3P1/PP3P1P/R2Q1RK1 b - - bm Qxe4; id "9.IQ.1148";
r2q1rk1/3nbppp/p1bpp3/6PQ/1p1BP3/1BN5/PPP2P1P/2KR2R1 w - - bm Rd3; id "9.IQ.1179";
r3kb1r/pp3p1p/4pp2/q2pN3/2P5/8/PP3PPP/R2QR1K1 w kq - bm Nxf7; id "9.IQ.1216";
r1qr2k1/p3bppp/b1p5/3nN3/NP1P4/P3B3/5PPP/1B1QR1K1 w - - bm Bxh7+; id "9.IQ.1254";
3r1rk1/1bq1bp1p/p3p1p1/3pP3/3N1P2/2PBB2R/n1P1Q1PP/5RK1 w - - bm Rxh7; id "9.IQ.1255";
2rq1rk1/pp3pp1/4pb1p/3bN3/3P4/PnPB3R/2Q2PPP/R1B3K1 w - - bm Bh7+; id "9.IQ.1269";
1b4k1/1pnq1r1p/4p1p1/1pPpP2Q/1P5B/3BPP2/7P/6RK w - - bm Bxg6; id "9.IQ.1278";
2r2rn1/p3q1bk/n2p2pp/1b1PPp2/1pp2P1N/1P2N1P1/P1QB2BP/4RRK1 w - - bm Nxg6; id "9.IQ.1279";
5r2/1pr1ppk1/1n1q2p1/p2P3p/5R2/1BP1RQ2/5PP1/6K1 w - - bm Re6; id "9.IQ.1284";
2k5/1p2b3/2p1p3/3pP1p1/nP1P2P1/NKP5/3B4/q3Q3 b - - bm Qb2+; id "10.IQ.937";
1r5r/3b1pk1/3p1np1/p1qPp3/p1N1PbP1/2P2PN1/1PB1Q1K1/R3R3 b - - bm Nxg4; id "10.IQ.950";
r2qk2r/1bp2pQp/p7/1p1pP3/4n3/1B2b3/PPP2PPP/RN3RK1 b kq - bm Qg5; id "10.IQ.953";
r2qrbk1/p2b2pp/1pn1pn2/3p4/2pP1P2/2P2NN1/PPQ3PP/RBB2RK1 w - - bm Ng5; id "10.IQ.993";
3qr1k1/4r1pp/2pp1p2/p1p2P2/2PnPNPP/1P1Q4/P5R1/4R1K1 b - - bm d5; id "10.IQ.1032";
rq6/1r1k4/1pRp1p1p/1P1Pp1pP/p1R1P1P1/PpQ2P2/1K6/8 w - - bm f4; id "10.IQ.1043";
r4rk1/3bpp1p/pqp3p1/4P2n/2B5/1P6/PBPbQ1PP/4RR1K w - - bm e6; id "10.IQ.1054";
4rr1k/ppp3pp/3bBqb1/3P4/3Pn1PP/4BN2/PP3PK1/R2Q1R2 b - - bm Rxe6; id "10.IQ.1070";
2r3k1/1rq2p1p/pp1p1bp1/2pP2N1/2P1R3/1Q6/PP3PP1/4R1K1 w - - bm Nxh7; id "10.IQ.1076";
8/8/pp1pPkpp/3P4/2P1K1P1/8/P7/8 w - - bm g5+; id "10.IQ.1117";
5rk1/2rnqp1p/b3p1pP/2ppP3/1p3QN1/pP1P1BP1/P1P2P2/R3R1K1 w - - bm Bxd5; id "10.IQ.1126";
8/1Rb2r1k/6p1/6qp/4QP2/3p3K/7P/3B4 b - - bm Rxf4; id "10.IQ.1136";
r5k1/1p3p1p/p1pqrPp1/3bN3/8/P1P3Q1/1P4PP/4RRK1 w - - bm Nxf7; id "10.IQ.1231";
2r2bk1/1bq1rppp/pp1p4/4n1P1/Pn1BP1N1/2N3Q1/1PP2RBP/5RK1 w - - bm Bxe5; id "10.IQ.1251";
2r1r1k1/pbq2ppp/1p2pn2/2b1N3/2P5/2B5/PPB1QPPP/3RR1K1 w - - bm Bxh7+; id "10.IQ.1257";
1k1r4/1bp4Q/p7/8/1pP2p2/1P2bNrP/1P4P1/5R1K b - - bm Bxf3; id "11.IQ.930";
7r/4q1k1/1p1p1ppr/p1nRp3/P1P1P1BP/1P4P1/2Q3K1/3R4 b - - bm f5; id "11.IQ.1033";
2r1r1k1/pp1b1ppp/1q6/3P4/7N/Pn1Q4/1P3PPP/1RB2RK1 b - - bm Nxc1; id "11.IQ.1080";
rnb2rk1/pp3p1p/3q2pQ/3pN3/2pP4/2PB4/P1P2PPP/4RRK1 w - - bm Nxf7; id "11.IQ.1232";
r1q2r1k/p1ppb1p1/1p2p1Qp/3n3P/3PN3/2P1B3/PP3PP1/3R1RK1 w - - bm Bxh6; id "11.IQ.1261";
r4rk1/pp2qpp1/2p1bn1p/8/1bP4Q/5N1P/PPB2PP1/R1BR2K1 w - - bm Bxh6; id "11.IQ.1263";
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn win_at_chess() -> Vec<Position> {
        let str = r#"
2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - bm Qg6; id "WAC.001";
8/7p/5k2/5p2/p1p2P2/Pr1pPK2/1P1R3P/8 b - - bm Rxb2; id "WAC.002";
5rk1/1ppb3p/p1pb4/6q1/3P1p1r/2P1R2P/PP1BQ1P1/5RKN w - - bm Rg3; id "WAC.003";
r1bq2rk/pp3pbp/2p1p1pQ/7P/3P4/2PB1N2/PP3PPR/2KR4 w - - bm Qxh7+; id "WAC.004";
5k2/6pp/p1qN4/1p1p4/3P4/2PKP2Q/PP3r2/3R4 b - - bm Qc4+; id "WAC.005";
7k/p7/1R5K/6r1/6p1/6P1/8/8 w - - bm Rb7; id "WAC.006";
rnbqkb1r/pppp1ppp/8/4P3/6n1/7P/PPPNPPP1/R1BQKBNR b KQkq - bm Ne3; id "WAC.007";
r4q1k/p2bR1rp/2p2Q1N/5p2/5p2/2P5/PP3PPP/R5K1 w - - bm Rf7; id "WAC.008";
3q1rk1/p4pp1/2pb3p/3p4/6Pr/1PNQ4/P1PB1PP1/4RRK1 b - - bm Bh2+; id "WAC.009";
2br2k1/2q3rn/p2NppQ1/2p1P3/Pp5R/4P3/1P3PPP/3R2K1 w - - bm Rxh7; id "WAC.010";
r1b1kb1r/3q1ppp/pBp1pn2/8/Np3P2/5B2/PPP3PP/R2Q1RK1 w kq - bm Bxc6; id "WAC.011";
4k1r1/2p3r1/1pR1p3/3pP2p/3P2qP/P4N2/1PQ4P/5R1K b - - bm Qxf3+; id "WAC.012";
5rk1/pp4p1/2n1p2p/2Npq3/2p5/6P1/P3P1BP/R4Q1K w - - bm Qxf8+; id "WAC.013";
r2rb1k1/pp1q1p1p/2n1p1p1/2bp4/5P2/PP1BPR1Q/1BPN2PP/R5K1 w - - bm Qxh7+; id "WAC.014";
1R6/1brk2p1/4p2p/p1P1Pp2/P7/6P1/1P4P1/2R3K1 w - - bm Rxb7; id "WAC.015";
r4rk1/ppp2ppp/2n5/2bqp3/8/P2PB3/1PP1NPPP/R2Q1RK1 w - - bm Nc3; id "WAC.016";
1k5r/pppbn1pp/4q1r1/1P3p2/2NPp3/1QP5/P4PPP/R1B1R1K1 w - - bm Ne5; id "WAC.017";
R7/P4k2/8/8/8/8/r7/6K1 w - - bm Rh8; id "WAC.018";
r1b2rk1/ppbn1ppp/4p3/1QP4q/3P4/N4N2/5PPP/R1B2RK1 w - - bm c6; id "WAC.019";
r2qkb1r/1ppb1ppp/p7/4p3/P1Q1P3/2P5/5PPP/R1B2KNR b kq - bm Bb5; id "WAC.020";
5rk1/1b3p1p/pp3p2/3n1N2/1P6/P1qB1PP1/3Q3P/4R1K1 w - - bm Qh6; id "WAC.021";
r1bqk2r/ppp1nppp/4p3/n5N1/2BPp3/P1P5/2P2PPP/R1BQK2R w KQkq - bm Ba2 Nxf7; id "WAC.022";
r3nrk1/2p2p1p/p1p1b1p1/2NpPq2/3R4/P1N1Q3/1PP2PPP/4R1K1 w - - bm g4; id "WAC.023";
6k1/1b1nqpbp/pp4p1/5P2/1PN5/4Q3/P5PP/1B2B1K1 b - - bm Bd4; id "WAC.024";
3R1rk1/8/5Qpp/2p5/2P1p1q1/P3P3/1P2PK2/8 b - - bm Qh4+; id "WAC.025";
3r2k1/1p1b1pp1/pq5p/8/3NR3/2PQ3P/PP3PP1/6K1 b - - bm Bf5; id "WAC.026";
7k/pp4np/2p3p1/3pN1q1/3P4/Q7/1r3rPP/2R2RK1 w - - bm Qf8+; id "WAC.027";
1r1r2k1/4pp1p/2p1b1p1/p3R3/RqBP4/4P3/1PQ2PPP/6K1 b - - bm Qe1+; id "WAC.028";
r2q2k1/pp1rbppp/4pn2/2P5/1P3B2/6P1/P3QPBP/1R3RK1 w - - bm c6; id "WAC.029";
1r3r2/4q1kp/b1pp2p1/5p2/pPn1N3/6P1/P3PPBP/2QRR1K1 w - - bm Nxd6; id "WAC.030";
rb3qk1/pQ3ppp/4p3/3P4/8/1P3N2/1P3PPP/3R2K1 w - - bm Qxa8 d6 dxe6 g3; id "WAC.031";
6k1/p4p1p/1p3np1/2q5/4p3/4P1N1/PP3PPP/3Q2K1 w - - bm Qd8+; id "WAC.032";
8/p1q2pkp/2Pr2p1/8/P3Q3/6P1/5P1P/2R3K1 w - - bm Qe5+ Qf4; id "WAC.033";
7k/1b1r2p1/p6p/1p2qN2/3bP3/3Q4/P5PP/1B1R3K b - - bm Bg1; id "WAC.034";
r3r2k/2R3pp/pp1q1p2/8/3P3R/7P/PP3PP1/3Q2K1 w - - bm Rxh7+; id "WAC.035";
3r4/2p1rk2/1pQq1pp1/7p/1P1P4/P4P2/6PP/R1R3K1 b - - bm Re1+; id "WAC.036";
2r5/2rk2pp/1pn1pb2/pN1p4/P2P4/1N2B3/nPR1KPPP/3R4 b - - bm Nxd4+; id "WAC.037";
4k3/p4prp/1p6/2b5/8/2Q3P1/P2R1PKP/4q3 w - - bm Qd3 Rd8+; id "WAC.038";
r1br2k1/pp2bppp/2nppn2/8/2P1PB2/2N2P2/PqN1B1PP/R2Q1R1K w - - bm Na4; id "WAC.039";
3r1r1k/1p4pp/p4p2/8/1PQR4/6Pq/P3PP2/2R3K1 b - - bm Rc8; id "WAC.040";
1k6/5RP1/1P6/1K6/6r1/8/8/8 w - - bm Ka5 Kc5 b7; id "WAC.041";
r1b1r1k1/pp1n1pbp/1qp3p1/3p4/1B1P4/Q3PN2/PP2BPPP/R4RK1 w - - bm Ba5; id "WAC.042";
r2q3k/p2P3p/1p3p2/3QP1r1/8/B7/P5PP/2R3K1 w - - bm Be7 Qxa8; id "WAC.043";
3rb1k1/pq3pbp/4n1p1/3p4/2N5/2P2QB1/PP3PPP/1B1R2K1 b - - bm dxc4; id "WAC.044";
7k/2p1b1pp/8/1p2P3/1P3r2/2P3Q1/1P5P/R4qBK b - - bm Qxa1; id "WAC.045";
r1bqr1k1/pp1nb1p1/4p2p/3p1p2/3P4/P1N1PNP1/1PQ2PP1/3RKB1R w K - bm Nb5; id "WAC.046";
r1b2rk1/pp2bppp/2n1pn2/q5B1/2BP4/2N2N2/PP2QPPP/2R2RK1 b - - bm Nxd4; id "WAC.047";
1rbq1rk1/p1p1bppp/2p2n2/8/Q1BP4/2N5/PP3PPP/R1B2RK1 b - - bm Rb4; id "WAC.048";
2b3k1/4rrpp/p2p4/2pP2RQ/1pP1Pp1N/1P3P1P/1q6/6RK w - - bm Qxh7+; id "WAC.049";
k4r2/1R4pb/1pQp1n1p/3P4/5p1P/3P2P1/r1q1R2K/8 w - - bm Rxb6+; id "WAC.050";
r1bq1r2/pp4k1/4p2p/3pPp1Q/3N1R1P/2PB4/6P1/6K1 w - - bm Rg4+; id "WAC.051";
r1k5/1p3q2/1Qpb4/3N1p2/5Pp1/3P2Pp/PPPK3P/4R3 w - - bm Re7 c4; id "WAC.052";
6k1/6p1/p7/3Pn3/5p2/4rBqP/P4RP1/5QK1 b - - bm Re1; id "WAC.053";
r3kr2/1pp4p/1p1p4/7q/4P1n1/2PP2Q1/PP4P1/R1BB2K1 b q - bm Qh1+; id "WAC.054";
r3r1k1/pp1q1pp1/4b1p1/3p2B1/3Q1R2/8/PPP3PP/4R1K1 w - - bm Qxg7+; id "WAC.055";
r1bqk2r/pppp1ppp/5n2/2b1n3/4P3/1BP3Q1/PP3PPP/RNB1K1NR b KQkq - bm Bxf2+; id "WAC.056";
r3q1kr/ppp5/3p2pQ/8/3PP1b1/5R2/PPP3P1/5RK1 w - - bm Rf8+; id "WAC.057";
8/8/2R5/1p2qp1k/1P2r3/2PQ2P1/5K2/8 w - - bm Qd1+; id "WAC.058";
r1b2rk1/2p1qnbp/p1pp2p1/5p2/2PQP3/1PN2N1P/PB3PP1/3R1RK1 w - - bm Nd5; id "WAC.059";
rn1qr1k1/1p2np2/2p3p1/8/1pPb4/7Q/PB1P1PP1/2KR1B1R w - - bm Qh8+; id "WAC.060";
3qrbk1/ppp1r2n/3pP2p/3P4/2P4P/1P3Q2/PB6/R4R1K w - - bm Qf7+; id "WAC.061";
6r1/3Pn1qk/p1p1P1rp/2Q2p2/2P5/1P4P1/P3R2P/5RK1 b - - bm Rxg3+; id "WAC.062";
r1brnbk1/ppq2pp1/4p2p/4N3/3P4/P1PB1Q2/3B1PPP/R3R1K1 w - - bm Nxf7; id "WAC.063";
8/6pp/3q1p2/3n1k2/1P6/3NQ2P/5PP1/6K1 w - - bm g4+; id "WAC.064";
1r1r1qk1/p2n1p1p/bp1Pn1pQ/2pNp3/2P2P1N/1P5B/P6P/3R1RK1 w - - bm Ne7+; id "WAC.065";
1k1r2r1/ppq5/1bp4p/3pQ3/8/2P2N2/PP4P1/R4R1K b - - bm Qxe5; id "WAC.066";
3r2k1/p2q4/1p4p1/3rRp1p/5P1P/6PK/P3R3/3Q4 w - - bm Rxd5; id "WAC.067";
6k1/5ppp/1q6/2b5/8/2R1pPP1/1P2Q2P/7K w - - bm Qxe3; id "WAC.068";
2k5/pppr4/4R3/4Q3/2pp2q1/8/PPP2PPP/6K1 w - - bm f3 h3; id "WAC.069";
2kr3r/pppq1ppp/3p1n2/bQ2p3/1n1PP3/1PN1BN1P/1PP2PP1/2KR3R b - - bm Na2+; id "WAC.070";
2kr3r/pp1q1ppp/5n2/1Nb5/2Pp1B2/7Q/P4PPP/1R3RK1 w - - bm Nxa7+; id "WAC.071";
r3r1k1/pp1n1ppp/2p5/4Pb2/2B2P2/B1P5/P5PP/R2R2K1 w - - bm e6; id "WAC.072";
r1q3rk/1ppbb1p1/4Np1p/p3pP2/P3P3/2N4R/1PP1Q1PP/3R2K1 w - - bm Qd2; id "WAC.073";
5r1k/pp4pp/2p5/2b1P3/4Pq2/1PB1p3/P3Q1PP/3N2K1 b - - bm Qf1+; id "WAC.074";
r3r1k1/pppq1ppp/8/8/1Q4n1/7P/PPP2PP1/RNB1R1K1 b - - bm Qd6; id "WAC.075";
r1b1qrk1/2p2ppp/pb1pnn2/1p2pNB1/3PP3/1BP5/PP2QPPP/RN1R2K1 w - - bm Bxf6; id "WAC.076";
3r2k1/ppp2ppp/6q1/b4n2/3nQB2/2p5/P4PPP/RN3RK1 b - - bm Ng3; id "WAC.077";
r2q3r/ppp2k2/4nbp1/5Q1p/2P1NB2/8/PP3P1P/3RR1K1 w - - bm Ng5+; id "WAC.078";
r3k2r/pbp2pp1/3b1n2/1p6/3P3p/1B2N1Pq/PP1PQP1P/R1B2RK1 b kq - bm Qxh2+; id "WAC.079";
r4rk1/p1B1bpp1/1p2pn1p/8/2PP4/3B1P2/qP2QP1P/3R1RK1 w - - bm Ra1; id "WAC.080";
r4rk1/1bR1bppp/4pn2/1p2N3/1P6/P3P3/4BPPP/3R2K1 b - - bm Bd6; id "WAC.081";
3rr1k1/pp3pp1/4b3/8/2P1B2R/6QP/P3q1P1/5R1K w - - bm Bh7+; id "WAC.082";
3rr1k1/ppqbRppp/2p5/8/3Q1n2/2P3N1/PPB2PPP/3R2K1 w - - bm Qxd7; id "WAC.083";
r2q1r1k/2p1b1pp/p1n5/1p1Q1bN1/4n3/1BP1B3/PP3PPP/R4RK1 w - - bm Qg8+; id "WAC.084";
kr2R3/p4r2/2pq4/2N2p1p/3P2p1/Q5P1/5P1P/5BK1 w - - bm Na6; id "WAC.085";
8/p7/1ppk1n2/5ppp/P1PP4/2P1K1P1/5N1P/8 b - - bm Ng4+; id "WAC.086";
8/p3k1p1/4r3/2ppNpp1/PP1P4/2P3KP/5P2/8 b - - bm Rxe5; id "WAC.087";
r6k/p1Q4p/2p1b1rq/4p3/B3P3/4P3/PPP3P1/4RRK1 b - - bm Rxg2+; id "WAC.088";
1r3b1k/p4rpp/4pp2/3q4/2ppbPPQ/6RK/PP5P/2B1NR2 b - - bm g5; id "WAC.089";
3qrrk1/1pp2pp1/1p2bn1p/5N2/2P5/P1P3B1/1P4PP/2Q1RRK1 w - - bm Nxg7; id "WAC.090";
2qr2k1/4b1p1/2p2p1p/1pP1p3/p2nP3/PbQNB1PP/1P3PK1/4RB2 b - - bm Be6; id "WAC.091";
r4rk1/1p2ppbp/p2pbnp1/q7/3BPPP1/2N2B2/PPP4P/R2Q1RK1 b - - bm Bxg4; id "WAC.092";
r1b1k1nr/pp3pQp/4pq2/3pn3/8/P1P5/2P2PPP/R1B1KBNR w KQkq - bm Bh6; id "WAC.093";
8/k7/p7/3Qp2P/n1P5/3KP3/1q6/8 b - - bm e4+; id "WAC.094";
2r5/1r6/4pNpk/3pP1qp/8/2P1QP2/5PK1/R7 w - - bm Ng4+; id "WAC.095";
r1b4k/ppp2Bb1/6Pp/3pP3/1qnP1p1Q/8/PPP3P1/1K1R3R w - - bm Qd8+ b3; id "WAC.096";
6k1/5p2/p5np/4B3/3P4/1PP1q3/P3r1QP/6RK w - - bm Qa8+; id "WAC.097";
1r3rk1/5pb1/p2p2p1/Q1n1q2p/1NP1P3/3p1P1B/PP1R3P/1K2R3 b - - bm Nxe4; id "WAC.098";
r1bq1r1k/1pp1Np1p/p2p2pQ/4R3/n7/8/PPPP1PPP/R1B3K1 w - - bm Rh5; id "WAC.099";
8/k1b5/P4p2/1Pp2p1p/K1P2P1P/8/3B4/8 w - - bm Be3 b6+; id "WAC.100";
5rk1/p5pp/8/8/2Pbp3/1P4P1/7P/4RN1K b - - bm Bc3; id "WAC.101";
2Q2n2/2R4p/1p1qpp1k/8/3P3P/3B2P1/5PK1/r7 w - - bm Qxf8+; id "WAC.102";
6k1/2pb1r1p/3p1PpQ/p1nPp3/1q2P3/2N2P2/PrB5/2K3RR w - - bm Qxg6+; id "WAC.103";
b4r1k/pq2rp2/1p1bpn1p/3PN2n/2P2P2/P2B3K/1B2Q2N/3R2R1 w - - bm Qxh5; id "WAC.104";
r2r2k1/pb3ppp/1p1bp3/7q/3n2nP/PP1B2P1/1B1N1P2/RQ2NRK1 b - - bm Bxg3 Qxh4; id "WAC.105";
4rrk1/pppb4/7p/3P2pq/3Qn3/P5P1/1PP4P/R3RNNK b - - bm Nf2+; id "WAC.106";
5n2/pRrk2p1/P4p1p/4p3/3N4/5P2/6PP/6K1 w - - bm Nb5; id "WAC.107";
r5k1/1q4pp/2p5/p1Q5/2P5/5R2/4RKPP/r7 w - - bm Qe5; id "WAC.108";
rn2k1nr/pbp2ppp/3q4/1p2N3/2p5/QP6/PB1PPPPP/R3KB1R b KQkq - bm c3; id "WAC.109";
2kr4/bp3p2/p2p2b1/P7/2q5/1N4B1/1PPQ2P1/2KR4 b - - bm Be3; id "WAC.110";
6k1/p5p1/5p2/2P2Q2/3pN2p/3PbK1P/7P/6q1 b - - bm Qf1+; id "WAC.111";
r4kr1/ppp5/4bq1b/7B/2PR1Q1p/2N3P1/PP3P1P/2K1R3 w - - bm Rxe6; id "WAC.112";
rnbqkb1r/1p3ppp/5N2/1p2p1B1/2P5/8/PP2PPPP/R2QKB1R b KQkq - bm Qxf6; id "WAC.113";
r1b1rnk1/1p4pp/p1p2p2/3pN2n/3P1PPq/2NBPR1P/PPQ5/2R3K1 w - - bm Bxh7+; id "WAC.114";
4N2k/5rpp/1Q6/p3q3/8/P5P1/1P3P1P/5K2 w - - bm Nd6; id "WAC.115";
r2r2k1/2p2ppp/p7/1p2P1n1/P6q/5P2/1PB1QP1P/R5RK b - - bm Rd2; id "WAC.116";
3r1rk1/q4ppp/p1Rnp3/8/1p6/1N3P2/PP3QPP/3R2K1 b - - bm Ne4; id "WAC.117";
r5k1/pb2rpp1/1p6/2p4q/5R2/2PB2Q1/P1P3PP/5R1K w - - bm Rh4; id "WAC.118";
r2qr1k1/p1p2ppp/2p5/2b5/4nPQ1/3B4/PPP3PP/R1B2R1K b - - bm Qxd3; id "WAC.119";
r4rk1/1bn2qnp/3p1B1Q/p2P1pP1/1pp5/5N1P/PPB2P2/2KR3R w - - bm Rhg1 g6; id "WAC.120";
6k1/5p1p/2bP2pb/4p3/2P5/1p1pNPPP/1P1Q1BK1/1q6 b - - bm Bxf3+; id "WAC.121";
1k6/ppp4p/1n2pq2/1N2Rb2/2P2Q2/8/P4KPP/3r1B2 b - - bm Rxf1+; id "WAC.122";
6k1/1b2rp2/1p4p1/3P4/PQ4P1/2N2q2/5P2/3R2K1 b - - bm Bxd5 Rc7 Re6; id "WAC.123";
6k1/3r4/2R5/P5P1/1P4p1/8/4rB2/6K1 b - - bm g3; id "WAC.124";
r1bqr1k1/pp3ppp/1bp5/3n4/3B4/2N2P1P/PPP1B1P1/R2Q1RK1 b - - bm Bxd4+; id "WAC.125";
r5r1/pQ5p/1qp2R2/2k1p3/4P3/2PP4/P1P3PP/6K1 w - - bm Rxc6+; id "WAC.126";
2k4r/1pr1n3/p1p1q2p/5pp1/3P1P2/P1P1P3/1R2Q1PP/1RB3K1 w - - bm Rxb7; id "WAC.127";
6rk/1pp2Qrp/3p1B2/1pb1p2R/3n1q2/3P4/PPP3PP/R6K w - - bm Qg6; id "WAC.128";
3r1r1k/1b2b1p1/1p5p/2p1Pp2/q1B2P2/4P2P/1BR1Q2K/6R1 b - - bm Bf3; id "WAC.129";
6k1/1pp3q1/5r2/1PPp4/3P1pP1/3Qn2P/3B4/4R1K1 b - - bm Qh6 Qh8; id "WAC.130";
2rq1bk1/p4p1p/1p4p1/3b4/3B1Q2/8/P4PpP/3RR1K1 w - - bm Re8; id "WAC.131";
4r1k1/5bpp/2p5/3pr3/8/1B3pPq/PPR2P2/2R2QK1 b - - bm Re1; id "WAC.132";
r1b1k2r/1pp1q2p/p1n3p1/3QPp2/8/1BP3B1/P5PP/3R1RK1 w kq - bm Bh4; id "WAC.133";
3r2k1/p6p/2Q3p1/4q3/2P1p3/P3Pb2/1P3P1P/2K2BR1 b - - bm Rd1+; id "WAC.134";
3r1r1k/N2qn1pp/1p2np2/2p5/2Q1P2N/3P4/PP4PP/3R1RK1 b - - bm Nd4; id "WAC.135";
6kr/1q2r1p1/1p2N1Q1/5p2/1P1p4/6R1/7P/2R3K1 w - - bm Rc8+; id "WAC.136";
3b1rk1/1bq3pp/5pn1/1p2rN2/2p1p3/2P1B2Q/1PB2PPP/R2R2K1 w - - bm Rd7; id "WAC.137";
r1bq3r/ppppR1p1/5n1k/3P4/6pP/3Q4/PP1N1PP1/5K1R w - - bm h5; id "WAC.138";
rnb3kr/ppp2ppp/1b6/3q4/3pN3/Q4N2/PPP2KPP/R1B1R3 w - - bm Nf6+; id "WAC.139";
r2b1rk1/pq4p1/4ppQP/3pB1p1/3P4/2R5/PP3PP1/5RK1 w - - bm Bc7 Rc7; id "WAC.140";
4r1k1/p1qr1p2/2pb1Bp1/1p5p/3P1n1R/1B3P2/PP3PK1/2Q4R w - - bm Qxf4; id "WAC.141";
r2q3n/ppp2pk1/3p4/5Pr1/2NP1Qp1/2P2pP1/PP3K2/4R2R w - - bm Re8 f6+; id "WAC.142";
5b2/pp2r1pk/2pp1pRp/4rP1N/2P1P3/1P4QP/P3q1P1/5R1K w - - bm Rxh6+; id "WAC.143";
r2q1rk1/pp3ppp/2p2b2/8/B2pPPb1/7P/PPP1N1P1/R2Q1RK1 b - - bm d3; id "WAC.144";
r1bq4/1p4kp/3p1n2/p4pB1/2pQ4/8/1P4PP/4RRK1 w - - bm Re8; id "WAC.145";
8/8/2Kp4/3P1B2/2P2k2/5p2/8/8 w - - bm Bc8 Bd3 Bh3; id "WAC.146";
r2r2k1/ppqbppbp/2n2np1/2pp4/6P1/1P1PPNNP/PBP2PB1/R2QK2R b KQ - bm Nxg4; id "WAC.147";
2r1k3/6pr/p1nBP3/1p3p1p/2q5/2P5/P1R4P/K2Q2R1 w - - bm Rxg7; id "WAC.148";
6k1/6p1/2p4p/4Pp2/4b1qP/2Br4/1P2RQPK/8 b - - bm Bxg2; id "WAC.149";
r3r1k1/5p2/pQ1b2pB/1p6/4p3/6P1/Pq2BP1P/2R3K1 b - - bm Ba3 Be5 Bf8 e3; c0 "All win but e3 is best."; id "WAC.150";
8/3b2kp/4p1p1/pr1n4/N1N4P/1P4P1/1K3P2/3R4 w - - bm Nc3; id "WAC.151";
1br2rk1/1pqb1ppp/p3pn2/8/1P6/P1N1PN1P/1B3PP1/1QRR2K1 w - - bm Ne4; id "WAC.152";
2r3k1/q4ppp/p3p3/pnNp4/2rP4/2P2P2/4R1PP/2R1Q1K1 b - - bm Nxd4; id "WAC.153";
r1b2rk1/2p2ppp/p7/1p6/3P3q/1BP3bP/PP3QP1/RNB1R1K1 w - - bm Qxf7+; id "WAC.154";
5bk1/1rQ4p/5pp1/2pP4/3n1PP1/7P/1q3BB1/4R1K1 w - - bm d6; id "WAC.155";
r1b1qN1k/1pp3p1/p2p3n/4p1B1/8/1BP4Q/PP3KPP/8 w - - bm Qxh6+; id "WAC.156";
5rk1/p4ppp/2p1b3/3Nq3/4P1n1/1p1B2QP/1PPr2P1/1K2R2R w - - bm Ne7+; id "WAC.157";
5rk1/n1p1R1bp/p2p4/1qpP1QB1/7P/2P3P1/PP3P2/6K1 w - - bm Rxg7+; id "WAC.158";
r1b2r2/5P1p/ppn3pk/2p1p1Nq/1bP1PQ2/3P4/PB4BP/1R3RK1 w - - bm Ne6+; id "WAC.159";
qn1kr2r/1pRbb3/pP5p/P2pP1pP/3N1pQ1/3B4/3B1PP1/R5K1 w - - bm Qxd7+; id "WAC.160";
3r3k/3r1P1p/pp1Nn3/2pp4/7Q/6R1/Pq4PP/5RK1 w - - bm Qxd8+; id "WAC.161";
r3kbnr/p4ppp/2p1p3/8/Q1B3b1/2N1B3/PP3PqP/R3K2R w KQkq - bm Bd5; id "WAC.162";
5rk1/2p4p/2p4r/3P4/4p1b1/1Q2NqPp/PP3P1K/R4R2 b - - bm Qg2+; id "WAC.163";
8/6pp/4p3/1p1n4/1NbkN1P1/P4P1P/1PR3K1/r7 w - - bm Rxc4+; id "WAC.164";
1r5k/p1p3pp/8/8/4p3/P1P1R3/1P1Q1qr1/2KR4 w - - bm Re2; id "WAC.165";
r3r1k1/5pp1/p1p4p/2Pp4/8/q1NQP1BP/5PP1/4K2R b K - bm d4; id "WAC.166";
7Q/ppp2q2/3p2k1/P2Ppr1N/1PP5/7R/5rP1/6K1 b - - bm Rxg2+; id "WAC.167";
r3k2r/pb1q1p2/8/2p1pP2/4p1p1/B1P1Q1P1/P1P3K1/R4R2 b kq - bm Qd2+; id "WAC.168";
5rk1/1pp3bp/3p2p1/2PPp3/1P2P3/2Q1B3/4q1PP/R5K1 b - - bm Bh6; id "WAC.169";
5r1k/6Rp/1p2p3/p2pBp2/1qnP4/4P3/Q4PPP/6K1 w - - bm Qxc4; id "WAC.170";
2rq4/1b2b1kp/p3p1p1/1p1nNp2/7P/1B2B1Q1/PP3PP1/3R2K1 w - - bm Bh6+; id "WAC.171";
5r1k/p5pp/8/1P1pq3/P1p2nR1/Q7/5BPP/6K1 b - - bm Qe1+; id "WAC.172";
2r1b3/1pp1qrk1/p1n1P1p1/7R/2B1p3/4Q1P1/PP3PP1/3R2K1 w - - bm Qh6+; id "WAC.173";
2r2rk1/6p1/p3pq1p/1p1b1p2/3P1n2/PP3N2/3N1PPP/1Q2RR1K b - - bm Nxg2; id "WAC.174";
r5k1/pppb3p/2np1n2/8/3PqNpP/3Q2P1/PPP5/R4RK1 w - - bm Nh5; id "WAC.175";
r1bq3r/ppp2pk1/3p1pp1/8/2BbPQ2/2NP2P1/PPP4P/R4R1K b - - bm Rxh2+; id "WAC.176";
r1b3r1/4qk2/1nn1p1p1/3pPp1P/p4P2/1p3BQN/PKPBN3/3R3R b - - bm Qa3+; id "WAC.177";
3r2k1/p1rn1p1p/1p2pp2/6q1/3PQNP1/5P2/P1P4R/R5K1 w - - bm Nxe6; id "WAC.178";
r1b2r1k/pp4pp/3p4/3B4/8/1QN3Pn/PP3q1P/R3R2K b - - bm Qg1+; id "WAC.179";
r1q2rk1/p3bppb/3p1n1p/2nPp3/1p2P1P1/6NP/PP2QPB1/R1BNK2R b KQ - bm Nxd5; id "WAC.180";
r3k2r/2p2p2/p2p1n2/1p2p3/4P2p/1PPPPp1q/1P5P/R1N2QRK b kq - bm Ng4; id "WAC.181";
r1b2rk1/ppqn1p1p/2n1p1p1/2b3N1/2N5/PP1BP3/1B3PPP/R2QK2R w KQ - bm Qh5; id "WAC.182";
1r2k1r1/5p2/b3p3/1p2b1B1/3p3P/3B4/PP2KP2/2R3R1 w - - bm Bf6; id "WAC.183";
4kn2/r4p1r/p3bQ2/q1nNP1Np/1p5P/8/PPP3P1/2KR3R w - - bm Qe7+; id "WAC.184";
1r1rb1k1/2p3pp/p2q1p2/3PpP1Q/Pp1bP2N/1B5R/1P4PP/2B4K w - - bm Qxh7+; id "WAC.185";
r5r1/p1q2p1k/1p1R2pB/3pP3/6bQ/2p5/P1P1NPPP/6K1 w - - bm Bf8+; id "WAC.186";
6k1/5p2/p3p3/1p3qp1/2p1Qn2/2P1R3/PP1r1PPP/4R1K1 b - - bm Nh3+; id "WAC.187";
3RNbk1/pp3p2/4rQpp/8/1qr5/7P/P4P2/3R2K1 w - - bm Qg7+; id "WAC.188";
3r1k2/1ppPR1n1/p2p1rP1/3P3p/4Rp1N/5K2/P1P2P2/8 w - - bm Re8+; id "WAC.189";
8/p2b2kp/1q1p2p1/1P1Pp3/4P3/3B2P1/P2Q3P/2Nn3K b - - bm Bh3; id "WAC.190";
2r1Rn1k/1p1q2pp/p7/5p2/3P4/1B4P1/P1P1QP1P/6K1 w - - bm Qc4; id "WAC.191";
r3k3/ppp2Npp/4Bn2/2b5/1n1pp3/N4P2/PPP3qP/R2QKR2 b Qq - bm Nd3+; id "WAC.192";
5bk1/p4ppp/Qp6/4B3/1P6/Pq2P1P1/2rr1P1P/R4RK1 b - - bm Qxe3; id "WAC.193";
5rk1/ppq2ppp/2p5/4bN2/4P3/6Q1/PPP2PPP/3R2K1 w - - bm Nh6+; id "WAC.194";
3r1rk1/1p3p2/p3pnnp/2p3p1/2P2q2/1P5P/PB2QPPN/3RR1K1 w - - bm g3; id "WAC.195";
rr4k1/p1pq2pp/Q1n1pn2/2bpp3/4P3/2PP1NN1/PP3PPP/R1B1K2R b KQ - bm Nb4; id "WAC.196";
7k/1p4p1/7p/3P1n2/4Q3/2P2P2/PP3qRP/7K b - - bm Qf1+; id "WAC.197";
2br2k1/ppp2p1p/4p1p1/4P2q/2P1Bn2/2Q5/PP3P1P/4R1RK b - - bm Rd3; id "WAC.198";
r1br2k1/pp2nppp/2n5/1B1q4/Q7/4BN2/PP3PPP/2R2RK1 w - - bm Bxc6 Rcd1 Rfd1; id "WAC.199";
2rqrn1k/pb4pp/1p2pp2/n2P4/2P3N1/P2B2Q1/1B3PPP/2R1R1K1 w - - bm Bxf6; id "WAC.200";
2b2r1k/4q2p/3p2pQ/2pBp3/8/6P1/1PP2P1P/R5K1 w - - bm Ra7; id "WAC.201";
QR2rq1k/2p3p1/3p1pPp/8/4P3/8/P1r3PP/1R4K1 b - - bm Rxa2; id "WAC.202";
r4rk1/5ppp/p3q1n1/2p2NQ1/4n3/P3P3/1B3PPP/1R3RK1 w - - bm Qh6; id "WAC.203";
r1b1qrk1/1p3ppp/p1p5/3Nb3/5N2/P7/1P4PQ/K1R1R3 w - - bm Rxe5; id "WAC.204";
r3rnk1/1pq2bb1/p4p2/3p1Pp1/3B2P1/1NP4R/P1PQB3/2K4R w - - bm Qxg5; id "WAC.205";
1Qq5/2P1p1kp/3r1pp1/8/8/7P/p4PP1/2R3K1 b - - bm Rc6; id "WAC.206";
r1bq2kr/p1pp1ppp/1pn1p3/4P3/2Pb2Q1/BR6/P4PPP/3K1BNR w - - bm Qxg7+; id "WAC.207";
3r1bk1/ppq3pp/2p5/2P2Q1B/8/1P4P1/P6P/5RK1 w - - bm Bf7+; id "WAC.208";
4kb1r/2q2p2/r2p4/pppBn1B1/P6P/6Q1/1PP5/2KRR3 w k - bm Rxe5+; id "WAC.209";
3r1rk1/pp1q1ppp/3pn3/2pN4/5PP1/P5PQ/1PP1B3/1K1R4 w - - bm Rh1; id "WAC.210";
r1bqrk2/pp1n1n1p/3p1p2/P1pP1P1Q/2PpP1NP/6R1/2PB4/4RBK1 w - - bm Qxf7+; id "WAC.211";
rn1qr2Q/pbppk1p1/1p2pb2/4N3/3P4/2N5/PPP3PP/R4RK1 w - - bm Qxg7+; id "WAC.212";
3r1r1k/1b4pp/ppn1p3/4Pp1R/Pn5P/3P4/4QP2/1qB1NKR1 w - - bm Rxh7+; id "WAC.213";
r2r2k1/1p2qpp1/1np1p1p1/p3N3/2PPN3/bP5R/4QPPP/4R1K1 w - - bm Ng5; id "WAC.214";
3r2k1/pb1q1pp1/1p2pb1p/8/3N4/P2QB3/1P3PPP/1Br1R1K1 w - - bm Qh7+; id "WAC.215";
r2qr1k1/1b1nbppp/p3pn2/1p1pN3/3P1B2/2PB1N2/PP2QPPP/R4RK1 w - - bm Nxf7 a4; id "WAC.216";
r3kb1r/1pp3p1/p3bp1p/5q2/3QN3/1P6/PBP3P1/3RR1K1 w kq - bm Qd7+; id "WAC.217";
6k1/pp5p/2p3q1/6BP/2nPr1Q1/8/PP3R1K/8 w - - bm Bh6; id "WAC.218";
7k/p4q1p/1pb5/2p5/4B2Q/2P1B3/P6P/7K b - - bm Qf1+; id "WAC.219";
3rr1k1/ppp2ppp/8/5Q2/4n3/1B5R/PPP1qPP1/5RK1 b - - bm Qxf1+; id "WAC.220";
r3k3/P5bp/2N1bp2/4p3/2p5/6NP/1PP2PP1/3R2K1 w q - bm Rd8+; id "WAC.221";
2r1r2k/1q3ppp/p2Rp3/2p1P3/6QB/p3P3/bP3PPP/3R2K1 w - - bm Bf6; id "WAC.222";
r1bqk2r/pp3ppp/5n2/8/1b1npB2/2N5/PP1Q2PP/1K2RBNR w kq - bm Nxe4; id "WAC.223";
5rk1/p1q3pp/1p1r4/2p1pp1Q/1PPn1P2/3B3P/P2R2P1/3R2K1 b - - bm Rh6 e4; id "WAC.224";
4R3/4q1kp/6p1/1Q3b2/1P1b1P2/6KP/8/8 b - - bm Qh4+; id "WAC.225";
2b2rk1/p1p4p/2p1p1p1/br2N1Q1/1p2q3/8/PB3PPP/3R1RK1 w - - bm Nf7; id "WAC.226";
2k1rb1r/ppp3pp/2np1q2/5b2/2B2P2/2P1BQ2/PP1N1P1P/2KR3R b - - bm d5; id "WAC.227";
r4rk1/1bq1bp1p/4p1p1/p2p4/3BnP2/1N1B3R/PPP3PP/R2Q2K1 w - - bm Bxe4; id "WAC.228";
8/8/8/1p5r/p1p1k1pN/P2pBpP1/1P1K1P2/8 b - - bm Rxh4 b4; id "WAC.229";
2b5/1r6/2kBp1p1/p2pP1P1/2pP4/1pP3K1/1R3P2/8 b - - bm Rb4; id "WAC.230";
r4rk1/1b1nqp1p/p5p1/1p2PQ2/2p5/5N2/PP3PPP/R1BR2K1 w - - bm Bg5; id "WAC.231";
1R2rq1k/2p3p1/Q2p1pPp/8/4P3/8/P1r3PP/1R4K1 w - - bm Qb5 Rxe8; id "WAC.232";
5rk1/p1p2r1p/2pp2p1/4p3/PPPnP3/3Pq1P1/1Q1R1R1P/4NK2 b - - bm Nb3; id "WAC.233";
2kr1r2/p6p/5Pp1/2p5/1qp2Q1P/7R/PP6/1KR5 w - - bm Rb3; id "WAC.234";
5r2/1p1RRrk1/4Qq1p/1PP3p1/8/4B3/1b3P1P/6K1 w - - bm Qe4 Qxf7+ Rxf7+; id "WAC.235";
1R6/p5pk/4p2p/4P3/8/2r3qP/P3R1b1/4Q1K1 b - - bm Rc1; id "WAC.236";
r5k1/pQp2qpp/8/4pbN1/3P4/6P1/PPr4P/1K1R3R b - - bm Rc1+; id "WAC.237";
1k1r4/pp1r1pp1/4n1p1/2R5/2Pp1qP1/3P2QP/P4PB1/1R4K1 w - - bm Bxb7; id "WAC.238";
8/6k1/5pp1/Q6p/5P2/6PK/P4q1P/8 b - - bm Qf1+; id "WAC.239";
2b4k/p1b2p2/2p2q2/3p1PNp/3P2R1/3B4/P1Q2PKP/4r3 w - - bm Qxc6; id "WAC.240";
2rq1rk1/pp3ppp/2n2b2/4NR2/3P4/PB5Q/1P4PP/3R2K1 w - - bm Qxh7+; id "WAC.241";
r1b1r1k1/pp1nqp2/2p1p1pp/8/4N3/P1Q1P3/1P3PPP/1BRR2K1 w - - bm Rxd7; id "WAC.242";
1r3r1k/3p4/1p1Nn1R1/4Pp1q/pP3P1p/P7/5Q1P/6RK w - - bm Qe2; id "WAC.243";
r6r/pp3ppp/3k1b2/2pb4/B4Pq1/2P1Q3/P5PP/1RBR2K1 w - - bm Qxc5+; id "WAC.244";
4rrn1/ppq3bk/3pPnpp/2p5/2PB4/2NQ1RPB/PP5P/5R1K w - - bm Qxg6+; id "WAC.245";
6R1/4qp1p/ppr1n1pk/8/1P2P1QP/6N1/P4PP1/6K1 w - - bm Qh5+; id "WAC.246";
2k1r3/1p2Bq2/p2Qp3/Pb1p1p1P/2pP1P2/2P5/2P2KP1/1R6 w - - bm Rxb5; id "WAC.247";
5r1k/1p4pp/3q4/3Pp1R1/8/8/PP4PP/4Q1K1 b - - bm Qc5+; id "WAC.248";
r4rk1/pbq2pp1/1ppbpn1p/8/2PP4/1P1Q1N2/PBB2PPP/R3R1K1 w - - bm c5 d5; id "WAC.249";
1b5k/7P/p1p2np1/2P2p2/PP3P2/4RQ1R/q2r3P/6K1 w - - bm Re8+; id "WAC.250";
k7/p4p2/P1q1b1p1/3p3p/3Q4/7P/5PP1/1R4K1 w - - bm Qe5 Qf4; id "WAC.251";
1rb1r1k1/p1p2ppp/5n2/2pP4/5P2/2QB4/qNP3PP/2KRB2R b - - bm Bg4 Re2; c0 "Bg4 wins, but Re2 is far better."; id "WAC.252";
k5r1/p4b2/2P5/5p2/3P1P2/4QBrq/P5P1/4R1K1 w - - bm Qe8+; id "WAC.253";
r6k/pp3p1p/2p1bp1q/b3p3/4Pnr1/2PP2NP/PP1Q1PPN/R2B2RK b - - bm Nxh3; id "WAC.254";
3r3r/p4pk1/5Rp1/3q4/1p1P2RQ/5N2/P1P4P/2b4K w - - bm Rfxg6+; id "WAC.255";
3r1rk1/1pb1qp1p/2p3p1/p7/P2Np2R/1P5P/1BP2PP1/3Q1BK1 w - - bm Nf5; id "WAC.256";
4r1k1/pq3p1p/2p1r1p1/2Q1p3/3nN1P1/1P6/P1P2P1P/3RR1K1 w - - bm Rxd4; id "WAC.257";
r3brkn/1p5p/2p2Ppq/2Pp3B/3Pp2Q/4P1R1/6PP/5R1K w - - bm Bxg6; id "WAC.258";
r1bq1rk1/ppp2ppp/2np4/2bN1PN1/2B1P3/3p4/PPP2nPP/R1BQ1K1R w - - bm Qh5; id "WAC.259";
2r2b1r/p1Nk2pp/3p1p2/N2Qn3/4P3/q6P/P4PP1/1R3K1R w - - bm Qe6+; id "WAC.260";
r5k1/1bp3pp/p2p4/1p6/5p2/1PBP1nqP/1PP3Q1/R4R1K b - - bm Nd4; id "WAC.261";
6k1/p1B1b2p/2b3r1/2p5/4p3/1PP1N1Pq/P2R1P2/3Q2K1 b - - bm Rh6; id "WAC.262";
rnbqr2k/pppp1Qpp/8/b2NN3/2B1n3/8/PPPP1PPP/R1B1K2R w KQ - bm Qg8+; id "WAC.263";
r2r2k1/1R2qp2/p5pp/2P5/b1PN1b2/P7/1Q3PPP/1B1R2K1 b - - bm Qe5 Rab8; id "WAC.264";
2r1k2r/2pn1pp1/1p3n1p/p3PP2/4q2B/P1P5/2Q1N1PP/R4RK1 w k - bm exf6; id "WAC.265";
r3q2r/2p1k1p1/p5p1/1p2Nb2/1P2nB2/P7/2PNQbPP/R2R3K b - - bm Rxh2+; id "WAC.266";
2r1kb1r/pp3ppp/2n1b3/1q1N2B1/1P2Q3/8/P4PPP/3RK1NR w Kk - bm Nc7+; id "WAC.267";
2r3kr/ppp2n1p/7B/5q1N/1bp5/2Pp4/PP2RPPP/R2Q2K1 w - - bm Re8+; id "WAC.268";
2kr2nr/pp1n1ppp/2p1p3/q7/1b1P1B2/P1N2Q1P/1PP1BPP1/R3K2R w KQ - bm axb4; id "WAC.269";
2r1r1k1/pp1q1ppp/3p1b2/3P4/3Q4/5N2/PP2RPPP/4R1K1 w - - bm Qg4; id "WAC.270";
2kr4/ppp3Pp/4RP1B/2r5/5P2/1P6/P2p4/3K4 w - - bm Rd6; id "WAC.271";
nrq4r/2k1p3/1p1pPnp1/pRpP1p2/P1P2P2/2P1BB2/1R2Q1P1/6K1 w - - bm Bxc5; id "WAC.272";
2k4B/bpp1qp2/p1b5/7p/1PN1n1p1/2Pr4/P5PP/R3QR1K b - - bm Ng3+ g3; id "WAC.273";
8/1p6/p5R1/k7/Prpp4/K7/1NP5/8 w - - am Rd6; bm Rb6 Rg5+; id "WAC.274";
r1b2rk1/1p1n1ppp/p1p2q2/4p3/P1B1Pn2/1QN2N2/1P3PPP/3R1RK1 b - - bm Nc5 Nxg2 b5; id "WAC.275";
r5k1/pp1RR1pp/1b6/6r1/2p5/B6P/P4qPK/3Q4 w - - bm Qd5+; id "WAC.276";
1r4r1/p2kb2p/bq2p3/3p1p2/5P2/2BB3Q/PP4PP/3RKR2 b - - bm Rg3 Rxg2; id "WAC.277";
r2qkb1r/pppb2pp/2np1n2/5pN1/2BQP3/2N5/PPP2PPP/R1B1K2R w KQkq - bm Bf7+; id "WAC.278";
r7/4b3/2p1r1k1/1p1pPp1q/1P1P1P1p/PR2NRpP/2Q3K1/8 w - - bm Nxf5 Rc3; id "WAC.279";
r1r2bk1/5p1p/pn4p1/N2b4/3Pp3/B3P3/2q1BPPP/RQ3RK1 b - - bm Bxa3; id "WAC.280";
2R5/2R4p/5p1k/6n1/8/1P2QPPq/r7/6K1 w - - bm Rxh7+; id "WAC.281";
6k1/2p3p1/1p1p1nN1/1B1P4/4PK2/8/2r3b1/7R w - - bm Rh8+; id "WAC.282";
3q1rk1/4bp1p/1n2P2Q/3p1p2/6r1/Pp2R2N/1B4PP/7K w - - bm Ng5; id "WAC.283";
3r3k/pp4pp/8/1P6/3N4/Pn2P1qb/1B1Q2B1/2R3K1 w - - bm Nf5; id "WAC.284";
2rr3k/1b2bppP/p2p1n2/R7/3P4/1qB2P2/1P4Q1/1K5R w - - bm Qxg7+; id "WAC.285";
3r1k2/1p6/p4P2/2pP2Qb/8/1P1KB3/P6r/8 b - - bm Rxd5+; id "WAC.286";
rn3k1r/pp2bBpp/2p2n2/q5N1/3P4/1P6/P1P3PP/R1BQ1RK1 w - - bm Qg4 Qh5; id "WAC.287";
r1b2rk1/p4ppp/1p1Qp3/4P2N/1P6/8/P3qPPP/3R1RK1 w - - bm Nf6+; id "WAC.288";
2r3k1/5p1p/p3q1p1/2n3P1/1p1QP2P/1P4N1/PK6/2R5 b - - bm Qe5; id "WAC.289";
2k2r2/2p5/1pq5/p1p1n3/P1P2n1B/1R4Pp/2QR4/6K1 b - - bm Ne2+; id "WAC.290";
5r1k/3b2p1/p6p/1pRpR3/1P1P2q1/P4pP1/5QnP/1B4K1 w - - bm h3; id "WAC.291";
4r3/1Q1qk2p/p4pp1/3Pb3/P7/6PP/5P2/4R1K1 w - - bm d6+; id "WAC.292";
1nbq1r1k/3rbp1p/p1p1pp1Q/1p6/P1pPN3/5NP1/1P2PPBP/R4RK1 w - - bm Nfg5; id "WAC.293";
3r3k/1r3p1p/p1pB1p2/8/p1qNP1Q1/P6P/1P4P1/3R3K w - - bm Bf8 Nf5 Qf4; id "WAC.294";
4r3/p4r1p/R1p2pp1/1p1bk3/4pNPP/2P1K3/2P2P2/3R4 w - - bm Rxd5+; id "WAC.295";
3r4/1p2k2p/p1b1p1p1/4Q1Pn/2B3KP/4pP2/PP2R1N1/6q1 b - - bm Rd4+ Rf8; id "WAC.296";
3r1rk1/p3qp1p/2bb2p1/2p5/3P4/1P6/PBQN1PPP/2R2RK1 b - - bm Bxg2 Bxh2+; id "WAC.297";
3Q4/p3b1k1/2p2rPp/2q5/4B3/P2P4/7P/6RK w - - bm Qh8+; id "WAC.298";
1n2rr2/1pk3pp/pNn2p2/2N1p3/8/6P1/PP2PPKP/2RR4 w - - bm Nca4; id "WAC.299";
b2b1r1k/3R1ppp/4qP2/4p1PQ/4P3/5B2/4N1K1/8 w - - bm g6; id "WAC.300";
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn mate_in_2() -> Vec<Position> {
        // http://wtharvey.com/m8n2.txt
        let strs = &[
            "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1
            pv 1. Nf6+ gxf6 2. Bxf7#;
            id 'Henry Buckle vs NN, London, 1840';",
            "r3k2r/p3bpp1/2q1p1b1/1ppPP1B1/3n3P/5NR1/PP2NP2/K1QR4 b kq - 0 1
            pv 1. .. Nb3+ 2. axb3 Qa6#;
            id 'Alexander Areshchenko vs Sergei Zhigalko, Kiev, 6/6/2013';",
            "4kb1r/p2n1ppp/4q3/4p1B1/4P3/1Q6/PPP2PPP/2KR4 w k - 1 0
            id 'Paul Morphy vs Duke Isouard, Paris, 1858';
            pv 1. Qb8+ Nxb8 2. Rd8#;",
            "r1b2k1r/ppp1bppp/8/1B1Q4/5q2/2P5/PPP2PPP/R3R1K1 w - - 1 0
            id 'Johannes Zukertort vs Adolf Anderssen, Breslau, 1865';
            pv 1. Qd8+ Bxd8 2. Re8#;",
            "5rkr/pp2Rp2/1b1p1Pb1/3P2Q1/2n3P1/2p5/P4P2/4R1K1 w - - 1 0
            id 'Gustav Neumann vs Carl Mayet, Berlin, 1866';
            pv 1. Qxg6+ fxg6 2. Rg7#;",
            // multiple pv
            // "1r1kr3/Nbppn1pp/1b6/8/6Q1/3B1P2/Pq3P1P/3RR1K1 w - - 1 0
            // id 'Joseph Blackburne vs Martin, England, 1876';
            // pv 1. Qxd7+ Kxd7 2. Bb5#;,
            // pv0 1. Qxd7+ Kxd7 2. Bf5#;",
            "5rk1/1p1q2bp/p2pN1p1/2pP2Bn/2P3P1/1P6/P4QKP/5R2 w - - 1 0
            id 'Wilfried Paulsen vs Adolf Anderssen, Frankfurt, 1878';
            pv 1. Qxf8+ Bxf8 2. Rxf8#;",
            "r1nk3r/2b2ppp/p3b3/3NN3/Q2P3q/B2B4/P4PPP/4R1K1 w - - 1 0
            id 'Joseph Blackburne vs Smith, Simul, 1882';
            pv 1. Qd7+ Bxd7 2. Nxf7#;",
            "r4br1/3b1kpp/1q1P4/1pp1RP1N/p7/6Q1/PPB3PP/2KR4 w - - 1 0
            id 'Wilhelm Steinitz vs David Sands, New York, 1887';
            pv 1. Qg6+ hxg6 2. fxg6;",
            "r1b2k1r/ppppq3/5N1p/4P2Q/4PP2/1B6/PP5P/n2K2R1 w - - 1 0
            id 'Wilhelm Steinitz vs Albert Hodges, New York, 1891';
            pv 1. Qxh6+ Rxh6 2. Rg8# ;",
            "6k1/pp4p1/2p5/2bp4/8/P5Pb/1P3rrP/2BRRN1K b - - 0 1
            pv 1... Rg1+ 2. Kxg1 Rxf1#;
            id 'James Mason vs Georg Marco, Leipzig, 1894';",
            "rnbqkbn1/ppppp3/7r/6pp/3P1p2/3BP1B1/PPP2PPP/RN1QK1NR w - - 1 0
            pv 1. Qxh5+ Rxh5 2. Bg6#;
            id 'Frank Teed vs Eugene Delmar, New York, 1896';",
            // blacks reply not forced
            // "r2qrb2/p1pn1Qp1/1p4Nk/4PR2/3n4/7N/P5PP/R6K w - - 1 0
            // pv 1. Ne7 Nxf5 2. Qg6#;
            // id 'Wilhelm Steinitz vs Herbert Trenchard, Vienna, 1898';",
            "8/2r5/1k5p/1pp4P/8/K2P4/PR2QB2/2q5 b - - 0 1
            pv 1... Qc3+ 2. Rb3 Ra7#;
            id 'James Mason vs Emanuel Lasker, London, 1899';",
        ];
        Position::parse_many_epd(strs).unwrap()
    }

    pub fn mate_in_3() -> [Position; 1] {
        [
            // http://wtharvey.com/m8n3.txt
            Position::parse_epd(
                "r3k2r/ppp2Npp/1b5n/4p2b/2B1P2q/BQP2P2/P5PP/RN5K w kq - 1 1 
                dm 3; 
                pv 1. Bb5+ c6 2. Qe6+ Qe7 3. Qxe7#; 
                c0 \"William Evans vs Alexander MacDonnell, London, 1826\"",
            )
            .unwrap(),
        ]
    }

    pub fn mate_in_4() -> Vec<Position> {
        let str = r#"
2k5/7Q/8/8/8/3K4/8/8 w - - 3 1 id 'KQ vs k'; dm 4;             
"#;
        let positions = Position::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn perft_kiwipete() -> (Board, Vec<u64>) {
        // https://www.chessprogramming.org/Perft_Results
        (
            Board::parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap()
                .as_board(),
            vec![1, 48, 2039, 97_862, 4_085_603, 193_690_690, 8_031_647_685],
        )
    }

    pub fn perft_cpw_number3() -> (Board, Vec<u64>) {
        // https://www.chessprogramming.org/Perft_Results
        (
            Board::parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 id 'CPW PERFT#3'")
                .unwrap()
                .as_board(),
            vec![1, 14, 191, 2812, 43_238, 674_624, 11_030_083, 178_633_661, 3_009_794_393],
        )
    }

    // FIXME! awful structure
    #[allow(clippy::vec_init_then_push)]    
    pub fn perfts() -> Vec<(Board, Vec<u64>)> {
        let mut vec = Vec::new();
        // https://www.chessprogramming.org/Perft_Results
        vec.push((
            Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap()
                .as_board(),
            vec![1, 20, 400, 8902, 197_281, 4_865_609],
        ));
        vec.push(Self::perft_kiwipete());
        // http://www.rocechess.ch/perft.html
        vec.push((
            Board::parse_fen("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1").unwrap().as_board(),
            vec![1, 24, 496, 9_483, 182_838, 3_605_103, 71_179_139],
        ));
        vec.push((
            Board::parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 id 'CPW PERFT#3'")
                .unwrap()
                .as_board(),
            vec![1, 14, 191, 2812, 43_238, 674_624, 11_030_083, 178_633_661, 3_009_794_393],
        ));
        vec.push(Self::perft_cpw_number3());
        vec.push((
            Board::parse_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1")
                .unwrap()
                .as_board(),
            vec![1, 6, 264, 9467, 422_333, 15_833_292, 706_045_033],
        ));
        vec.push((
            Board::parse_fen("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1")
                .unwrap()
                .as_board(),
            vec![1, 6, 264, 9467, 422_333, 15_833_292, 706_045_033],
        ));
        vec.push((
            Board::parse_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8")
                .unwrap()
                .as_board(),
            vec![1, 44, 1486, 62379, 2_103_487, 89_941_194],
        ));
        vec.push((
            Board::parse_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10")
                .unwrap()
                .as_board(),
            vec![1, 46, 2079, 89_890, 3_894_594, 164_075_551, 6_923_051_137],
        ));
        vec.push((
            Board::parse_fen("r1b2rk1/2p2ppp/p7/1p6/3P3q/1BP3bP/PP3QP1/RNB1R1K1 w - - 1 0")
                .unwrap()
                .as_board(),
            vec![1, 40, 1334, 50_182, 1_807_137],
        ));
        vec
    }

    pub fn bench() -> Vec<Position> {
        Self::bratko_kopec()
    }

    // logic for chess960 starting positions
    // http://www.russellcottrell.com/Chess/Chess960.htm
    //
    // Bw = n mod 4
    // n = floor(n/4)
    //
    // Bb = n mod 4
    // n = floor(n/4)
    //
    // Q = n mod 6
    // n = floor(n/6)
    //
    // N2 = n + (3-N1)(4-N1)/2 - 5  where N1<N2, 0 <= N1 <= 3, 1 <= N2 <= 4
    //
    pub fn chess960(id: u32) -> Position {
        let mut n = id as usize;
        let bw = n % 4;
        n /= 4;

        let bb = n % 4;
        n /= 4;

        let q = n % 6;
        n /= 6;

        let mut n1 = 0;
        let mut n2;
        loop {
            n2 = n + (3 - n1) * (4 - n1) / 2 - 5;
            if (1..=4).contains(&n2) && n1 < n2 {
                break;
            }
            n1 += 1;
        }
        // println!("chess960: n1, n2, bw, bb, q {} {} {} {} {}", n1, n2, bw, bb, q);
        // now work out actual squares
        let mut chars = vec!['.'; 8];
        let bw = bw * 2 + 1;
        let bb = bb * 2;
        chars[bw] = 'B';
        chars[bb] = 'B';
        let q = (0..8).into_iter().filter(|&x| chars[x] == '.').nth(q).unwrap();
        chars[q] = 'Q';
        let n1 = (0..8).into_iter().filter(|&x| chars[x] == '.').nth(n1).unwrap();
        let n2 = (0..8).into_iter().filter(|&x| chars[x] == '.').nth(n2).unwrap();
        chars[n1] = 'N';
        chars[n2] = 'N';
        let r1 = (0..8).into_iter().position(|x| chars[x] == '.').unwrap();
        chars[r1] = 'R';
        let r2 = (0..8).into_iter().rposition(|x| chars[x] == '.').unwrap();
        chars[r2] = 'R';
        let k = (0..8).into_iter().position(|x| chars[x] == '.').unwrap();
        chars[k] = 'K';
        let mut b = Board::new_empty();
        b.set(RANK_1, &chars.iter().collect::<String>()).unwrap();
        b.set(RANK_2, "PPPPPPPP").unwrap();
        b.set(RANK_7, "pppppppp").unwrap();
        b.set(RANK_8, &chars.iter().collect::<String>().to_lowercase()).unwrap();
        b.set_castling(CastlingRights::ALL);
        let mut pos = Position::from_board(b);
        pos.set(Tag::Id(format!("Chess960(SP{})", id)));
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_serde() {
        assert_eq!(serde_json::to_string(&CatalogSuite::WinAtChess).unwrap(), r#""WinAtChess""#);
    }

    #[test]
    fn test_catalog_wac() {
        let epds = Catalog::win_at_chess();
        assert_eq!(epds.len(), 300);
        assert_eq!(epds[0].id().ok(), Some("WAC.001"));
    }

    #[test]
    fn test_catalog_bratko_kopec() {
        let epds = Catalog::bratko_kopec();
        assert_eq!(epds.len(), 24);
        assert_eq!(epds[0].id().ok(), Some("BK.01"));
    }

    #[test]
    fn test_catalog_chess960() {
        //  testing data https://www.mark-weeks.com/cfaa/chess960/c960strt.htm
        let p = Catalog::chess960(518);
        assert_eq!(p.board(), &Catalog::starting_board());
        assert_eq!(Catalog::chess960(0).board().get(RANK_1), "BBQNNRKR");
        assert_eq!(Catalog::chess960(1).board().get(RANK_1), "BQNBNRKR");
        assert_eq!(Catalog::chess960(15).board().get(RANK_1), "QNNRKRBB");
        assert_eq!(Catalog::chess960(959).board().get(RANK_1), "RKRNNQBB");
        // check unique
        let mut set = HashSet::new();
        for id in 0..960 {
            set.insert(Catalog::chess960(id).board().hash());
        }
        set.insert(Catalog::starting_board().hash()); // add a duplicate
        assert_eq!(set.len(), 960);
    }
}
