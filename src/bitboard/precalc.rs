use crate::types::{Color, Piece};
// use crate::bitboard::bb_classical::ClassicalBitboard;
use crate::bitboard::bb_hyperbola::Hyperbola;
use crate::bitboard::bb_sliders::SlidingPieceAttacks;
use crate::bitboard::bitboard::{Bitboard, Dir};
use crate::bitboard::square::Square;
use static_init::dynamic;

pub type BitboardDefault = PreCalc;

pub type BestSlidingPieceAttacks = Hyperbola;

#[dynamic]
static STATIC_INSTANCE: Box<PreCalc> = PreCalc::new();

impl PreCalc {
    // doesnt impl Default as too large to copy by value
    #[inline]
    pub fn default() -> &'static Self {
        &STATIC_INSTANCE
    }
}

pub struct PreCalc {
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
    pawn_front_span: [[Bitboard; 64];2], 
    pawn_push: [[Bitboard; 64];2],  
    pawn_capture_east: [[Bitboard; 64];2],
    pawn_capture_west: [[Bitboard; 64];2],
    pawn_attack_span: [[Bitboard; 64];2],
    strictly_between: [[Bitboard; 64]; 64],
    line: [[Bitboard; 64]; 64],
    sliding_piece_attacks: Box<BestSlidingPieceAttacks>,
}

impl PreCalc {
    fn new() -> Box<Self> {
        let mut me = Box::new(Self {
            sliding_piece_attacks: BestSlidingPieceAttacks::new(),
            king_moves: [Bitboard::EMPTY; 64],
            knight_moves: [Bitboard::EMPTY; 64],
            pawn_front_span: [[Bitboard::EMPTY; 64];2],
            pawn_push: [[Bitboard::EMPTY; 64];2],  
            pawn_capture_east: [[Bitboard::EMPTY; 64];2],
            pawn_capture_west: [[Bitboard::EMPTY; 64];2],
            pawn_attack_span: [[Bitboard::EMPTY; 64];2],
            strictly_between: [[Bitboard::EMPTY; 64]; 64],
            line: [[Bitboard::EMPTY; 64]; 64],
        });

        Self::pop_strictly_between(&mut me.strictly_between);
        Self::pop_king_moves(&mut me.king_moves);
        Self::pop_knight_moves(&mut me.knight_moves);
        Self::pop_line(&mut me.line);
        me.pop_pawn();
        me
    }

    fn pop_pawn(&mut self) {
        for c in Color::ALL {
            for pawn in Square::all() {
                self.pawn_front_span[c][pawn] = pawn.as_bb().rays(c.forward());
                self.pawn_push[c][pawn] = pawn.as_bb().shift(c.forward());
                let e = pawn.as_bb().shift(c.pawn_capture_east());
                let w = pawn.as_bb().shift(c.pawn_capture_west());
                self.pawn_capture_east[c][pawn] = e;
                self.pawn_capture_west[c][pawn] = w;
                self.pawn_attack_span[c][pawn] = (e | w).rays(c.forward()) | e | w;
            }
        }
    }

    fn pop_king_moves(king_moves: &mut [Bitboard; 64]) {
        for sq in 0..64_usize {
            for &dir in Dir::ALL.iter() {
                let bb = Bitboard::from_sq(sq as u8);
                king_moves[sq] |= bb.shift(dir);
            }
        }
    }

    fn pop_knight_moves(knight_moves: &mut [Bitboard; 64]) {
        for sq in 0..64_usize {
            for &dir in Dir::ALL.iter() {
                let bb = Bitboard::from_sq(sq as u8);
                // for example a night attack might be step N followed by step NE
                let next_dir = dir.rotate_clockwise();
                knight_moves[sq] |= bb.shift(dir).shift(next_dir);
            }
        }
    }

    fn pop_strictly_between(strictly_between: &mut [[Bitboard; 64]; 64]) {
        for s1 in Bitboard::all().squares() {
            for s2 in Bitboard::all().squares() {
                strictly_between[s1][s2] = Square::calc_strictly_between(s1, s2);
            }
        }
    }

    fn pop_line(line: &mut [[Bitboard; 64]; 64]) {
        for s1 in Bitboard::all().squares() {
            for s2 in Bitboard::all().squares() {
                line[s1][s2] = Square::calc_line_through(s1, s2);
            }
        }
    }

    #[inline]
    pub fn bishop_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard {
        self.sliding_piece_attacks.bishop_attacks(occupied, from)
    }

    #[inline]
    pub fn rook_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard {
        self.sliding_piece_attacks.rook_attacks(occupied, from)
    }

    // inclusive of end points
    #[inline]
    pub fn strictly_between(&self, s1: Square, s2: Square) -> Bitboard {
        self.strictly_between[s1][s2]
    }

    #[inline]
    pub fn between(&self, s1: Square, s2: Square) -> Bitboard {
        self.strictly_between(s1, s2).include(s1).include(s2)
    }

