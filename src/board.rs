use crate::bitboard::{Bitboard, Dir};

pub struct Color {
    pub index: usize,
    pub pawn_move: Dir,
    pub pawn_capture_east: Dir,
    pub pawn_capture_west: Dir,
    pub kingside_castle_sqs: Bitboard,
    pub queenside_castle_sqs: Bitboard,
    pub double_push_dest_rank: Bitboard,
    pub castle_rights_queen: CastlingRights,
    pub castle_rights_king: CastlingRights,
}

bitflags! {
    pub struct CastlingRights: u8 {
        const WHITE_KING = 1 << 0;
        const WHITE_QUEEN = 1 << 1;
        const BLACK_KING = 1 << 2;
        const BLACK_QUEEN = 1 << 3;
    }
}

impl Color {
    pub const WHITE: Self = Color {
        index: 0,
        pawn_move: Dir::N,
        pawn_capture_east: Dir::NE,
        pawn_capture_west: Dir::NW,
        kingside_castle_sqs: Bitboard::F1.or(Bitboard::G1),
        queenside_castle_sqs: Bitboard::D1.or(Bitboard::C1).or(Bitboard::B1),
        double_push_dest_rank: Bitboard::RANK_4,
        castle_rights_queen: CastlingRights::WHITE_QUEEN,
        castle_rights_king: CastlingRights::WHITE_KING,
    };
    pub const BLACK: Self = Color {
        index: 1,
        pawn_move: Dir::S,
        pawn_capture_east: Dir::SE,
        pawn_capture_west: Dir::SW,
        kingside_castle_sqs: Bitboard::F8.or(Bitboard::G8),
        queenside_castle_sqs: Bitboard::D8.or(Bitboard::C8),
        double_push_dest_rank: Bitboard::RANK_5,
        castle_rights_queen: CastlingRights::BLACK_QUEEN,
        castle_rights_king: CastlingRights::BLACK_KING,
    };

    pub fn opposite(&self) -> &Color {
        [&Color::BLACK, &Color::WHITE][self.index]
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Piece {
    None = 0,
    Pawn = 1,
    Knight = 2,
    Bishop = 3,
    Rook = 4,
    Queen = 5,
    King = 6,
}

impl Piece {
    const ALL: [Piece; 6] = [Piece::Pawn, Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen, Piece::King];
}

pub struct Board {
    pieces: [Bitboard; Piece::ALL.len()],
    colors: [Bitboard; 2],
    castling: CastlingRights,
    en_passant: Bitboard,
    turn: Color,
    move_count: u16,
    fifty_clock: u16,
}

impl Board {

    pub fn piece_at(&self, at: Bitboard) -> Piece {
        for p in &Piece::ALL {
            if self.pieces[*p as usize].contains(at) {
                return *p;
            }
        }
        Piece::None
    }

    // pub fn piece_at(&self, at: Bitboard) -> (Piece, Color) {
    //     for p in &Piece::ALL {
    //         if self.pieces[*p as usize].contains(at) {
    //             let c = if self.colors[Color::WHITE.index].contains(at) { Color::WHITE } else { Color::BLACK };
    //             return (*p, c);
    //         }
    //     }
    //     (Piece::None, Color::BLACK)  
    // }

    pub fn pieces(&self, c: &Color, p: Piece) -> Bitboard {
        self.pieces[p as usize] & self.colors[c.index]
    }

    pub fn colors(&self, c: &Color) -> Bitboard {
        self.colors[c.index]
    }

}


// impl  std::ops::IndexMut<Bitboard> for Board {
//     fn index_mut(&mut self, index: Bitboard) -> &mut String {
//     }
// }

// impl std::ops::Index<Bitboard> for Board {
//     let mut pieces = String::new();
//     fn index(&mut self, index: Bitboard) -> &String {
//         for sq in index.iter() {
//             let pc = self.piece_at(sq);
//             if pc.0 != Piece:None {
//                 pieces.push(pc.to_char())     
//             } 
//         }
//     }
//     pieces
// }