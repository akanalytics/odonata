use crate::bitboard::attacks::{BitboardAttacks, BitboardDefault};
use crate::bitboard::bitboard::{Bitboard};
use crate::board::makemove::MoveMaker;
use crate::board::rules::Rules;
use crate::board::Board;
use crate::globals::counts;
use crate::movelist::{Move, MoveList};
use crate::types::{Color, Piece};



pub fn pinned(b: &Board, us: Color, occ: Bitboard ) -> Bitboard {
    let kings = b.kings() & b.color(us);
    if kings.is_ empty() { 
        return Bitboard.empty()
    };

    let xray_checkers = threats_to(b, us, Bitboard::EMPTY);
    let pinned =  Bitboard::empty();
    for c in xray_checkers.squares() {
        let ray = ray(c, king);
        if (ray & us).popcount() == 1 {
            pinned |= ray & us;
        }
    }
    pinned
}


pub fn threats_to(board: &Board, us: Color, occ: Bitboard) -> Bitboard {
    let opponent = us.opposite();
    let pawns = board.pawns() & board.color(opponent);
    let knights = board.knights() & board.color(opponent);
    let bishops = board.bishops() & board.color(opponent);
    let rooks = board.rooks() & board.color(opponent);
    let queens = board.queens() & board.color(opponent);
    let kings = board.kings() & board.color(opponent);

    let attack_gen = BitboardDefault::default();
    let (east, west) = attack_gen.pawn_attacks(pawns, opponent);
    let mut threats = east | west;

    for each in knights.iter() {
        let sq = each.square();
        threats |= attack_gen.knight_attacks(sq);
    }
    for each in (bishops | queens).iter() {
        let sq = each.square();
        threats |= attack_gen.bishop_attacks(occ, sq);
    }

    for each in (rooks | queens).iter() {
        let sq = each.square();
        threats |= attack_gen.rook_attacks(occ, sq);
    }

    for each in kings.iter() {
        let sq = each.square();
        threats |= attack_gen.king_attacks(sq);
    }
    threats
}

pub fn attacked_by(targets: Bitboard, occ: Bitboard, board: &Board) -> Bitboard {
    if targets.is_empty() {
        return Bitboard::empty();
    }
    let pawns = board.pawns();
    let knights = board.knights();
    let bishops = board.bishops();
    let rooks = board.rooks();
    let queens = board.queens();
    let kings = board.kings();

    let attack_gen = BitboardDefault::default();
    let white = attack_gen.pawn_attackers(targets, Color::White) & pawns & board.white();
    let black = attack_gen.pawn_attackers(targets, Color::Black) & pawns & board.black();
    let mut attackers = white | black;

    for each in targets.iter() {
        let sq = each.square();
        attackers |= attack_gen.knight_attacks(sq) & knights
            | attack_gen.king_attacks(sq) & kings
            | attack_gen.bishop_attacks(occ, sq) & (bishops | queens)
            | attack_gen.rook_attacks(occ, sq) & (rooks | queens);
    }
    debug!("{}target\n{}attackers\n{}", board, targets, attackers);

    attackers
}

// fn attacked_by_colorX(targets: Bitboard, occ: Bitboard, board: &Board, opponent: Color) -> Bitboard {
//     let pawns = board.pawns() & board.color(opponent);
//     let knights = board.knights() & board.color(opponent);
//     let bishops = board.bishops() & board.color(opponent);
//     let rooks = board.rooks() & board.color(opponent);
//     let queens = board.queens() & board.color(opponent);
//     let kings = board.kings() & board.color(opponent);

//     let attack_gen = global_classical_bitboard();
//     let (east, west) = attack_gen.pawn_attacks(pawns, opponent);
//     let mut attackers = (east | west) & targets;

//     for each in targets.iter() {
//         let sq = each.first_square();
//         attackers |= attack_gen.knight_attacks(sq) & knights
//             | attack_gen.king_attacks(sq) & kings
//             | attack_gen.bishop_attacks(occ, sq) & (bishops | queens)
//             | attack_gen.rook_attacks(occ, sq) & (rooks | queens);
//         // TODO: en passant!!
//     }
//     debug!("opponent:{}\n{}target\n{}attackers\n{}", opponent, board, targets, attackers);

