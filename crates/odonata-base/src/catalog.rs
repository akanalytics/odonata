use crate::{
    bits::{bitboard::Bitboard, castling::CastlingRights},
    epd::Epd,
    globals::constants::*,
    infra::resources::read_resource_file,
    prelude::Board,
};
use serde::{Deserialize, Serialize};
use std::fmt;

pub struct Catalog;

// http://computer-chess.org/doku.php?id=computer_chess:wiki:download:epd_contents

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CatalogSuite {
    Empty,
    BratkoKopec,
    WinAtChess,
    Iq81,
    Tricky,
    Checkmate,
    Zugzwang,
    Repetitions,
    Famous,
    EndGame,
    Pin,
    Recogs,
    Move,
    Quiesce,
    See,
    MateIn2,
    MateIn3,
    MateIn4,
    Chess960,
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
    pub fn positions(suite: CatalogSuite) -> Vec<Epd> {
        match suite {
            CatalogSuite::Empty => Vec::new(),
            CatalogSuite::BratkoKopec => Self::bratko_kopec(),
            CatalogSuite::WinAtChess => Self::win_at_chess(),
            CatalogSuite::Iq81 => Self::iq(),
            CatalogSuite::Tricky => Self::tricky(),
            CatalogSuite::Checkmate => Self::checkmates(),
            CatalogSuite::Repetitions => Self::repetitions(),
            CatalogSuite::Zugzwang => Self::zugzwangs(),
            CatalogSuite::Famous => Self::famous(),
            CatalogSuite::EndGame => Self::end_games(),
            CatalogSuite::Pin => Self::pins(),
            CatalogSuite::Recogs => Self::recogs(),
            CatalogSuite::Move => Self::moves(),
            CatalogSuite::Quiesce => Self::quiesce(),
            CatalogSuite::See => Self::see(),
            CatalogSuite::MateIn2 => Self::mate_in_2(),
            CatalogSuite::MateIn3 => Self::mate_in_3(),
            CatalogSuite::MateIn4 => Self::mate_in_4(),
            CatalogSuite::Bench => Self::bench(),
            CatalogSuite::ExampleGame => Self::example_game(),
            CatalogSuite::Chess960 => (0..960).map(Self::chess960).collect(),
            CatalogSuite::Perft => todo!(),
        }
    }

    pub const STARTING_POSITION_FEN: &'static str =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    pub fn starting_board() -> Board {
        Board::parse_fen(Self::STARTING_POSITION_FEN).unwrap()
    }

    pub fn starting_position() -> Epd {
        Epd::from_board(Self::starting_board())
    }

    pub fn test_position() -> Epd {
        let epd = r#"3r2k1/6p1/1p1q3p/1Q1prp1P/Pp6/1Pp1P3/2PnBPP1/3RR1K1 w - - 0 29 bm Qd3;
        id 'https://lichess.org/IieZFrPE#56'
        "#;
        Epd::parse_epd(epd).unwrap()
    }

    pub fn example_game() -> Vec<Epd> {
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

        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn white_starting_position() -> Board {
        // FIXME: set calls
        Board::parse_fen(Self::STARTING_POSITION_FEN)
            .unwrap()
            .set(Bitboard::RANK_7 | Bitboard::RANK_8, "................")
            .unwrap()
            .clone()
    }

    pub fn black_starting_position() -> Board {
        // FIXME: set calls
        Board::parse_fen(Self::STARTING_POSITION_FEN)
            .unwrap()
            .set(Bitboard::RANK_1 | Bitboard::RANK_2, "................")
            .unwrap()
            .clone()
    }

    pub fn zugzwangs() -> Vec<Epd> {
        let str = r#"
        #
        ## https://www.chessprogramming.org/Null_Move_Test-Positions
        #
        8/8/p1p5/1p5p/1P5p/8/PPP2K1p/4R1rk w - - 0 1 bm Rf1; id "zugzwang.001";
        1q1k4/2Rr4/8/2Q3K1/8/8/8/8 w - - 0 1 bm Kh6;  id "zugzwang.002";
        7k/5K2/5P1p/3p4/6P1/3p4/8/8 w - - 0 1 bm g5; id "zugzwang.003";
        8/6B1/p5p1/Pp4kp/1P5r/5P1Q/4q1PK/8 w - - 0 32 bm Qxh4; id "zugzwang.004";
        8/8/1p1r1k2/p1pPN1p1/P3KnP1/1P6/8/3R4 b - - 0 1 bm Nxd5; id "zugzwang.005";
"#;
        Epd::parse_many_epd(str.lines()).unwrap()
    }

    pub fn repetitions() -> Vec<Epd> {
        let str = r#"
        startpos moves; rc 1; c0 'search_reps:1';
        startpos moves Nf3 Nf6; rc 1;c0 'search_reps:1';
        startpos moves; pv Nf3 Nf6; rc 1;c0 'search_reps:1';
        startpos moves Nf3 Nf6 Ng1 Ng8 Nf3 Nf6; rc 2;c0 'search_reps:1';
        startpos moves; pv Nf3 Nf6 Ng1 Ng8 Nf3 Nf6; rc 1;c0 'search_reps:2';
        startpos moves Nf3 Nf6 Ng1 Ng8 Nf3; pv Nf6; rc 2; c1 'pv counts as played'; c0 'search_reps:1';
        startpos moves Nf3 Nf6 Ng1 Ng8 Nf3 Nf6 Ng1 Ng8 Nf3 Nf6; rc 3;c0 'search_reps:1';
        "#;
        Epd::parse_many_epd(str.lines()).unwrap()
    }

    pub fn tricky() -> Vec<Epd> {
        let str = r#"
3n3k/rb5b/1p5r/pPp5/PpPp1p2/3PpPp1/RN2P1P1/QBNR1BK1 w - - 0 1 id 'TR.01'; c0 'https://chess.stackexchange.com/questions/29530'; c1 'ce<0 as black can checkmate';
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn pawn_structure() -> Vec<Epd> {
        let str = r#"
        k7/p7/P7/8/8/8/P7/K7 w - - 0 1 id 'PAWN.01'; c0 'isolated'; c1 '2'; c2 '1';
        k7/p7/P7/8/8/8/P7/K7 w - - 0 1 id 'PAWN.01'; c0 'passed';   c1 '0'; c2 '0';
        k7/p7/P7/8/8/8/P7/K7 w - - 0 1 id 'PAWN.01'; c0 'doubled';  c1 '0'; c2 '0'; c3 'isolated doubled';
        k7/p5p1/8/7P/8/3p4/P2P4/K7 w - - 0 1 id 'PAWN.02'; c0 'isolated'; c1 '3'; c2 '3';
        k7/p5p1/8/7P/8/3p4/P2P4/K7 w - - 0 1 id 'PAWN.02'; c0 'passed';   c1 '0'; c2 '0';
        k7/p5p1/8/7P/8/3p4/P2P4/K7 w - - 0 1 id 'PAWN.02'; c0 'doubled';  c1 '0'; c2 '0';
        k7/p5p1/8/7P/3P2p1/3p4/P2PP3/K7 w - - 0 1 id 'PAWN.03'; c0 'isolated'; c1 '2'; c2 '4';
        k7/p5p1/8/7P/3P2p1/3p4/P2PP3/K7 w - - 0 1 id 'PAWN.03'; c0 'passed_r4';   c1 '1'; c2 '0';
        k7/p5p1/8/7P/3P2p1/3p4/P2PP3/K7 w - - 0 1 id 'PAWN.03'; c0 'passed_r5';   c1 '0'; c2 '1';
        k7/p5p1/8/7P/3P2p1/3p4/P2PP3/K7 w - - 0 1 id 'PAWN.03'; c0 'doubled';  c1 '1'; c2 '0'; c3 'isolated double for b';
        2k5/ppp2ppp/8/4P3/P7/3p4/PP5P/K7 b - - 0 1 id 'PAWN.04'; c0 'isolated'; c1 '2'; c2 '0';
        2k5/ppp2ppp/8/4P3/P7/3p4/PP5P/K7 b - - 0 1 id 'PAWN.04'; c0 'passed_r6';   c1 '0'; c2 '1';
        2k5/ppp2ppp/8/4P3/P7/3p4/PP5P/K7 b - - 0 1 id 'PAWN.04'; c0 'doubled';  c1 '1'; c2 '0';
        2k5/ppp2p1p/5p2/5p1P/3P3P/3P3P/P2P3P/K7 w - - 0 1 id 'PAWN.05'; c0 'isolated'; c1 '8'; c2 '4'; 
        2k5/ppp2p1p/5p2/5p1P/3P3P/3P3P/P2P3P/K7 w - - 0 1 id 'PAWN.05'; c0 'passed';   c1 '0'; c2 '2'; 
        2k5/ppp2p1p/5p2/5p1P/3P3P/3P3P/P2P3P/K7 w - - 0 1 id 'PAWN.05'; c0 'passed_r4';   c1 '0'; c2 '1'; 
        2k5/ppp2p1p/5p2/5p1P/3P3P/3P3P/P2P3P/K7 w - - 0 1 id 'PAWN.05'; c0 'doubled';  c1 '0'; c2 '0'; 
        k7/p2p3p/3p3p/3p3p/5P1p/5P2/PPP2P1P/2K5 b - - 0 1 id 'PAWN.06'; c0 'isolated'; c1 '4'; c2 '8';
        k7/p2p3p/3p3p/3p3p/5P1p/5P2/PPP2P1P/2K5 b - - 0 1 id 'PAWN.06'; c0 'passed_r4';   c1 '1'; c2 '0';
        k7/p2p3p/3p3p/3p3p/5P1p/5P2/PPP2P1P/2K5 b - - 0 1 id 'PAWN.06'; c0 'doubled';  c1 '0'; c2 '0';
        k7/p5p1/4P3/8/3P4/3p4/P2PP1p1/K7 w - - 0 1 id 'PAWN.07'; c0 'passed_r5';  c1 '0'; c2 '0';
        k7/p5p1/4P3/8/3P4/3p4/P2PP1p1/K7 w - - 0 1 id 'PAWN.07'; c0 'passed_r6';  c1 '1'; c2 '0';
        k7/p5p1/4P3/8/3P4/3p4/P2PP1p1/K7 w - - 0 1 id 'PAWN.07'; c0 'passed_r7';  c1 '0'; c2 '1';
        k7/3p4/PPp3P1/1p3P2/4P3/8/6pp/K7 w - - 0 1 id 'PAWN.08'; c0 'connected_r67';   c1 '1'; c2 '0';
        k7/3p4/PPp3P1/1p3P2/4P3/8/6pp/K7 w - - 0 1 id 'PAWN.08'; c0 'connected_r345';  c1 '1'; c2 '2';
        k7/3p4/PPp3P1/1p3P2/4P3/8/6pp/K7 w - - 0 1 id 'PAWN.08'; c0 'backward_half_open';  c1 '0'; c2 '0';
        k7/8/8/5p2/6p1/6Pp/7P/K7 w - - 0 1 id 'PAWN.09'; c0 'backward_half_open';  c1 '0'; c2 '1';
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn checkmates() -> Vec<Epd> {
        let str = r#"
k7/Q7/K7/8/8/8/8/8 b - - 0 1
K7/7r/8/8/8/8/8/rr5k w - - 0 1
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn stalemates() -> Vec<Epd> {
        let strs = &[
            "k7/8/1Q6/8/8/8/8/7K b - - 0 1 id 'KQk';",
            "K7/7r/8/8/8/8/8/1r5k w - - 0 1 id 'Krrk';",
        ];
        Epd::parse_many_epd(strs).unwrap()
    }

    pub fn end_games() -> Vec<Epd> {
        // avoid king on side/corner, so distance win-metrics are calculated
        let strs = &[
            "8/1k6/8/8/8/2K4R/8/8 w - - 0 1 id 'KRK';",
            "8/1K6/8/8/8/2k4r/8/8 w - - 0 1 id 'krK';",
            "8/1k6/8/8/8/2K4Q/8/8 w - - 0 1 id 'KQK';",
            "8/1k6/8/8/8/2K3BB/8/8 w - - 0 1 id 'KBBK';",
            "8/1K6/8/8/8/2k3bb/8/8 w - - 0 1 id 'kbbK';",
            "8/1k6/8/8/8/2K3BN/8/8 w - - 0 1 id 'KBNK';",
            "8/1K6/8/8/8/2k3bn/8/8 w - - 0 1 id 'kbnK';",
            "8/1k6/8/8/8/2K3NN/8/8 w - - 0 1 id 'KNNK';",
        ];
        Epd::parse_many_epd(strs).unwrap()
    }

    pub fn famous() -> Vec<Epd> {
        // http://wtharvey.com/m8n2.txt
        let strs = &[
            // https://www.stmintz.com/ccc/index.php?id=14581
            "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1
            bm Kb1;
            dm 32;
            acd 23;
            id 'Lasker-Reichhelm Position Fine#70';",
        ];
        Epd::parse_many_epd(strs).unwrap()
    }

    pub fn pins() -> Vec<Epd> {
        let str = r#"
k6b/8/n4N2/8/Rr1K1P1b/2R5/8/q7 w - - 0 1 id "PIN.01"; c0 "Pins"; Sq c3 f6;
k2r4/8/3B4/3B4/3KQr2/5n2/8/8 w - - 0 1 id "PIN.02"; c0 "Pins"; Sq e4;
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn discovered_check() -> Vec<Epd> {
        let str = r#"
        k2N1R1b/8/n4N2/8/Rr1K1P1b/2R5/5n2/q5b1 w - - 0 1 id "DC.01"; c0 "Discovered checks"; Sq f2;
        k2N1R1b/8/n4N2/8/Rr1K1P1b/2R5/5n2/q5b1 b - - 0 1 id "DC.01"; c0 "Discovered checks"; Sq d8;
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn recogs() -> Vec<Epd> {
        let str = r#"
8/NN6/8/8/8/2K2nk1/4P3/8 w - - 0 1; id 'RECOG.01'; am e2f3; am exf3; c0 'white shouldnt take knight as recapture of pawn makes it KNN v k';
k7/1p6/3N4/8/8/8/6NB/K7 w - - 5 1; id 'RECOG.02'; bm Nxb7; c0 'white should take pawn to leave winning KBN v k';
k7/8/K1p5/8/3N4/8/6N1/7B w - - 5 1; id 'RECOG.03'; am Nxc6; bm Kb6; c0 'white shouldnt take pawn with knight as it triggers stalemate';
k1K5/8/8/2p5/8/6B1/5B2/8 w - - 0 1;  id 'RECOG.04'; bm Bxc5; c0 'white should force stalemate by capturing pawn on c5 as black could win';
6k1/7n/8/8/8/8/1N6/B5KR w - - 26 1; id 'RECOG.05'; bm Rxh7; acd 7; c0 'winning position despite king recapture';
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    // given in UCI format so that we dont need a working "legal moves" to parse
    pub fn moves() -> Vec<Epd> {
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

        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn move_ordering() -> Vec<Epd> {
        let str = r#"
r3k2r/1P6/1N3P2/2Pp4/3QP2Q/5B2/8/R3K2R w KQkq d6 0 1 id "MO.01"; c0 "Promos";
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    // FIXME! QS.10 - recaptures
    pub fn quiesce() -> Vec<Epd> {
        let str = r#"
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
r2qkb1r/pp2nNpp/3p4/2pN2B1/2BnP3/3P4/PPP2PPP/R2bK2R b KQkq - 1 1 acd 0; pv Kxf7 Rxd1 Nxc2+ Ke2; id QS.90; c0 'from mate in 2 - mate found in qsearch somehow?';
2b1k3/3r4/3b4/3p4/8/8/3Q4/R3K3 w - - 0 1 acd 0; id QS.91; pv Qxd5; c0 'qsearch fail!'; c1 'chess.stackexchange.com/questions/29602';
r4rk1/pp2qppp/2nbbn2/3Np3/2B1P3/P4N1P/1P1B1PP1/R2QR1K1 b - - 6 16 acd 0; am Nxd5; bm Qd8; pv; ce 0; id 'QS.00'; c0 'pawn fork'; c1 'https://lichess.org/q5XvkApk/black#31';
"#;
        // k7/p7/1p6/P1B5/8/8/8/K7 w - - 0 1 acd 0; pv; id QS.11; c0 'capture then bad recapture due to draw'; c1 'game a draw if white exchanges';
        // rnb1k2r/pp3ppp/4p3/q2p4/1bpPnB2/2N1PN2/PPPQ1PPP/2KR1B1R w kq - 1 9 "QS.01"; c0 "xray"; c1 "https://lichess.org/PqvjbdtB#16";
        // rnb1k2r/pp3ppp/4p3/3pB3/2pPn3/2P1PN2/q1P1QPPP/2KR1B1R b kq - 1 11 "QS.02"; c0 "bug?";
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn see() -> Vec<Epd> {
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
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn draws() -> Vec<Epd> {
        let str = r#"
6k1/6p1/8/6KQ/1r6/q2b4/8/8 w - - 2 1 bm Qe8+; pv Qe8+ Kh7 Qg6+ Kh8 Qe8+ Kh7; id DR.01; c0 'https://en.wikipedia.org/wiki/Perpetual_check'; c1 'Reinfeld 1958';   
5rk1/1rP3pp/p4n2/3Pp3/1P2Pq2/2Q4P/P5P1/R3R1K1 b - - 2 1 id DR.02; bm Rxc7; ce 0; pv Rxc7 Qxc7 Ng4 hxg4 Qf2+ Kh2 Qh4+ Kg1 Qf2+; c0 'https://en.wikipedia.org/wiki/Perpetual_check'; c1 'Unzicker versus Averbakh';
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 id DR.03; sv 1. d4 Nf6 2. c4 g6 3. Nc3 d5 4. cxd5 Nxd5 5. e4 Nxc3 6. bxc3 Bg7 7. Nf3 O-O 8. Bc4 Nc6 9. O-O Bg4 10. Ba3 Rc8 11. Bc5 Na5 12. Bb5 a6 13. Be2 Bxf3 14. Bxf3 Qd7 15. Bg4 f5 16. exf5 gxf5 17. Bf3 Rfe8 18. Bxb7 Nxb7 19. Qb3+ e6 20. Qxb7 a5 21. Rfb1 a4 22. Rb4 Ra8 23. Qf3 Rab8 24. Rc1 Rxb4 25. cxb4 Qd5 26. Qh5 Rc8 27. a3 Qb3 28. Qe2 Re8 29. Qb5 Rd8 30. Be7 Bh6 31. Re1 Qd5 32. Qe2 Re8 33. Bf6 Qd7 34. Be5 Bf8 35. Rc1 Bd6 36. Bxd6 cxd6 37. Qa6 f4 38. b5 f3 39. g3 Rb8 40. Rb1 e5 41. Qxa4 d5 42. Qb3 e4 43. b6 Kh8 44. Kh1 Qh3 45. Rg1 Qe6 46. Rb1 Qh3 47. Rg1 Qe6 48. Rb1 Qh3; 
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 id DR.04; sv 1. d4 Nf6 2. c4 g6 3. Nc3 d5 4. cxd5 Nxd5 5. e4 Nxc3 6. bxc3 Bg7 7. Nf3 O-O 8. Bc4 Nc6 9. O-O Bg4 10. Ba3 Rc8 11. Bc5 Na5 12. Bb5 a6 13. Be2 Bxf3 14. Bxf3 Qd7 15. Bg4 f5 16. exf5 gxf5 17. Bf3 Rfe8 18. Bxb7 Nxb7 19. Qb3+ e6 20. Qxb7 a5 21. Rfb1 a4 22. Rb4 Ra8 23. Qf3 Rab8 24. Rc1 Rxb4 25. cxb4 Qd5 26. Qh5 Rc8 27. a3 Qb3 28. Qe2 Re8 29. Qb5 Rd8 30. Be7 Bh6 31. Re1 Qd5 32. Qe2 Re8 33. Bf6 Qd7 34. Be5 Bf8 35. Rc1 Bd6 36. Bxd6 cxd6 37. Qa6 f4 38. b5 f3 39. g3 Rb8 40. Rb1 e5 41. Qxa4 d5 42. Qb3 e4 43. b6 Kh8 44. Kh1 Qh3 45. Rg1 Qe6 46. Rb1 Qh3 47. Rg1 Qe6; 
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn bratko_kopec() -> Vec<Epd> {
        let contents = read_resource_file("bk.epd");
        let positions = Epd::parse_many_epd(contents.lines()).unwrap();
        positions
    }

    pub fn iq() -> Vec<Epd> {
        let contents = read_resource_file("iq.epd");
        let positions = Epd::parse_many_epd(contents.lines()).unwrap();
        positions
    }

    pub fn win_at_chess() -> Vec<Epd> {
        let contents = read_resource_file("wac.epd");
        let positions = Epd::parse_many_epd(contents.lines()).unwrap();
        positions
    }

    pub fn mate_in_2() -> Vec<Epd> {
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
        Epd::parse_many_epd(strs).unwrap()
    }

    pub fn mate_in_3() -> Vec<Epd> {
        // http://wtharvey.com/m8n3.txt
        let str = r#"
r3k2r/ppp2Npp/1b5n/4p2b/2B1P2q/BQP2P2/P5PP/RN5K w kq - 1 1 dm 3; pv 1. Bb5+ c6 2. Qe6+ Qe7 3. Qxe7#; c0 "William Evans vs Alexander MacDonnell, London, 1826"
"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn mate_in_4() -> Vec<Epd> {
        let str = r#"2k5/7Q/8/8/8/3K4/8/8 w - - 3 1 id 'KQ vs k'; dm 4;"#;
        let positions = Epd::parse_many_epd(str.lines()).unwrap();
        positions
    }

    pub fn perft_kiwipete() -> (Board, Vec<u64>) {
        // https://www.chessprogramming.org/Perft_Results
        (
            Board::parse_fen(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            )
            .unwrap(),
            vec![1, 48, 2039, 97_862, 4_085_603, 193_690_690, 8_031_647_685],
        )
    }

    pub fn perft_cpw_number3() -> (Board, Vec<u64>) {
        // https://www.chessprogramming.org/Perft_Results
        (
            Board::parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 id 'CPW PERFT#3'").unwrap(),
            vec![
                1,
                14,
                191,
                2812,
                43_238,
                674_624,
                11_030_083,
                178_633_661,
                3_009_794_393,
            ],
        )
    }

    // FIXME! awful structure
    #[allow(clippy::vec_init_then_push)]
    pub fn perfts() -> Vec<(Board, Vec<u64>)> {
        let mut vec = Vec::new();
        // https://www.chessprogramming.org/Perft_Results
        vec.push((
            Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap(),
            vec![1, 20, 400, 8902, 197_281, 4_865_609],
        ));
        vec.push(Self::perft_kiwipete());
        // http://www.rocechess.ch/perft.html
        vec.push((
            Board::parse_fen("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1").unwrap(),
            vec![1, 24, 496, 9_483, 182_838, 3_605_103, 71_179_139],
        ));
        vec.push((
            Board::parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 id 'CPW PERFT#3'").unwrap(),
            vec![
                1,
                14,
                191,
                2812,
                43_238,
                674_624,
                11_030_083,
                178_633_661,
                3_009_794_393,
            ],
        ));
        vec.push(Self::perft_cpw_number3());
        vec.push((
            Board::parse_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1")
                .unwrap(),
            vec![1, 6, 264, 9467, 422_333, 15_833_292, 706_045_033],
        ));
        vec.push((
            Board::parse_fen("r2q1rk1/pP1p2pp/Q4n2/bbp1p3/Np6/1B3NBn/pPPP1PPP/R3K2R b KQ - 0 1")
                .unwrap(),
            vec![1, 6, 264, 9467, 422_333, 15_833_292, 706_045_033],
        ));
        vec.push((
            Board::parse_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap(),
            vec![1, 44, 1486, 62379, 2_103_487, 89_941_194],
        ));
        vec.push((
            Board::parse_fen(
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            )
            .unwrap(),
            vec![1, 46, 2079, 89_890, 3_894_594, 164_075_551, 6_923_051_137],
        ));
        vec.push((
            Board::parse_fen("r1b2rk1/2p2ppp/p7/1p6/3P3q/1BP3bP/PP3QP1/RNB1R1K1 w - - 1 0")
                .unwrap(),
            vec![1, 40, 1334, 50_182, 1_807_137],
        ));
        vec
    }

    pub fn bench() -> Vec<Epd> {
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
    pub fn chess960(id: u32) -> Epd {
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
        let mut chars = ['.'; 8];
        let bw = bw * 2 + 1;
        let bb = bb * 2;
        chars[bw] = 'B';
        chars[bb] = 'B';
        let q = (0..8).filter(|&x| chars[x] == '.').nth(q).unwrap();
        chars[q] = 'Q';
        let n1 = (0..8).filter(|&x| chars[x] == '.').nth(n1).unwrap();
        let n2 = (0..8).filter(|&x| chars[x] == '.').nth(n2).unwrap();
        chars[n1] = 'N';
        chars[n2] = 'N';
        let r1 = (0..8).position(|x| chars[x] == '.').unwrap();
        chars[r1] = 'R';
        let r2 = (0..8).rposition(|x| chars[x] == '.').unwrap();
        chars[r2] = 'R';
        let k = (0..8).position(|x| chars[x] == '.').unwrap();
        chars[k] = 'K';
        let mut b = Board::new_empty();
        b.set(RANK_1, &chars.iter().collect::<String>()).unwrap();
        b.set(RANK_2, "PPPPPPPP").unwrap();
        b.set(RANK_7, "pppppppp").unwrap();
        b.set(RANK_8, &chars.iter().collect::<String>().to_lowercase())
            .unwrap();
        b.set_castling(CastlingRights::ALL);
        let mut pos = Epd::from_board(b);
        pos.set_tag("id", &format!("Chess960(SP{id})"));
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_serde() {
        assert_eq!(
            serde_json::to_string(&CatalogSuite::WinAtChess).unwrap(),
            r#""WinAtChess""#
        );
    }

    #[test]
    fn test_catalog_wac() {
        let epds = Catalog::win_at_chess();
        assert_eq!(epds.len(), 300);
        assert_eq!(epds[0].tag("id").unwrap(), "WAC.001");
    }

    #[test]
    fn test_catalog_bratko_kopec() {
        let epds = Catalog::bratko_kopec();
        assert_eq!(epds.len(), 24);
        assert_eq!(epds[0].tag("id").unwrap(), "BK.01");
    }

    #[test]
    fn test_catalog_chess960() {
        //  testing data https://www.mark-weeks.com/cfaa/chess960/c960strt.htm
        let p = Catalog::chess960(518);
        assert_eq!(p.board(), Catalog::starting_board());
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
