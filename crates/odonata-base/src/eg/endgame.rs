use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumCount, IntoStaticStr};

use crate::prelude::*;
use crate::trace::stat::{SliceStat, Stat};
use crate::PreCalc;

/// Recognize several known end games, and determine
/// (a) what the action should be in search (stop / continue / reduce depth)
/// (b) An eval tweak to ensure we iterate towards mate
///
/// Color is side ahead in material
/// ///
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LikelyOutcome {
    UnknownOutcome,
    WhiteWin,
    WhiteWinOrDraw,
    LikelyDraw,
    DrawImmediate,
    WhiteLossOrDraw,
    WhiteLoss,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EndGameScoring {
    enabled:           bool,
    win_bonus:         Score,
    certain_win_bonus: Score,
    likely_draw_scale: f32,
    scale_by_hmvc:     bool,
}

impl Default for EndGameScoring {
    fn default() -> Self {
        Self {
            enabled:           true,
            win_bonus:         0.cp(),
            certain_win_bonus: 1000.cp(),
            likely_draw_scale: 1.0,
            scale_by_hmvc:     true,
        }
    }
}

impl Configurable for EndGameScoring {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.win_bonus.set(p.get("win_bonus"))?;
        self.certain_win_bonus.set(p.get("certain_win_bonus"))?;
        self.likely_draw_scale.set(p.get("likely_draw_scale"))?;
        self.scale_by_hmvc.set(p.get("scale_by_hmvc"))?;
        Ok(p.is_modified())
    }
}

#[derive(Copy, Default, Clone, PartialEq, Debug, IntoStaticStr, EnumCount, Display)]
pub enum EndGame {
    #[default]
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

    KNNkn,
    KNNkb,
    KNknn,
    KBknn,

    KBBkn,
    KBBkb,
    KNkbb,
    KBkbb,

    KBNkn,
    KBNkb,
    KNkbn,
    KBkbn,
}

use static_init::dynamic;
#[dynamic]
static ENDGAME_COUNTS: Vec<Stat> = {
    let vec = vec![];
    // for eg in EndGame::iter() {
    //     let s: &'static str = eg.into();
    //     vec.push(Stat::new(s));
    // }
    vec
};

impl EndGame {
    pub fn likely_outcome(&self, _b: &Board) -> LikelyOutcome {
        use EndGame::*;
        use LikelyOutcome::*;
        match self {
            Unknown => UnknownOutcome,
            Kk | KMk | Kkm | KNNk | Kknn => DrawImmediate,

            KMkm => LikelyDraw,
            KBBk => LikelyDraw,
            Kkbb => LikelyDraw,

            KRk | KQk | KBNk | KBbk | KJJk | KJMk => WhiteWin,
            Kkr | Kkq | Kkbn | KkBb | Kkjj | Kkjm => WhiteLoss,

            // // Guesses
            KPkp => LikelyDraw,

            KMkp => WhiteLossOrDraw,
            KPkm => WhiteWinOrDraw,

            KQkp => WhiteWin,
            KPkq => WhiteLoss,

            KRkp => UnknownOutcome,
            KPkr => UnknownOutcome,

            // //
            KPk => WhiteWinOrDraw,
            Kkp => WhiteLossOrDraw,

            KRkb => LikelyDraw, // usually
            KRkn => LikelyDraw, // usually
            KBkr => LikelyDraw, // usually
            KNkr => LikelyDraw, // usually
            KQkm => WhiteWin,
            KMkq => WhiteLoss,

            // KRkr => UnknownOutcome,
            KQkr => WhiteWin,
            KRkq => WhiteLoss,
            // KQkq => UnknownOutcome,
            KPPk => WhiteWin,
            Kkpp => WhiteLoss,

            KPPPk => WhiteWin,
            Kkppp => WhiteLoss,

            KNPk => WhiteWinOrDraw,
            Kknp => WhiteLossOrDraw,
            KBPk => WhiteWinOrDraw,
            Kkbp => WhiteLossOrDraw,

            // KNNkn => Draw,
            // KNNkb => Draw,
            // KNknn => Draw,
            // KBknn => Draw,

            // KBBkn => Draw,
            // KBBkb => Draw,
            // KNkbb => Draw,
            // KBkbb => Draw,

            // KBNkn => Draw,
            // KBNkb => Draw,
            // KNkbn => Draw,
            // KBkbn => Draw,
            _ => UnknownOutcome,
        }
    }

