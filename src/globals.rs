#[allow(non_upper_case_globals)]
#[allow(dead_code)]
pub mod constants {
    use crate::bits::bitboard::Bitboard;
    pub const a1: Bitboard = Bitboard::A1;
    pub const b1: Bitboard = Bitboard::B1;
    pub const c1: Bitboard = Bitboard::C1;
    pub const d1: Bitboard = Bitboard::D1;
    pub const e1: Bitboard = Bitboard::E1;
    pub const f1: Bitboard = Bitboard::F1;
    pub const g1: Bitboard = Bitboard::G1;
    pub const h1: Bitboard = Bitboard::H1;
    pub const a2: Bitboard = Bitboard::A2;
    pub const b2: Bitboard = Bitboard::B2;
    pub const c2: Bitboard = Bitboard::C2;
    pub const d2: Bitboard = Bitboard::D2;
    pub const e2: Bitboard = Bitboard::E2;
    pub const f2: Bitboard = Bitboard::F2;
    pub const g2: Bitboard = Bitboard::G2;
    pub const h2: Bitboard = Bitboard::H2;
    pub const a3: Bitboard = Bitboard::A3;
    pub const b3: Bitboard = Bitboard::B3;
    pub const c3: Bitboard = Bitboard::C3;
    pub const d3: Bitboard = Bitboard::D3;
    pub const e3: Bitboard = Bitboard::E3;
    pub const f3: Bitboard = Bitboard::F3;
    pub const g3: Bitboard = Bitboard::G3;
    pub const h3: Bitboard = Bitboard::H3;
    pub const a4: Bitboard = Bitboard::A4;
    pub const b4: Bitboard = Bitboard::B4;
    pub const c4: Bitboard = Bitboard::C4;
    pub const d4: Bitboard = Bitboard::D4;
    pub const e4: Bitboard = Bitboard::E4;
    pub const f4: Bitboard = Bitboard::F4;
    pub const g4: Bitboard = Bitboard::G4;
    pub const h4: Bitboard = Bitboard::H4;
    pub const a5: Bitboard = Bitboard::A5;
    pub const b5: Bitboard = Bitboard::B5;
    pub const c5: Bitboard = Bitboard::C5;
    pub const d5: Bitboard = Bitboard::D5;
    pub const e5: Bitboard = Bitboard::E5;
    pub const f5: Bitboard = Bitboard::F5;
    pub const g5: Bitboard = Bitboard::G5;
    pub const h5: Bitboard = Bitboard::H5;
    pub const a6: Bitboard = Bitboard::A6;
    pub const b6: Bitboard = Bitboard::B6;
    pub const c6: Bitboard = Bitboard::C6;
    pub const d6: Bitboard = Bitboard::D6;
    pub const e6: Bitboard = Bitboard::E6;
    pub const f6: Bitboard = Bitboard::F6;
    pub const g6: Bitboard = Bitboard::G6;
    pub const h6: Bitboard = Bitboard::H6;
    pub const a7: Bitboard = Bitboard::A7;
    pub const b7: Bitboard = Bitboard::B7;
    pub const c7: Bitboard = Bitboard::C7;
    pub const d7: Bitboard = Bitboard::D7;
    pub const e7: Bitboard = Bitboard::E7;
    pub const f7: Bitboard = Bitboard::F7;
    pub const g7: Bitboard = Bitboard::G7;
    pub const h7: Bitboard = Bitboard::H7;
    pub const a8: Bitboard = Bitboard::A8;
    pub const b8: Bitboard = Bitboard::B8;
    pub const c8: Bitboard = Bitboard::C8;
    pub const d8: Bitboard = Bitboard::D8;
    pub const e8: Bitboard = Bitboard::E8;
    pub const f8: Bitboard = Bitboard::F8;
    pub const g8: Bitboard = Bitboard::G8;
    pub const h8: Bitboard = Bitboard::H8;

    pub const FILE_A: Bitboard = Bitboard::FILE_A;
    pub const FILE_B: Bitboard = Bitboard::FILE_B;
    pub const FILE_C: Bitboard = Bitboard::FILE_C;
    pub const FILE_D: Bitboard = Bitboard::FILE_D;
    pub const FILE_E: Bitboard = Bitboard::FILE_E;
    pub const FILE_F: Bitboard = Bitboard::FILE_F;
    pub const FILE_G: Bitboard = Bitboard::FILE_G;
    pub const FILE_H: Bitboard = Bitboard::FILE_H;
    pub const RANK_1: Bitboard = Bitboard::RANK_1;
    pub const RANK_2: Bitboard = Bitboard::RANK_2;
    pub const RANK_3: Bitboard = Bitboard::RANK_3;
    pub const RANK_4: Bitboard = Bitboard::RANK_4;
    pub const RANK_5: Bitboard = Bitboard::RANK_5;
    pub const RANK_6: Bitboard = Bitboard::RANK_6;
    pub const RANK_7: Bitboard = Bitboard::RANK_7;
    pub const RANK_8: Bitboard = Bitboard::RANK_8;
}

// pub mod counts {
//     use crate::trace::stat::{ArrayStat, Stat};

//     pub static EVAL_CACHE_COUNT: Stat = Stat::new("EVAL CACHE COUNT");
//     pub static EVAL_COUNT: Stat = Stat::new("EVAL COUNT");
//     pub static QEVAL_CACHE_COUNT: Stat = Stat::new("QEVAL CACHE COUNT");
//     pub static QEVAL_COUNT: Stat = Stat::new("QEVAL COUNT");
//     pub static LEGAL_MOVE_COUNT: Stat = Stat::new("LEGAL_MOVE COUNT");
//     pub static MOVE_HASH_COUNT: Stat = Stat::new("MOVE HASH COUNT");
//     pub static BOARD_HASH_COUNT: Stat = Stat::new("BOARD HASH COUNT");
//     pub static SEARCH_IDS_COMPLETES: Stat = Stat::new("IDS COMPLETES");
//     pub static SEARCH_IDS_TIMEOUTS: Stat = Stat::new("IDS TIMEOUTS");

