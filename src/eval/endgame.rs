use crate::Bitboard;
use crate::board::Board;
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
    KingMinorVsKingPawns(Color), // draw at best, color is winning/has pawns
    KingVsKing, // automatic draw
    KingMinorVsKing, // automatic draw
    KingMinorVsKingMinor, // draw but not automatic (helpmate)
    TwoKnightsVsKing, // draw but not automatic 
    BishopKnightVsKing(Color), // win
    TwoBishopsOppositeColorSquares(Color),  // win
    TwoBishopsSameColorSquares,  // draw but not automatic 
    KingMajorsVsKing(Color), // win
}

impl Default for EndGame {
    fn default() -> Self { EndGame::Unknown }
}


impl EndGame {
    pub fn is_likely_draw(&self) -> bool {
        use EndGame::*;
        match self {
            Unknown => false,
            KingMinorVsKingMinor => true,
            TwoKnightsVsKing => true,  
            TwoBishopsSameColorSquares => true,
            _ => false,
        }
    }

    pub fn cannot_win(&self) -> Option<Color> {
        use EndGame::*;
        match self {
            // c has pawns so opponent cant win 
            KingMinorVsKingPawns(c) => Some(c.opposite()),  
            KingMajorsVsKing(c) => Some(c.opposite()),  
            _ => None,
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

    /// should be a win unless piece can be captures immediately
    pub fn try_winner(&self) -> Option<Color> {
        use EndGame::*;
        match self {
            BishopKnightVsKing(c) => Some(*c), // win
            TwoBishopsOppositeColorSquares(c) => Some(*c),  // win
            KingMajorsVsKing(c) => Some(*c),  // win
            _ => None,
        }
    }
    pub fn from_board(b: &Board) -> Self {

        if b.pawns().is_empty() && (b.rooks().any() || b.queens().any()) {
            if (b.black() - b.kings()).is_empty() {
                return EndGame::KingMajorsVsKing(Color::White);
            }
            if (b.white() - b.kings()).is_empty() {
                return EndGame::KingMajorsVsKing(Color::Black);
            }
        }

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

        // either size could win if both have pawns 
        if (b.pawns() & b.black()).any() && (b.pawns() & b.white()).any() {
            return EndGame::Unknown;
        }
        
        if (b.pawns() & b.black()).any() && ((b.bishops() | b.knights()) & b.white()).popcount() <= 1 {
            return EndGame::KingMinorVsKingPawns(Color::Black)
        }    
        if (b.pawns() & b.white()).any() && ((b.bishops() | b.knights()) & b.black()).popcount() <= 1 {
            return EndGame::KingMinorVsKingPawns(Color::White)
        }    

        // pawns plus opponent has 2+ minors, so uncertain outcome
        if b.pawns().any() {
            return  EndGame::Unknown;
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


        // no bishops
        if wb + bb == 0 && (wn == 0 && bn >= 2 || wn >= 2 && bn == 0 ) {
            return EndGame::TwoKnightsVsKing;
        }
        if wn + wb == 1 && bn + bb == 1 {
            return EndGame::KingMinorVsKingMinor;
        }

        if wn >= 1 && wb >= 1 && bn + bb == 0 {
            return EndGame::BishopKnightVsKing(Color::White);
        } 
        if bn >= 1 && bb >= 1 && wn + wb == 0 {
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
    use crate::eval::switches::Switches;
    use crate::eval::score::Score;
    use test_log::test;

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
        assert_eq!(eg.cannot_win(), None);

        let b = Board::parse_fen("kb1n4/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::BishopKnightVsKing(Color::Black));


        let b = Board::parse_fen("8/k7/1p6/3N4/8/8/8/K7 w - - 5 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KingMinorVsKingPawns(Color::Black));
        assert_eq!(eg.cannot_win(), Some(Color::White));
        
        let b = Board::parse_fen("Q7/1K6/8/8/8/8/6k1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KingMajorsVsKing(Color::White));
        assert_eq!(eg.cannot_win(), Some(Color::Black));
        assert_eq!(eg.try_winner(), Some(Color::White));

        let b = Board::parse_fen("R7/1K6/8/8/8/8/6k1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KingMajorsVsKing(Color::White));
        assert_eq!(eg.cannot_win(), Some(Color::Black));
        assert_eq!(eg.try_winner(), Some(Color::White));

        let b = Board::parse_fen("r7/1k6/8/8/8/8/6K1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KingMajorsVsKing(Color::Black));
        assert_eq!(eg.cannot_win(), Some(Color::White));
        assert_eq!(eg.try_winner(), Some(Color::Black));

        let b = Board::parse_fen("r7/1k6/8/8/8/8/6K1/B7 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);

        let b = Board::parse_fen("Q7/1K6/8/8/8/8/6kp/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);
    }
}
