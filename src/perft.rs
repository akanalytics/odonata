use crate::board::makemove::MoveMaker;
use crate::board::movegen::MoveGen;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use crate::catalog::Catalog;

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

    #[test]
    #[ignore]
    fn test_perft_slow() {
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