    // metrics we want to minimise as a checkmater
    pub fn metrics(&self, winner: Color, b: &Board) -> Option<(i32, i32)> {
        use crate::eg::EndGame as Eg;
        let loser = winner.flip_side();
        match self {
            Eg::KBNk | Eg::Kkbn => {
                use std::cmp::max;
                let ksq = b.king(loser);
                let wksq = b.king(winner);
                let endgame_metric1 = 40 * Self::king_distance_to_bishops_corner(b, ksq, wksq);
                let king_distance = Self::king_distance(b);
                let nsq = (b.knights() & b.color(winner))
                    .find_first_square()
                    .expect("missing knight");
                let bsq = (b.bishops() & b.color(winner))
                    .find_first_square()
                    .expect("missing bishop");
                let knight_distance = max(0, PreCalc::instance().chebyshev_distance(nsq, ksq));
                let bishop_distance = max(0, PreCalc::instance().chebyshev_distance(bsq, ksq));
                let endgame_metric2 = 20 * king_distance
                    + 2 * bishop_distance
                    + 3 * knight_distance
                    + 2 * Self::king_distance_to_side(b, loser);
                Some((endgame_metric1, endgame_metric2))
            }

            Eg::KBbk | Eg::KkBb => {
                let endgame_metric1 = 30 * Self::king_distance_to_any_corner(b, loser);
                let endgame_metric2 = 20 * Self::king_distance(b);
                Some((endgame_metric1, endgame_metric2))
            }

            Eg::KRk | Eg::Kkr | Eg::KQk | Eg::Kkq => {
                let endgame_metric1 = 30 * Self::king_distance_to_side(b, loser);
                let endgame_metric2 = 20 * Self::king_distance(b);
                Some((endgame_metric1, endgame_metric2))
            }
            _ => Option::None,
        }
    }

    pub fn endgame_score_adjust(&self, b: &Board, mut pov: Score, es: &EndGameScoring) -> Score {
        if !es.enabled {
            return pov;
        }
        if es.scale_by_hmvc {
            pov = Score::from_f32((100 - b.halfmove_clock()) as f32 / 100.0 * pov.as_i16() as f32);
        }
        if let Some(winner) = self.likely_winner(b) {
            let us = winner == b.turn();
            if let Some((metric1, metric2)) = self.metrics(winner, b) {
                pov = if us {
                    pov + 3 * Score::from_cp(-metric1) + 3 * Score::from_cp(-metric2)
                } else {
                    pov + 3 * Score::from_cp(metric1) + 3 * Score::from_cp(metric2)
                };

                // win specific scoring, so we award win_bonus as other features will be ignored
                if us {
                    pov = pov + es.win_bonus;

                    // as we wont be adding structural or mobility things we add another bonus
                    pov = pov + es.certain_win_bonus;
                } else {
                    pov = pov - es.win_bonus;

                    // as we wont be adding structural or mobility things we add another bonus
                    pov = pov - es.certain_win_bonus;
                }

                return pov;
            }
            // award a win bonus even if we dont have win-specific scoring
            if us {
                pov = pov + es.win_bonus;
            } else {
                pov = pov - es.win_bonus;
            }
            return pov;
        }

        match self.likely_outcome(b) {
            LikelyOutcome::LikelyDraw => {
                return Score::from_f32(es.likely_draw_scale * pov.as_i16() as f32);
            }
            LikelyOutcome::DrawImmediate => {
                return Score::DRAW;
            }
            _ => {}
        };

        pov
    }

    fn king_distance(b: &Board) -> i32 {
        let wk = b.king(Color::White);
        let bk = b.king(Color::Black);
        PreCalc::instance().chebyshev_distance(wk, bk)
    }

    fn king_distance_to_side(b: &Board, c: Color) -> i32 {
        use std::cmp::min;
        let ksq = (b.kings() & b.color(c)).find_first_square();
        if let Some(ksq) = ksq {
            let r = ksq.rank_index() as i32;
            let f = ksq.file_index() as i32;
            let m1 = min(r, f);
            let m2 = min(7 - r, 7 - f);
            min(m1, m2)
        } else {
            0
        }
    }

    fn king_distance_to_any_corner(b: &Board, c: Color) -> i32 {
        use std::cmp::min;
        let ksq = (b.kings() & b.color(c)).find_first_square();
        if let Some(ksq) = ksq {
            let d1 = PreCalc::instance().chebyshev_distance(Square::A1, ksq);
            let d2 = PreCalc::instance().chebyshev_distance(Square::A8, ksq);
            let d3 = PreCalc::instance().chebyshev_distance(Square::H1, ksq);
            let d4 = PreCalc::instance().chebyshev_distance(Square::H8, ksq);
            min(min(d1, d2), min(d3, d4))
        } else {
            0
        }
    }

