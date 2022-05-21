
use crate::trace::stat::{Stat, SliceStat};
use crate::{board::Board};
use crate::types::Color;
use crate::Bitboard;
use strum_macros::{Display, EnumCount};
use strum_macros::EnumIter;
use strum_macros::IntoStaticStr;
use strum::IntoEnumIterator;

/// Recognize several known end games, and determine
/// (a) what the action should be in search (stop / continue / reduce depth)
/// (b) An eval tweak to ensure we iterate towards mate
///
/// Color is side ahead in material
/// ///
///
///
///
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LikelyOutcome {
    UnknownOutcome,
    WhiteWin,
    WhiteWinOrDraw,
    Draw,
    DrawImmediate,
    WhiteLossOrDraw,
    WhiteLoss,
}

#[derive(Copy, Clone, PartialEq, Debug, IntoStaticStr, EnumCount, EnumIter, Display)]
pub enum EndGame {
    Unknown, // for when its too costly to work out who wins
    // 1v1
    Kk, // automatic draw
    // 2v1
    KMk, // automatic draw
    Kkm, // automatic draw
    KRk, // win
    Kkr, // win
    KQk, // win
    Kkq, // win
    KPk, // ??
    Kkp, // ??

    // 2v kp
    KPkp, // ??
    KMkp, // draw at best, color is winning/has pawns
    KPkm, // draw at best, color is winning/has pawns
    KRkp, // ??
    KPkr, // ??
    KQkp, // ??
    KPkq, // ??

    // 2v km
    KMkm, // draw but not automatic (helpmate)
    KRkm, // ??
    KMkr, // ??
    KQkm, // ??
    KMkq, // ??

    // 2v k+major
    KRkr, // ??
    KQkr, // ??
    KRkQ, // ??
    KQkQ, // ??

    // 3v k
    KPPk, // ??
    Kkpp, // ??
    KNPk, // ??
    Kknp, // ??
    KBPk, // ??
    Kkbp, // ??

    KNNk, // draw but not automatic
    Kknn, // draw but not automatic
    KBNk, // win
    Kkbn, // win
    KBbk, // win
    KBBk, // draw
    KkBb, // win
    Kkbb, // draw
    KJMk, // win
    Kkjm, // win
    KJJk, // win
    Kkjj, // win
}

use static_init::dynamic;
#[dynamic]
static ENDGAME_COUNTS: Vec<Stat> =  {
    let mut vec = vec![];
    for eg in EndGame::iter() {
        let s: &'static str = eg.into();
        vec.push(Stat::new(s));
    }
    vec
};



impl Default for EndGame {
    fn default() -> Self {
        EndGame::Unknown
    }
}

impl EndGame {
    pub fn likely_outcome(&self, _b: &Board) -> LikelyOutcome {
        use EndGame::*;
        use LikelyOutcome::*;
        match self {
            Unknown => UnknownOutcome,
            Kk | KMk | Kkm => DrawImmediate,

            KNNk | KMkm | Kknn | KBBk | Kkbb => Draw, // (helpmate?)

            KRk | KQk | KBNk | KBbk | KJJk | KJMk => WhiteWin,
            Kkr | Kkq | Kkbn | KkBb | Kkjj | Kkjm => WhiteLoss,

            // Guesses
            KPkp => Draw,

            KMkp => WhiteLossOrDraw,
            KPkm  => WhiteWinOrDraw,

            KQkp => WhiteWin,
            KPkq => WhiteLoss,

            KRkp => UnknownOutcome,
            KPkr => UnknownOutcome,

            //
            KPk => WhiteWinOrDraw,
            Kkp => WhiteLossOrDraw,

            KRkm => UnknownOutcome,
            KMkr => UnknownOutcome,
            KQkm => UnknownOutcome,
            KMkq => UnknownOutcome,

            KRkr => UnknownOutcome,
            KQkr => UnknownOutcome,
            KRkQ => UnknownOutcome,
            KQkQ => UnknownOutcome,

            KPPk => WhiteWin,
            Kkpp => WhiteLoss,
            KNPk => WhiteWinOrDraw,
            Kknp => WhiteLossOrDraw,
            KBPk => WhiteWinOrDraw,
            Kkbp => WhiteLossOrDraw,
        }
    }

    // pub fn is_likely_draw(&self) -> bool {
    //     use EndGame::*;
    //     match self {
    //         Unknown => false,
    //         KMkm => true,
    //         KNNk => true,
    //         KBBk => true,
    //         _ => false,
    //     }
    // }

