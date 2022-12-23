use crate::infra::utils::win_probability_from_cp_and_k;
use crate::{
    infra::utils::Uci,
    piece::{Ply, MAX_PLY},
};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fmt;

// pub struct ScoreBound {
//     score: Score,
//     bound: NodeType,
// }

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
// from the point of view of the player: +ve = winning, -ve = losing
pub struct Score {
    cp: i16,
}

pub trait ToScore {
    fn cp(self) -> Score;
}

impl ToScore for i32 {
    fn cp(self) -> Score {
        Score::from_cp(self)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct WhiteScore(pub Score);

impl WhiteScore {
    pub fn as_white_cp(&self) -> i16 {
        self.0.as_i16()
    }
}

impl fmt::Display for WhiteScore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_numeric() {
            write!(f, "{}", self.0.cp)
        } else if self.0.cp == -Score::INF {
            f.write_str("-")
        } else if self.0.cp == Score::INF {
            f.write_str("+inf")
        } else if self.0.cp < 0 {
            write!(f, "B({})", self.0.ply_loss())
        } else {
            write!(f, "W({})", self.0.ply_win())
        }
    }
}

impl Score {
    pub fn parse_pgn(s: &str) -> anyhow::Result<Score> {
        if let Ok(cp) = s.parse::<f32>() {
            return Ok(Score::from_cp((cp * 100.0) as i32));
        }
        match s.split_once("M") {
            Some(("+", s)) => Ok(Score::from_mate_in_moves(s.parse()?)),
            Some(("-", s)) => Ok(Score::from_mate_in_moves(-s.parse()?)),
            _ => anyhow::bail!("unable to parse pgn score '{s}'"),
        }
    }
}

impl Uci for Score {
    // * score
    // 	* cp
    // 		the score from the engine's point of view in centipawns.
    // 	* mate
    // 		mate in y moves, not plies.
    // 		If the engine is getting mated use negativ values for y.
    // 	* lowerbound
    //       the score is just a lower bound.
    // 	* upperbound
    // 	   the score is just an upper bound.
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // we assume we are now from engines's point of view
        if self.cp == -Self::INF {
            write!(f, "cp -9999")
        } else if self.cp == i16::MAX {
            write!(f, "cp 9999")
        } else if self.is_numeric() {
            write!(f, "cp {}", self.cp)
        // } else if self.cp < 0 {
        //     write!(f, "mate {}", self.mate_in().unwrap())
        } else if self.is_mate() {
            write!(f, "mate {}", self.mate_in().unwrap())
        } else {
            panic!("unable to format score {self:?}");
        }
    }

    /// eg "cp 345" or mate 4
    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let t = s.replace("upperbound", "").trim().to_string();
        let t = t.replace("lowerbound", "").trim().to_string();
        match t.split_once(" ") {
            Some(("cp", text)) => Ok(Score::from_cp(
                text.parse::<i32>().with_context(|| text.to_string())?,
            )),
            Some(("mate", text)) => Ok(Score::from_mate_in_moves(text.parse::<i32>()?)),
            _ => anyhow::bail!("expected score to have cp or mate but found '{s}'"),
        }
    }
}

// WHITE LOSS IN 0<=N< MAX_PLY   is  -Self::INF + 1 + N
// WHITE WIN IN 0<=N< MAX_PLY   is  i16::MAX -1 - N
// MIN + 1 + MAX_PLY and i16::MAX -1 - MAX_PLY
impl Score {
    pub const fn zero() -> Score {
        Score { cp: 0 }
    }

    // note MAX = 32767 but MIN = -32768. So we use -MAX
    // pub const MINUS_INF : Score = Score { cp: -i16::MAX };
    pub const INFINITY: Score = Score { cp: i16::MAX };
    pub const DRAW: Score = Score::zero();

    const INF: i16 = i16::MAX;

