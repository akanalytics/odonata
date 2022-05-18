use crate::outcome::Outcome;
use crate::types::{Color, Ply, MAX_PLY};
use serde::{Deserialize, Serialize};
use std::fmt;

// pub struct ScoreBound {
//     score: Score,
//     bound: NodeType,
// }



#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Score {
    cp: i16,
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
        Score { cp: centipawn as i16 }
        // Score { cp: centipawn.clamp(-Self::INF as i32, Self::INF as i32) as i16 }  // adds 4% to eval
    }

    #[inline]
    pub fn from_f32(centipawn: f32) -> Score {
        debug_assert!(
            (centipawn as i32).clamp(-Self::INF as i32, Self::INF as i32) == centipawn as i32,
            "centipawns {} out of range",
            centipawn
        );
        Score { cp: centipawn as i16 }
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

    const MIN_NUMERIC: i16 = -Self::INF + 1 + MAX_PLY as i16;
    const MAX_NUMERIC: i16 = i16::MAX - 1 - MAX_PLY as i16;

    #[inline]
    pub const fn is_numeric(&self) -> bool {
        self.cp >= Self::MIN_NUMERIC && self.cp <= Self::MAX_NUMERIC
    }

    #[inline]
    pub const fn is_numeric_or_mate(&self) -> bool {
        self.cp > -Self::INF && self.cp < Self::INF
    }

    #[inline]
    pub fn clamp_score(&self) -> Score {
        Self::from_cp(self.cp.clamp(-20000, 20000) as i32)
    }

    #[inline]
    pub const fn white_win(ply: Ply) -> Score {
        Score {
            cp: i16::MAX - 1 - ply as i16,
        }
    }

    #[inline]
    pub fn white_loss(ply: Ply) -> Score {
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
    fn ply_loss(&self) -> Ply {
        (self.cp + Self::INF - 1) as Ply
    }

    #[inline]
    fn ply_win(&self) -> Ply {
        (i16::MAX - 1 - self.cp) as Ply
    }

    // engine -> oppo -> engine -> 3 plys == mate in 2
    // engine -> oppo -> 2 plys == mated in 1
    // engine -> oppo -> engine -> opp  4 plys == mated in 2
    // engine -> oppo -> engine -> opp -> eng  5 plys == mated in 3
    #[inline]
    pub fn mate_in(&self) -> Option<Ply> {
        if !self.is_mate() {
            None
        } else if self.cp < 0 {
            Some((self.ply_loss() + 1) / 2)
        } else {
            Some((self.ply_win() + 1) / 2)
        }
    }

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
    pub fn uci(self) -> String {
        // we assume we are now from engines's point of view
        if self.cp == -Self::INF {
            "cp -9999".to_string()
        } else if self.cp == i16::MAX {
            "cp 9999".to_string()
        } else if self.is_numeric() {
            format!("cp {}", self.cp)
        } else if self.cp < 0 {
            format!("mate {}", -self.mate_in().unwrap())
        } else {
            format!("mate {}", self.mate_in().unwrap())
        }
    }

    #[inline]
    pub fn negate(self) -> Score {
        Score { cp: -self.cp }
    }

    // #[inline]
    // pub fn negate(self) -> Score {
    //     match self {
    //         Self::MinusInf => Self::PlusInf,
    //         -Self::INF+1.. -Self::INF+1+MAX_PLY => Self::white_win(self.ply_loss()),
    //         i16::MAX-1-MAX_PLY .. i16::MAX-1=> Self::white_loss(self.ply_win()),,
    //         Self::PlusInf => Self::MinusInf,
    //         _ => Self::cp(-self.cp),
    //     }
    // }

    // https://www.chessprogramming.org/Pawn_Advantage,_Win_Percentage,_and_Elo
    #[inline]
    pub fn win_probability_from_cp_and_k(centipawns: f32, k: f32) -> f32 {
        1.0 / (1.0 + 10_f32.powf(-centipawns / (k * 100.0)))
    }



    #[inline]
    pub fn win_probability_from_cp_and_k_fast(centipawns: f32, k: f32) -> f32 {
        #[inline]
        fn pow10(b: f32) -> f32 {
            const LOG_OF_10: f32 = 2.302585125;  // ln(10.0)
            fast_math::exp(b * LOG_OF_10)
        }
        1.0 / (1.0 + pow10(-centipawns / (k * 100.0)))
    }

    #[inline]
    pub fn win_probability(self) -> f32 {
        self.win_probability_using_k(4.0)
    }

    #[inline]
    pub fn win_probability_using_k(self, k: f32) -> f32 {
        if self.is_numeric() {
            Self::win_probability_from_cp_and_k(self.cp as f32, k)
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

    #[inline]
    fn add(self, other: Self) -> Self {
        if let Some(s2) = other.cp() {
            if let Some(s1) = self.cp() {
                return Score::from_cp(s1 as i32 + s2 as i32);
            } else {
                return self; // if self is an infinite or mate then adding cp/mp makes no difference
            }
        }
        panic!("Can only add centipawns: can't add {} + {}", self, other);
    }
}

impl std::ops::Mul<Score> for i32 {
    type Output = Score;

    #[inline]
    fn mul(self, other: Score) -> Score {
        if other.is_numeric() {
            Score::from_cp(self * other.cp as i32)
        } else if self > 0 {
            other
        } else if self < 0 {
            other.negate()
        } else {
            Score::from_cp(0)
        }
    }
}

impl std::ops::Sub for Score {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        if let Some(s2) = other.cp() {
            if let Some(s1) = self.cp() {
                return Score::from_cp(s1 as i32 - s2 as i32);
            } else {
                return self; // if self is an infinite or mate then subtracting cp/mp makes no difference
            }
        }
        panic!("Can only subtract centipawns: can't subtract {} - {}", self, other);
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
            write!(f, "loss({})", self.ply_loss())
        } else {
            write!(f, "win({})", self.ply_win())
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Score2 {
    MinusInf,
    WhiteLoss { ply: i32 }, // WhiteLoss(1) < WhiteLoss(5)
    Cp(i32),
    WhiteWin { minus_ply: i32 }, // // WhiteWin(-5) < WhiteWin(-1)
    PlusInf,
}

impl Default for Score2 {
    #[inline]
    fn default() -> Self {
        Self::MinusInf
    }
}

impl Score2 {
    #[inline]
    pub fn cp(centipawn: i32) -> Score2 {
        Score2::Cp(centipawn)
    }

    pub fn to_root_score(&self, _current_depth: Ply) -> Score2 {
        // if current_depth % 2 == 1 {
        //     self.negate()
        // } else {
        *self
        // }
    }

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
    pub fn uci(self, _pov: Color) -> String {
        // let score = match pov {
        //     Color::White => self,
        //     Color::Black => self.negate(),
        // };

        // we assume we are now from engines's point of view
        match self {
            Self::MinusInf => "cp -9999".to_string(),
            Self::WhiteLoss { ply: _ } => format!("mate {}", -self.mate_in().unwrap()),
            Self::Cp(cp) => format!("cp {}", cp),
            Self::WhiteWin { minus_ply: _ } => format!("mate {}", self.mate_in().unwrap()),
            Self::PlusInf => "cp 9999".to_string(),
        }
    }

    pub fn side_to_move_score(tempo: i32, us: Color) -> Score2 {
        // axiom: were white
        // white to move => advantage, black to move means white has a disadvantage
        if us == Color::White {
            #[allow(clippy::erasing_op)]
            Score2::cp(tempo * 0)
        } else {
            #[allow(clippy::erasing_op)]
            Score2::cp(-tempo * 0)
        }
    }

    /// Outcome must be game ending else panic
    #[inline]
    pub fn score_from_outcome(contempt: i32, o: Outcome, us: Color, total_half_moves: Ply) -> Score2 {
        if o.is_draw() {
            // draw score is +ve for playing a stronger opponent (we want a draw), neg for weaker
            //
            //  Engine Col   |  search ply   |  value to searcher   | Score2 to white
            //     W               0                   +ve               +ve
            //     B               0                   +ve               -ve
            //     W               1 (oppo B)          -ve               +ve (a bonus to white opponet)
            //     B               1 (oppo W)          -ve               -ve
            // board.color_us() == Color::White => maximising
            // +ve contempt => +ve score => aim for draw => opponent stronger than us
            // board.color_us() == Color::Black => minimising
            // +ve contempt => -ve score => aim for draw => opponent stronger than us
            let contempt = us.chooser_wb(contempt, -contempt);
            return Score2::Cp(contempt);
        }
        if let Some(c) = o.winning_color() {
            return c.chooser_wb(
                Score2::WhiteWin {
                    minus_ply: -total_half_moves,
                },
                Score2::WhiteLoss { ply: total_half_moves },
            );
        }
        panic!("Tried to final score a non-final board outcome:{}", o);
    }

    // engine -> oppo -> engine -> 3 plys == mate in 2
    // engine -> oppo -> 2 plys == mated in 1
    // engine -> oppo -> engine -> opp  4 plys == mated in 2
    // engine -> oppo -> engine -> opp -> eng  5 plys == mated in 3
    pub fn mate_in(&self) -> Option<Ply> {
        match self {
            Self::WhiteLoss { ply } => Some((*ply + 1) / 2),
            Self::WhiteWin { minus_ply } => Some((-*minus_ply + 1) / 2),
            _ => None,
        }
    }

    pub fn is_mate(&self) -> bool {
        matches!(self, Self::WhiteLoss { ply: _ } | Self::WhiteWin { minus_ply: _ })
    }

    #[inline]
    pub fn negate(self) -> Score2 {
        match self {
            Self::MinusInf => Self::PlusInf,
            Self::WhiteLoss { ply } => Self::WhiteWin { minus_ply: -ply },
            Self::Cp(s) => Self::Cp(-s),
            Self::WhiteWin { minus_ply } => Self::WhiteLoss { ply: -minus_ply },
            Self::PlusInf => Self::MinusInf,
        }
    }

    // https://www.chessprogramming.org/Pawn_Advantage,_Win_Percentage,_and_Elo
    pub fn win_probability(&self) -> f32 {
        match &self {
            Self::MinusInf => 0.0,
            Self::WhiteLoss { ply: _ } => 0.0,
            Self::Cp(cp) => {
                let k = 4_f32;
                1.0 / (1.0 + 10_f32.powf(-cp as f32 / k))
            }
            Self::WhiteWin { minus_ply: _ } => 1.0,
            Self::PlusInf => 1.0,
        }
    }
}

impl std::ops::Add for Score2 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        if let Score2::Cp(s2) = other {
            if let Score2::Cp(s1) = self {
                return Score2::Cp(s1 + s2);
            } else {
                return self; // if self is an infinite or mate then adding cp/mp makes no difference
            }
        }
        panic!("Can only add centipawns not {} + {}", self, other);
    }
}

impl std::ops::Mul<Score2> for i32 {
    type Output = Score2;

    #[inline]
    fn mul(self, other: Score2) -> Score2 {
        match other {
            Score2::Cp(s) => Score2::Cp(self * s),
            _ if self > 0 => other,
            _ if self < 0 => -other,
            _ => Score2::Cp(0),
        }
    }
}

impl std::ops::Sub for Score2 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        if let Score2::Cp(s2) = other {
            if let Score2::Cp(s1) = self {
                return Score2::Cp(s1 - s2);
            } else {
                return self; // if self is an infinite or mate then subtracting cp/mp makes no difference
            }
        }
        panic!("Can only subtract centipawns not {} - {}", self, other);
    }
}

