
use crate::bitboard::bitboard::Bitboard;
use crate::types::{Piece, Color};


#[derive(Clone, Debug, PartialEq, Default, Eq)]
pub struct Multiboard {
    pieces: [Bitboard; Piece::len()],
    colors: [Bitboard; Color::len()],
}


impl Multiboard {
    #[inline]
    pub fn pieces(&self, p: Piece) -> Bitboard {
        self.pieces[p]
    }


    #[inline]
    pub fn line_pieces(&self) -> Bitboard {
        self.rooks() | self.bishops() | self.queens()
    }

    #[inline]
    pub fn non_line_pieces(&self) -> Bitboard {
        self.pawns() | self.knights() | self.kings()
    }

    #[inline]
    pub fn pawns(&self) -> Bitboard {
        self.pieces(Piece::Pawn)
    }

    #[inline]
    pub fn knights(&self) -> Bitboard {
        self.pieces(Piece::Knight)
    }

    #[inline]
    pub fn bishops(&self) -> Bitboard {
        self.pieces(Piece::Bishop)
    }

    #[inline]
    pub fn rooks(&self) -> Bitboard {
        self.pieces(Piece::Rook)
    }

    #[inline]
    pub fn queens(&self) -> Bitboard {
        self.pieces(Piece::Queen)
    }

    #[inline]
    pub fn kings(&self) -> Bitboard {
        self.pieces(Piece::King)
    }

    #[inline]
    pub fn color(&self, c: Color) -> Bitboard {
        self.colors[c.index()]
    }

    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.black() | self.white()
    }

    #[inline]
    pub fn white(&self) -> Bitboard {
        self.colors[Color::White.index()]
    }

    #[inline]
    pub fn black(&self) -> Bitboard {
        self.colors[Color::Black.index()]
    }


    #[inline]
    pub fn piece_at(&self, sq: Bitboard) -> Piece {
        debug_assert!(sq != Bitboard::EMPTY);
        for &p in &Piece::ALL_BAR_NONE {
            if self.pieces(p).contains(sq) {
                return p;
            }
        }
        Piece::None
    }

    #[inline]
    pub fn least_valuable_piece(&self, region: Bitboard) -> Bitboard {
        for &p in &Piece::ALL_BAR_NONE {
            if self.pieces(p).intersects(region) {
                return (self.pieces(p) & region).first();
            }
        }
        Bitboard::EMPTY
    }

    #[inline]
    pub fn remove_piece(&mut self, sq: Bitboard, p: Piece, c: Color) {
        self.pieces[p].remove(sq);
        self.colors[c].remove(sq);
    }

    #[inline]
    pub fn move_piece(&mut self, from_sq: Bitboard, to_sq: Bitboard, p: Piece, c: Color) {
        self.pieces[p] ^= from_sq | to_sq;
        self.colors[c] ^= from_sq | to_sq;
    }

    #[inline]
    pub fn change_piece(&mut self, sq: Bitboard, from: Piece, to: Piece) {
        self.pieces[from].remove(sq);
        self.pieces[to].insert(sq);
    }

    #[inline]
    pub fn set_piece_at(&mut self, sq: Bitboard, p: Piece) {
        for bb in self.pieces.iter_mut() {
            bb.remove(sq);
        }
        // self.0.pieces(p).remove(sq);
        if p != Piece::None {
            self.pieces[p].insert(sq);
        }
    }

    #[inline]
    pub fn set_color_at(&mut self, sq: Bitboard, c: Option<Color>) {
        if let Some(c) = c {
            self.colors[c.opposite()].remove(sq);
            self.colors[c].insert(sq);
        }
        else {
            self.colors[Color::White].remove(sq);
            self.colors[Color::Black].remove(sq);
        }
    }

    #[inline]
    pub fn color_flip(&self) -> Multiboard {
        let mut mb = self.clone();
        mb.colors = [self.colors[1].flip_vertical(), self.colors[0].flip_vertical()];
        mb.pieces.iter_mut().for_each(|bb| *bb = bb.flip_vertical() );
        mb
    }


}