    #[inline]
    pub fn from_cp(centipawn: i32) -> Score {
        debug_assert!(
            centipawn.clamp(-Self::INF as i32, Self::INF as i32) == centipawn,
            "centipawns {} out of range",
            centipawn
        );
        Score {
            cp: centipawn as i16,
        }
        // Score { cp: centipawn.clamp(-Self::INF as i32, Self::INF as i32) as i16 }  // adds 4% to eval
    }

    #[inline]
    pub fn from_f32(centipawn: f32) -> Score {
        debug_assert!(
            (centipawn as i32).clamp(-Self::INF as i32, Self::INF as i32) == centipawn as i32,
            "centipawns {} out of range",
            centipawn
        );
        Score {
            cp: centipawn as i16,
        }
        // Score { cp: centipawn.clamp(-Self::INF as i32, Self::INF as i32) as i16 }  // adds 4% to eval
    }

    #[inline]
    pub fn as_i16(&self) -> i16 {
        self.cp as i16
    }

    // #[inline]
    // pub fn Cp(centipawn: i32) -> Score {
    //     Score { cp: centipawn as i16 }
    // }

    #[inline]
    pub fn cp(self) -> Option<i16> {
        if self.is_numeric() {
            Some(self.cp)
        } else {
            None
        }
    }

    pub fn in_window(&self, lower: Score, upper: Score) -> bool {
        *self >= lower && *self <= upper
    }

    /// +1.35
    /// -0.34
    /// +M8
    /// -M5
    pub fn to_pgn(&self) -> String {
        if let Some(cp) = self.cp() {
            format!("{eval:+0.2}", eval = cp as f32 / 100.0)
        } else if let Some(x) = self.mate_in() {
            if x >= 0 {
                format!("+M{x}")
            } else {
                format!("-M{y}", y = x.abs())
            }
        } else if *self == Score::INFINITY {
            "+inf".to_string()
        } else if *self == -Score::INFINITY {
            "-inf".to_string()
        } else {
            unreachable!("Formatting score {self:?} as pgn")
        }
    }

    const MIN_NUMERIC: i16 = -Self::INF + 1 + MAX_PLY as i16;
    const MAX_NUMERIC: i16 = i16::MAX - 1 - MAX_PLY as i16;

    #[inline]
    pub const fn is_numeric(&self) -> bool {
        self.cp >= Self::MIN_NUMERIC && self.cp <= Self::MAX_NUMERIC
    }

    #[inline]
    pub const fn is_finite(&self) -> bool {
        self.cp > -Self::INF && self.cp < Self::INF
    }

    #[inline]
    pub fn clamp_score(&self) -> Score {
        Self::from_cp(self.cp.clamp(-20000, 20000) as i32)
    }

    #[inline]
    /// ply  <---> moves
    ///
    pub fn from_mate_in_moves(moves: i32) -> Score {
        match moves {
            x if x < 0 => Score::we_lose_in(-moves * 2 - 1),
            _ => Score::we_win_in(moves * 2 - 1),
        }
    }

    #[inline]
    pub const fn we_win_in(ply: Ply) -> Score {
        Score {
            cp: i16::MAX - 1 - ply as i16,
        }
    }

    #[inline]
    pub fn we_lose_in(ply: Ply) -> Score {
        Score {
            cp: -Self::INF + 1 + ply as i16,
        }
    }

    // #[inline]
    // pub fn to_root_score(&self, _current_depth: Ply) -> Score {
    //     *self
    // }

    #[inline]
    pub const fn is_mate(&self) -> bool {
        !(self.cp == -Self::INF || self.cp == Self::INF || self.is_numeric())
    }

    #[inline]
    pub fn ply_loss(&self) -> Ply {
        (self.cp + Self::INF - 1) as Ply
    }

    #[inline]
    pub fn ply_win(&self) -> Ply {
        (i16::MAX - 1 - self.cp) as Ply
    }

    // engine -> oppo -> engine -> 3 plys == mate in 2
    // engine -> oppo -> 2 plys == mated in 1
    // engine -> oppo -> engine -> opp  4 plys == mated in 2
    // engine -> oppo -> engine -> opp -> eng  5 plys == mated in 3
    #[inline]
    pub fn mate_in(&self) -> Option<i32> {
        if !self.is_mate() {
            None
        } else if self.cp < 0 {
            Some(-(self.ply_loss() + 1) / 2)
        } else {
            Some((self.ply_win() + 1) / 2)
        }
    }

