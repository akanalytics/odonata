use crate::board::makemove::MoveMaker;
use crate::board::Board;

pub struct Perft;



impl Perft {
    pub fn perft(board: &mut Board, depth: u32) -> u64 {
        if depth == 0 {
            1
        } else {
            let moves = board.legal_moves();
            let mut count = 0u64;
            for m in moves.iter() {
                let res = Self::perft(&mut board.make_move(m), depth - 1);
                count += res;
            }
            count
        }
    }


    pub fn perft_ext(board: &mut Board, depth: u32) -> u64 {
        if depth == 0 {
            1
        } else {
            let moves = board.legal_moves_ext();
            let mut count = 0u64;
            for m in moves.iter() {
                let mut child = board.clone();
                child.make_move_ext(m);
                let res = Self::perft(&mut child, depth - 1);
                count += res;
            }
            count
        }
    }

    pub fn perft_compare(board: &mut Board, depth: u32) -> u64 {
        if depth == 0 {
            1
        } else {
            let mut moves = board.pseudo_legal_moves();
            let mut moves_ext = board.pseudo_legal_moves_ext();
            moves.sort();
            moves_ext.sort();
            assert_eq!(moves.to_string(), moves_ext.to_string());
            if moves.len() == moves_ext.len() {
                print!("+");
            } else {
                print!("-");
            }

            let count = 0u64;
            for (m1, m2) in moves.iter().zip(moves_ext.iter()) {
                let mut b2 = board.clone();
                b2.make_move_ext(m2);
                assert_eq!(board.make_move(m1), b2, "for move {} {} from {}", m1, m2, board.to_fen());
                // let res = Self::perft(&mut board.make_move(m), depth - 1);
                // count += res;
            }
            count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use std::time::Instant;

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
                if depth <= 6 {
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

    #[test]
    #[ignore]
    fn test_compare_perft() {
        for (mut board, perfts) in Catalog::perfts() {
            for (depth, &_expected) in perfts.iter().enumerate() {
                if depth <= 2 {
                    let _now = Instant::now();
                    let _count = Perft::perft_compare(&mut board, depth as u32);
                }
            }
        }
    }

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
