use crate::attacks::{BitboardAttacks, ClassicalBitboard};
use crate::bitboard::{Bitboard, Dir};
use crate::board::makemove::MoveMaker;
use crate::board::{Board, Color, Piece};
use crate::board::{Move, MoveList};
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

pub trait MoveValidator {
    fn validate_uci_move(&self, mv: &str) -> Result<Move, String>;
    fn validate_san_move(&self, mv: &str) -> Result<Move, String>;
}

impl MoveValidator for Board {
    fn validate_uci_move(&self, mv: &str) -> Result<Move, String> {
        // FIXME! *legal* moves
        let mut moves = self.pseudo_legal_moves();
        if let Some(pos) = moves.iter().position(|m| m.uci() == mv) {
            return Ok(moves.remove(pos));
        }
        Err(format!("Move {} is not legal", mv))
    }

    fn validate_san_move(&self, _mv: &str) -> Result<Move, String> {
        Err("Not implemented".into())
    }
}

pub trait MoveGen {
    fn is_in_check(&self, c: Color) -> bool;
    fn is_legal_move(&self, mv: &Move) -> bool;
    fn pseudo_legal_moves(&self) -> MoveList;
    fn legal_moves(&self) -> MoveList;
}

impl MoveGen for Board {
    fn is_in_check(&self, c: Color) -> bool {
        let board = &self;
        let us = board.color(c);
        let them = board.color(c.opposite());
        let king = board.kings() & us;
        debug_assert!(!king.is_empty(), "king ({}) not found {}", c, board );
        let occ = us | them;
        let king_color = c;
        !attacked_by(king, occ, board, king_color.opposite()).is_empty()
    }

    fn is_legal_move(&self, mv: &Move) -> bool {
        mv.is_castle || mv.is_drop || !self.make_move(mv).is_in_check(self.color_us())
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
            let mut from = to.shift(&color.pawn_move.opposite());
            let mut ep = Bitboard::EMPTY;
            if !pawns.contains(from) {
                // must have been double push
                ep = from;
                from = from.shift(&color.pawn_move.opposite());
            }
            // let m = MoveEnum::Push { to, from };
            let m = Move { from, to, ep, mover: Piece::Pawn, ..Default::default() };
            moves.push(m);
        }
        // pawn promos - only makes sense for single push
        for to in (pawn_push & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_move.opposite());
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
            let from = to.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(to);
            let m = Move { from, to, mover: Piece::Pawn, capture, ..Default::default() };
            // MoveEnum::Capture { to, from, mover: Piece::Pawn, capture });
            moves.push(m);
        }
        for to in (pawn_captures_w & them & !Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_west.opposite());
            let capture = board.piece_at(to);
            let m = Move { from, to, mover: Piece::Pawn, capture, ..Default::default() };
            // MoveEnum::Capture { to, from, mover: Piece::Pawn, capture };
            moves.push(m);
        }
        // e/p pawn_captures
        let ep = board.en_passant();
        if ep.intersects(pawn_captures_e) {
            let from = ep.shift(&color.pawn_capture_east.opposite());
            let capture_square = ep.shift(&color.opposite().pawn_move);
            let m = Move { from, to: ep, mover: Piece::Pawn, capture: Piece::Pawn, ep: capture_square, ..Default::default() };
            moves.push(m);
        }
        if ep.intersects(pawn_captures_w) {
            let from = ep.shift(&color.pawn_capture_west.opposite());
            let capture_square = ep.shift(&color.opposite().pawn_move);
            let m = Move { from, to: ep, mover: Piece::Pawn, capture: Piece::Pawn, ep: capture_square, ..Default::default() };
            moves.push(m);
        }

        // pawn capture-promos
        for to in (pawn_captures_e & them & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(to);
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                // MoveEnum::PromoCapture { to, from, promo, capture });
                let m = Move { from, to, mover: Piece::Pawn, capture, promo, ..Default::default() };
                moves.push(m);
            }
        }
        for to in (pawn_captures_w & them & Bitboard::PROMO_RANKS).iter() {
            let from = to.shift(&color.pawn_capture_west.opposite());
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
                    let m = Move { from, to, mover: Piece::Knight, capture: board.piece_at(to), ..Default::default() };
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
                    let m = Move { from, to, mover: Piece::Bishop, capture: board.piece_at(to), ..Default::default() };
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
                    let m = Move { from, to, mover: Piece::Rook, capture: board.piece_at(to), ..Default::default() };
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
                    let m = Move { from, to, mover: Piece::Queen, capture: board.piece_at(to), ..Default::default() };
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
                    let m = Move { from, to, mover: Piece::King, capture: board.piece_at(to), ..Default::default() };
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

        let right = color.castle_rights_king;
        if rights.contains(right) && !color.kingside_castle_sqs.intersects(occupied) && !king.is_empty() {
            let rook_to = king.shift(&Dir::E);
            let king_to = rook_to.shift(&Dir::E);
            let king_moves = king | rook_to | king_to;
            if attacked_by(king_moves, occupied, board, color.opposite()).is_empty() {
                // let rook_from = Bitboard::FILE_A & color.back_rank;
                // let m = MoveEnum::Castle { king_dest, king_from: king, rook_dest, rook_from, right };
                let m = Move { from: king, to: king_to, mover: Piece::King, is_castle: true, ..Default::default() };
                moves.push(m);
            }
        }

        let right = color.castle_rights_queen;
        if rights.contains(right) && !color.queenside_castle_sqs.intersects(occupied) && !king.is_empty() {
            let rook_to = king.shift(&Dir::W);
            let king_to = rook_to.shift(&Dir::W);
            let king_moves = king | rook_to | king_to;
                if attacked_by(king_moves, occupied, board, color.opposite()).is_empty() {
                    // let rook_from = Bitboard::FILE_H & color.back_rank;
                // let m = MoveEnum::Castle { king_dest, king_from: king, rook_dest, rook_from, right };
                let m = Move { from: king, to: king_to, mover: Piece::King, is_castle: true, ..Default::default() };
                moves.push(m);
            }
        }
        moves
    }
}


