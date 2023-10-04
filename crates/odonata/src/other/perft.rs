use crate::{
    boards::{board::Var, Board},
    mv::Move,
};

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Perft {
    pub captures:         u64,
    pub en_passant:       u64,
    pub castles:          u64,
    pub promos:           u64,
    pub checks:           u64,
    pub discovery_checks: u64,
    pub double_checks:    u64,
    pub checkmates:       u64,
}

impl Perft {
    // callback on board + legal mv at the leaf level. board.make_mv(mv) -> each leaf node
    pub fn perft_with(board: &mut Board, depth: u32, f: &mut impl FnMut(&Board, Move)) {
        if depth == 0 {
            return;
        }
        if depth == 1 {
            board.legal_moves_with(|mv| f(board, mv))
        } else {
            let moves = board.legal_moves();
            for &m in moves.iter() {
                Self::perft_with(&mut board.make_move(m), depth - 1, f);
            }
        }
    }

    pub fn perft_with2(var: &mut Var, depth: u32, f: &mut impl FnMut(&Board, Move)) {
        if depth == 0 {
            return;
        }
        let board = var.board();
        if depth == 1 {
            board.legal_moves_with(|mv| f(board, mv))
        } else {
            let moves = board.legal_moves();
            for &m in moves.iter() {
                var.push_move(m);
                Self::perft_with2(var, depth - 1, f);
                var.pop_move();
            }
        }
    }

    pub fn perft(board: &mut Board, depth: u32) -> u64 {
        let mut count = 0;
        Self::perft_with2(&mut Var::new(board.clone()), depth, &mut |_b, _mv| {
            count += 1
        });
        count.max(1) // we count perft(0) as 1
    }

    pub fn perft_v2(board: &mut Board, depth: u32) -> u64 {
        let mut count = 0;
        Self::perft_with(board, depth, &mut |_b, _mv| count += 1);
        count.max(1) // we count perft(0) as 1
    }

    pub fn perft_cat(&mut self, board: &mut Board, depth: u32) -> u64 {
        let mut count = 0;

        Self::perft_with(board, depth, &mut |bd, mv| {
            if mv.is_capture() {
                self.captures += 1
            }
            if mv.is_ep_capture(bd) {
                self.en_passant += 1
            }
            if mv.is_castle(bd) {
                self.castles += 1;
            }
            if mv.is_promo() {
                self.promos += 1;
            }
            count += 1;
        });
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use std::time::Instant;
    use test_log::test;

    #[test]
    fn test_perft_fn() {
        for (mut board, perfts) in Catalog::perfts() {
            for (depth, &expected) in perfts.iter().enumerate().skip(1) {
                if depth <= 4 {
                    let now = Instant::now();
                    let mut count = 0;
                    let mut func = |_: &Board, _: Move| *&mut count += 1;
                    Perft::perft_with(&mut board, depth as u32, &mut func);
                    println!(
                        "perft({depth})={count} in {time} millis (expected {expected})",
                        depth = depth,
                        count = count,
                        time = now.elapsed().as_millis(),
                    );
                    assert_eq!(count, expected, "fen {} perft({})", board.to_fen(), depth);
                }
                // assert_eq!(&count, expected, "fen: {}", board.to_fen());
            }
        }
    }

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
                if depth <= 4 {
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
