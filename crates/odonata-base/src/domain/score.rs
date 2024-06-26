use std::fmt;
use std::str::FromStr;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::infra::utils::Uci;
use crate::piece::MAX_PLY;
use crate::prelude::*;

// pub struct ScoreBound {
//     score: Score,
//     bound: NodeType,
// }

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
/// from the point of view of the player: +ve = winning, -ve = losing
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct WhiteScore(pub Score);

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_numeric() {
            self.cp.fmt(f)?; // call display on underlying int
            f.write_str("cp")
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

impl FromStr for Score {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_pgn_pawn_value(s)
    }
}

impl Score {
    pub fn pov_score(&self) -> Score {
        *self
    }

    pub fn parse_pgn_pawn_value(s: &str) -> anyhow::Result<Score> {
        // try cp suffix
        if let Some(s) = s.strip_suffix("cp") {
            let s = s.trim_end();
            if let Ok(cp) = s.parse::<i32>() {
                anyhow::ensure!(Score::within_range(cp), "out of range cp score");
                return Ok(Score::from_cp(cp));
            }
        }

        // try as floating point pawn value if contains "."
        if s.contains('.') {
            if let Ok(pawn) = s.parse::<f32>() {
                let cp = (pawn * 100.0) as i32;
                anyhow::ensure!(Score::within_range(cp), "out of range cp score");
                return Ok(Score::from_cp(cp));
            }
        }
        match s.split_once('M') {
            Some(("+", n)) => Ok(Score::from_mate_in_moves(
                n.parse().with_context(|| format!("parsing mate score '{s}')"))?,
            )),
            Some(("-", n)) => Ok(Score::from_mate_in_moves(
                -n.parse().with_context(|| format!("parsing mate score '{s}')"))?,
            )),
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
        match t.split_once(' ') {
            Some(("cp", text)) => {
                let cp = text.parse::<i32>().with_context(|| text.to_string())?;
                anyhow::ensure!(Self::within_range(cp), "centipawns {cp} out of range");
                Ok(Score::from_cp(cp))
            }
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
    pub fn from_white_cp(centipawn: i32, turn: Color) -> Score {
        Self::from_cp(turn.chooser_wb(centipawn, -centipawn))
    }

    pub fn as_white(&self, turn: Color) -> WhiteScore {
        WhiteScore(turn.chooser_wb(*self, self.negate()))
    }

    pub fn within_range(cp: i32) -> bool {
        cp.clamp(-Self::INF as i32, Self::INF as i32) == cp
    }

    fn assert_within_range(cp: i32) {
        debug_assert!(Self::within_range(cp), "centipawns {cp} out of range");
    }

    #[inline]
    pub fn from_cp(centipawn: i32) -> Score {
        Self::assert_within_range(centipawn);
        Score { cp: centipawn as i16 }
    }

    #[inline]
    pub fn from_f32(centipawn: f32) -> Score {
        Self::assert_within_range(centipawn as i32);
        Score { cp: centipawn as i16 }
    }

    #[inline]
    pub fn from_f64(centipawn: f64) -> Score {
        Self::assert_within_range(centipawn as i32);
        Score { cp: centipawn as i16 }
    }

    #[inline]
    pub fn as_i16(&self) -> i16 {
        self.cp
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

    /// score in centipawn pawn value, from pov stm
    /// -345
    /// +234
    /// -32000
    pub fn to_epd(&self) -> String {
        format!("{}", self.as_i16())
    }

    /// score in pawn value, with centipawns as the decimal
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
            Math::win_probability_from_cp_and_k(self.cp as f32, k)
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

impl Score {
    fn add_score(&self, o: Score) -> Score {
        debug_assert!(
            self.is_finite() && o.is_finite(),
            "cannot add scores when either is infinite: {self} + {o}"
        );
        debug_assert!(
            self.is_numeric() || self.is_mate() && (o.as_i16() == 1 || o.as_i16() == -1),
            "cannot add non unity scores to a mate score: {self} + {o}"
        );
        let oo = if !self.is_mate() {
            o
        } else {
            Score::from_cp(o.cp.signum() as i32)
        };

        if oo > 0.cp() {
            Score {
                cp: self.cp.saturating_add(oo.cp),
            }
        } else if o == 0.cp() {
            *self
        } else {
            Score {
                cp: self.cp.saturating_sub(-oo.cp),
            }
        }
    }

    fn sub_score(&self, o: Score) -> Score {
        self.add_score(-o)
    }
}

impl std::ops::Add for Score {
    type Output = Self;

    // we allow adding of 1 to a mate score (null window around mate)
    #[inline]
    fn add(self, o: Self) -> Self {
        self.add_score(o)
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
        self.sub_score(o)
    }
}

impl std::ops::Neg for Score {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.negate()
    }
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use test_log::test;

    use super::*;
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
        assert_eq!(-(-(-Score::INFINITY)), (-Score::INFINITY));
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
        assert_eq!(Score::from_cp(100) + Score::from_cp(150), Score::from_cp(250));

        // subtraction
        assert_eq!(Score::from_cp(100) - Score::from_cp(150), Score::from_cp(-50));

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
    fn test_score_serde() -> anyhow::Result<()> {
        let s = Score::from_cp(123);
        assert_eq!(serde_json::to_string(&s).unwrap(), "123");
        assert_eq!(s, serde_json::from_str("123").unwrap());
        // assert_eq!(s, serde_json::from_str("123").unwrap());
        Ok(())
    }

    #[test]
    fn test_pgn_score() -> anyhow::Result<()> {
        assert_eq!(Score::from_cp(100).to_pgn(), "+1.00".to_string());
        assert_eq!(Score::from_cp(0).to_pgn(), "+0.00".to_string());
        assert_eq!(Score::from_cp(-870).to_pgn(), "-8.70".to_string());
        assert_eq!(Score::from_mate_in_moves(5).to_pgn(), "+M5".to_string());
        assert_eq!(Score::from_mate_in_moves(-3).to_pgn(), "-M3".to_string());

        assert_eq!(Score::parse_pgn_pawn_value("+1.00")?, Score::from_cp(100));
        assert_eq!(Score::parse_pgn_pawn_value("100cp")?, Score::from_cp(100));
        assert_eq!(Score::parse_pgn_pawn_value("100 cp")?, Score::from_cp(100));
        assert_eq!(Score::parse_pgn_pawn_value("+0.00")?, Score::from_cp(0));
        assert_eq!(Score::parse_pgn_pawn_value("-8.70")?, Score::from_cp(-870));
        assert_eq!(Score::parse_pgn_pawn_value("+M5")?, Score::from_mate_in_moves(5));
        assert_eq!(Score::parse_pgn_pawn_value("-M3")?, Score::from_mate_in_moves(-3));

        assert_eq!(Score::from_cp(100), "+1.00".parse()?);
        assert_eq!(Score::from_cp(-100), "-1.00".parse()?);
        assert_eq!(Score::from_cp(1), "1cp".parse()?);
        assert_eq!(true, "1".parse::<Score>().is_err());

        Ok(())
    }

    // panic is caused by debug assertions - so only run this test in debug
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn test_score_panic1() {
        assert_eq!((-Score::INFINITY) + Score::from_cp(150), (-Score::INFINITY));
    }

    #[test]
    #[should_panic]
    fn test_score_panic2() {
        assert_eq!(Score::we_win_in(1) + Score::from_cp(150), Score::we_win_in(1));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn test_score_panic3() {
        assert_eq!(-2 * (-Score::INFINITY), Score::INFINITY);
    }

    #[test]
    fn score_fmt() {
        assert_eq!(format!("{}", Score::from_cp(1000)), "1000cp");
        assert_eq!(format!("{}", Score::INFINITY), "+inf");
        assert_eq!(format!("{}", -Score::INFINITY), "-inf");
        assert_eq!(format!("{}", -(-Score::INFINITY)), "+inf");

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
        let mut p = PerfProfiler::new("standard_exp");
        p.start();
        for cp in (-1000..1000).step_by(100) {
            black_box(Math::win_probability_from_cp_and_k(cp as f32, 4.0));
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