    // king moves - see https://www.chessprogramming.org/Distance
    #[inline]
    pub fn chebyshev_distance(&self, s1: Square, s2: Square) -> u32 {
        i32::max(
            i32::abs(s1.rank_index() as i32 - s2.rank_index() as i32),
            i32::abs(s1.file_index() as i32 - s2.file_index() as i32),
        ) as u32
    }

    #[inline]
    pub fn line_through(&self, s1: Square, s2: Square) -> Bitboard {
        self.line[s1][s2]
    }

    #[inline]
    pub fn king_attacks(&self, from: Square) -> Bitboard {
        self.king_moves[from]
    }

    #[inline]
    pub fn knight_attacks(&self, from: Square) -> Bitboard {
        self.knight_moves[from]
    }

    #[inline]
    pub fn non_pawn_attacks(
        &self,
        c: Color,
        p: Piece,
        us: Bitboard,
        them: Bitboard,
        from: Square,
    ) -> Bitboard {
        match p {
            Piece::Bishop => self.bishop_attacks(us | them, from),
            Piece::Rook => self.rook_attacks(us | them, from),
            Piece::Queen => self.rook_attacks(us | them, from) | self.bishop_attacks(us | them, from),
            Piece::King => self.king_attacks(from),
            Piece::Knight => self.knight_attacks(from),
            Piece::Pawn => self.pawn_attacks_ext(c, us, them, from),
            _ => panic!(),
        }
    }

    #[inline]
    pub fn pawn_attacks_ext(&self, c: Color, us: Bitboard, them: Bitboard, fr: Square) -> Bitboard {
        let empty = !(us | them);
        let single = self.pawn_push[c][fr] & empty;
        let double = single.shift(c.forward()) & empty & c.double_push_dest_rank();
        let capture = them & (self.pawn_capture_east[c][fr] | self.pawn_capture_west[c][fr]);
        single | double | capture
    }

    // #[inline]
    // fn pawn_pushes(&self, occupied: Bitboard, pawns: Bitboard, color: Color) -> Bitboard {
    //     let empty = !occupied;
    //     let single = pawns.shift(color.forward()) & empty;
    //     single | (single.shift(color.forward()) & empty & color.double_push_dest_rank())
    // }

    #[inline]
    pub fn pawn_attacks(&self, pawns: Bitboard, c: Color) -> (Bitboard, Bitboard) {
        (
            pawns.shift(c.pawn_capture_east()),
            pawns.shift(c.pawn_capture_west()),
        )
    }

    #[inline]
    pub fn pawn_attackers(&self, targets: Bitboard, pawn_color: Color) -> Bitboard {
        // white east attack = black west attacker. hence w/b reversal
        let opp = pawn_color.opposite();
        targets.shift(opp.pawn_capture_east()) | targets.shift(opp.pawn_capture_west())
    }

    // tripled too (basically every pawn with a pawn north of it)
    // two pawns on same file, only one pawn is doubled
    #[inline]
    pub fn doubled_pawns(&self, pawns: Bitboard) -> Bitboard {
        pawns.fill_north().shift(Dir::N) & pawns
    }

    // front span = squares strictly in front of pawn
    #[inline]
    pub fn pawn_front_span(&self, c: Color, pawn_sq: Square) -> Bitboard {
        self.pawn_front_span[c][pawn_sq] 
        // let pawn = pawn_sq.as_bb();
        // let atts = pawn.shift(c.pawn_capture_east()) | pawn.shift(c.pawn_capture_west());
        // if c == Color::White {
        //     (pawn | atts).fill_north()
        // } else {
        //     (pawn | atts).fill_south()
        // }
    }

    // attack span = squares attacked and those in front of squares attacked 
    #[inline]
    pub fn pawn_attack_span(&self, c: Color, pawn_sq: Square) -> Bitboard {
        self.pawn_attack_span[c][pawn_sq] 
    }

    // arr span = front span UNION attack span 
    #[inline]
    pub fn pawn_front_span_union_attack_span(&self, c: Color, pawn_sq: Square) -> Bitboard {
        self.pawn_front_span[c][pawn_sq] | self.pawn_attack_span[c][pawn_sq]
    }

    // square in front of pawn (or empty)
    #[inline]
    pub fn pawn_stop(&self, c: Color, pawn_sq: Square) -> Bitboard {
        self.pawn_push[c][pawn_sq]
    }


    #[inline]
    pub fn open_files(&self, pawns: Bitboard) -> Bitboard {
        !pawns.fill_south().fill_north()
    }

    #[inline]
    pub fn isolated_pawns(&self, pawns: Bitboard) -> Bitboard {
        let closed = !self.open_files(pawns);
        // non-isolated pawns have a closed (wt their color) file next to them on one side
        let non_isolated = pawns & closed.shift(Dir::E) | pawns & closed.shift(Dir::W);
        pawns - non_isolated
    }

