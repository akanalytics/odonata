use crate::bitboard::{Bitboard, Dir};
use crate::board::{Board, Move, BoardBuf, CastlingRights, Color, Piece};




impl Board {

    pub fn make_move(&self, m: Move) -> Board {
        let mut board = Board {
            en_passant: Bitboard::EMPTY,
            turn: self.turn.opposite(),
            move_count: if self.turn == Color::BLACK { self.move_count + 1 } else { self.move_count },
            fifty_clock: self.fifty_clock + 1,
            .. *self
        };
        match m {
            Move::Quiet{ dest, src, mover } => {
                // clear one bit and set another for the move
                let fromToBits = dest ^ src;
                board.pieces[mover.index()] ^= fromToBits;
                board.colors[self.turn.index] ^= fromToBits;
            },
            Move::Push{ dest, src } => {
                board.fifty_clock = 0;
                let fromToBits = dest ^ src;
                board.pieces[Piece::Pawn.index()] ^= fromToBits;
                board.colors[self.turn.index] ^= fromToBits;
            }
            Move::Null() => {
            },
            Move::Capture { dest, src, mover, capture } => {
                board.fifty_clock = 0;
                board.pieces[capture.index()].remove(dest);
                board.colors[board.turn.index].remove(dest);

                let fromToBits = dest ^ src;
                board.pieces[mover.index()] ^= fromToBits;
                board.colors[self.turn.index] ^= fromToBits;
            },
            Move::Promo { dest, src, promo } => {
                board.fifty_clock = 0;
                let fromToBits = dest ^ src;
                board.pieces[Piece::Pawn.index()].remove(src);
                board.pieces[promo.index()].insert(dest);
                board.colors[self.turn.index] ^= fromToBits;
            },
            Move::PromoCapture { dest, src, promo, capture } => {
                board.fifty_clock = 0;
                board.pieces[capture.index()].remove(dest);
                board.colors[board.turn.index].remove(dest);

                let fromToBits = dest ^ src;
                board.pieces[Piece::Pawn.index()].remove(src);
                board.pieces[promo.index()].insert(dest);
                board.colors[self.turn.index] ^= fromToBits;
            },
            Move::EnPassant { dest, src, capture_sq } => {
                board.fifty_clock = 0;
                board.pieces[Piece::Pawn.index()].remove(capture_sq);
                board.colors[board.turn.index].remove(capture_sq);

                let fromToBits = dest ^ src;
                board.pieces[Piece::Pawn.index()] ^= fromToBits;
                board.colors[self.turn.index] ^= fromToBits;
            }
            Move::Castle { king_dest, king_src, rook_dest, rook_src, right } => {
                // NO reset of fifty clock. Move king then move rook.
                board.castling.remove(right);
                let fromToBits = king_dest ^ king_src;
                board.pieces[Piece::King.index()] ^= fromToBits;
                board.colors[self.turn.index] ^= fromToBits;

                let fromToBits = rook_dest ^ rook_src;
                board.pieces[Piece::King.index()] ^= fromToBits;
                board.colors[self.turn.index] ^= fromToBits;
            }
        }
        board
    }
}
