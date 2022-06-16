use crate::bits::bitboard::Bitboard;
use crate::bits::precalc::PreCalc;
use crate::infra::metric::*;
use crate::movelist::MoveList;
use crate::mv::Move;
use crate::piece::Piece;
use crate::search::node::{Counter, Timing};
use crate::board::rules::Rules;
use crate::board::Board;


// fn is_in_check(&self, king_color: Color) -> bool;
// fn will_check_them(&self, mv: &Move) -> bool;
// fn is_legal_move(&self, mv: &Move) -> bool;
// fn pseudo_legal_moves(&self) -> MoveList;
// fn legal_moves(&self) -> impl Iterator<Item=Move>;
// fn legal_capture_moves(&self) -> MoveList;
// fn threats_to(&self, c: Color) -> Bitboard;

impl Board {

    pub fn is_pseudo_legal_and_legal_move(&self, m: Move) -> bool {
        let t = Metrics::timing_start();
        let ret = self.is_pseudo_legal_move(&m) && self.is_legal_move(&m);
        Metrics::profile(t, Timing::TimingPseudoLegalAndLegal);
        ret
    }

    pub fn is_pseudo_legal_move(&self, m: &Move) -> bool {
        if !m.from().is_in(self.us()) {
            return false;
        }
        if m.mover_piece() != self.piece_at(m.from().as_bb()) {
            return false;
        }
        if !m.is_capture() {
            if m.to().is_in(self.occupied()) {
                return false;
            }
            if m.capture_piece() != Piece::None {
                return false;
            }
        }
        if m.is_capture() {
            if !m.is_ep_capture() {
                if !m.to().is_in(self.them()) {
                    return false;
                }
                if m.capture_piece() != self.piece_at(m.to().as_bb()) {
                    // FIXME! allow capture of another type of piece?
                    return false;
                }
            } else if !m.ep().is_in(self.them() & self.pawns()) {
                return false;
            }
        }
        if m.is_promo() {
            if !m.to().as_bb().intersects(Bitboard::RANKS_18) {
                // TODO! exact promo rank for white/black
                return false;
            }
            let pp = m.promo_piece();
            if pp != Piece::Queen && pp != Piece::Rook && pp != Piece::Bishop && pp != Piece::Knight
            {
                return false;
            }
        }

        if m.mover_piece().is_line_piece()
            && (PreCalc::default().strictly_between(m.from(), m.to()) & self.occupied()).any()
        {
            return false;
        }
        if m.mover_piece() == Piece::Pawn
            && (PreCalc::default().strictly_between(m.from(), m.to()) & self.occupied()).any()
        {
            return false;
        }
        // check piece move
        let precalc = PreCalc::default();
        let destinations = precalc.attacks(
            self.color_us(),
            m.mover_piece(),
            self.us(),
            self.them(),
            m.from(),
        );
        if !m.to().is_in(destinations | self.en_passant()) && !m.is_castle() {
            return false;
        }
        true
    }

    pub fn is_legal_variation(&self, moves: &[Move]) -> bool {
        if let Some(m) = moves.first() {
            if !self.is_pseudo_legal_move(m) || !self.is_legal_move(m) {
                return false;
            }
            self.make_move(m).is_legal_variation(&moves[1..])
        } else {
            true
        }
    }

    // the move is pseudo legal
    pub fn is_legal_move(&self, mv: &Move) -> bool {
        // castling and kings moves already done above
        let mut us = self.us();
        let mut kings = self.kings() & us;
        if kings.is_empty() {
            return true; // a test position without king on the board - we allow
        }

        // idea - lightweight make_move - no hash - just enough to check rays of sliders etc
        let mut them = self.them();
        let from_to_bits = mv.from().as_bb() | mv.to().as_bb();
        us ^= from_to_bits;

        if mv.mover_piece() == Piece::King {
            kings ^= from_to_bits;
        }
        let sq = kings.square();

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
        // we rely on self.pieces & them NOT being affected by our move other than by capture

        let attack_gen = PreCalc::default();
        let occ = us | them;
        if (attack_gen.rook_attacks(occ, sq) & (self.rooks() | self.queens()) & them).any() {
            return false;
        }

        if (attack_gen.knight_attacks(sq) & self.knights() & them).any() {
            return false;
        }

        if (attack_gen.bishop_attacks(occ, sq) & (self.bishops() | self.queens()) & them).any() {
            return false;
        }

        if (attack_gen.pawn_attackers(kings, self.color_them()) & self.pawns() & them).any() {
            return false;
        }

        if (attack_gen.king_attacks(sq) & self.kings() & them).any() {
            return false;
        }

        // FIXME! need to check castling

        true
    }

