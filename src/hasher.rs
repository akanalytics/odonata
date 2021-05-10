use crate::bitboard::Square;
use crate::board::Board;
use crate::castling::CastlingRights;
use crate::globals::constants::*;
use crate::globals::counts;
use crate::movelist::Move;
use crate::types::{Color, Hash, Piece};
use once_cell::sync::Lazy;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;

use std::fmt;

// CPW:
// One number for each piece at each square
// One number to indicate the side to move is black
// Four numbers to indicate the castling rights, though usually 16 (2^4) are used for speed
// Eight numbers to indicate the file of a valid En passant square, if any
//
// https://web.archive.org/web/20071031100138/http://www.brucemo.com/compchess/programming/zobrist.htm
//
// chosen so hash of empty borad = 0
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Hasher {
    seed: u64,
    squares: [[[u64; 64]; Piece::len()]; Color::len()], // [colour][piece][square]
    side: u64,
    castling: [u64; 4],
    ep: [u64; 8],
}

impl fmt::Display for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Zobrist({})", self.seed)?;
        if f.alternate() {
            writeln!(f)?;
            for c in &Color::ALL {
                for p in &Piece::ALL_BAR_NONE {
                    for sq in 0..64 {
                        writeln!(
                            f,
                            "[{}][{}][{:2}] = {:x}",
                            c,
                            p,
                            sq,
                            self.squares[c.index()][p.index()][sq]
                        )?;
                    }
                }
            }
            writeln!(f, "side = {:x}", self.side)?;
            for cr in CastlingRights::iter() {
                writeln!(f, "castling[{}] = {:x}", cr, self.castling[cr.index()])?;
            }
            for sq in 0..self.ep.len() {
                writeln!(f, "ep[{}] = {:x}", sq, self.ep[sq])?;
            }
        }
        Ok(())
    }
}

// static INSTANCE: OnceCell<Hasher> = OnceCell::new();

static HASHER: Lazy<Hasher> = Lazy::new(|| Hasher::new(3141592653589793));

// https://docs.rs/rand/0.8.3/rand/rngs/struct.StdRng.html
// For a secure reproducible generator, we recommend use of the rand_chacha crate directly.
// hence https://crates.io/crates/rand_chacha
//
// I think the motivation is to ensure cross platform reproducibility - which I want
//
impl Hasher {
    pub fn new(seed: u64) -> Self {
        let mut rng = ChaChaRng::seed_from_u64(seed);
        let mut h = Hasher {
            seed,
            squares: [[[0; 64]; Piece::len()]; Color::len()],
            side: 0,
            castling: [0; 4],
            ep: [0; 8],
        };
        // let i = rng.gen::<u64>();

        // fill seems fine to use "On big-endian platforms this performs
        // byte-swapping to ensure portability of results from reproducible generators."
        for c in &Color::ALL {
            for p in &Piece::ALL_BAR_NONE {
                rng.fill(&mut h.squares[c.index()][p.index()]);
            }
        }
        h.side = rng.gen();
        rng.fill(&mut h.castling);
        rng.fill(&mut h.ep);
        h
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    // doesnt impl Default as large to copy by value
    pub fn default() -> &'static Self {
        &HASHER
    }

    fn get(&self, c: Color, p: Piece, sq: Square) -> &Hash {
        unsafe { self.squares[c.index()][p.index()].get_unchecked(sq.index()) }
    }

    pub fn hash_board(&self, b: &Board) -> Hash {
        counts::BOARD_HASH_COUNT.increment();
        let mut hash = b.color_us().chooser_wb(0, self.side);
        for cr in CastlingRights::iter() {
            if b.castling().contains(*cr) {
                hash ^= self.castling[cr.index()];
            }
        }
        if !b.en_passant().is_empty() {
            hash ^= self.ep[b.en_passant().first_square().index() & 7];
        }
        for &p in &Piece::ALL_BAR_NONE {
            for bb in b.pieces(p).iter() {
                let sq = bb.first_square();
                if b.color(Color::White).contains(bb) {
                    hash ^= self.get(Color::White, p, sq);
                } else {
                    hash ^= self.get(Color::Black, p, sq);
                }
            }
        }
        hash
    }

