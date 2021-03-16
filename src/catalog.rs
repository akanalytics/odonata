use crate::board::boardbuf::BoardBuf;
use crate::board::Board;
use crate::bitboard::Bitboard;

pub struct Catalog;

impl Catalog {
    pub const STARTING_POSITION_FEN: &'static str =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    pub fn starting_position() -> Board {
        Board::parse_fen(Self::STARTING_POSITION_FEN).unwrap().as_board()
    }

    pub fn white_starting_position() -> Board {
        // FIXME: set calls
        Board::parse_fen(Self::STARTING_POSITION_FEN).unwrap().set(Bitboard::RANK_7 | Bitboard::RANK_8, "................").unwrap().as_board()
    }

    pub fn black_starting_position() -> Board {
        // FIXME: set calls
        Board::parse_fen(Self::STARTING_POSITION_FEN).unwrap().set(Bitboard::RANK_1 | Bitboard::RANK_2, "................").unwrap().as_board()
    }


    pub fn checkmates() -> [Board;2] {
        [
            Board::parse_fen("k7/Q7/K7/8/8/8/8/8 b - - 0 1").unwrap().as_board(),
            Board::parse_fen("K7/7r/8/8/8/8/8/rr5k w - - 0 1").unwrap().as_board(),
        ]
    }
    pub fn stalemates() -> [Board;2] {
        [
            Board::parse_fen("k7/8/1Q6/8/8/8/8/7K b - - 0 1").unwrap().as_board(),
            Board::parse_fen("K7/7r/8/8/8/8/8/1r5k w - - 0 1").unwrap().as_board(),
        ]
    }
    pub fn mate_in_2() -> [Board;2] {
        [
            // http://wtharvey.com/m8n2.txt
            // Henry Buckle vs NN, London, 1840
            // 1. Nf6+ gxf6 2. Bxf7# 
            Board::parse_fen("r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 0").unwrap().as_board(),
            
            
            // Alexander Areshchenko vs Sergei Zhigalko, Kiev, 6/6/2013
            // 1... Nb3+ 2. axb3 Qa6#
            Board::parse_fen("r3k2r/p3bpp1/2q1p1b1/1ppPP1B1/3n3P/5NR1/PP2NP2/K1QR4 b kq - 0 1").unwrap().as_board(),
        ]
    }









    // FIXME! awful structure
    pub fn perfts() -> Vec<(Board, Vec<u64>)> {
        let mut vec = Vec::new();
        vec.push((
            Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap()
                .as_board(),
            vec![1, 20, 400, 8902, 197_281, 4_865_609],
        ));
        // https://www.chessprogramming.org/Perft_Results
        vec.push((
            Board::parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap()
                .as_board(),
            vec![1, 48, 2039, 97_862, 4_085_603, 193_690_690, 8_031_647_685],
        ));
        vec.push((
            Board::parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 id 'CPW PERFT#3'")
                .unwrap()
                .as_board(),
            vec![1, 14, 191, 2812, 43_238, 674_624, 11_030_083, 178_633_661, 3_009_794_393],
        ));
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
}