    #[inline]
    pub fn negate(self) -> Score {
        Score { cp: -self.cp }
    }

    #[inline]
    pub fn win_probability(self) -> f32 {
        self.win_probability_using_k(4.0)
    }

    #[inline]
    pub fn win_probability_using_k(self, k: f32) -> f32 {
        if self.is_numeric() {
            win_probability_from_cp_and_k(self.cp as f32, k)
        } else if self.cp > 0 {
            1.0
        } else {
            0.0
        }
    }
}

impl Default for Score {
    #[inline]
    fn default() -> Self {
        -Self::INFINITY
    }
}

impl std::ops::Add for Score {
    type Output = Self;

    // we allow adding of 1 to a mate score (null window around mate)
    #[inline]
    fn add(self, o: Self) -> Self {
        // debug_assert!(
        //     self.is_numeric() && o.is_numeric(),
        //     "cannot add scores {self} + {o}"
        // );
        // debug_assert!(
        //     self.is_numeric() || self.is_mate() && (o.as_i16() == 1 || o.as_i16() == -1),
        //     "cannot add scores {self} + {o}"
        // );
        return Score {
            cp: self.cp.saturating_add(o.cp),
        };
    }
}

impl std::ops::Mul<Score> for i32 {
    type Output = Score;

    #[inline]
    fn mul(self, o: Score) -> Score {
        debug_assert!(o.is_numeric(), "Score {o} cannot be multipled by {self}");
        Score {
            cp: (self as i16).saturating_mul(o.cp),
        }
    }
}

impl std::ops::Sub for Score {
    type Output = Self;

    #[inline]
    fn sub(self, o: Self) -> Self {
        debug_assert!(
            self.is_finite() && o.is_finite(),
            "Score {self} + {o} cannot subtract infinities"
        );
        debug_assert!(
            !self.is_mate() && !o.is_mate(),
            "Score {self} - {o} subtraction with mate scores"
        );
        Score {
            cp: self.cp.saturating_sub(o.cp),
        }
    }
}

