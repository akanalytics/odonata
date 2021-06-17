use crate::bitboard::bitboard::Bitboard;
use crate::bitboard::castling::CastlingRights;
use crate::bitboard::square::Square;
use crate::globals::constants::*;
use crate::types::{Color, Piece};
use crate::utils::StringUtils;
use crate::board::Board;
// use arrayvec::ArrayVec;
use std::fmt;

// FIXME: public methods
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub ep: Square,
    pub promo: Piece,
    pub capture: Piece,
    pub mover: Piece,

    pub castle_side: CastlingRights,
    pub is_null: bool,
}

// piece
// from
// to
// pice2
// from2
// to2
//
// promo/capture
//
// P from
// Q-to
// cap-from
//
// Promo/capture

impl Move {
    pub const NULL_MOVE: Move = Move {
        from: Square::null(),
        to: Square::null(),
        ep: Square::null(),
        promo: Piece::None,
        capture: Piece::None,
        mover: Piece::None,
        castle_side: CastlingRights::NONE,
        is_null: true,
    };
    #[inline]
    pub fn new_null() -> Move {
        Move {
            is_null: true,
            ..Default::default()
        }
    }

    #[inline]
    pub const fn to(&self) -> Square {
        self.to
    }

    #[inline]
    pub const fn from(&self) -> Square {
        self.from
    }

    #[inline]
    pub const fn ep(&self) -> Square {
        self.ep
    }