    // fn pawn_ep_captures(
    //     &self,
    //     pawns: Bitboard,
    //     opp: Bitboard,
    //     c: Color,
    //     ep: Bitboard,
    // ) -> (Bitboard, Bitboard) {
    //     debug_assert!(!ep.is_empty());
    //     let (east, west) = self.pawn_attacks(pawns, c);

    //     let enemy_pawn = ep.shift(c.opposite().forward());

    //     // check enemy have occupied the square one beyond en passant square
    //     if (enemy_pawn & opp).is_empty() {
    //         return (Bitboard::EMPTY, Bitboard::EMPTY);
    //     }

    //     (east & ep, west & ep)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::bitboard::bb_classical::ClassicalBitboard;
    use crate::globals::constants::*;

    #[test]
    fn test_king_attacks() {
        let classical = PreCalc::new();
        let attacks = classical.king_attacks(a6.square());
        assert_eq!(attacks, a5 | b5 | b6 | b7 | a7);

        let attacks = classical.king_attacks(c6.square());
        assert_eq!(attacks, b5 | c5 | d5 | b6 | d6 | b7 | c7 | d7)
    }

    #[test]
    fn test_knight_attacks() {
        let classical = PreCalc::new();
        let attacks = classical.knight_attacks(a1.square());
        assert_eq!(attacks, b3 | c2);

        let attacks = classical.knight_attacks(c6.square());
        assert_eq!(attacks, a5 | a7 | b4 | b8 | d4 | d8 | e5 | e7)
    }

    #[test]
    fn test_pawns() {
        let bb = BitboardDefault::default();
        let pawns_w = a2 | b3 | c2 | d7 | f5 | g4 | h4 | h5;
        let opponent = a4 | b4 | d3 | g5;
        let _occupied = pawns_w | opponent;

        // let pawn_single_push = bb.pawn_pushes(occupied, pawns_w, Color::White);
        // let single = a3 | c3 | d8 | f6 | h6;
        // let double = c4;
        // assert_eq!(pawn_single_push, single | double);

        let (pawn_capture_e, pawn_capture_w) = bb.pawn_attacks(pawns_w, Color::White);
        assert_eq!(pawn_capture_e & opponent, d3);

        assert_eq!(pawn_capture_w & opponent, a4 | g5);

        // let ep_square = g6;
        // let (east, west) = bb.pawn_ep_captures(pawns_w, opponent, Color::White, ep_square);
        // assert_eq!(east, g6);
        // assert_eq!(west, g6);

        let pawns = b2 | b4 | c5 | c6 | d3 | d7 | h5;
        assert_eq!(BitboardDefault::default().doubled_pawns(pawns), b4 | c6 | d7);

        assert_eq!(
            BitboardDefault::default().open_files(pawns),
            FILE_A | FILE_E | FILE_F | FILE_G
        );
        assert_eq!(BitboardDefault::default().isolated_pawns(pawns), h5);
        assert_eq!(BitboardDefault::default().isolated_pawns(opponent), d3 | g5);

        let calced = BitboardDefault::default().pawn_front_span_union_attack_span(Color::White, Square::B2); 
        let expect = (Bitboard::FILE_A | Bitboard::FILE_B| Bitboard::FILE_C) - (Bitboard::RANK_1 | Bitboard::RANK_2);
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect );

        let calced = BitboardDefault::default().pawn_front_span_union_attack_span(Color::White, Square::A2);
        let expect = (Bitboard::FILE_A | Bitboard::FILE_B) - (Bitboard::RANK_1 | Bitboard::RANK_2);
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect );

        let calced = BitboardDefault::default().pawn_front_span_union_attack_span(Color::White, Square::H8);
        let expect = Bitboard::EMPTY;
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect );

        let calced = BitboardDefault::default().pawn_front_span_union_attack_span(Color::Black, Square::D7);
        let expect = (Bitboard::FILE_C | Bitboard::FILE_D | Bitboard::FILE_E) - (Bitboard::RANK_8 | Bitboard::RANK_7);
        println!("{}\n{}", calced, expect);
        assert_eq!(calced, expect );
    }

    #[test]
    fn test_between() {
        let bb = BitboardDefault::default();
        assert_eq!(bb.between(a1.square(), a3.square()), a1 | a2 | a3);
        assert_eq!(bb.between(a3.square(), a1.square()), a1 | a2 | a3);
        assert_eq!(bb.between(a1.square(), a8.square()), FILE_A);
        assert_eq!(bb.between(a1.square(), a1.square()), a1);
        assert_eq!(bb.between(a1.square(), b2.square()), a1 | b2);

        assert_eq!(bb.strictly_between(a1.square(), a3.square()), a2);
        assert_eq!(bb.strictly_between(a3.square(), a1.square()), a2);
        assert_eq!(bb.strictly_between(a1.square(), a8.square()), FILE_A - a1 - a8);
        assert_eq!(bb.strictly_between(a1.square(), a1.square()), Bitboard::empty());
        assert_eq!(bb.strictly_between(a1.square(), b2.square()), Bitboard::empty());
    }
}