    #[inline]
    pub fn legal_moves_into(&self, moves: &mut MoveList) {
        Metrics::incr(Counter::MoveGen);
        Rules::legals_for(self, moves);
    }

    #[inline]
    pub fn legal_moves(&self) -> MoveList {
        Metrics::incr(Counter::MoveGen);
        let mut moves = MoveList::new();
        Rules::legals_for(self, &mut moves);
        moves
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::{catalog::*, Color};
    use crate::globals::constants::*;
    use anyhow::Result;
    extern crate env_logger;
    // use crate::movelist::MoveValidator;

    fn _init() {
        // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[test]
    fn test_pseudo_legal_moves() -> Result<()> {
        let mut buf =
            Board::parse_piece_placement("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").unwrap();
        buf.set(a2, ".")?;
        buf.set(d2, ".")?;
        buf.set(d4, "P")?;
        let board = buf.as_board();
        let moves = board.legal_moves();
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
        let mut moves = board.legal_moves();
        println!("{}\n{:#?}", board, moves);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves.sort().to_string(), "a2a3, a2a4");

        let board = Board::parse_fen("8/p7/8/8/8/8/8/8 b - - 0 0 id 'lone P flipped'")
            .unwrap()
            .as_board();
        let mut moves = board.legal_moves();
        assert_eq!(moves.sort().to_string(), "a7a5, a7a6");

        let board = Board::parse_fen("8/8/8/8/8/p7/P7/8 w - - 0 0 id PP")
            .unwrap()
            .as_board();
        let mut moves = board.legal_moves();
        assert_eq!(moves.sort().to_string(), "");

        let board = Board::parse_fen("8/8/8/8/8/8/PPP5/8 w - - 0 0 id PPP")
            .unwrap()
            .as_board();
        let mut moves = board.legal_moves();
        assert_eq!(
            moves.sort().to_string(),
            "a2a3, a2a4, b2b3, b2b4, c2c3, c2c4"
        );

        let board = Board::parse_fen("8/8/8/8/8/p1p5/1P6/8 w - - 0 0 id P 'capture white'")
            .unwrap()
            .as_board();
        let mut moves = board.legal_moves();
        assert_eq!(moves.sort().to_string(), "b2a3, b2b3, b2b4, b2c3");

        let board = Board::parse_fen("8/1p6/P1P5/8/8/8/1P6/8 b - - 0 0 id 'P capture black'")
            .unwrap()
            .as_board();
        let mut moves = board.legal_moves();
        assert_eq!(moves.sort().to_string(), "b7a6, b7b5, b7b6, b7c6");

        let board = Board::parse_fen("8/8/p6p/1N6/8/8/8/8 b - - 0 0 id 'PxN black'")
            .unwrap()
            .as_board();
        let mut moves = board.legal_moves();
        assert_eq!(moves.sort().to_string(), "a6a5, a6b5, h6h5");
    }

    #[test]
    fn pawn_en_passant() {
        let board = Board::parse_fen("8/8/8/pP6/8/8/8/8 w - a6 0 0 id en-passant-#1")
            .unwrap()
            .as_board();
        assert_eq!(board.en_passant(), a6);
        assert_eq!(board.legal_moves().sort().to_string(), "b5a6, b5b6");
        let board = Board::parse_fen("8/8/8/PpP5/8/8/8/8 w - b6 0 0 id 'en passant #2'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "a5a6, a5b6, c5b6, c5c6"
        );
    }

    #[test]
    fn pawn_promotions() {
        let board = Board::parse_fen("8/P7/8/8/8/8/7k/K7 w - - 0 0 id 'promos #1'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "a1a2, a1b1, a1b2, a7a8b, a7a8n, a7a8q, a7a8r"
        );
    }

    #[test]
    fn rook_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/8/R7 w - - 0 0 id 'R'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1a7, a1a8, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board = Board::parse_fen("8/p7/P7/8/8/8/8/R7 w - - 0 0 id 'R with P blocking'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );

        let board = Board::parse_fen("8/p7/p7/8/8/8/8/R7 w - - 0 0 id 'R with p to capture'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "a1a2, a1a3, a1a4, a1a5, a1a6, a1b1, a1c1, a1d1, a1e1, a1f1, a1g1, a1h1"
        );
    }

    #[test]
    fn knight_moves() {
        let board = Board::parse_fen("8/8/8/3N4/8/8/8/8 w - - 0 0 id 'N d5'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "d5b4, d5b6, d5c3, d5c7, d5e3, d5e7, d5f4, d5f6"
        );
    }

    #[test]
    fn bishop_moves() {
        let board = Board::parse_fen("8/8/8/8/8/8/8/B7 w - - 0 0 id 'B a1'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "a1b2, a1c3, a1d4, a1e5, a1f6, a1g7, a1h8"
        );

        let board = Board::parse_fen("8/8/8/8/8/8/1B6/8 w - - 0 0 id 'B b2'")
            .unwrap()
            .as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "b2a1, b2a3, b2c1, b2c3, b2d4, b2e5, b2f6, b2g7, b2h8"
        );
    }

