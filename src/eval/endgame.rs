use crate::Bitboard;
use crate::bound::NodeType;
use crate::infra::parsed_config::Component;
use crate::search::node::Node;
use crate::board::Board;
use crate::mv::Move;
use std::{fmt};
use serde::{Deserialize, Serialize};
use crate::search::algo::Algo;
use crate::types::MoveType;
use super::score::Score;
use crate::eval::switches::Switches;
use crate::types::Color;



/// Recognize several known end games, and determine 
/// (a) what the action should be in search (stop / continue / reduce depth)
/// (b) An eval tweak to ensure we iterate towards mate
/// Inspired by Rebel
/// 
/// https://www.chess.com/article/view/how-chess-games-can-end-8-ways-explained
/// 
#[derive(Copy, Clone, PartialEq, Debug)]
    pub enum EndGame {
    Unknown,
    KingVsKing, // automatic draw
    KingMinorVsKing, // automatic draw
    KingMinorVsKingMinor, // draw but not automatic (helpmate)
    TwoKnightsVsKing, // draw but not automatic 
    BishopKnightVsKing(Color), // win
    TwoBishopsOppositeColorSquares(Color),  // win
    TwoBishopsSameColorSquares,  // draw but not automatic 
}

impl EndGame {
    pub fn is_draw(&self) -> bool {
        use EndGame::*;
        match self {
            Unknown => false,
            BishopKnightVsKing(_) => false,
            TwoBishopsOppositeColorSquares(_) => false,
            _ => true,
        }
    }

    /// immediately declared draw
    pub fn is_immediately_declared_draw(&self) -> bool {
        use EndGame::*;
        match self {
            KingVsKing => true, // automatic draw
            KingMinorVsKing => true, // automatic draw
            _ => false,
        }
    }

    /// immediately declared draw
    pub fn try_winner(&self) -> Option<Color> {
        use EndGame::*;
        match self {
            BishopKnightVsKing(c) => Some(*c), // win
            TwoBishopsOppositeColorSquares(c) => Some(*c),  // win
            _ => None,
        }
    }
    pub fn from_board(b: &Board) -> Self {
        // If both sides have any one of the following, 
        // and there are no pawns on the board:
        // 1. A lone king
        // 2. a king and bishop
        // 3. a king and knight
        // 4. K+B v K+B (same color Bs)
        //
        // queens, rooks or pawns => can still checkmate
        if (b.rooks() | b.queens()).any() {
            return EndGame::Unknown;
        }

        if (b.pawns()).any() {
            return EndGame::Unknown;
        }

        // can assume just bishops, knights and kings now
        let wb = (b.bishops() & b.white()).popcount();
        let bb = (b.bishops() & b.black()).popcount();
        let wn = (b.knights() & b.white()).popcount();
        let bn = (b.knights() & b.black()).popcount();
        // 0 minor pieces
        if wb + bb + wn + bn  == 0 {
            return EndGame::KingVsKing;
        }
        
        // 1 minor pieces
        if wb + bb + wn + bn == 1 {
            return EndGame::KingMinorVsKing;
        }

        // 2 minor pieces
        if wb + bb == 0 && (wn == 0 && bn == 2 || wn == 2 && bn == 0 ) {
            return EndGame::TwoKnightsVsKing;
        }
        if wn + wb == 1 && bn + bb == 1 {
            return EndGame::KingMinorVsKingMinor;
        }

        if wn ==1 && wb == 1 && bn + bb == 0 {
            return EndGame::BishopKnightVsKing(Color::White);
        } 
        if bn == 1 && bb == 1 && wn + wb == 0 {
            return EndGame::BishopKnightVsKing(Color::Black);
        } 

        if wn + bn == 0 && wb + bb == 2 {
            // bishops must below to same player as not king+minor endgame
            if (b.bishops() & Bitboard::WHITE_SQUARES).popcount() == 1 {
                if wb == 2 {
                    return EndGame::TwoBishopsOppositeColorSquares(Color::White);
                } else {
                    return EndGame::TwoBishopsOppositeColorSquares(Color::Black);
                }
            } else {
                return EndGame::TwoBishopsSameColorSquares;
            }
        }
        return EndGame::Unknown;

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{board::boardbuf::*, eval::{eval::SimpleScorer, weight::Weight}};
    use test_env_log;

    #[test]
    fn test_endgame() {
        let b = Board::parse_fen("k7/1p6/3N4/8/8/8/6N1/K6B w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);

        let b = Board::parse_fen("k7/8/3N4/8/8/8/8/K61 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KingMinorVsKing);

        let b = Board::parse_fen("k7/8/3n4/8/8/8/8/K6N w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KingMinorVsKingMinor);
        let mut eval = SimpleScorer::default();
        assert!(eval.win_bonus.e() > 0.1);
        eval.win_bonus = Weight::from_i32(100, 100);
        let sc_wi_bonus = b.eval_some(&eval, Switches::MATERIAL);
        eval.win_bonus = Weight::zero();
        let sc_wo_bonus = b.eval_some(&eval, Switches::MATERIAL);
        assert_eq!(sc_wi_bonus - sc_wo_bonus, Score::from_cp(0));

        let b = Board::parse_fen("k7/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KingVsKing);

        let b = Board::parse_fen("k7/8/8/8/8/8/6BB/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::TwoBishopsOppositeColorSquares(Color::White));
        let mut eval = SimpleScorer::default();
        eval.win_bonus = Weight::from_i32(100, 100);
        let sc_wi_bonus = b.eval_some(&eval, Switches::MATERIAL);
        eval.win_bonus = Weight::zero();
        let sc_wo_bonus = b.eval_some(&eval, Switches::MATERIAL);
        assert_eq!(sc_wi_bonus - sc_wo_bonus, Score::from_cp(100));

        let b = Board::parse_fen("kbb5/8/8/8/8/8/6BB/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);
        let mut eval = SimpleScorer::default();
        eval.win_bonus = Weight::from_i32(100, 100);
        let sc_wi_bonus = b.eval_some(&eval, Switches::MATERIAL);
        eval.win_bonus = Weight::zero();
        let sc_wo_bonus = b.eval_some(&eval, Switches::MATERIAL);
        assert_eq!(sc_wi_bonus - sc_wo_bonus, Score::from_cp(0));

        let b = Board::parse_fen("kbb5/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::TwoBishopsOppositeColorSquares(Color::Black));
        let mut eval = SimpleScorer::default();
        eval.win_bonus = Weight::from_i32(100, 100);
        let sc_wi_bonus = b.eval_some(&eval, Switches::MATERIAL);
        eval.win_bonus = Weight::zero();
        let sc_wo_bonus = b.eval_some(&eval, Switches::MATERIAL);
        assert_eq!(sc_wi_bonus - sc_wo_bonus, Score::from_cp(-100));

        let b = Board::parse_fen("kb1b4/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::TwoBishopsSameColorSquares);

        let b = Board::parse_fen("kb1n4/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::BishopKnightVsKing(Color::Black));
    }
}