    fn king_distance_to_bishops_corner(b: &Board, loser_ksq: Square, winner_ksq: Square) -> i32 {
        // we assume only one side has bishops
        let bis = b.bishops();
        let bad_corner1;
        let bad_corner2;
        // let gd_corner1;
        // let gd_corner2;
        // for losing king, these are undesirable corners
        if bis.intersects(Bitboard::WHITE_SQUARES) {
            bad_corner1 = Square::H1;
            bad_corner2 = Square::A8;
            // gd_corner1 = Square::A1;
            // gd_corner2 = Square::H8;
        } else {
            bad_corner1 = Square::A1;
            bad_corner2 = Square::H8;
            // gd_corner1 = Square::H1;
            // gd_corner2 = Square::A8;
        };

        // losing king distance to bad corner
        let bad_d1 = PreCalc::instance().manhattan_distance(bad_corner1, loser_ksq);
        let gd_d1 = PreCalc::instance().manhattan_distance(bad_corner1, winner_ksq);
        let bad_d2 = PreCalc::instance().manhattan_distance(bad_corner2, loser_ksq);
        let gd_d2 = PreCalc::instance().manhattan_distance(bad_corner2, winner_ksq);

        // we need winner king to be further from the corner than losing king
        let d1 = if bad_d1 < gd_d1 { bad_d1 } else { bad_d2 };
        let d2 = if bad_d2 < gd_d2 { bad_d2 } else { bad_d1 };
        std::cmp::min(d1, d2)
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
    pub fn likely_winner(&self, b: &Board) -> Option<Color> {
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
        // ENDGAME_COUNTS[eg as usize].increment();
        eg
    }

    #[inline]
    fn private_ctor(b: &Board) -> Self {
        let n_pieces = b.occupied().popcount();

        if n_pieces >= 6 {
            return Self::Unknown;
        }

        if n_pieces == 2 {
            return Self::Kk;
        }

        let wp = (b.pawns() & b.white()).popcount();
        let bp = (b.pawns() & b.black()).popcount();
        let n_pawns = wp + bp;
        if wp == 3 && bp == 0 {
            return Self::KPPPk;
        }
        if bp == 3 && wp == 0 {
            return Self::Kkppp;
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
        if n_pieces >= 5 && (b.rooks_or_queens().any() || b.pawns().any()) {
            return Self::Unknown;
        }
        if n_pieces == 5 {
            return EndGame::Unknown;
            // match (wn, wb, bn, bb) {
            //     //     (2,0,1,0) => return EndGame::KNNkn,
            //     //     (2,0,0,1) => return EndGame::KNNkb,
            //     //     (1,0,2,0) => return EndGame::KNknn,
            //     //     (0,1,2,0) => return EndGame::KBknn,
            //     //     (1,1,1,0) => return EndGame::KBNkn,
            //     //     (1,1,0,1) => return EndGame::KBNkb,
            //     //     (1,0,1,1) => return EndGame::KNkbn,
            //     //     (0,1,1,1) => return EndGame::KBkbn,
            //     //     (0,2,1,0) => return EndGame::KBBkn,
            //     //     (0,2,0,1) => return EndGame::KBBkb,
            //     //     (1,0,0,2) => return EndGame::KNkbb,
            //     //     (0,1,0,2) => return EndGame::KBkbb,
            //     _ => return EndGame::Unknown,
            // }
        }

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
                match wb {
                    2 => return EndGame::KBBk,
                    _ => return EndGame::Kkbb,
                };
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
    use std::hint::black_box;

    use test_log::test;

    use super::*;
    use crate::infra::profiler::PerfProfiler;

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

        // @todo
        // let b = Board::parse_fen("kbb5/8/8/8/8/8/8/KN6 w - - 0 1").unwrap();
        // let eg = EndGame::from_board(&b);
        // assert_eq!(eg, EndGame::KNkbb);
        // assert_eq!(eg.likely_outcome(&b), LikelyOutcome::Draw);

        let b = Board::parse_fen("knn5/8/8/8/8/8/8/KBB5 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);

        let b = Board::parse_fen("kb1b4/8/8/8/8/8/8/K7 w - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Kkbb);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::LikelyDraw);

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
        assert_eq!(eg.likely_winner(&b), Some(Color::White));

        let b = Board::parse_fen("R7/1K6/8/8/8/8/6k1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KRk);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::WhiteWin);
        assert_eq!(eg.likely_winner(&b), Some(Color::White));

        let b = Board::parse_fen("r7/1k6/8/8/8/8/6K1/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Kkr);
        assert_eq!(eg.likely_outcome(&b), LikelyOutcome::WhiteLoss);
        assert_eq!(eg.likely_winner(&b), Some(Color::Black));

        let b = Board::parse_fen("r7/1k6/8/8/8/8/6K1/BB6 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::Unknown);

        let b = Board::parse_fen("Q7/1K6/8/8/8/8/6kp/8 b - - 0 1").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KQkp);

        println!("{}", EndGame::counts_to_string());
    }

    #[test]
    fn test_eg_score() {
        let b = Board::parse_fen("2k5/7Q/8/8/8/3K4/8/8 w - - 3 1 ").unwrap();
        let eg = EndGame::from_board(&b);
        assert_eq!(eg, EndGame::KQk);
        let es = EndGameScoring::default();
        let _sc = eg.endgame_score_adjust(&b, Score::zero(), &es);
        // assert_eq!(sc, Score::from_cp(0 - 50));
    }

    #[test]
    fn bench_endgame() {
        let mut prof1 = PerfProfiler::new("endgame-ctor");
        let mut prof2 = PerfProfiler::new("outcome-enum");
        let mut prof3 = PerfProfiler::new("material-insuff2");

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