    // // the color that cannot win
    // pub fn cannot_win(&self) -> Option<Color> {
    //     use EndGame::*;
    //     match self {
    //         // c has pawns so opponent cant win
    //         KMkp(c) => Some(c), // cannot win with a minor
    //         KRk(c) => Some(c),
    //         _ => None,
    //     }
    // }

    // /// immediately declared draw
    // pub fn is_immediately_declared_draw(&self) -> bool {
    //     use EndGame::*;
    //     #[allow(clippy::match_like_matches_macro)]
    //     match self {
    //         Kk => true,  // automatic draw
    //         KMk => true, // automatic draw
    //         _ => false,
    //     }
    // }

    /// should be a win unless piece can be captures immediately
    pub fn try_winner(&self, b: &Board) -> Option<Color> {
        match self.likely_outcome(b) {
            LikelyOutcome::WhiteWin => Some(Color::White),
            LikelyOutcome::WhiteLoss => Some(Color::Black),
            _ => None,
        }
    }

    pub fn counts_to_string() -> String {
        format!("{}", SliceStat(&ENDGAME_COUNTS[..]))
    }

    pub fn from_board(b: &Board) -> Self {
        let eg = Self::private_ctor(b);
        ENDGAME_COUNTS[eg as usize].increment();
        eg
    }

    fn private_ctor(b: &Board) -> Self {
        let n_pieces = b.occupied().popcount();

        if n_pieces >= 5 {
            return Self::Unknown;
        }

        if n_pieces == 2 {
            return Self::Kk;
        }

        if n_pieces == 3 {
            if b.rooks().any() {
                if (b.black() - b.kings()).is_empty() {
                    return EndGame::KRk;
                }
                if (b.white() - b.kings()).is_empty() {
                    return EndGame::Kkr;
                }
            }
            if b.queens().any() {
                if (b.black() - b.kings()).is_empty() {
                    return EndGame::KQk;
                }
                if (b.white() - b.kings()).is_empty() {
                    return EndGame::Kkq;
                }
            }
        }

        let wb = (b.bishops() & b.white()).popcount();
        let wn = (b.knights() & b.white()).popcount();
        let bn = (b.knights() & b.black()).popcount();
        let bb = (b.bishops() & b.black()).popcount();
        if n_pieces == 3 {
            if wb + wn > 0 {
                return EndGame::KMk;
            }
            if bb + bn > 0 {
                return EndGame::Kkm;
            }
            if b.pawns().intersects(b.white()) {
                return EndGame::KPk;
            } else {
                return EndGame::Kkp;
            }
        }

        // now assume we have 4 pieces
        if wn + wb == 1 && bn + bb == 1 {
            return EndGame::KMkm;
        }

        // no bishops
        if wb + bb == 0 {
            if wn == 2 && bn == 0 {
                return EndGame::KNNk;
            }
            if wn == 0 && bn == 2 {
                return EndGame::Kknn;
            }
        }
        if wn + wb == 1 && bn + bb == 1 {
            return EndGame::KMkm;
        }

        if wn == 1 && wb == 1 {
            return EndGame::KBNk;
        }
        if bn == 1 && bb == 1 {
            return EndGame::Kkbn;
        }
        if wn == 0 && bn == 0 && wb + bb == 2 {
            if (b.bishops() & Bitboard::WHITE_SQUARES).popcount() == 1 {
                if wb == 2 {
                    return EndGame::KBbk;
                } else {
                    return EndGame::KkBb;
                }
            } else {
                if wb == 2 {
                    return EndGame::KBBk;
                } else {
                    return EndGame::Kkbb;
                }
            }
        }
        let n_pawns = b.pawns().popcount();
        let wp = (b.pawns() & b.white()).popcount();
        let bp = (b.pawns() & b.black()).popcount();
        let wq = (b.queens() & b.white()).popcount();
        let bq = (b.queens() & b.black()).popcount();
        let wr = (b.rooks() & b.white()).popcount();
        let br = (b.rooks() & b.black()).popcount();

        if n_pawns == 1 {
            if wb + wn + bb + bn == 1 {
                if b.pawns().intersects(b.black()) {
                    return EndGame::KMkp;
                } else {
                    return EndGame::KPkm;
                }
            }
            // wb + wn + bb + bn == 0
            if wq == 1 && bp == 1 {
                return EndGame::KQkp;
            }
            if bq == 1 && wp == 1 {
                return EndGame::KPkq;
            }
            if wr == 1 && bp == 1 {
                return EndGame::KRkp;
            }
            if br == 1 && wp == 1 {
                return EndGame::KPkr;
            }
        }
        if n_pawns == 2 {
            if wp == 2 {
                return EndGame::KPPk;
            }
            if bp == 2 {
                return EndGame::Kkpp;
            }
            return EndGame::KPkp;
        }
        Self::Unknown
    }
}