    #[inline]
    pub fn capture_square(&self) -> Square {
        if self.is_ep_capture() {
            self.ep()
        } else if self.is_capture() {
            self.to()
        } else {
            Square::null()
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.is_null
    }

    #[inline]
    pub fn is_promo(&self) -> bool {
        self.promo != Piece::None
    }

    #[inline]
    pub const fn promo_piece(&self) -> Piece {
        self.promo
    }

    #[inline]
    pub const fn capture_piece(&self) -> Piece {
        self.capture
    }

    #[inline]
    pub const fn mover_piece(&self) -> Piece {
        self.mover
    }

    #[inline]
    pub fn is_capture(&self) -> bool {
        self.capture != Piece::None
    }

    #[inline]
    pub const fn is_castle(&self) -> bool {
        !self.castle_side.is_empty()
    }

    #[inline]
    pub const fn castling_side(&self) -> CastlingRights {
        self.castle_side
    }

    #[inline]
    pub fn is_ep_capture(&self) -> bool {
        !self.ep.is_null() && self.is_capture()
    }

    #[inline]
    pub fn is_pawn_double_push(&self) -> bool {
        !self.ep.is_null() && !self.is_capture()
    }

    #[inline]
    pub fn new_quiet(p: Piece, from: Square, to: Square) -> Move {
        Move {
            from,
            to,
            mover: p,
            ..Self::default()
        }
    }

    #[inline]
    pub fn rook_move(&self) -> Move {
        if self.is_castle() {
            let (from, to) = self.rook_move_from_to();
            Move::new_quiet(Piece::Rook, from, to)
        } else {
            Move::NULL_MOVE
        }
    }

    #[inline]
    pub const fn rook_move_from_to(&self) -> (Square, Square) {
        #[allow(non_upper_case_globals)]
        match self.to().as_bb() {
            c1 => (a1.square(), d1.square()),
            g1 => (h1.square(), f1.square()),
            c8 => (a8.square(), d8.square()),
            g8 => (h8.square(), f8.square()),
            _ => (Square::null(), Square::null()),
        }
    }

    #[inline]
    pub fn castling_rights_lost(&self) -> CastlingRights {
        let squares_changing = self.to().as_bb() | self.from().as_bb();
        CastlingRights::rights_lost(squares_changing)
    }



    #[inline]
    pub fn new_pawn_move(from: Square, to: Square, b: &Board) -> Move {
        if to.is_in(b.them()) {
            let cap = b.piece_at(to.as_bb());
            Move::new_capture(Piece::Pawn, from, to, cap)
        } else {
            // its a push
            let behind = to.shift(b.color_us().backward());
            let ep = behind;
            if behind.as_bb().disjoint(b.pawns()) {
                // no one behind us => double push
                Move::new_double_push(from, to, ep)
            } else {
                Move::new_quiet(Piece::Pawn, from, to) 
            }
        }
    }


    #[inline]
    pub fn new_double_push(from: Square, to: Square, ep_square: Square) -> Move {
        Move {
            from,
            to,
            ep: ep_square,
            mover: Piece::Pawn,
            ..Self::default()
        }
    }

    #[inline]
    pub fn new_capture(p: Piece, from: Square, to: Square, captured: Piece) -> Move {
        Move {
            from,
            to,
            mover: p,
            capture: captured,
            ..Self::default()
        }
    }

    #[inline]
    pub fn new_ep_capture(from: Square, to: Square, captured_sq: Square) -> Move {
        Move {
            from,
            to,
            mover: Piece::Pawn,
            capture: Piece::Pawn,
            ep: captured_sq,
            ..Self::default()
        }
    }

    #[inline]
    pub fn new_promo(from: Square, to: Square, promo: Piece) -> Move {
        Move {
            from,
            to,
            promo,
            mover: Piece::Pawn,
            ..Default::default()
        }
    }

    #[inline]
    pub fn new_promo_capture(from: Square, to: Square, promo: Piece, capture: Piece) -> Move {
        Move {
            from,
            to,
            mover: Piece::Pawn,
            capture,
            promo,
            ..Default::default()
        }
    }

    #[inline]
    pub fn new_castle(
        king_from: Square,
        king_to: Square,
        castle: CastlingRights,
    ) -> Move {
        Move {
            from: king_from,
            to: king_to,
            mover: Piece::King,
            castle_side: castle,
            // p3: Piece::Rook,
            // t3: rook_to,
            // p4: Piece::Rook,
            // f4: rook_from,
            ..Default::default()
        }
    }

    #[inline]
    pub fn mvv_lva_score(&self) -> i32 {
        let mut score = 0;
        if self.is_capture() {
            score += self.capture.centipawns() * 10 - self.mover.centipawns() / 10;
        }
        if self.is_promo() {
            score += self.promo.centipawns() * 10 - self.mover.centipawns() / 10;
        }
        score
    }

    pub fn uci(&self) -> String {
        if self.is_null() {
            return String::from("0000");
        }
        let mut res = String::new();
        res.push_str(&self.from.uci());
        res.push_str(&self.to.uci());
        if self.is_promo() {
            res.push(self.promo.to_char(Some(Color::Black)));
        }
        res
    }

    pub fn parse_uci(s: &str) -> Result<Move, String> {
        let from = Bitboard::parse_square(s.take_slice(0..2))?;
        let to = Bitboard::parse_square(s.take_slice(2..4))?;
        let promo;
        if let Some(ch) = s.take_char_at(4) {
            promo = Piece::from_char(ch)?;
        } else {
            promo = Piece::None;
        }
        Ok(Move {
            to,
            from,
            promo,
            ..Default::default()
        })
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.uci())?;
        if f.alternate() {
            write!(f, " m:{}", self.mover_piece())?;

            if !self.ep().is_null() {
                write!(f, " ep:{}", self.ep().uci())?;
            }
            if self.is_capture() {
                write!(f, " c:{}", self.capture_piece())?;
            }
            if self.is_castle() {
                write!(f, " cs:{}", self.castling_side())?;
            }
            if self.is_ep_capture() {
                write!(f, " e/p cap")?;
            }
        }
        Ok(())
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::boardbuf::*;
    use crate::catalog::Catalog;
    // use crate::movelist::MoveValidator;

    #[test]
    fn test_move() {
        assert_eq!(Move::new_null().to_string(), "0000");

        let move_a1b2 = Move {
            from: a1.square(),
            to: b2.square(),
            ..Default::default()
        };
        let promo_a7a8 = Move {
            from: a7.square(),
            to: a8.square(),
            promo: Piece::Queen,
            ..Default::default()
        };
        assert_eq!(move_a1b2.to_string(), "a1b2");
        assert_eq!(promo_a7a8.to_string(), "a7a8q");

        let move_e2e4 = Move::parse_uci("e2e4").unwrap();
        assert_eq!(move_e2e4.to_string(), "e2e4");

        let move_e7e8 = Move::parse_uci("e7e8p").unwrap();
        assert_eq!(move_e7e8.to_string(), "e7e8p");

        let board = Catalog::starting_position();
        assert_eq!(board.parse_san_move("Nc3").unwrap().to_string(), "b1c3");
        assert_eq!(board.parse_san_move("c3").unwrap().to_string(), "c2c3");
        assert_eq!(board.parse_san_move("c2c4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("c2-c4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("Pc4").unwrap().to_string(), "c2c4");
        assert_eq!(board.parse_san_move("Pc2c4").unwrap().to_string(), "c2c4");
    }



    
    #[test]
    fn test_mvv_lva() {
        let def = Move::default();
        let pxq = Move {
            capture: Piece::Queen,
            mover: Piece::Pawn,
            ..def
        };
        let pxr = Move {
            capture: Piece::Rook,
            mover: Piece::Pawn,
            ..def
        };
        let pxb = Move {
            capture: Piece::Bishop,
            mover: Piece::Pawn,
            ..def
        };
        let pxn = Move {
            capture: Piece::Knight,
            mover: Piece::Pawn,
            ..def
        };
        let pxp = Move {
            capture: Piece::Pawn,
            mover: Piece::Pawn,
            ..def
        };

        let qxp = Move {
            capture: Piece::Pawn,
            mover: Piece::Queen,
            ..def
        };
        let qxn = Move {
            capture: Piece::Knight,
            mover: Piece::Queen,
            ..def
        };
        let qxb = Move {
            capture: Piece::Bishop,
            mover: Piece::Queen,
            ..def
        };
        let qxr = Move {
            capture: Piece::Knight,
            mover: Piece::Queen,
            ..def
        };
        let qxq = Move {
            capture: Piece::Queen,
            mover: Piece::Queen,
            ..def
        };

        let pxq_q = Move {
            capture: Piece::Queen,
            mover: Piece::Pawn,
            promo: Piece::Queen,
            ..def
        };
        let p_q = Move {
            mover: Piece::Pawn,
            promo: Piece::Queen,
            ..def
        };

        assert_eq!(pxq.mvv_lva_score(), 8990);
        assert_eq!(pxr.mvv_lva_score(), 4990);
        assert_eq!(pxb.mvv_lva_score(), 3490);
        assert_eq!(pxn.mvv_lva_score(), 3240);
        assert_eq!(pxp.mvv_lva_score(), 990);

        assert_eq!(qxp.mvv_lva_score(), 910);
        assert_eq!(qxn.mvv_lva_score(), 3160);
        assert_eq!(qxb.mvv_lva_score(), 3410);
        assert_eq!(qxr.mvv_lva_score(), 3160);
        assert_eq!(qxq.mvv_lva_score(), 8910);

        assert_eq!(pxq_q.mvv_lva_score(), 17980);
        assert_eq!(p_q.mvv_lva_score(), 8990);
    }


    #[test]
    fn test_to_san() {
        let mut board = Catalog::starting_position();
        let a2a3 = board.parse_uci_move("a2a3").unwrap();
        let b1c3 = board.parse_uci_move("b1c3").unwrap();
        assert_eq!(board.to_san(&a2a3), "a3");
        assert_eq!(board.to_san(&b1c3), "Nc3");

        let board = board.set(d3, "p").unwrap();
        let board = board.set(f3, "p").unwrap();

        let c2d3 = board.parse_uci_move("c2d3").unwrap();
        assert_eq!(board.to_san(&c2d3), "cxd3");

        let e2d3 = board.parse_uci_move("e2d3").unwrap();
        assert_eq!(board.to_san(&e2d3), "exd3");

        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "Nxf3");

        // knight ambiguity
        let board = board.set(g5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g1f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "N1xf3");

        // two knights same rank and file as g5
        let board = board.set(e5, "N").unwrap();
        let g1f3 = board.parse_uci_move("g5f3").unwrap();
        assert_eq!(board.to_san(&g1f3), "Ng5xf3");

        // remove some minor pieces to allow castling
        let board = board.set(Bitboard::RANK_8, "r...k..r").unwrap();
        board.set_turn(Color::Black);
        let castle_k = board.parse_uci_move("e8g8").unwrap();
        assert_eq!(board.to_san(&castle_k), "O-O");
        let castle_q = board.parse_uci_move("e8c8").unwrap();
        assert_eq!(board.to_san(&castle_q), "O-O-O");
    }

}