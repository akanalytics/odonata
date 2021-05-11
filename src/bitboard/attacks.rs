use crate::bitboard::bitboard::{Bitboard, Square, Dir};
use crate::types::{Color, Piece};
// use crate::bitboard::bb_classical::ClassicalBitboard;
use crate::bitboard::bb_hyperbola::Hyperbola;


pub type BitboardDefault = Hyperbola;

pub trait BitboardAttacks {


    // fn new() -> Self;
    fn bishop_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard;
    fn rook_attacks(&self, occupied: Bitboard, from: Square) -> Bitboard;
    fn knight_attacks(&self, from: Square) -> Bitboard;
    fn king_attacks(&self, from: Square) -> Bitboard;

    fn non_pawn_attacks(&self, p: Piece, occ: Bitboard, from: Square) -> Bitboard {
        match p {
            Piece::Bishop => self.bishop_attacks(occ, from),
            Piece::Rook => self.rook_attacks(occ, from),
            Piece::Queen => self.rook_attacks(occ, from) | self.bishop_attacks(occ, from),
            Piece::King => self.king_attacks(from),
            Piece::Knight => self.knight_attacks(from),
            _ => panic!(),
        }
    }

    #[inline]
    fn pawn_pushes(&self, occupied: Bitboard, pawns: Bitboard, color: &Color) -> Bitboard {
        let empty = !occupied;
        let single = pawns.shift(color.pawn_move()) & empty;
        single | (single.shift(color.pawn_move()) & empty & color.double_push_dest_rank())
    }

    #[inline]
    fn pawn_attacks(&self, pawns: Bitboard, c: Color) -> (Bitboard, Bitboard) {
        (pawns.shift(c.pawn_capture_east()), pawns.shift(c.pawn_capture_west()))
    }

    fn pawn_attackers(&self, targets: Bitboard, pawn_color: Color) -> Bitboard {
        // white east attack = black west attacker. hence w/b reversal
        let opp = pawn_color.opposite();
        targets.shift(opp.pawn_capture_east()) | targets.shift(opp.pawn_capture_west())
    }

    // tripled too (basically every pawn with a pawn south of it)
    fn doubled_pawns(pawns: Bitboard) -> Bitboard {
        pawns.fill_north().shift(Dir::N) & pawns
    }

    fn open_files(pawns: Bitboard) -> Bitboard {
        !pawns.fill_south().fill_north()
    }

    fn isolated_pawns(pawns: Bitboard) -> Bitboard {
        let closed = !Self::open_files(pawns);
        
        // non-isolated pawns have a closed (wt their color) file next to them on one side
        let non_isolated = pawns & closed.shift(Dir::E) | pawns & closed.shift(Dir::W);
        pawns - non_isolated
    }


    

    fn pawn_ep_captures(
        &self,
        pawns: Bitboard,
        opp: Bitboard,
        c: Color,
        ep: Bitboard,
    ) -> (Bitboard, Bitboard) {
        assert!(!ep.is_empty());
        let (east, west) = self.pawn_attacks(pawns, c);

        let enemy_pawn = ep.shift(c.opposite().pawn_move());

        // check enemy have occupied the square one beyond en passant square
        if (enemy_pawn & opp).is_empty() {
            return (Bitboard::EMPTY, Bitboard::EMPTY);
        }

        (east & ep, west & ep)
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::bb_classical::*;
    use crate::globals::constants::*;

    fn init() {
        // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

 

    #[test]
    fn test_pawns() {
        let classical = BitboardDefault::default();
        let pawns_w = a2 | b3 | c2 | d7 | f5 | g4 | h4 | h5;
        let opponent = a4 | b4 | d3 | g5;
        let occupied = pawns_w | opponent;

        let pawn_single_push = classical.pawn_pushes(occupied, pawns_w, &Color::White);
        let single = a3 | c3 | d8 | f6 | h6;
        let double = c4;
        assert_eq!(pawn_single_push, single | double);

        let (pawn_capture_e, pawn_capture_w) = classical.pawn_attacks(pawns_w, Color::White);
        assert_eq!(pawn_capture_e & opponent, d3);

        assert_eq!(pawn_capture_w & opponent, a4 | g5);

        let ep_square = g6;
        let (east, west) = classical.pawn_ep_captures(pawns_w, opponent, Color::White, ep_square);
        assert_eq!(east, g6);
        assert_eq!(west, g6);

        let pawns = b2 | b4 | c5 | c6 | d3 | d7 | h5;
        assert_eq!(BitboardDefault::doubled_pawns(pawns), b4 | c6 | d7);

        assert_eq!(BitboardDefault::open_files(pawns), FILE_A | FILE_E | FILE_F | FILE_G );
        assert_eq!(BitboardDefault::isolated_pawns(pawns), h5 );
        assert_eq!(BitboardDefault::isolated_pawns(opponent), d3 | g5 );

    }
}
