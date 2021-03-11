


    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }


// macro_rules! log {
//     ($msg:expr) => {
//         LOGGER.with(|logger| {
//             if let Some(logger) = logger.borrow().last() {
//                 logger.log($msg)
//             }
//         })
//     }
// }

// fn main() {
//     log!("a");
//     push_context(NetworkLogger, || {
//         log!("b");
//     });
//     log!("c");
// }

//     pub fn make_move_enum(&self, m: MoveEnum) -> Board {
//         let mut board = Board {
//             en_passant: Bitboard::EMPTY,
//             turn: self.turn.opposite(),
//             fullmove_count: if self.turn == Color::Black { self.fullmove_count + 1 } else { self.fullmove_count },
//             fifty_clock: self.fifty_clock + 1,
//             .. *self
//         };
//         match m {
//             MoveEnum::Quiet{ dest, src, mover } => {
//                 // clear one bit and set another for the move
//                 let from_to_bits = dest ^ src;
//                 board.pieces[mover.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::Push { dest, src } => {
//                 board.fifty_clock = 0;
//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             }
//             MoveEnum::Null() => {
//             },
//             MoveEnum::Capture { dest, src, mover, capture } => {
//                 board.fifty_clock = 0;
//                 board.pieces[capture.index()].remove(dest);
//                 board.colors[board.turn.index].remove(dest);

//                 let from_to_bits = dest ^ src;
//                 board.pieces[mover.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::Promo { dest, src, promo } => {
//                 board.fifty_clock = 0;
//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()].remove(src);
//                 board.pieces[promo.index()].insert(dest);
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::PromoCapture { dest, src, promo, capture } => {
//                 board.fifty_clock = 0;
//                 board.pieces[capture.index()].remove(dest);
//                 board.colors[board.turn.index].remove(dest);

//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()].remove(src);
//                 board.pieces[promo.index()].insert(dest);
//                 board.colors[self.turn.index] ^= from_to_bits;
//             },
//             MoveEnum::EnPassant { dest, src, capture_sq } => {
//                 board.fifty_clock = 0;
//                 board.pieces[Piece::Pawn.index()].remove(capture_sq);
//                 board.colors[board.turn.index].remove(capture_sq);

//                 let from_to_bits = dest ^ src;
//                 board.pieces[Piece::Pawn.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             }
//             MoveEnum::Castle { king_dest, king_src, rook_dest, rook_src, right } => {
//                 // NO reset of fifty clock. MoveEnum king then move rook.
//                 board.castling.remove(right);
//                 let from_to_bits = king_dest ^ king_src;
//                 board.pieces[Piece::King.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;

//                 let from_to_bits = rook_dest ^ rook_src;
//                 board.pieces[Piece::King.index()] ^= from_to_bits;
//                 board.colors[self.turn.index] ^= from_to_bits;
//             }
 

//         }
//         board
//     }
// }