//     pub static GLOBAL_COUNTS: ArrayStat = ArrayStat(&[
//         &EVAL_CACHE_COUNT,
//         &QEVAL_CACHE_COUNT,
//         &EVAL_COUNT,
//         &QEVAL_COUNT,
//         &LEGAL_MOVE_COUNT,
//         &MOVE_HASH_COUNT,
//         &BOARD_HASH_COUNT,
//         &SEARCH_IDS_COMPLETES,
//         &SEARCH_IDS_TIMEOUTS,
//     ]);
// }

#[cfg(test)]
mod tests {
    use test_log::test;
    use crate::bits::bitboard::*;
    use crate::bits::castling::*;
    use crate::bits::precalc::*;
    use crate::bits::square::*;
    use crate::boards::Board;
    use crate::domain::NodeType;
    use crate::cache::hasher::*;
    use crate::cache::tt2::*;
    use crate::clock::Clock;
    use crate::eval::eval::Eval;
    use crate::eval::recognizer::Recognizer;
    use crate::eval::score::*;
    use crate::infra::metric::Metrics;
    use crate::movelist::*;
    use crate::mv::*;
    use crate::piece::*;
    use crate::position::*;
    use crate::boards::Repetition;
    use crate::search::aspiration::Aspiration;
    use crate::search::extensions::Extensions;
    use crate::search::futility::Futility;
    use crate::search::history_heuristic::HistoryHeuristic;
    use crate::search::iterative_deepening::IterativeDeepening;
    use crate::search::killers::Killers;
    use crate::search::lmr::Lmr;
    use crate::search::move_orderer::MoveOrderer;
    use crate::search::move_time_estimator::MoveTimeEstimator;
    use crate::search::nmp::NullMovePruning;
    use crate::search::node::*;
    use crate::search::pvs::Pvs;
    use crate::search::razor::Razor;
    use crate::search::restrictions::Restrictions;
    use crate::search::search_explainer::Explainer;
    use crate::other::Tag;
    use crate::variation::*;
    use crate::Algo;
    use std::mem::size_of;
    use std::time::Duration;


    fn compare(s1: usize, s2: usize, s: &str) {
        if s1 != s2 {
            warn!("{s} actual {s1} != expected {s2}");
        }
    }

    // RUST_LOG=warn cargo t test_sizes --nocapture
    #[test]
    fn test_sizes() {
        warn!("Testing sizes...");
        compare(size_of::<Piece>(), 1, "Piece");
        compare(size_of::<Option<Piece>>(), 1, "Piece");
        compare(size_of::<Color>(), 1, "Color");
        compare(size_of::<Ply>(), 4, "Ply");
        compare(size_of::<CastlingRights>(), 1, "CastlingRights");
        compare(size_of::<Square>(), 2, "Square");
        compare(size_of::<Score>(), 2, "Score");
        compare(size_of::<Move>(), 2, "Move");
        compare(size_of::<Bitboard>(), 8, "Bitboard");
        compare(size_of::<LazyBitboard<1u64>>(), 8, "LazyBitboard");
        compare(size_of::<Board>(), 160, "Board");
        compare(size_of::<Option<Bitboard>>(), 16, "Bitboard");
        compare(size_of::<String>(), 24, "String");
        compare(size_of::<Duration>(), 16, "Duration");
        compare(size_of::<Node>(), 12, "Node");
        compare(size_of::<NodeType>(), 1, "NodeType");
        compare(size_of::<TtNode>(), 16, "TtNode");
        compare(size_of::<Tag>(), 448, "Tag");
        compare(size_of::<Variation>(), 24, "Variation");
        compare(size_of::<MoveList>(), 440, "MoveList");
        compare(size_of::<Position>(), 208, "Position");
        compare(size_of::<PreCalc>(), 77832, "PreCalc");
        compare(size_of::<Hasher>(), 6400, "Hasher");
        compare(size_of::<Algo>(), 7432, "Algo");
        compare(size_of::<Clock>(), 112, "Clock");
        compare(size_of::<NullMovePruning>(), 32, "NullMovePruning");
        compare(size_of::<Futility>(), 56, "Futility");
        compare(size_of::<IterativeDeepening>(), 16, "IterativeDeepening");
        compare(size_of::<Eval>(), 3400, "SimpleScorer");
        compare(size_of::<Pvs>(), 12, "Pvs");
        compare(size_of::<Extensions>(), 40, "Extensions");
        compare(size_of::<Lmr>(), 64, "Lmr");
        compare(size_of::<MoveTimeEstimator>(), 376, "MoveTimeEstimator");
        compare(size_of::<MoveOrderer>(), 296, "MoveOrderer");
        compare(size_of::<Repetition>(), 40, "Repetition");
        compare(size_of::<TranspositionTable2>(), 64, "TranspositionTable2");
        compare(size_of::<Killers>(), 32, "Killers");
        compare(size_of::<HistoryHeuristic>(), 56, "HistoryHeuristic");
        compare(size_of::<Explainer>(), 504, "SearchExplainer");
        compare(size_of::<Restrictions>(), 896, "Restrictions");
        compare(size_of::<Razor>(), 36, "Razor");
        compare(size_of::<Recognizer>(), 12, "Recognizer");
        compare(size_of::<Aspiration>(), 24, "Aspiration");
        compare(size_of::<Metrics>(), 144, "Metrics");
    }
}