impl std::ops::Neg for Score2 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        self.negate()
    }
}

impl fmt::Display for Score2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::MinusInf => f.write_str("-inf"),
            Self::WhiteLoss { ply } => write!(f, "win({})", ply),
            Self::Cp(cp) => write!(f, "{}", cp),
            Self::WhiteWin { minus_ply } => write!(f, "loss({})", -minus_ply),
            Self::PlusInf => f.write_str("+inf"),
        }
    }
}

// score config needs to be by colour and by MG/EG
// option to have minimizing nodes use different config
// what can we cache
// pass in alpha beta so eval can short circuit (lazy evaluation)
// some human-like tweaks: aggresive/defensive, open/closed preference, test an opening, lay traps, complicate the position,
// consider odd / even parity and tempo

#[cfg(test)]
mod tests {
    use crate::infra::profiler::*;
    use super::*;


    #[test]
    fn test_score() {
        assert_eq!(Score::from_cp(1).negate(), Score::from_cp(-1));
        assert_eq!(Score::white_win(1).negate(), Score::white_loss(1));
        assert_eq!(Score::white_win(0).negate(), Score::white_loss(0));
        assert_eq!(Score::white_win(0).is_mate(), true);
        assert_eq!(Score::white_loss(0).is_mate(), true);
        assert_eq!((-Score::INFINITY).is_mate(), false);
        assert_eq!(Score::INFINITY.is_mate(), false);
        assert_eq!(Score::from_cp(123).is_mate(), false);
        assert_eq!(Score::white_loss(1).negate(), Score::white_win(1));
        assert_eq!((-Score::INFINITY).negate(), Score::INFINITY);
        assert_eq!(-(-Score::INFINITY), Score::INFINITY);
        assert_eq!((-Score::INFINITY), -Score::INFINITY);
        assert_eq!(--(-Score::INFINITY), (-Score::INFINITY));
        assert_eq!((-Score::INFINITY).is_numeric(), false);
        assert_eq!(Score::INFINITY.is_numeric(), false);
        assert_eq!(Score::white_win(0).is_numeric(), false);
        assert_eq!(Score::white_loss(0).is_numeric(), false);
        assert_eq!(Score::white_loss(60).is_numeric(), false);
        assert_eq!(Score::white_win(60).is_numeric(), false);
        assert_eq!(Score::white_win(6) > Score::white_win(7), true);
        assert_eq!(Score::white_win(3) < Score::white_win(1), true);
        assert!((-Score::INFINITY) < Score::INFINITY);
        assert_eq!((-Score::INFINITY).is_mate(), false);
        assert_eq!(Score::white_win(1).is_mate(), true);
        assert!(Score::from_cp(-5) < Score::from_cp(5));
        assert!(Score::from_cp(5) < Score::white_win(0));
        assert!(Score::from_cp(100) > Score::from_cp(0));

        // addition
        assert_eq!(Score::from_cp(100) + Score::from_cp(150), Score::from_cp(250));
        assert_eq!((-Score::INFINITY) + Score::from_cp(150), (-Score::INFINITY));
        assert_eq!(Score::white_win(1) + Score::from_cp(150), Score::white_win(1));

        // subtraction
        assert_eq!(Score::from_cp(100) - Score::from_cp(150), Score::from_cp(-50));
        assert_eq!((-Score::INFINITY) - Score::from_cp(150), (-Score::INFINITY));
        assert_eq!(Score::white_win(1) - Score::from_cp(150), Score::white_win(1));

        assert_eq!(2 * Score::from_cp(100), Score::from_cp(200));
        assert_eq!(-2 * Score::from_cp(200), Score::from_cp(-400));
        assert_eq!(-2 * (-Score::INFINITY), Score::INFINITY);
        assert_eq!(-2 * Score::INFINITY, (-Score::INFINITY));
        assert_eq!(1 * Score::INFINITY, Score::INFINITY);
        assert_eq!(-1 * Score::white_win(2), Score::white_loss(2));
        // changes sign bit not magnitude
        assert_eq!(-3 * Score::white_win(2), Score::white_loss(2));
        assert_eq!(1 * Score::white_win(2), Score::white_win(2));
        assert!(Score::white_win(1) < Score::INFINITY);
        assert!(Score::white_win(0) == Score::white_win(0));
        assert!(Score::from_cp(0).win_probability() > 0.499);
        assert!(Score::from_cp(0).win_probability() < 0.501);
        assert!(Score::from_cp(1000).win_probability() > 0.95);
        assert!(Score::from_cp(-1000).win_probability() < 0.05);
        assert!((-Score::INFINITY).win_probability() < 0.001);
    }

