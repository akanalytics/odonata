use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::movelist::MoveList;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Perft {
    pub captures: u64,
    pub en_passant: u64,
    pub castles: u64,
    pub promos: u64,
    pub checks: u64,
    pub discovery_checks: u64,
    pub double_checks: u64,
    pub checkmates: u64,
}



impl Perft {
    pub fn perft(board: &mut Board, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut moves = MoveList::new();
        board.legal_moves_into(&mut moves);
        if depth == 1 {
            return moves.len() as u64;
        } else {
            let mut count = 0u64;
            for m in moves.iter() {
                count += Self::perft(&mut board.make_move(m), depth - 1);
            }
            count
        }
    }

    pub fn perft_cat(&mut self, board: &mut Board, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut moves = MoveList::new();
        board.legal_moves_into(&mut moves);
        if depth == 1 {
            self.count_types(&moves);
            return moves.len() as u64;
        } else {
            let mut count = 0u64;
            for m in moves.iter() {
                count += self.perft_cat(&mut board.make_move(m), depth - 1);
            }
            count
        }
    }

    #[inline]
    fn count_types(&mut self, moves: &MoveList) {
        for mv in moves.iter() {
            if mv.is_capture() {
                self.captures +=1
            }
            if mv.is_ep_capture() {
                self.en_passant +=1
            }
            if mv.is_castle() {
                self.castles += 1;
            }
            if mv.is_promo() {
                self.promos += 1;
            }
            // to do 
            // checks
            // discovery_checks
            // double_checks
            // checkmates
        }
    }

}

    // pub fn perft_ext(board: &mut Board, depth: u32) -> u64 {
    //     if depth == 0 {
    //         1
    //     } else {
    //         let moves = board.legal_moves_ext();
    //         let mut count = 0u64;
    //         for m in moves.iter() {
    //             let res = Self::perft_ext(&mut board.make_move(m), depth - 1);
    //             count += res;
    //         }
    //         count
    //     }
    // }

//     pub fn perft_compare(board: &mut Board, depth: u32) -> u64 {
//         if depth == 0 {
//             1
//         } else {
//             let mut moves = board.legal_moves();
//             let mut moves_ext = board.legal_moves_ext();
//             moves.sort();
//             moves_ext.sort();
//             assert_eq!(moves.to_string(), moves_ext.to_string());
//             if moves.len() == moves_ext.len() {
//                 print!("+");
//             } else {
//                 print!("-");
//             }

//             let mut count = 0u64;
//             for (m1, m2) in moves.iter().zip(moves_ext.iter()) {
//                 let b2 = board.make_move(m2);
//                 assert_eq!(board.make_move(m1).to_fen(), b2.to_fen(), "for m1 {} m2 (ext) {} from {}", m1, m2, board.to_fen());
//                 let res = Self::perft_compare(&mut board.make_move(m1), depth - 1);
//                 count += res;
//             }
//             count
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use std::time::Instant;
    use test_env_log::test;

    #[test]
    fn test_perft_1() {
        let (board, perfts) = &Catalog::perfts()[1];
        assert_eq!(
            board.legal_moves().len() as u64,
            perfts[1],
            "{}",
            board.legal_moves()
        );
    }

    #[test]
    fn test_perft() {
        for (mut board, perfts) in Catalog::perfts() {
            for (depth, &expected) in perfts.iter().enumerate() {
                if depth <= 2 {
                    let now = Instant::now();
                    let count = Perft::perft(&mut board, depth as u32);
                    assert_eq!(count, expected, "fen {} perft({})", board.to_fen(), depth);
                    println!(
                        "perft({depth})={count} in {time} millis",
                        depth = depth,
                        count = count,
                        time = now.elapsed().as_millis()
                    );
                }
                // assert_eq!(&count, expected, "fen: {}", board.to_fen());
            }
        }
    }

    // #[test]
    // #[ignore]
    // fn test_compare_perft() {
    //     for (mut board, perfts) in Catalog::perfts() {
    //         for (depth, &_expected) in perfts.iter().enumerate() {
    //             if depth <= 3 {
    //                 let _now = Instant::now();
    //                 let _count = Perft::perft_compare(&mut board, depth as u32);
    //             }
    //         }
    //     }
    // }

    #[test]
    #[ignore]
    fn test_slow_perft() {
        for (mut board, perfts) in Catalog::perfts() {
            println!("\n{}", board.to_fen());
            for (depth, &expected) in perfts.iter().enumerate() {
                if expected < 1_000_000_000_u64 {
                    let now = Instant::now();
                    let count = Perft::perft(&mut board, depth as u32);
                    assert_eq!(count, expected, "fen {} perft({})", board.to_fen(), depth);
                    println!(
                        "perft({depth})={count} in {time} millis",
                        depth = depth,
                        count = count,
                        time = now.elapsed().as_millis()
                    );
                }
                // assert_eq!(&count, expected, "fen: {}", board.to_fen());
            }
        }
    }
}
