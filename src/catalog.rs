use crate::bitboard::Bitboard;
use crate::board::boardbuf::BoardBuf;
use crate::board::Board;
use crate::epd::Epd;

pub struct Catalog;

impl Catalog {
    pub const STARTING_POSITION_FEN: &'static str =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    pub fn starting_position() -> Board {
        Board::parse_fen(Self::STARTING_POSITION_FEN).unwrap().as_board()
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

    pub fn checkmates() -> [Board; 2] {
        [
            Board::parse_fen("k7/Q7/K7/8/8/8/8/8 b - - 0 1").unwrap().as_board(),
            Board::parse_fen("K7/7r/8/8/8/8/8/rr5k w - - 0 1").unwrap().as_board(),
        ]
    }
    pub fn stalemates() -> [Board; 2] {
        [
            Board::parse_fen("k7/8/1Q6/8/8/8/8/7K b - - 0 1").unwrap().as_board(),
            Board::parse_fen("K7/7r/8/8/8/8/8/1r5k w - - 0 1").unwrap().as_board(),
        ]
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

            "1r1kr3/Nbppn1pp/1b6/8/6Q1/3B1P2/Pq3P1P/3RR1K1 w - - 1 0
            id 'Joseph Blackburne vs Martin, England, 1876';
            pv 1. Qxd7+ Kxd7 2. Bb5#;",

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
        Epd::parse_many(strs).unwrap()
    }
                    // [
        //     // http://wtharvey.com/m8n2.txt
        //     // Henry Buckle vs NN, London, 1840
        //     // 1. Nf6+ gxf6 2. Bxf7#
        //     Board::parse_fen("r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1")
        //         .unwrap()
        //         .as_board(),
        //     // Alexander Areshchenko vs Sergei Zhigalko, Kiev, 6/6/2013
        //     // 1... Nb3+ 2. axb3 Qa6#
        //     Board::parse_fen("r3k2r/p3bpp1/2q1p1b1/1ppPP1B1/3n3P/5NR1/PP2NP2/K1QR4 b kq - 0 1")
        //         .unwrap()
        //         .as_board(),

    pub fn mate_in_3() -> [Epd; 1] {
        [
            // http://wtharvey.com/m8n3.txt
            // William Evans vs Alexander MacDonnell, London, 1826
            //1. Bb5+ c6 2. Qe6+ Qe7 3. Qxe7#
            Epd::parse(
                "r3k2r/ppp2Npp/1b5n/4p2b/2B1P2q/BQP2P2/P5PP/RN5K w kq - 1 1 
                dm 3; 
                pv 1. Bb5+ c6 2. Qe6+ Qe7 3. Qxe7#; 
                c0 \"William Evans vs Alexander MacDonnell, London, 1826\"",
            )
            .unwrap(),
        ]
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
    pub fn perfts() -> Vec<(Board, Vec<u64>)> {
        let mut vec = Vec::new();
        vec.push(Self::perft_kiwipete());
        // http://www.rocechess.ch/perft.html
        vec.push((
            Board::parse_fen("n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1").unwrap().as_board(),
            vec![1, 24, 496, 9_483, 182_838, 3_605_103, 71_179_139],
        ));
        // https://www.chessprogramming.org/Perft_Results
        vec.push((
            Board::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap().as_board(),
            vec![1, 20, 400, 8902, 197_281, 4_865_609],
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
            Board::parse_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap().as_board(),
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