pub fn perft( board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    } 
    else {
        let moves = board.legal_moves();
        let mut count = 0u64;
        for m in moves.iter() {
            let res = perft(&board.make_move(m), depth - 1);                
            // if depth == 2 {
            //     println!("Move: {} perft: {} fen {}", m, res, board.to_fen());
            // }
            count += res;
        }           
        count
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::board::catalog::*;
    use crate::board::*;
    use crate::globals::constants::*;
    extern crate env_logger;

    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[test]
    fn pseudo_legal_moves() -> Result<(), String> {
        let mut buf = BoardBuf::parse_pieces("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
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
    fn test_tricky() {
        let board = BoardBuf::parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap().as_board();
        let board2 = BoardBuf::parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/5Q1p/PPPBBPPP/RN2K2R b KQkq - 1 1").unwrap().as_board();  // c3b1
        let board3 = BoardBuf::parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/4P3/1p3Q1p/PPPBBPPP/RN2K2R w KQkq - 0 1").unwrap().as_board();  // c3b1
        println!("{}", board3.legal_moves().sort().to_string() );
        let count = perft(&board2, 2);
        println!("{}", count);
        // assert_eq!(board2.legal_moves().to_string(), "");
    }
        // let mov = board.validate_uci_move("e2e4").unwrap();
        // let board2 = board.make_move(&mov);
        // assert_eq!(board2.to_fen(), "8/2p5/3p4/KP5r/1R2Pp1k/8/6P1/8 b - e3 0 1");
        // // assert_eq!(board2.legal_moves().to_string(), "");
        // let mov2 = board2.validate_uci_move("f4e3").unwrap();
        // let board3 = board2.make_move(&mov2);
        // assert_eq!(board3.to_fen(), "8/2p5/3p4/KP5r/1R5k/4p3/6P1/8 w - - 0 2");
        // // assert_eq!(boar2.legal_moves().to_string(), "");

        // // assert!( board.castling().contains(CastlingRights::WHITE_QUEEN) ); 
        // depth:3 count:97903 expected:97862 fen: r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1
        // ******depth:3 count:97903 expected:97862 fen: r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1
        // depth:4 count:4087391 expected:4085603 fen: r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1
        // ******depth:4 count:4087391 expected:4085603 fen: r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1
        // test board::movegen::tests::test_perft ... test board::movegen::tests::test_perft has been running for over 60 seconds
        // depth:5 count:193902441 expected:119060324 fen: r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1
        // ******depth:5 count:193902441 expected:119060324 fen: r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1

        // [b1a3, b1c3, e1f1, e1d1, h1g1, h1f1, d2e3, d2f4, d2g5, d2h6,
        //  d2c3, d2b4, d2a5, d2c1, e2d3, e2c4, e2b5, e2a6, e2f1, e2d1,
        //   f3g4, f3h5, f3f4, f3f5, f3f6, f3g3, f3h3, f3e3, f3d3, f3c3,
        //    f3b3, e5d7, e5f7, e5g6, e5c6, e5d3, e5g4, e5c4, a2a3, a2b3,
        //     c2b3, c2c3, g2g3, g2h3, a2a4, c2c4, g2g4, d5d6, d5e6, e1g1]

            /// PYTHON
        // [a2a3, a2a4, a2b3, b1a3, b1c3, c2b3, c2c3, c2c4, d2a5, d2b4, 
        // d2c1, d2c3, d2e3, d2f4, d2g5, d2h6, d5d6, d5e6, e1d1, e1f1,
        //  e1g1, e2a6, e2b5, e2c4, e2d1, e2d3, e2f1, e5c4, e5c6, e5d3,
        //   e5d7, e5f7, e5g4, e5g6, f3b3, f3c3, f3d3, f3e3, f3f4, f3f5,
        //    f3f6, f3g3, f3g4, f3h3, f3h5, g2g3, g2g4, g2h3, h1f1, h1g1]

            // a2a3, a2a4, a2b3, b1a3, b1c3, c2b3, c2c3, c2c4, d2a5, d2b4, 
            // d2c1, d2c3, d2e3, d2f4, d2g5, d2h6, d5d6, d5e6, e1c1, e1d1,
            //  e1f1, e1g1, e2a6, e2b5, e2c4, e2d1, e2d3, e2f1, e5c4, e5c6,
            //   e5d3, e5d7, e5f7, e5g4, e5g6, f3b3, f3c3, f3d3, f3e3, f3f4, f3f5, f3f6, f3g3, f3g4, f3h3, f3h5, g2g3, g2g4, g2h3, h1f1, h1g1

        // Perft:B(depth=2):Div: b4b3 50
        // Div: g6g5 48
        // Div: c7c6 50
        // Div: d7d6 48
        // Div: c7c5 50
        // Div: h3g2 47
        // Div: e6d5 48
        // Div: b6a4 48
        // Div: b6c4 46
        // Div: b6d5 48
        // Div: b6c8 49
        // Div: f6e4 51
        // Div: f6g4 48
        // Div: f6d5 49
        // Div: f6h5 50
        // Div: f6h7 50
        // Div: f6g8 50
        // Div: a6e2 42
        // Div: a6d3 44
        // Div: a6c4 46
        // Div: a6b5 48
        // Div: a6b7 49
        // Div: a6c8 49
        // Div: g7h6 49
        // Div: g7f8 49
        // Div: a8b8 49
        // Div: a8c8 49
        // Div: a8d8 49
        // Div: h8h4 49
        // Div: h8h5 49
        // Div: h8h6 49
        // Div: h8h7 49
        // Div: h8f8 49
        // Div: h8g8 49
        // Div: e7c5 49
        // Div: e7d6 48
        // Div: e7d8 49
        // Div: e7f8 49
        // Div: e8d8 49
        // Div: e8f8 49
        // Div: e8c8 49
        // Div: e8g8 49
        // result=2038 

//         running 1 test
// Move: a2a3 perft: 2186
// Move: b2b3 perft: 1964
// Move: g2g3 perft: 1882
// Move: a2a4 perft: 2149
// Move: g2g4 perft: 1843
// Move: d5d6 perft: 1991
// Move: g2h3 perft: 1970
// Move: d5e6 perft: 2241
// ****** Move: c3b1 perft: 2079
// Move: c3d1 perft: 2040
// Move: c3a4 perft: 2203
// Move: c3b5 perft: 2138
// Move: e5d3 perft: 1803
// Move: e5c4 perft: 1880
// Move: e5g4 perft: 1878
// Move: e5c6 perft: 2027
// Move: e5g6 perft: 1997
// Move: e5d7 perft: 2124
// Move: e5f7 perft: 2080
// Move: d2c1 perft: 1963
// Move: d2e3 perft: 2136
// Move: d2f4 perft: 2000
// Move: d2g5 perft: 2134
// Move: d2h6 perft: 2019
// Move: e2d1 perft: 1733
// Move: e2f1 perft: 2060
// Move: e2d3 perft: 2050
// Move: e2c4 perft: 2082
// Move: e2b5 perft: 2057
// Move: e2a6 perft: 1907
// Move: a1b1 perft: 1969
// Move: a1c1 perft: 1968
// Move: a1d1 perft: 1885
// Move: h1f1 perft: 1929
// Move: h1g1 perft: 2013
// Move: f3d3 perft: 2005
// Move: f3e3 perft: 2174
// Move: f3g3 perft: 2214
// Move: f3h3 perft: 2360
// Move: f3f4 perft: 2132
// Move: f3g4 perft: 2169
// Move: f3f5 perft: 2396
// Move: f3h5 perft: 2267
// Move: f3f6 perft: 2111
// Move: e1d1 perft: 1894
// Move: e1f1 perft: 1855
// Move: e1g1 perft: 2059
// Move: e1c1 perft: 1887
// 97903



        

  


    #[test]
    fn test_perft() {
        for (board, perfts) in Catalog::perfts() {
            for (depth, expected) in perfts.iter().enumerate() {
                let count = perft(&board, depth as u32);
                println!("depth:{} count:{} expected:{} fen: {}", depth, count, expected, board.to_fen());
                if &count != expected {
                    println!("******depth:{} count:{} expected:{} fen: {}", depth, count, expected, board.to_fen());
                }
                // assert_eq!(&count, expected, "fen: {}", board.to_fen());
            }
        };
    }

    #[test]
    fn pawn_moves() {
        let board = BoardBuf::parse_fen("8/8/8/8/8/8/P7/8 w - - 0 0 id 'lone P'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        println!("{}\n{:#?}", board, moves);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4");

        let board = BoardBuf::parse_fen("8/p7/8/8/8/8/8/8 b - - 0 0 id 'lone P flipped'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a7a5, a7a6");

        let board = BoardBuf::parse_fen("8/8/8/8/8/p7/P7/8 w - - 0 0 id PP").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "");

        let board = BoardBuf::parse_fen("8/8/8/8/8/8/PPP5/8 w - - 0 0 id PPP").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4, b2b3, b2b4, c2c3, c2c4");

        let board = BoardBuf::parse_fen("8/8/8/8/8/p1p5/1P6/8 w - - 0 0 id P 'capture white'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "b2a3, b2b3, b2b4, b2c3");

        let board = BoardBuf::parse_fen("8/1p6/P1P5/8/8/8/1P6/8 b - - 0 0 id 'P capture black'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "b7a6, b7b5, b7b6, b7c6");

        let board = BoardBuf::parse_fen("8/8/p6p/1N6/8/8/8/8 b - - 0 0 id 'PxN black'").unwrap().as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a6a5, a6b5, h6h5");
    }

    #[test]
    fn pawn_en_passant() {
        let board = BoardBuf::parse_fen("8/8/8/pP6/8/8/8/8 w - a6 0 0 id en-passant-#1").unwrap().as_board();
        assert_eq!(board.en_passant(), a6);
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "b5a6, b5b6");
        let board = BoardBuf::parse_fen("8/8/8/PpP5/8/8/8/8 w - b6 0 0 id 'en passant #2'").unwrap().as_board();
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "a5a6, a5b6, c5b6, c5c6");
    }

    #[test]
    fn pawn_promotions() {
        let board = BoardBuf::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'").unwrap().as_board();
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "a1a2, a1b1, a1b2, a7a8b, a7a8n, a7a8q, a7a8r");
    }

    #[test]
    fn rook_moves() {
        let board = BoardBuf::parse_fen("8/8/8/8/8/8/8/R7 w - - 0 0 id 'R'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1a7, a1a8, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board = BoardBuf::parse_fen("8/p7/P7/8/8/8/8/R7 w - - 0 0 id 'R with P blocking'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board = BoardBuf::parse_fen("8/p7/p7/8/8/8/8/R7 w - - 0 0 id 'R with p to capture'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );
    }

    #[test]
    fn knight_moves() {
        let board = BoardBuf::parse_fen("8/8/8/3N4/8/8/8/8 w - - 0 0 id 'N d5'").unwrap().as_board();
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "d5b4, d5b6, d5c3, d5c7, d5e3, d5e7, d5f4, d5f6");
    }

    #[test]
    fn bishop_moves() {
        let board = BoardBuf::parse_fen("8/8/8/8/8/8/8/B7 w - - 0 0 id 'B a1'").unwrap().as_board();
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "a1b2, a1c3, a1d4, a1e5, a1f6, a1g7, a1h8");

        let board = BoardBuf::parse_fen("8/8/8/8/8/8/1B6/8 w - - 0 0 id 'B b2'").unwrap().as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "b2a1, b2a3, b2c1, b2c3, b2d4, b2e5, b2f6, b2g7, b2h8"
        );
    }

    #[test]
    fn moves_in_check() {
        let board =
            BoardBuf::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2").unwrap().as_board();
        assert!(board.en_passant().is_empty());
        // FIXME! assert b1.validate_move(Move.parse('e8e7'))
        // assert!(board.pseudo_legal_moves().contains(Move.parse("e8e7")));
    }


    #[test]
    fn test_is_in_check() {
        let fen = "r4r1k/p1ppqpb1/bn2pnp1/3PN2Q/1p2P3/2N4p/PPPBBPPP/2R1K2R b K - 3 2";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), true);

        let fen = "k7/8/8/8/8/8/7K/R7 w - - 0 0 id 'check #1'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), true, "\n{}", board);
        assert_eq!(board.is_in_check(Color::WHITE), false);

        let fen = "8/k2P3R/8/8/8/8/8/7K w - - 0 0 id 'pawn blocks rook #1'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), false);

        let fen = "k7/8/8/8/8/8/7K/7B w - - 0 0 id 'check Bishop #2'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), true);

        let fen = "k7/8/2p5/8/8/8/7K/7B w - - 0 0 id 'check blocked bishop #3'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), false);

        let fen = "k7/7R/2p5/8/8/8/7K/7B w - - 0 0 id 'check blocked bishop #4'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), false);

        let fen = "k7/8/8/8/8/8/7K/7Q w - - 0 0 id 'check Queen #1'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), true);

        let fen = "k7/8/1N6/8/8/8/7K/8 w - - 0 0 id 'check N #5'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::BLACK), true);

        // pawn capture+promo attack
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q2/PPPBBPpP/1R3K1R w kq - 0 2";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::WHITE), true, "\n{}", board);

        // checks by king
        let fen = "8/8/8/8/8/8/1k6/K7 w - - 0 0 id 'check by king!'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.is_in_check(Color::WHITE), true);
    }

    #[test]
    fn test_legal_move() -> Result<(), String> {
        // https://lichess.org/editor/8/8/8/8/8/8/6r1/7K
        let fen = "8/8/8/8/8/8/6r1/7K w - - 0 0 id 'rook+king'";
        let board = BoardBuf::parse_fen(fen).unwrap().as_board();
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "h1g1, h1g2, h1h2");
        let mov_h1g1 = board.validate_uci_move("h1g1")?;
        let mov_h1g2 = board.validate_uci_move("h1g2")?;
        let mov_h1h2 = board.validate_uci_move("h1h2")?;
        assert_eq!(board.is_legal_move(&mov_h1g1), false);
        assert_eq!(board.is_legal_move(&mov_h1g2), true);
        assert_eq!(board.is_legal_move(&mov_h1h2), false);
        Ok(())
    }
}