    #[test]
    fn test_score_fmt() {
        assert_eq!(format!("{}", Score::from_cp(1000)), "1000");
        assert_eq!(format!("{}", Score::INFINITY), "+inf");
        assert_eq!(format!("{}", Score::white_win(2)), "win(2)");
        assert_eq!(format!("{}", Score::white_loss(3)), "loss(3)");
        assert_eq!(format!("{:>8}", Score::white_loss(3)), "loss(3)");
        assert_eq!(format!("{:>8}", Score::white_loss(3).to_string()), " loss(3)");
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
        let mut p = Profiler::new("standard_exp".into());
        p.start();
        for cp in (-1000..1000).step_by(100) {
            black_box(Score::win_probability_from_cp_and_k(cp as f32, 4.0));
        }
        p.stop();

        let mut p = Profiler::new("fast_exp".into());
        p.start();
        for cp in (-1000..1000).step_by(100) {
            black_box(Score::win_probability_from_cp_and_k_fast(cp as f32, 4.0));
        }
        p.stop();

        for cp in (-1000..1000).step_by(100) {
            let fast = Score::win_probability_from_cp_and_k_fast(cp as f32, 4.0);
            let slow = Score::win_probability_from_cp_and_k(cp as f32, 4.0);

            println!("{} {}", fast, slow);
            assert!((fast - slow).abs() < 0.01);
        }
    }
}