    #[test]
    fn moves_in_check() {
        let board =
            Board::parse_fen("rnbqkbnr/pppp1ppp/4p3/3N4/8/8/PPPPPPPP/R1BQKBNR b KQkq - 1 2")
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
        assert_eq!(board.is_in_check(crate::Color::Black), true);

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
    fn test_legal_move() -> Result<()> {
        // https://lichess.org/editor/8/8/8/8/8/8/6r1/7K
        let fen = "8/8/8/8/8/8/6r1/7K w - - 0 0 id 'rook+king'";
        let board = Board::parse_fen(fen).unwrap().as_board();
        assert_eq!(
            board.legal_moves().sort().to_string(),
            "h1g2",
            "{:#}",
            board
        );
        let mov_h1g2 = board.parse_uci_move("h1g2")?;
        assert_eq!(board.is_legal_move(&mov_h1g2), true);
        println!("{}", Metrics::to_string());

        for b in Catalog::stalemates().iter() {
            assert_eq!(
                b.legal_moves().to_string(),
                "".to_string(),
                "{}",
                b.to_fen()
            );
        }

        let board = Catalog::starting_board();
        let mv = board.parse_uci_move("e2e3").unwrap();
        assert!(board.is_legal_move(&mv), "{:?}", mv);
        Ok(())
    }

    #[test]
    fn test_legal_variation() -> Result<()> {
        let b = Catalog::starting_board();
        let bd2 = Catalog::test_position();
        let mvs = b.parse_uci_variation("a2a3 e7e6 b2b4")?;
        assert_eq!(b.is_legal_variation(&mvs), true);
        assert_eq!(bd2.board().is_legal_variation(&mvs), false);
        Ok(())
    }

    #[test]
    fn test_is_valid_move() {
        let b = Catalog::starting_board();
        let a2sq = a2.square();
        let a3sq = a3.square();
        let a6sq = a6.square();
        let a7sq = a7.square();
        assert_eq!(
            b.is_pseudo_legal_move(&Move::new_quiet(Piece::Pawn, a2sq, a3sq)),
            true
        );
        assert_eq!(
            b.is_pseudo_legal_move(&Move::new_quiet(Piece::Bishop, a2sq, a3sq)),
            false
        );
        assert_eq!(
            b.is_pseudo_legal_move(&Move::new_quiet(Piece::Pawn, a7sq, a6sq)),
            false
        );
        assert_eq!(
            b.is_pseudo_legal_move(&Move::new_quiet(Piece::Pawn, a7sq, a6sq)),
            false
        );
        assert_eq!(
            b.is_pseudo_legal_move(&Move::new_capture(Piece::Pawn, a2sq, a3sq, Piece::Pawn)),
            false
        );
        for mv in b.legal_moves().iter() {
            assert!(b.is_legal_move(mv));
            assert!(b.is_pseudo_legal_move(mv));
        }
    }

    #[test]
    fn test_is_legal_variation() {
        let b =
            Board::from_str("b3r1kr/ppppqppp/3np3/6b1/1n1PP1N1/2NQ4/PPP1BPPP/B3R1KR w - - 3 12")
                .unwrap();
        let var = b.parse_uci_variation("d3d1 b4c6 e2c4").unwrap();
        assert_eq!(var.len(), 3);
        assert!(b.is_legal_variation(&var));
    }

    #[test]
    fn test_catalog_moves() {
        let positions = Catalog::moves();
        for pos in positions {
            let mut lm: Vec<String> = pos
                .board()
                .legal_moves()
                .uci()
                .split_ascii_whitespace()
                .map(str::to_string)
                .collect();
            lm.sort();
            let lm = lm.join(" ");

            let mut expected: Vec<String> = pos
                .get("c1")
                .unwrap()
                .value_uci()
                .split_ascii_whitespace()
                .map(str::to_string)
                .collect();
            expected.sort();
            let expected = expected.join(" ");

            assert_eq!(lm, expected, "{} {:#}", pos, pos.board());
        }
    }
}
