use std::fmt;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use static_init::dynamic;

use crate::bits::castling::CastlingRights;
use crate::bits::square::Square;
use crate::domain::node::Counter;
use crate::infra::metric::*;
use crate::mv::Move;
use crate::piece::{Color, Hash, Piece};
use crate::prelude::Board;

// CPW:
// One number for each piece at each square
// One number to indicate the side to move is black
// Four numbers to indicate the castling rights, though usually 16 (2^4) are used for speed
// Eight numbers to indicate the file of a valid En passant square, if any
//
// https://web.archive.org/web/20071031100138/http://www.brucemo.com/compchess/programming/zobrist.htm
//
// chosen so hash of empty borad = 0

// dont have mutable data in same cacheline
// https://stackoverflow.com/questions/72126606/should-the-cache-padding-size-of-x86-64-be-128-bytes
#[repr(align(128))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Hasher {
    seed:                u64,
    squares:             [[[u64; Square::len()]; Piece::len()]; Color::len()], // [colour][piece][square]
    ep:                  [u64; 8],
    castling_by_bitflag: [u64; 16],
    side:                u64,
}

impl fmt::Display for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Zobrist({})", self.seed)?;
        if f.alternate() {
            writeln!(f)?;
            for c in Color::ALL {
                for p in Piece::ALL {
                    for sq in Square::all() {
                        writeln!(f, "[{}][{}][{:2}] = {:x}", c, p, sq, self.get(c, p, sq))?;
                    }
                }
            }
            writeln!(f, "side = {:x}", self.side)?;
            for cr in CastlingRights::values_array() {
                writeln!(
                    f,
                    "castling[{}] = {:x}",
                    cr,
                    self.castling_by_bitflag[cr.bits() as usize]
                )?;
            }
            for sq in 0..self.ep.len() {
                writeln!(f, "ep[{}] = {:x}", sq, self.ep[sq])?;
            }
        }
        Ok(())
    }
}

// static INSTANCE: OnceCell<Hasher> = OnceCell::new();

// static STATIC_INSTANCE: Lazy<Box<Hasher>> = Lazy::new(|| Hasher::new(3141592653589793));

#[dynamic]
static STATIC_INSTANCE: Box<Hasher> = Hasher::new(3141592653589793);

impl Hasher {
    // // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn instance() -> &'static Self {
        &STATIC_INSTANCE
    }
}

// fn init_module() {
//     Hasher::init();
// }

// static mut STATIC_INSTANCE: *const Hasher = std::ptr::null();

// //
// impl Hasher {

//     pub fn init() {
//         let me = Self::new(3141592653589793);
//         unsafe {
//             // leak the value, so it will never be dropped or freed
//             STATIC_INSTANCE = Box::leak(me) as *const Self;
//         }
//     }

//     // // doesnt impl Default as too large to copy by value
//     #[inline]
//     pub fn default() -> &'static Self {
//         unsafe {
//             &*STATIC_INSTANCE
//         }
//     }
// }

