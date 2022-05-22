use crate::board::Board;
use crate::trace::stat::{SliceStat, Stat};
use crate::types::Color;
use crate::Bitboard;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use strum_macros::IntoStaticStr;
use strum_macros::{Display, EnumCount};

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
    Kk,
    // 2v1
    KMk,
    Kkm,
    KRk,
    Kkr,
    KQk,
    Kkq,
    KPk,
    Kkp,

    // 2v kp
    KPkp,
    KMkp,
    KPkm,
    KRkp,
    KPkr,
    KQkp,

    KPkq,

    // 2v km
    KMkm,
    KRkb,
    KRkn,
    KBkr,
    KNkr,
    KQkm,
    KMkq,

    // 2v k+major
    KRkr,
    KQkr,
    KRkq,
    KQkq,

    // 3v k
    KPPk,
    Kkpp,
    KNPk,
    Kknp,
    KBPk,
    Kkbp,

    KNNk,
    Kknn,
    KBNk,
    Kkbn,
    KBbk,
    KBBk,
    KkBb,
    Kkbb,
    KJMk,
    Kkjm,
    KJJk,
    Kkjj,

    KPPPk,
    Kkppp,
}

use static_init::dynamic;
#[dynamic]
static ENDGAME_COUNTS: Vec<Stat> = {
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
            Kk | KMk | Kkm | KNNk | Kknn => DrawImmediate,

            KMkm | KBBk | Kkbb => Draw, // (helpmate?)

            KRk | KQk | KBNk | KBbk | KJJk | KJMk => WhiteWin,
            Kkr | Kkq | Kkbn | KkBb | Kkjj | Kkjm => WhiteLoss,

            // Guesses
            KPkp => Draw,

            KMkp => WhiteLossOrDraw,
            KPkm => WhiteWinOrDraw,

            KQkp => WhiteWin,
            KPkq => WhiteLoss,

            KRkp => UnknownOutcome,
            KPkr => UnknownOutcome,

            //
            KPk => WhiteWinOrDraw,
            Kkp => WhiteLossOrDraw,

            KRkb => Draw, // usually
            KRkn => Draw, // usually
            KBkr => Draw, // usually
            KNkr => Draw, // usually
            KQkm => WhiteWin,
            KMkq => WhiteLoss,

            KRkr => UnknownOutcome,
            KQkr => WhiteWin,
            KRkq => WhiteLoss,
            KQkq => UnknownOutcome,

            KPPk => WhiteWin,
            Kkpp => WhiteLoss,

            KPPPk => WhiteWin,
            Kkppp => WhiteLoss,

            KNPk => WhiteWinOrDraw,
            Kknp => WhiteLossOrDraw,
            KBPk => WhiteWinOrDraw,
            Kkbp => WhiteLossOrDraw,
        }
    }


    pub fn is_insufficient_material(bd: &Board) -> bool {
        // If both sides have any one of the following, and there are no pawns on the board:
        // 1. A lone king
        // 2. a king and bishop
        // 3. a king and knight
        // 4. K+B v K+B (same color Bs)
        //
        // queens, rooks or pawns => can still checkmate
        if bd.pawns().any() || bd.rooks().any() || bd.queens().any() {
            return false;
        }
        // can assume just bishops, knights and kings now
        let bishops_w = (bd.bishops() & bd.white()).popcount();
        let bishops_b = (bd.bishops() & bd.black()).popcount();
        let knights = bd.knights().popcount();
        if bishops_w + bishops_b + knights <= 1 {
            return true; // cases 1, 2 & 3
        }
        if knights == 0 && bishops_w == 1 && bishops_b == 1 {
            return true; // FIXME: color of bishop  case 4
        }
        false
    }

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

    #[inline]
    fn private_ctor(b: &Board) -> Self {
        let n_pieces = b.occupied().popcount();

        if n_pieces >= 6 {
            return Self::Unknown;
        }

        let wp = (b.pawns() & b.white()).popcount();
        let bp = (b.pawns() & b.black()).popcount();
        let n_pawns = wp + bp;
        if wp == 3 {
            return Self::KPPPk;
        }
        if bp == 3 {
            return Self::Kkppp;
        }

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
        //
        // now assume we have 4 pieces
        //
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
        if wr == 1 && bb == 1 {
            return EndGame::KRkb;
        }
        if wr == 1 && bn == 1 {
            return EndGame::KRkn;
        }
        if br == 1 && wb == 1 {
            return EndGame::KBkr;
        }
        if br == 1 && wn == 1 {
            return EndGame::KNkr;
        }
        Self::Unknown
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        board::boardbuf::*,
        infra::{black_box, profiler::Profiler},
    };
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

        println!("{}", EndGame::counts_to_string());
    }

    #[test]
    fn bench_endgame() {
        let mut prof1 = Profiler::new("endgame-ctor".into());
        let mut prof2 = Profiler::new("outcome-enum".into());
        let mut prof3 = Profiler::new("material-insuff".into());

        let b1 = Board::parse_fen("k7/8/8/8/NN6/8/8/K7 w - - 0 1").unwrap();
        let b2 = Board::parse_fen("k7/8/3N4/8/8/8/8/K61 w - - 0 1").unwrap();

        prof1.start();
        let lo1 = black_box(EndGame::from_board(&b1).likely_outcome(&b1));
        let lo2 = black_box(EndGame::from_board(&b2).likely_outcome(&b2));
        prof1.stop();

        prof2.start();
        let o1 = black_box(b1.material().is_insufficient());
        let o2 = black_box(b2.material().is_insufficient());
        prof2.stop();

        prof3.start();
        let _ = black_box(EndGame::is_insufficient_material(&b1));
        let _ = black_box(EndGame::is_insufficient_material(&b2));
        prof3.stop();

        assert_eq!(o1, true);
        assert_eq!(o2, true);
        assert_eq!(lo1, LikelyOutcome::DrawImmediate);
        assert_eq!(lo2, LikelyOutcome::DrawImmediate);
    }
}