//     attackers
// }

trait MoveGen {}

// fn is_in_check(&self, king_color: Color) -> bool;
// fn will_check_them(&self, mv: &Move) -> bool;
// fn is_legal_move(&self, mv: &Move) -> bool;
// fn pseudo_legal_moves(&self) -> MoveList;
// fn legal_moves(&self) -> impl Iterator<Item=Move>;
// fn legal_capture_moves(&self) -> MoveList;
// fn threats_to(&self, c: Color) -> Bitboard;

// no king on board => no attackers
pub fn calc_checkers_of(board: &Board, king_color: Color) -> Bitboard {
    let us = board.color(king_color);
    let them = board.color(king_color.opposite());
    let our_king = board.kings() & us;
    // debug_assert!(!our_king.is_empty(), "king ({}) not found {}", king_color, board);
    let occ = us | them;
    attacked_by(our_king, occ, board) & them
}

impl Board {
    pub fn attacked_by(&self, targets: Bitboard) -> Bitboard {
        attacked_by(targets, self.black() | self.white(), self)
    }

    pub fn will_check_them(&self, mv: &Move) -> bool {
        debug_assert!(self.is_legal_move(mv));
        let their_king_color = self.color_them();
        self.make_move(mv).is_in_check(their_king_color)
    }

    pub fn checkers_of(&self, king_color: Color) -> Bitboard {
        let mut ch = self.checkers_of[king_color].get();
        if ch == Bitboard::niche() {
            ch = calc_checkers_of(self, king_color);
            self.checkers_of[king_color].set(ch);
        }
        ch
    }

    pub fn threats_to(&self, c: Color) -> Bitboard {
        let mut th = self.threats_to[c].get();
        if th == Bitboard::niche() {
            th = threats_to(self, c, self.occupied());
            self.threats_to[c].set(th);
        }
        th
    }

    pub fn has_legal_moves(&self) -> bool {
        self.pseudo_legal_moves().iter().rev().any(|m| self.is_legal_move(m))
    }

    // fn is_in_check(&self, c: Color) -> bool {
    //     let king = self.kings() & self.color(c);
    //     king.intersects(self.threats_to(c))
    // }

    /// called with is_in_check( board.turn() ) to see if currently in check
    pub fn is_in_check(&self, king_color: Color) -> bool {
        let them = self.color(king_color.opposite());
        self.checkers_of(king_color).intersects(them)
    }


    // the move is pseudo legal
    pub fn is_legal_move(&self, mv: &Move) -> bool {
        if mv.is_known_legal() {
            return true;
        }
        // castling and kings moves already done above
        let mut us = self.us();
        let kings = self.kings() & us;
        if kings.is_empty() {
            return true;  // a test position without king on the board - we allow
        }
        let sq = kings.square();
        
        // idea - lightweight make_move - no hash - just enough to check rays of sliders 
        let mut them = self.them();
        let from_to_bits = mv.from().as_bb() | mv.to().as_bb();
        us ^= from_to_bits;  // wont be a king move so sq still valid

        if mv.is_capture() {
            if mv.is_ep_capture() {
                // ep capture is like capture but with capture piece on *ep* square not *dest*
                them.remove(mv.ep().as_bb());
            } else {
                // regular capture
                them.remove(mv.to().as_bb());
            }   
        }
        // in (rough) order of computation cost / likelyhood - this code from "attacked_by" 
        // their pieces wont have moved, but they may have been taken 

        let attack_gen = BitboardDefault::default();
        let occ = us | them;
        if (attack_gen.rook_attacks(occ, sq) & (self.rooks() | self.queens()) & them).any() {
            return false;
        }        

        // for knight attacks, we must have been in check already
        // not true since 13/5/21
        // XX you cant have a discovered knight check, 
        // XX so the move needs to be a capture
        if (attack_gen.knight_attacks(sq) & self.knights() & them).any() {
            return false;
        }

        if (attack_gen.bishop_attacks(occ, sq) & (self.bishops() | self.queens() ) & them).any() {
            return false;
        }
        // not since 13/5. We do need to check, and hence do so
        // WRONG: no need to check enemy king or pawn, as we are looking for discovered checks  
        if (attack_gen.pawn_attackers(kings, self.color_them()) & self.pawns() & them).any() {
             return false;
        }

        true
    }





