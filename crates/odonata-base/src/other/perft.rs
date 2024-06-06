use crate::boards::board::Var;
use crate::boards::{Board, Position};
use crate::mv::Move;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Perft<const BULK_COUNT: bool = true> {
    pub captures:         u64,
    pub en_passant:       u64,
    pub castles:          u64,
    pub promos:           u64,
    pub checks:           u64,
    pub discovery_checks: u64,
    pub double_checks:    u64,
    pub checkmates:       u64,
}

impl<const BULK_COUNT: bool> Perft<BULK_COUNT> {
    // callback on board + legal mv at the leaf level. board.make_mv(mv) -> each leaf node
    pub fn perft_with(board: &mut Board, depth: u32, f: &mut impl FnMut(&Board, Move)) {
        if depth == 0 {
            f(board, Move::new_null());
            return;
        }
        if BULK_COUNT && depth == 1 {
            board.legal_moves_with(|mv| f(board, mv))
        } else {
            board.legal_moves_with(|mv| Self::perft_with(&mut board.make_move(mv), depth - 1, f));
        }
    }

    fn perft_with_var(var: &mut Var, depth: u32, f: &mut impl FnMut(&Board, Move)) {
        let board = var.board();
        if depth == 0 {
            f(board, Move::new_null());
            return;
        }
        if BULK_COUNT && depth == 1 {
            board.legal_moves_with(|mv| f(board, mv))
        } else {
            let moves = board.legal_moves();
            for &m in moves.iter() {
                var.push_move(m);
                Self::perft_with_var(var, depth - 1, f);
                var.pop_move();
            }
        }
    }

    pub fn perft_with_position(pos: &mut Position, depth: u32, f: &mut impl FnMut(&Position, Move)) {
        if depth == 0 {
            f(pos, Move::new_null());
            return;
        }
        if BULK_COUNT && depth == 1 {
            pos.board().legal_moves_with(|mv| f(pos, mv))
        } else {
            let moves = pos.board().legal_moves();
            for &m in moves.iter() {
                pos.push_move(m);
                Self::perft_with_position(pos, depth - 1, f);
                pos.pop_move();
            }
        }
    }

    pub fn count(board: &Board, depth: u32) -> u64 {
        Self::count_using_position(board, depth)
    }

    fn count_using_var(board: &Board, depth: u32) -> u64 {
        let mut count = 0;
        let mut var = Var::new(board.clone());
        Self::perft_with_var(&mut var, depth, &mut |_b, _mv| count += 1);
        count.max(1) // we count perft(0) as 1
    }

    pub fn count_using_board(board: &Board, depth: u32) -> u64 {
        let mut count = 0;
        Self::perft_with(&mut board.clone(), depth, &mut |_b, _mv| count += 1);
        count.max(1) // we count perft(0) as 1
    }

    fn count_using_position(board: &Board, depth: u32) -> u64 {
        let mut count = 0;
        Self::perft_with_position(&mut Position::from_board(board.clone()), depth, &mut |_b, _mv| {
            count += 1
        });
        count.max(1) // we count perft(0) as 1
    }

    pub fn perft_cat(&mut self, board: &mut Board, depth: u32) -> u64 {
        let mut count = 0;
        const BULK_COUNT: bool = true;
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
    use std::hint::black_box;
    use std::time::Instant;

    use test_log::test;

    use super::*;
    use crate::catalog::Catalog;
    use crate::infra::profiler::PerfProfiler;

    #[test]
    fn test_perft_fn() {
        for (mut board, perfts) in Catalog::perfts() {
            for (depth, &expected) in perfts.iter().enumerate().skip(1) {
                if depth <= 4 {
                    let now = Instant::now();
                    let mut count = 0;
                    let mut func = |_: &Board, _: Move| count += 1;
                    Perft::<true>::perft_with(&mut board, depth as u32, &mut func);
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
        assert_eq!(board.legal_moves().len() as u64, perfts[1], "{}", board.legal_moves());
    }

    #[test]
    fn test_perft() {
        for (board, perfts) in Catalog::perfts() {
            for (depth, &expected) in perfts.iter().enumerate() {
                if depth <= 4 {
                    let now = Instant::now();
                    let count = Perft::<true>::count(&board, depth as u32);
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

    #[test]
    fn bench_perft() {
        let mut p1 = PerfProfiler::new("perft var");
        let mut p2 = PerfProfiler::new("perft board.make_move");
        let mut p3 = PerfProfiler::new("perft position");
        let mut p4 = PerfProfiler::new("perft without bulk count (raw b)");
        let mut p5 = PerfProfiler::new("perft without bulk count (pos)");
        let board = Board::starting_pos();
        let depth = 5;
        let c1 = p1.bench(|| Perft::<true>::count_using_var(black_box(&board), depth));
        let c2 = p2.bench(|| Perft::<true>::count_using_board(black_box(&board), depth));
        let c3 = p3.bench(|| Perft::<true>::count_using_position(black_box(&board), depth));
        let c4 = p4.bench(|| Perft::<false>::count_using_board(black_box(&board), depth));
        let c5 = p5.bench(|| Perft::<false>::count_using_position(black_box(&board), depth));
        assert_eq!(c1, c2);
        assert_eq!(c1, c3);
        assert_eq!(c1, c4);
        assert_eq!(c1, c5);
        info!("perft({depth})={c1}");
        let iters = Perft::<true>::count(&board, depth - 1);
        for p in &mut [p1, p2, p3, p4, p5] {
            p.set_iters(iters);
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
        for (board, perfts) in Catalog::perfts() {
            println!("\n{}", board.to_fen());
            for (depth, &expected) in perfts.iter().enumerate() {
                if expected < 1_000_000_000_u64 {
                    let now = Instant::now();
                    let count = Perft::<true>::count(&board, depth as u32);
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