impl std::ops::Neg for Score {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.negate()
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_numeric() {
            write!(f, "{}", self.cp)
        } else if self.cp == -Self::INF {
            f.write_str("-inf")
        } else if self.cp == i16::MAX {
            f.write_str("+inf")
        } else if self.cp < 0 {
            write!(f, "L({})", self.ply_loss())
        } else {
            write!(f, "W({})", self.ply_win())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::black_box;
    use crate::infra::profiler::*;

    #[test]
    fn score_basics() {
        assert_eq!(Score::from_cp(1).negate(), Score::from_cp(-1));
        assert_eq!(Score::we_win_in(1).negate(), Score::we_lose_in(1));
        assert_eq!(Score::we_win_in(0).negate(), Score::we_lose_in(0));
        assert_eq!(Score::we_win_in(0).is_mate(), true);
        assert_eq!(Score::we_lose_in(0).is_mate(), true);
        assert_eq!((-Score::INFINITY).is_mate(), false);
        assert_eq!(Score::INFINITY.is_mate(), false);
        assert_eq!(Score::from_cp(123).is_mate(), false);
        assert_eq!(Score::we_lose_in(1).negate(), Score::we_win_in(1));
        assert_eq!((-Score::INFINITY).negate(), Score::INFINITY);
        assert_eq!(-(-Score::INFINITY), Score::INFINITY);
        assert_eq!((-Score::INFINITY), -Score::INFINITY);
        assert_eq!(--(-Score::INFINITY), (-Score::INFINITY));
        assert_eq!((-Score::INFINITY).is_numeric(), false);
        assert_eq!(Score::INFINITY.is_numeric(), false);
        assert_eq!(Score::we_win_in(0).is_numeric(), false);
        assert_eq!(Score::we_lose_in(0).is_numeric(), false);
        assert_eq!(Score::we_lose_in(60).is_numeric(), false);
        assert_eq!(Score::we_win_in(60).is_numeric(), false);
        assert_eq!(Score::we_win_in(6) > Score::we_win_in(7), true);
        assert_eq!(Score::we_win_in(3) < Score::we_win_in(1), true);
        assert_eq!(Score::we_lose_in(4) > Score::we_lose_in(2), true); // better to lose in 4 moves
        assert!((-Score::INFINITY) < Score::INFINITY);
        assert_eq!((-Score::INFINITY).is_mate(), false);
        assert_eq!(Score::we_win_in(1).is_mate(), true);
        assert_eq!(Score::we_lose_in(3).is_mate(), true);
        assert!(Score::from_cp(-5) < Score::from_cp(5));
        assert!(Score::from_cp(5) < Score::we_win_in(0));
        assert!(Score::from_cp(100) > Score::from_cp(0));

        // addition
        assert_eq!(
            Score::from_cp(100) + Score::from_cp(150),
            Score::from_cp(250)
        );

        // subtraction
        assert_eq!(
            Score::from_cp(100) - Score::from_cp(150),
            Score::from_cp(-50)
        );

        assert_eq!(2 * Score::from_cp(100), Score::from_cp(200));
        assert_eq!(-2 * Score::from_cp(200), Score::from_cp(-400));
        // assert_eq!(-2 * (-Score::INFINITY), Score::INFINITY);
        // assert_eq!(-2 * Score::INFINITY, (-Score::INFINITY));
        // assert_eq!(1 * Score::INFINITY, Score::INFINITY);
        // assert_eq!(-1 * Score::we_win_in(2), Score::we_lose_in(2));
        // // changes sign bit not magnitude
        // assert_eq!(-3 * Score::we_win_in(2), Score::we_lose_in(2));
        // assert_eq!(1 * Score::we_win_in(2), Score::we_win_in(2));
        assert!(Score::we_win_in(1) < Score::INFINITY);
        assert_eq!(Score::we_win_in(0), Score::we_win_in(0));
        assert_eq!((-Score::INFINITY).clamp_score(), Score::from_cp(-20000));


        assert!(Score::from_cp(0).win_probability() > 0.499);
        assert!(Score::from_cp(0).win_probability() < 0.501);
        assert!(Score::from_cp(1000).win_probability() > 0.95);
        assert!(Score::from_cp(-1000).win_probability() < 0.05);
        assert!((-Score::INFINITY).win_probability() < 0.001);
    }

    #[test]
    fn test_mate_score() {
        assert_eq!(Score::we_win_in(1).ply_win(), 1);
        assert_eq!(Score::we_win_in(2).ply_win(), 2);
        assert_eq!(Score::we_win_in(3).ply_win(), 3);

        assert_eq!(Score::we_lose_in(1).ply_loss(), 1);
        assert_eq!(Score::we_lose_in(2).ply_loss(), 2);
        assert_eq!(Score::we_lose_in(3).ply_loss(), 3);

        assert_eq!(Score::from_mate_in_moves(1).mate_in(), Some(1));
        assert_eq!(Score::from_mate_in_moves(2).mate_in(), Some(2));
        assert_eq!(Score::from_mate_in_moves(3).mate_in(), Some(3));
        assert_eq!(Score::from_mate_in_moves(-1).mate_in(), Some(-1));
        assert_eq!(Score::from_mate_in_moves(-2).mate_in(), Some(-2));
        assert_eq!(Score::from_mate_in_moves(-3).mate_in(), Some(-3));
    }
    #[test]
    fn test_uci_score() -> anyhow::Result<()> {
        assert_eq!(Score::from_cp(100).to_uci(), "cp 100");
        assert_eq!(Score::from_cp(-1).to_uci(), "cp -1");
        assert_eq!(Score::from_mate_in_moves(3).to_uci(), "mate 3");
        assert_eq!(Score::from_mate_in_moves(5).to_uci(), "mate 5");
        assert_eq!(Score::from_mate_in_moves(-3).to_uci(), "mate -3");
        assert_eq!(Score::from_mate_in_moves(-5).to_uci(), "mate -5");
        assert_eq!(Score::parse_uci("cp 100")?.to_uci(), "cp 100");
        assert_eq!(Score::parse_uci("cp -1")?.to_uci(), "cp -1");
        assert_eq!(Score::parse_uci("mate 3")?.to_uci(), "mate 3");
        assert_eq!(Score::parse_uci("mate -3")?.to_uci(), "mate -3");
        Ok(())
    }

    #[test]
    fn test_pgn_score() -> anyhow::Result<()> {
        assert_eq!(Score::from_cp(100).to_pgn(), "+1.00".to_string());
        assert_eq!(Score::from_cp(0).to_pgn(), "+0.00".to_string());
        assert_eq!(Score::from_cp(-870).to_pgn(), "-8.70".to_string());
        assert_eq!(Score::from_mate_in_moves(5).to_pgn(), "+M5".to_string());
        assert_eq!(Score::from_mate_in_moves(-3).to_pgn(), "-M3".to_string());

        assert_eq!(Score::parse_pgn("+1.00")?, Score::from_cp(100));
        assert_eq!(Score::parse_pgn("+0.00")?, Score::from_cp(0));
        assert_eq!(Score::parse_pgn("-8.70")?, Score::from_cp(-870));
        assert_eq!(Score::parse_pgn("+M5")?, Score::from_mate_in_moves(5));
        assert_eq!(Score::parse_pgn("-M3")?, Score::from_mate_in_moves(-3));

        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_score_panic1() {
        assert_eq!((-Score::INFINITY) + Score::from_cp(150), (-Score::INFINITY));
    }

    #[test]
    #[should_panic]
    fn test_score_panic2() {
        assert_eq!(
            Score::we_win_in(1) + Score::from_cp(150),
            Score::we_win_in(1)
        );
    }

    #[test]
    #[should_panic]
    fn test_score_panic3() {
        assert_eq!(-2 * (-Score::INFINITY), Score::INFINITY);
    }

    #[test]
    fn score_fmt() {
        assert_eq!(format!("{}", Score::from_cp(1000)), "1000");
        assert_eq!(format!("{}", Score::INFINITY), "+inf");
        assert_eq!(format!("{}", -Score::INFINITY), "-inf");
        assert_eq!(format!("{}", --Score::INFINITY), "+inf");

        assert_eq!(format!("{}", Score::we_win_in(2)), "W(2)");
        assert_eq!(format!("{}", Score::we_lose_in(3)), "L(3)");
        assert_eq!(format!("{:>8}", Score::we_lose_in(3)), "L(3)");
        assert_eq!(format!("{:>5}", Score::we_lose_in(3).to_string()), " L(3)");
    }

    #[test]
    fn test_win_prob() {
        for cp in -300..300 {
            let s = Score::from_cp(cp);
            let wp = s.win_probability();
            println!("wp[cp]: {},{}", s, wp);
        }
    }

    #[test]
    fn bench_exp() {
        // b^x = e^(x*log(b))
        let mut p = PerfProfiler::new("standard_exp".into());
        p.start();
        for cp in (-1000..1000).step_by(100) {
            black_box(win_probability_from_cp_and_k(cp as f32, 4.0));
        }
        p.stop();

        // let mut p = Profiler::new("fast_exp".into());
        // p.start();
        // for cp in (-1000..1000).step_by(100) {
        //     black_box(Score::win_probability_from_cp_and_k_fast(cp as f32, 4.0));
        // }
        // p.stop();

        // for cp in (-1000..1000).step_by(100) {
        //     let fast = Score::win_probability_from_cp_and_k_fast(cp as f32, 4.0);
        //     let slow = Score::win_probability_from_cp_and_k(cp as f32, 4.0);

        //     println!("{} {}", fast, slow);
        //     assert!((fast - slow).abs() < 0.01);
        // }
    }
}