    pub fn legal_moves(&self) -> MoveList {
        counts::LEGAL_MOVE_COUNT.increment();
        let mut moves = MoveList::new();

        let b = self;
        Rules::pawn_captures_incl_promo(b, &mut moves);
        Rules::pawn_promos(b, &mut moves);
        Rules::pawn_push(b, &mut moves);
        // Rules::pawn_capture_promos(b, &mut moves);
        Rules::non_pawn(Piece::Knight, b, &mut moves);
        Rules::non_pawn(Piece::Bishop, b, &mut moves);
        Rules::non_pawn(Piece::Rook, b, &mut moves);
        Rules::non_pawn(Piece::Queen,b,  &mut moves);
        // Rules::non_pawn(Piece::King,b,  &mut moves);
        Rules::king_legal(b, &mut moves);
        Rules::castles(b, &mut moves);
        moves.retain(|m| self.is_legal_move(m));
        moves
    }



    pub fn legal_capture_moves(&self) -> MoveList {
        let mut moves = self.legal_moves();
        moves.retain(|m| m.is_capture() || m.is_promo());
        moves
    }

    // fn king_pseudo_legals_to(&self, to: Bitboard, moves: &mut MoveList) {
    //     let attack_gen = BitboardDefault::default();
    //     let board = &self;
    //     for from_sq in (board.kings() & board.us()).squares() {
    //         let attacks = !board.us() & attack_gen.king_attacks(from_sq) & to;
    //         for to in attacks.squares() {
    //             if to.is_in(board.them()) {
    //                 let m = Move {
    //                     from: from_sq,
    //                     to,
    //                     mover: Piece::King,
    //                     capture: board.piece_at(to.as_bb()),
    //                     ..Default::default()
    //                 };
    //                 moves.push(m);
    //             } else {
    //                 let m = Move {
    //                     from: from_sq,
    //                     to,
    //                     mover: Piece::King,
    //                     ..Default::default()
    //                 };
    //                 moves.push(m);
    //             }
    //         }
    //     }
    // }


    pub fn pseudo_legal_moves(&self) -> MoveList {
        let mut moves = MoveList::new();

        let b = self;
        Rules::pawn_captures_incl_promo(b, &mut moves);
        Rules::pawn_promos(b, &mut moves);
        Rules::pawn_push(b, &mut moves);
        // Rules::pawn_capture_promos(b, &mut moves);
        Rules::non_pawn(Piece::Knight, b, &mut moves);
        Rules::non_pawn(Piece::Bishop, b, &mut moves);
        Rules::non_pawn(Piece::Rook, b, &mut moves);
        Rules::non_pawn(Piece::Queen,b,  &mut moves);
        // Rules::non_pawn(Piece::King,b,  &mut moves);
        Rules::king_legal(b, &mut moves);
        Rules::castles(b, &mut moves);
        moves
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::board::*;
    use crate::catalog::*;
    use crate::globals::constants::*;
    extern crate env_logger;
    use crate::movelist::MoveValidator;

    fn _init() {
        // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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
        let board = Board::parse_fen("8/8/8/8/8/8/P7/8 w - - 0 0 id 'lone P'")
            .unwrap()
            .as_board();
        let mut moves = board.pseudo_legal_moves();
        println!("{}\n{:#?}", board, moves);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4");

        let board = Board::parse_fen("8/p7/8/8/8/8/8/8 b - - 0 0 id 'lone P flipped'")
            .unwrap()
            .as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a7a5, a7a6");

        let board = Board::parse_fen("8/8/8/8/8/p7/P7/8 w - - 0 0 id PP")
            .unwrap()
            .as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "");

        let board = Board::parse_fen("8/8/8/8/8/8/PPP5/8 w - - 0 0 id PPP")
            .unwrap()
            .as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4, b2b3, b2b4, c2c3, c2c4");

        let board = Board::parse_fen("8/8/8/8/8/p1p5/1P6/8 w - - 0 0 id P 'capture white'")
            .unwrap()
            .as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "b2a3, b2b3, b2b4, b2c3");

