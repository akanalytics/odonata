use crate::attacks::{BitboardAttacks, ClassicalBitboard};
use crate::bitboard::{Bitboard, Dir};
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::movelist::{Move, MoveList};
use crate::types::{CastlingRights, Color, Piece};
use once_cell::sync::OnceCell;

fn global_classical_bitboard() -> &'static ClassicalBitboard {
    static INSTANCE: OnceCell<ClassicalBitboard> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        debug!("Initilizing classical bitboard lookup tables");
        ClassicalBitboard::new()
    })
}

fn attacked_by(targets: Bitboard, occ: Bitboard, board: &Board, opponent: Color) -> Bitboard {
    let pawns = board.pawns() & board.color(opponent);
    let knights = board.knights() & board.color(opponent);
    let bishops = board.bishops() & board.color(opponent);
    let rooks = board.rooks() & board.color(opponent);
    let queens = board.queens() & board.color(opponent);
    let kings = board.kings() & board.color(opponent);

    let attack_gen = global_classical_bitboard();
    let (east, west) = attack_gen.pawn_attacks(pawns, &opponent);
    let mut attackers = (east | west) & targets;

    for each in targets.iter() {
        let sq = each.first_square();
        attackers |= attack_gen.knight_attacks(sq) & knights
            | attack_gen.king_attacks(sq) & kings
            | attack_gen.bishop_attacks(occ, sq) & (bishops | queens)
            | attack_gen.rook_attacks(occ, sq) & (rooks | queens);
        // TODO: en passant!!
    }
    debug!("opponent:{}\n{}target\n{}attackers\n{}", opponent, board, targets, attackers);

    attackers
}

pub trait MoveGen {
    fn is_in_check(&self, c: Color) -> bool;
    fn will_check(&self, mv: &Move) -> bool;
    fn is_legal_move(&self, mv: &Move) -> bool;
    fn pseudo_legal_moves(&self) -> MoveList;
    fn legal_moves(&self) -> MoveList;
}

impl MoveGen for Board {
    fn will_check(&self, mv: &Move) -> bool {
        debug_assert!(self.is_legal_move(mv));
        let their_king_color = self.color_them();
        self.make_move(mv).is_in_check(their_king_color)
    }

    /// called with is_in_check( board.turn() ) to see if currently in check
    fn is_in_check(&self, king_color: Color) -> bool {
        let board = &self;
        let us = board.color(king_color);
        let them = board.color(king_color.opposite());
        let our_king = board.kings() & us;
        debug_assert!(!our_king.is_empty(), "king ({}) not found {}", king_color, board);
        let occ = us | them;
        !attacked_by(our_king, occ, board, king_color.opposite()).is_empty()
    }

    fn is_legal_move(&self, mv: &Move) -> bool {
        mv.is_castle() || mv.is_drop() || !self.make_move(mv).is_in_check(self.color_us())
    }

    fn legal_moves(&self) -> MoveList {
        let mut moves = self.pseudo_legal_moves();
        moves.retain(|m| self.is_legal_move(m));
        moves
    }