//     // If both sides have any one of the following,
//     // and there are no pawns on the board:
//     // 1. A lone king
//     // 2. a king and bishop
//     // 3. a king and knight
//     // 4. K+B v K+B (same color Bs)
//     //
//     // queens, rooks or pawns => can still checkmate
//     if (b.rooks() | b.queens()).any() {
//         return EndGame::Unknown;
//     }

//     // either size could win if both have pawns
//     if (b.pawns() & b.black()).any() && (b.pawns() & b.white()).any() {
//         return EndGame::Unknown;
//     }

//     if (b.pawns() & b.black()).any() && ((b.bishops() | b.knights()) & b.white()).popcount() <= 1 {
//         return EndGame::KMkp(Color::White);
//     }
//     if (b.pawns() & b.white()).any() && ((b.bishops() | b.knights()) & b.black()).popcount() <= 1 {
//         return EndGame::KMkp(Color::Black);
//     }

//     // pawns plus opponent has 2+ minors, so uncertain outcome
//     if b.pawns().any() {
//         return EndGame::Unknown;
//     }

//     // can assume just bishops, knights and kings now
//     let wb = (b.bishops() & b.white()).popcount();
//     let bb = (b.bishops() & b.black()).popcount();
//     let wn = (b.knights() & b.white()).popcount();
//     let bn = (b.knights() & b.black()).popcount();
//     // 0 minor pieces
//     if wb + bb + wn + bn == 0 {
//         return EndGame::Kk;
//     }

//     // 1 minor pieces
//     if wb + bb + wn + bn == 1 {
//         return EndGame::KMk;
//     }

//     // no bishops
//     if wb + bb == 0 && (wn == 0 && bn >= 2 || wn >= 2 && bn == 0) {
//         return EndGame::KNNk;
//     }
//     if wn + wb == 1 && bn + bb == 1 {
//         return EndGame::KMkm;
//     }

//     if wn >= 1 && wb >= 1 && bn + bb == 0 {
//         return EndGame::KBNk(Color::White);
//     }
//     if bn >= 1 && bb >= 1 && wn + wb == 0 {
//         return EndGame::KBNk(Color::Black);
//     }

//     if wn + bn == 0 && wb + bb == 2 {
//         // bishops must below to same player as not king+minor endgame
//         if (b.bishops() & Bitboard::WHITE_SQUARES).popcount() == 1 {
//             if wb == 2 {
//                 return EndGame::KBbk(Color::White);
//             } else {
//                 return EndGame::KBbk(Color::Black);
//             }
//         } else {
//             return EndGame::KBBk;
//         }
//     }
//     EndGame::Unknown
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{board::boardbuf::*};
    use test_log::test;

    #[test]
    fn test_endgame() {
        let b = Board::parse_fen("k7/1p6/3N4/8/8/8/6N1/K6B w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);

        let b = Board::parse_fen("k7/8/3N4/8/8/8/8/K61 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KMk);

        let b = Board::parse_fen("k7/8/3n4/8/8/8/8/K6N w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KMkm);

        let b = Board::parse_fen("k7/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Kk);

        let b = Board::parse_fen("k7/8/8/8/8/8/6BB/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KBbk);

        let b = Board::parse_fen("kbb5/8/8/8/8/8/6BB/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);

        let b = Board::parse_fen("kbb5/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KkBb);
        let b = Board::parse_fen("kb1b4/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Kkbb);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::Draw);

        let b = Board::parse_fen("kb1n4/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Kkbn);

        let b = Board::parse_fen("8/k7/1p6/3N4/8/8/8/K7 w - - 5 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KMkp);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::WhiteLossOrDraw);

        let b = Board::parse_fen("Q7/1K6/8/8/8/8/6k1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KQk);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::WhiteWin);
        assert_eq!(eg.try_winner(&b), Some(Color::White));

        let b = Board::parse_fen("R7/1K6/8/8/8/8/6k1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KRk);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::WhiteWin);
        assert_eq!(eg.try_winner(&b), Some(Color::White));

        let b = Board::parse_fen("r7/1k6/8/8/8/8/6K1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Kkr);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::WhiteLoss);
        assert_eq!(eg.try_winner(&b), Some(Color::Black));

        let b = Board::parse_fen("r7/1k6/8/8/8/8/6K1/BB6 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);

        let b = Board::parse_fen("Q7/1K6/8/8/8/8/6kp/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KQkp);

        println!("{}", EndGame::counts_to_string() );
    }
}
