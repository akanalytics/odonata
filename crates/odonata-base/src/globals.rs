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