    pub fn hash_move(&self, m: &Move, pre_move: &Board) -> Hash {
        counts::MOVE_HASH_COUNT.increment();
        // either we're moving to an empty square or its a capture
        let us = pre_move.color_us();
        let them = pre_move.color_them();
        let mut hash = self.side;
        if !pre_move.en_passant().is_empty() {
            hash ^= self.ep[pre_move.en_passant().first_square().index() & 7];
        }

        if m.is_capture() {
            if m.is_ep_capture() {
                // ep capture is like capture but with capture piece on *ep* square not *dest*
                hash ^= self.get(them, m.capture_piece(), m.ep().first_square());
            } else {
                // regular capture
                hash ^= self.get(them, m.capture_piece(), m.to().first_square());
            }
        }

        hash ^= self.get(us, m.mover_piece(), m.from().first_square());
        hash ^= self.get(us, m.mover_piece(), m.to().first_square());

        if m.mover_piece() == Piece::Pawn && m.is_pawn_double_push() {
            debug_assert!(
                !m.ep().is_empty(),
                "e/p square must be set for pawn double push {:?}",
                m
            );
            hash ^= self.ep[m.ep().first_square().index() & 7];
        }

        if m.is_promo() {
            hash ^= self.get(us, Piece::Pawn, m.to().first_square());
            hash ^= self.get(us, m.promo_piece(), m.to().first_square());
        }

        // castling *moves*
        if m.is_castle() {
            let rook_from;
            let rook_to;

            #[allow(non_upper_case_globals)]
            match m.to() {
                c1 => {
                    rook_from = a1;
                    rook_to = d1;
                }
                g1 => {
                    rook_from = h1;
                    rook_to = f1;
                }
                c8 => {
                    rook_from = a8;
                    rook_to = d8;
                }
                g8 => {
                    rook_from = h8;
                    rook_to = f8;
                }
                _ => panic!("Castling move from square {}", m.to()),
            }
            hash ^= self.get(us, Piece::Rook, rook_from.first_square());
            hash ^= self.get(us, Piece::Rook, rook_to.first_square());
        }

        // castling *rights*
        //  if a piece moves TO (=capture) or FROM the rook squares - appropriate castling rights are lost
        //  if a piece moves FROM the kings squares, both castling rights are lost
        //  possible with a rook x rook capture that both sides lose castling rights
        if (m.from() | m.to()).intersects(CastlingRights::rook_and_king_squares()) {
            if (m.from() == e1 || m.from() == a1 || m.to() == a1)
                && pre_move.castling().contains(CastlingRights::WHITE_QUEEN)
            {
                hash ^= self.castling[CastlingRights::WHITE_QUEEN.index()];
            }
            if (m.from() == e1 || m.from() == h1 || m.to() == h1)
                && pre_move.castling().contains(CastlingRights::WHITE_KING)
            {
                hash ^= self.castling[CastlingRights::WHITE_KING.index()];
            }

            if (m.from() == e8 || m.from() == a8 || m.to() == a8)
                && pre_move.castling().contains(CastlingRights::BLACK_QUEEN)
            {
                hash ^= self.castling[CastlingRights::BLACK_QUEEN.index()];
            }
            if (m.from() == e8 || m.from() == h8 || m.to() == h8)
                && pre_move.castling().contains(CastlingRights::BLACK_KING)
            {
                hash ^= self.castling[CastlingRights::BLACK_KING.index()];
            }
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::makemove::MoveMaker;
    use crate::catalog::Catalog;

    #[test]
    fn test_hasher_display() {
        // println!("{:?}", Hasher::new(1));
        println!("{}", Hasher::new(1));
        println!("{:#}", Hasher::new(1));
        assert_eq!(format!("{:x}", Hasher::new(1).ep[7]), "fc31329afb42f9a7");
    }

    #[test]
    fn test_hash_board() {
        let hasher_def = Hasher::default();
        let b = Board::default();
        assert_eq!(hasher_def.hash_board(&b), 0);
        let hasher1 = Hasher::new(1);
        let b = Catalog::starting_position();
        assert_eq!(format!("{:x}", hasher1.hash_board(&b)), "5deb2bf6a1e5765");

        // check same seed => same results
        let hasher2 = Hasher::new(1);
        assert_eq!(format!("{:x}", hasher2.hash_board(&b)), "5deb2bf6a1e5765");

        // hash(ep=a7)
        assert_eq!(hasher1.ep[0], 15632562519469102039);

        // a1a2 hash
        assert_eq!(a1.first_square().index(), 0);
        assert_eq!(g1.first_square().index(), 6);
        assert_eq!(h3.first_square().index(), 23);
        let hash_a1a2 = hasher1.squares[Color::White.index()][Piece::Rook.index()][0]
            ^ hasher1.squares[Color::White.index()][Piece::Rook.index()][8];
        assert_eq!(hash_a1a2, 4947796874932763259);
        assert_eq!(hash_a1a2 ^ hasher1.ep[0] ^ hasher1.side, 17278715166005629523);

        // g1h3 hash
        let hash_g1h3 = hasher1.squares[Color::White.index()][Piece::Knight.index()][6]
            ^ hasher1.squares[Color::White.index()][Piece::Knight.index()][23];
        assert_eq!(hash_g1h3, 2343180499638894504);
        assert_eq!(hash_g1h3 ^ hasher1.side, 5987230143978898519);
        assert_eq!(hash_g1h3 ^ hasher1.ep[0] ^ hasher1.side, 10080444449497094016);
    }

    #[test]
    fn test_hash_move() {
        let hasher = Hasher::new(1);
        let bd1 = Catalog::starting_position();
        let moves = bd1.legal_moves();
        let hash_bd1 = hasher.hash_board(&bd1);
        for mv in moves.iter() {
            let hash_mv = hasher.hash_move(mv, &bd1);
            let hash_bd2 = hasher.hash_board(&bd1.make_move(mv));
            // println!("Move: {} => {}", mv, hash_mv);
            assert_eq!(hash_bd1 ^ hash_mv, hash_bd2);
        }
    }

    #[test]
    fn test_hash_perft() {
        let hasher = Hasher::new(1);
        for (board, _perfts) in Catalog::perfts() {
            let _count = perft_with_hash(&board, 3, &hasher);
        }
    }

    #[test]
    #[ignore]
    fn test_hash_perft_slow() {
        let hasher = Hasher::new(1);
        for (board, _perfts) in Catalog::perfts() {
            let _count = perft_with_hash(&board, 5, &hasher);
        }
    }

    pub fn perft_with_hash(b: &Board, depth: u32, hasher: &Hasher) -> u64 {
        let b = &mut b.clone();
        if depth == 0 {
            1
        } else {
            let moves = b.legal_moves();
            let hash_bd1 = hasher.hash_board(&b);
            let mut count = 0u64;
            // println!("-->");
            for m in moves.iter() {
                let bd2 = b.make_move(m);
                let hash_mv = hasher.hash_move(m, b);
                let hash_bd2 = hasher.hash_board(&bd2);
                // println!("Move: {:#} = {}", m, hash_mv);
                assert_eq!(
                    hash_bd1 ^ hash_mv,
                    hash_bd2,
                    "board1:{:#}\nmv:{:#}\nboard2:{:#}",
                    b,
                    m,
                    bd2
                );
                let res = perft_with_hash(&bd2, depth - 1, hasher);
                count += res;
            }
            // println!("<--");
            count
        }
    }
}