        let board = Board::parse_fen("8/1p6/P1P5/8/8/8/1P6/8 b - - 0 0 id 'P capture black'")
            .unwrap()
            .as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "b7a6, b7b5, b7b6, b7c6");

        let board = Board::parse_fen("8/8/p6p/1N6/8/8/8/8 b - - 0 0 id 'PxN black'")
            .unwrap()
            .as_board();
        let mut moves = board.pseudo_legal_moves();
        assert_eq!(moves.sort().to_string(), "a6a5, a6b5, h6h5");
    }

    #[test]
    fn pawn_en_passant() {
        let board = Board::parse_fen("8/8/8/pP6/8/8/8/8 w - a6 0 0 id en-passant-#1")
            .unwrap()
            .as_board();
        assert_eq!(board.en_passant(), a6);
        assert_eq!(board.pseudo_legal_moves().sort().to_string(), "b5a6, b5b6");
        let board = Board::parse_fen("8/8/8/PpP5/8/8/8/8 w - b6 0 0 id 'en passant #2'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a5a6, a5b6, c5b6, c5c6"
        );
    }

    #[test]
    fn pawn_promotions() {
        let board = Board::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1b1, a1b2, a7a8b, a7a8n, a7a8q, a7a8r"
        );
    }

    #[test]
    fn rook_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/8/R7 w - - 0 0 id 'R'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1a7, a1a8, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board = Board::parse_fen("8/p7/P7/8/8/8/8/R7 w - - 0 0 id 'R with P blocking'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board = Board::parse_fen("8/p7/p7/8/8/8/8/R7 w - - 0 0 id 'R with p to capture'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );
    }

    #[test]
    fn knight_moves() {
        let board = Board::parse_fen("8/8/8/3N4/8/8/8/8 w - - 0 0 id 'N d5'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "d5b4, d5b6, d5c3, d5c7, d5e3, d5e7, d5f4, d5f6"
        );
    }

    #[test]
    fn bishop_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/8/B7 w - - 0 0 id 'B a1'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "a1b2, a1c3, a1d4, a1e5, a1f6, a1g7, a1h8"
        );

        let board = Board::parse_fen("8/8/8/8/8/8/1B6/8 w - - 0 0 id 'B b2'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.pseudo_legal_moves().sort().to_string(),
            "b2a1, b2a3, b2c1, b2c3, b2d4, b2e5, b2f6, b2g7, b2h8"
        );
    }

    #[test]
    fn test_threats() {
        let board = Board::parse_fen("k5r1/3q1p2/4b2r/1n6/6pp/b2N3n/8/K1QR4 w - - 0 1")
            .unwrap()
            .as_board();
        let bb = threats_to(&board, Color::White, board.occupied());
        println!("{}", !bb);
        assert_eq!(
            !bb,
            a1 | b1 | d1 | e1 | f1 | h1 | c2 | d2 | e2 | g2 | h2 | e3 | a4 | e4 | a5 | e5 | a6 | b6 | h6 | g8
        );
    }

    #[test]
    fn test_attacked_by() {
        let board = Board::parse_fen("5Q2/8/7p/4P1p1/8/3NK1P1/8/8 w - - 0 1")
            .unwrap()
            .as_board();
        let bb = attacked_by(f4, board.white() | board.black(), &board);
        println!("{}", bb);
        assert_eq!(bb, g3 | g5 | e3 | d3 | f8);
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
        assert_eq!(board.legal_moves().sort().to_string(), "h1g2", "{:#}", board);
        let mov_h1g2 = board.parse_uci_move("h1g2")?;
        assert_eq!(board.is_legal_move(&mov_h1g2), true);
        println!("{}", counts::GLOBAL_COUNTS);

        for b in Catalog::stalemates().iter() {
            assert_eq!(b.legal_moves().to_string(), "".to_string(), "{}", b.to_fen());
        }
        Ok(())
    }
}