    // TODO: Vec::with_capacity(100).
    fn pseudo_legal_moves(&self) -> MoveList {
        let board = &self;
        let mut moves = MoveList::new();
        let color = board.color_us();
        let them = board.them();
        let us = board.us();
        let occupied = us | them;

        let pawns = board.pawns() & us;

        let attack_gen = global_classical_bitboard();

        // non-promoted single-push pawns
        let pawn_push = attack_gen.pawn_pushes(occupied, pawns, &color);
        for to in (pawn_push & !Bitboard::PROMO_RANKS).iter() {
            let mut from = to.shift(&color.pawn_move().opposite());
            let mut ep = Bitboard::EMPTY;
            if !pawns.contains(from) {
                // must have been double push
                ep = from;
                from = from.shift(&color.pawn_move().opposite());
            }
            // let m = MoveEnum::Push { to, from };
            let m = Move { from, to, ep, mover: Piece::Pawn, ..Default::default() };
            moves.push(m);
        }
        // pawn promos - only makes sense for single push
        for to in (pawn_push & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_move().opposite());
            // try and pre-sort promos by likely usefulness
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                let m = Move { from, to, promo, mover: Piece::Pawn, ..Default::default() };
                // let m = MoveEnum::Promo { to, from, promo };
                moves.push(m);
            }
        }
        // pawn_captures
        let (pawn_captures_e, pawn_captures_w) = attack_gen.pawn_attacks(pawns, &color);
        for to in (pawn_captures_e & them & !Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_east().opposite());
            let capture = board.piece_at(to);
            let m = Move { from, to, mover: Piece::Pawn, capture, ..Default::default() };
            // MoveEnum::Capture { to, from, mover: Piece::Pawn, capture });
            moves.push(m);
        }
        for to in (pawn_captures_w & them & !Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_west().opposite());
            let capture = board.piece_at(to);
            let m = Move { from, to, mover: Piece::Pawn, capture, ..Default::default() };
            // MoveEnum::Capture { to, from, mover: Piece::Pawn, capture };
            moves.push(m);
        }
        // e/p pawn_captures
        let ep = board.en_passant();
        if ep.intersects(pawn_captures_e) {
            let from = ep.shift(&color.pawn_capture_east().opposite());
            let capture_square = ep.shift(&color.opposite().pawn_move());
            let m = Move {
                from,
                to: ep,
                mover: Piece::Pawn,
                capture: Piece::Pawn,
                ep: capture_square,
                ..Default::default()
            };
            moves.push(m);
        }
        if ep.intersects(pawn_captures_w) {
            let from = ep.shift(&color.pawn_capture_west().opposite());
            let capture_square = ep.shift(&color.opposite().pawn_move());
            let m = Move {
                from,
                to: ep,
                mover: Piece::Pawn,
                capture: Piece::Pawn,
                ep: capture_square,
                ..Default::default()
            };
            moves.push(m);
        }

        // pawn capture-promos
        for to in (pawn_captures_e & them & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_east().opposite());
            let capture = board.piece_at(to);
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                // MoveEnum::PromoCapture { to, from, promo, capture });
                let m = Move { from, to, mover: Piece::Pawn, capture, promo, ..Default::default() };
                moves.push(m);
            }
        }
        for to in (pawn_captures_w & them & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_west().opposite());
            let capture = board.piece_at(to);
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                // MoveEnum::PromoCapture { to, from, promo, capture });
                let m = Move { from, to, mover: Piece::Pawn, capture, promo, ..Default::default() };
                moves.push(m);
            }
        }
        // knights
        for from in (board.knights() & us).iter() {
            let attacks = attack_gen.knight_attacks(from.first_square()) & !us;
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move {
                        from,
                        to,
                        mover: Piece::Knight,
                        capture: board.piece_at(to),
                        ..Default::default()
                    };
                    // MoveEnum::Capture { to, from, mover: Piece::Knight, capture: board.piece_at(to) }
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Knight, ..Default::default() };
                    // MoveEnum::Quiet { to, from, mover: Piece::Knight }
                    moves.push(m);
                }
            }
        }
        // sliders
        for from in (board.bishops() & us).iter() {
            let attacks = !us & attack_gen.bishop_attacks(occupied, from.first_square());
            // println!("{}\n{}\n{}\n", from, attacks, occupied);
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move {
                        from,
                        to,
                        mover: Piece::Bishop,
                        capture: board.piece_at(to),
                        ..Default::default()
                    };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Bishop, ..Default::default() };
                    moves.push(m);
                }
            }
        }
        for from in (board.rooks() & us).iter() {
            let attacks = !us & attack_gen.rook_attacks(occupied, from.first_square());
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move {
                        from,
                        to,
                        mover: Piece::Rook,
                        capture: board.piece_at(to),
                        ..Default::default()
                    };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Rook, ..Default::default() };
                    moves.push(m);
                }
            }
        }
        for from in (board.queens() & us).iter() {
            let attacks = !us
                & (attack_gen.rook_attacks(occupied, from.first_square())
                    | attack_gen.bishop_attacks(occupied, from.first_square()));
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move {
                        from,
                        to,
                        mover: Piece::Queen,
                        capture: board.piece_at(to),
                        ..Default::default()
                    };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::Queen, ..Default::default() };
                    moves.push(m);
                }
            }
        }
        for from in (board.kings() & us).iter() {
            let attacks = !us & attack_gen.king_attacks(from.first_square());
            for to in attacks.iter() {
                if them.contains(to) {
                    let m = Move {
                        from,
                        to,
                        mover: Piece::King,
                        capture: board.piece_at(to),
                        ..Default::default()
                    };
                    moves.push(m);
                } else {
                    let m = Move { from, to, mover: Piece::King, ..Default::default() };
                    moves.push(m);
                }
            }
        }
        // castling
        // check castling rights (cheap)
        // check there is a king (for testing board setups)
        // check king not in check
        // side = +/-2
        // check king+1 and king+2 for being clear on kings side
        // check king-1, king-2, king-3 clear on queens
        // check that king +/- 1 and king +/- 2 isnt in check
        // addMoveEnum King +/- 2, add rook -2/+3
        // castling rights
        let king = board.kings() & us;
        let rights = board.castling();

        let right = CastlingRights::king_side_right(color);
        if rights.contains(right)
            && !CastlingRights::king_side_squares(color).intersects(occupied)
            && !king.is_empty()
        {
            let rook_to = king.shift(&Dir::E);
            let king_to = rook_to.shift(&Dir::E);
            let king_moves = king | rook_to | king_to;
            if attacked_by(king_moves, occupied, board, color.opposite()).is_empty() {
                // let rook_from = Bitboard::FILE_A & color.back_rank;
                // let m = MoveEnum::Castle { king_dest, king_from: king, rook_dest, rook_from, right };
                let m = Move {
                    from: king,
                    to: king_to,
                    mover: Piece::King,
                    castle_side: right,
                    ..Default::default()
                };
                moves.push(m);
            }
        }

        let right = CastlingRights::queen_side_right(color);
        if rights.contains(right)
            && !CastlingRights::queen_side_squares(color).intersects(occupied)
            && !king.is_empty()
        {
            let rook_to = king.shift(&Dir::W);
            let king_to = rook_to.shift(&Dir::W);
            let king_moves = king | rook_to | king_to;
            if attacked_by(king_moves, occupied, board, color.opposite()).is_empty() {
                // let rook_from = Bitboard::FILE_H & color.back_rank;
                // let m = MoveEnum::Castle { king_dest, king_from: king, rook_dest, rook_from, right };
                let m = Move {
                    from: king,
                    to: king_to,
                    mover: Piece::King,
                    castle_side: right,
                    ..Default::default()
                };
                moves.push(m);
            }
        }
        moves
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::board::*;
    use crate::globals::constants::*;
    extern crate env_logger;
    use crate::movelist::MoveValidator;

    fn _init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[test]
    fn test_pseudo_legal_moves() -> Result<(), String> {
        let mut buf = Board::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        buf.set(a2, ".")?;
        buf.set(d2, ".")?;
        buf.set(d4, "P")?;
        let board = buf.as_board();
        let moves = board.pseudo_legal_moves();
        assert_eq!(moves.len(), 32);
        // println!("{}\n{:#?}", board, moves);
        //assert_eq!(format!("{:#?}", moves), "vec![]");
        Ok(())
    }

    #[test]
    fn pawn_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/P7/8 w - - 0 0 id 'lone P'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        println!("{}\n{:#?}", board, moves);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4");

        let board = Board::parse_fen("8/p7/8/8/8/8/8/8 b - - 0 0 id 'lone P flipped'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a7a5, a7a6");

        let board = Board::parse_fen("8/8/8/8/8/p7/P7/8 w - - 0 0 id PP").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "");

        let board = Board::parse_fen("8/8/8/8/8/8/PPP5/8 w - - 0 0 id PPP").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4, b2b3, b2b4, c2c3, c2c4");

        let board =
            Board::parse_fen("8/8/8/8/8/p1p5/1P6/8 w - - 0 0 id P 'capture white'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "b2a3, b2b3, b2b4, b2c3");

        let board =
            Board::parse_fen("8/1p6/P1P5/8/8/8/1P6/8 b - - 0 0 id 'P capture black'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "b7a6, b7b5, b7b6, b7c6");

        let board = Board::parse_fen("8/8/p6p/1N6/8/8/8/8 b - - 0 0 id 'PxN black'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a6a5, a6b5, h6h5");
    }

    #[test]
    fn pawn_en_passant() {
        let board = Board::parse_fen("8/8/8/pP6/8/8/8/8 w - a6 0 0 id en-passant-#1").unwrap().as_board();
        assert_eq!(board.en_passant(), a6);
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "b5a6, b5b6");
        let board = Board::parse_fen("8/8/8/PpP5/8/8/8/8 w - b6 0 0 id 'en passant #2'").unwrap().as_board();
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "a5a6, a5b6, c5b6, c5c6");
    }

    #[test]
    fn pawn_promotions() {
        let board = Board::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1b1, a1b2, a7a8b, a7a8n, a7a8q, a7a8r"
        );
    }

    #[test]
    fn rook_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/8/R7 w - - 0 0 id 'R'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1a7, a1a8, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board =
            Board::parse_fen("8/p7/P7/8/8/8/8/R7 w - - 0 0 id 'R with P blocking'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board =
            Board::parse_fen("8/p7/p7/8/8/8/8/R7 w - - 0 0 id 'R with p to capture'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );
    }

    #[test]
    fn knight_moves() {
        let board = Board::parse_fen("8/8/8/3N4/8/8/8/8 w - - 0 0 id 'N d5'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "d5b4, d5b6, d5c3, d5c7, d5e3, d5e7, d5f4, d5f6"
        );
    }

    #[test]
    fn bishop_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/8/B7 w - - 0 0 id 'B a1'").unwrap().as_board();
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "a1b2, a1c3, a1d4, a1e5, a1f6, a1g7, a1h8");

        let board = Board::parse_fen("8/8/8/8/8/8/1B6/8 w - - 0 0 id 'B b2'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "b2a1, b2a3, b2c1, b2c3, b2d4, b2e5, b2f6, b2g7, b2h8"
        );
    }

    #[test]
    fn moves_in_check() {
        let board = Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
            .unwrap()
            .as_board();
        assert!(board.en_passant().is_empty());
        // FIXME! assert b1.validate_move(Move.parse('e8e7'))
        // assert!(board.pseudo_legal_moves().contains(Move.parse("e8e7")));
    }

    #[test]
    fn test_is_in_check() {
        let fen = "r4r1k/p1ppqpb1/bn2pnp1/3PN2Q/1p2P3/2N4p/PPPBBPPP/2R1K2R b K - 3 2";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), true);

        let fen = "k7/8/8/8/8/8/7K/R7 w - - 0 0 id 'check #1'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), true, "\n{}", board);
        assert_eq!(board.is_in_check(Color::White), false);

        let fen = "8/k2P3R/8/8/8/8/8/7K w - - 0 0 id 'pawn blocks rook #1'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), false);

        let fen = "k7/8/8/8/8/8/7K/7B w - - 0 0 id 'check Bishop #2'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), true);

        let fen = "k7/8/2p5/8/8/8/7K/7B w - - 0 0 id 'check blocked bishop #3'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), false);

        let fen = "k7/7R/2p5/8/8/8/7K/7B w - - 0 0 id 'check blocked bishop #4'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), false);

        let fen = "k7/8/8/8/8/8/7K/7Q w - - 0 0 id 'check Queen #1'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), true);

        let fen = "k7/8/1N6/8/8/8/7K/8 w - - 0 0 id 'check N #5'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::Black), true);

        // pawn capture+promo attack
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q2/PPPBBPpP/1R3K1R w kq - 0 2";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::White), true, "\n{}", board);

        // checks by king
        let fen = "8/8/8/8/8/8/1k6/K7 w - - 0 0 id 'check by king!'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::White), true);
    }

    #[test]
    fn test_legal_move() -> Result<(), String> {
        // https://lichess.org/editor/8/8/8/8/8/8/6r1/7K
        let fen = "8/8/8/8/8/8/6r1/7K w - - 0 0 id 'rook+king'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.legal_moves().sort().to_string(), "h1g2");
        let mov_h1g2 = board.parse_uci_move("h1g2")?;
        assert_eq!(board.is_legal_move(&mov_h1g2), true);
        Ok(())
    }
}