// https://docs.rs/rand/0.8.3/rand/rngs/struct.StdRng.html
// For a secure reproducible generator, we recommend use of the rand_chacha crate directly.
// hence https://crates.io/crates/rand_chacha
//
// I think the motivation is to ensure cross platform reproducibility - which I want
//
impl Hasher {
    pub fn new(seed: u64) -> Box<Self> {
        let mut rng = ChaChaRng::seed_from_u64(seed);
        let mut h = Hasher {
            seed,
            squares: [[[0; Square::len()]; Piece::len()]; Color::len()],
            side: 0,
            castling_by_bitflag: [0; 16],
            ep: [0; 8],
        };
        // let i = rng.gen::<u64>();

        // fill seems fine to use "On big-endian platforms this performs
        // byte-swapping to ensure portability of results from reproducible generators."
        for c in &Color::ALL {
            for p in &Piece::ALL {
                rng.fill(&mut h.squares[c.index()][p.index()]);
            }
        }
        h.side = rng.gen();
        let mut castling_by_flag = [0u64; 4];
        rng.fill(&mut castling_by_flag);

        // fill out the castling by bitflag using castling by flag
        for bits in 0..16 {
            for bit in 0..4 {
                if (1 << bit) & bits > 0 {
                    h.castling_by_bitflag[bits] ^= castling_by_flag[bit];
                }
            }
        }
        // h.castling[CastlingRights::NONE] = 0;
        rng.fill(&mut h.ep);
        Box::new(h)
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    #[inline(always)]
    fn get(&self, c: Color, p: Piece, sq: Square) -> Hash {
        self.squares[c][p][sq]
    }

    pub fn hash_pawns(&self, b: &Board) -> Hash {
        Metrics::incr(Counter::CalcHashPawns);
        let mut hash = 0; // b.color_us().chooser_wb(0, self.side);
        for sq in b.pawns().squares() {
            hash ^= self.get(b.color_of(sq).unwrap(), Piece::Pawn, sq);
        }
        if hash == 0 {
            hash = self.side;
        }
        hash
    }

    pub fn hash_board(&self, b: &Board) -> Hash {
        Metrics::incr(Counter::CalcHashBoard);

        let mut hash = b.color_us().chooser_wb(0, self.side);

        hash ^= self.castling_by_bitflag[b.castling().bits() as usize];

        if let Some(ep) = b.en_passant_square() {
            hash ^= self.ep[ep.index() & 7];
        }
        for &p in &Piece::ALL {
            for sq in (b.pieces(p) & b.white()).squares() {
                hash ^= self.get(Color::White, p, sq);
            }
            for sq in (b.pieces(p) & b.black()).squares() {
                hash ^= self.get(Color::Black, p, sq);
            }
        }
        hash
    }

    pub fn hash_move(&self, m: Move, pre_move: &Board) -> Hash {
        Metrics::incr(Counter::CalcHashMove);
        let mut hash = self.side;

        // for null move we still need to remove the e/p square
        if let Some(ep) = pre_move.en_passant_square() {
            hash ^= self.ep[ep.file_index()];
        }

        if m.is_null() {
            return hash;
        }

        // either we're moving to an empty square or its a capture
        let us = pre_move.color_us();

        let mover = m.mover_piece(pre_move);
        hash ^= self.get(us, mover, m.from());
        hash ^= self.get(us, mover, m.to());

        let castling_rights_change = pre_move.castling() & CastlingRights::rights_lost(m.from(), m.to());
        hash ^= self.castling_by_bitflag[castling_rights_change.bits() as usize];

        if let Some(promo) = m.promo_piece() {
            hash ^= self.get(us, Piece::Pawn, m.to());
            hash ^= self.get(us, promo, m.to());
        }

        // we return early if a capture as double_push and castle are non-captures
        if let Some(cap) = m.capture_piece(pre_move) {
            let them = pre_move.color_them();
            if m.is_ep_capture(pre_move) {
                // ep capture is like capture but with capture piece on *ep* square not *dest*
                hash ^= self.get(them, cap, m.capture_square(pre_move));
                return hash;
            } else {
                // regular capture
                hash ^= self.get(them, cap, m.to());
                return hash;
            }
        }

        if m.is_pawn_double_push(pre_move) {
            hash ^= self.ep[m.double_push_en_passant_square().file_index()];
        }

        // castling *moves*
        if m.is_castle(pre_move) {
            let (rook_from, rook_to) = m.rook_move_from_to();
            hash ^= self.get(us, Piece::Rook, rook_from);
            hash ^= self.get(us, Piece::Rook, rook_to);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use super::*;
    use crate::catalog::Catalog;
    use crate::infra::profiler::PerfProfiler;
    use crate::other::Perft;

    #[test]
    #[ignore]
    fn test_hasher_display_all() {
        // println!("{:?}", Hasher::new(1));
        println!("{}", Hasher::new(1));
        println!("{:#}", Hasher::new(1));
    }

    #[test]
    fn test_hasher_display_single() {
        assert_eq!(format!("{:x}", Hasher::new(1).ep[7]), "fc31329afb42f9a7");
    }

    #[test]
    fn test_hash_board() {
        let hasher_def = Hasher::instance();
        let b = Board::default();
        assert_eq!(hasher_def.hash_board(&b), 0);
        let hasher1 = Hasher::new(1);
        let b = Catalog::starting_board();
        assert_eq!(format!("{:x}", hasher1.hash_board(&b)), "5deb2bf6a1e5765");

        // check same seed => same results
        let hasher2 = Hasher::new(1);
        assert_eq!(format!("{:x}", hasher2.hash_board(&b)), "5deb2bf6a1e5765");

        // hash(ep=a7)
        assert_eq!(hasher1.ep[0], 15632562519469102039);

        // a1a2 hash
        assert_eq!(Square::A1.index(), 0);
        assert_eq!(Square::G1.index(), 6);
        assert_eq!(Square::H3.index(), 23);
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
        let bd1 = Catalog::starting_board();
        let mut moves = bd1.legal_moves();
        moves.push(Move::new_null());
        let hash_bd1 = hasher.hash_board(&bd1);
        for &mv in moves.iter() {
            let hash_mv = hasher.hash_move(mv, &bd1);
            let hash_bd2 = hasher.hash_board(&bd1.make_move(mv));
            // println!("Move: {} => {}", mv, hash_mv);
            assert_eq!(hash_bd1 ^ hash_mv, hash_bd2);
        }
        let mut bd1_plus_nulls = bd1.clone();
        bd1_plus_nulls.set_halfmove_clock(2); // 0+2
        bd1_plus_nulls.set_ply(2); // 0+2
        bd1_plus_nulls.set_en_passant(None);
        bd1_plus_nulls.set_fullmove_number(1 + 1);
        let bd2 = bd1.make_move(Move::new_null()).make_move(Move::new_null());
        assert_eq!(bd2, bd1_plus_nulls, "double null move {bd2:#} {bd1_plus_nulls:#}");

        let bd1 = bd1.make_move(bd1.parse_san_move("e4").unwrap());
        assert_eq!(bd1.halfmove_clock(), 0); // coz pawn move
        let mut bd1_plus_nulls = bd1.clone();
        bd1_plus_nulls.set_halfmove_clock(2); // 0+2
        bd1_plus_nulls.set_en_passant(None);
        bd1_plus_nulls.set_fullmove_number(1 + 1);
        bd1_plus_nulls.set_ply(3);
        let bd2 = bd1.make_move(Move::new_null()).make_move(Move::new_null());
        assert_eq!(bd2, bd1_plus_nulls, "e4 + double null move {bd2:#} {bd1_plus_nulls:#}");
    }

    #[test]
    fn bench_hash() {
        let mut starting_pos = Catalog::perft_kiwipete().0;

        let mut hash_move = PerfProfiler::new("hash: move");
        let mut hash_board = PerfProfiler::new("hash: board");

        let hasher = Hasher::instance();
        let mut func = |bd: &Board, mv: Move| {
            hash_move.bench(|| hasher.hash_move(black_box(mv), black_box(bd)));
            hash_board.bench(|| hasher.hash_board(black_box(bd)));
        };
        Perft::<true>::perft_with(&mut starting_pos, 2, &mut func);
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
            let hash_bd1 = hasher.hash_board(b);
            let mut count = 0u64;
            // println!("-->");
            for &m in moves.iter() {
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
