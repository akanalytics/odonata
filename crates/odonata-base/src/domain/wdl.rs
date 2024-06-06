use std::fmt::{self, Display};
use std::str::FromStr;

use itertools::Itertools;

use crate::other::outcome::Outcome;
use crate::prelude::Math;
use crate::Color;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum WdlOutcome {
    WhiteWin,
    Draw,
    BlackWin,
}

impl WdlOutcome {
    pub fn color_flip(&self) -> Self {
        match self {
            Self::WhiteWin => Self::BlackWin,
            Self::Draw => Self::Draw,
            Self::BlackWin => Self::WhiteWin,
        }
    }

    pub fn to_white_win_fraction(&self) -> f64 {
        match self {
            Self::WhiteWin => 1.0,
            Self::Draw => 0.5,
            Self::BlackWin => 0.0,
        }
    }

    pub fn to_white_win_fraction_str(&self) -> &str {
        match self {
            Self::WhiteWin => "1",
            Self::Draw => "0.5",
            Self::BlackWin => "0",
        }
    }
    pub fn to_pgn_wdl(&self) -> &'static str {
        match self {
            Self::WhiteWin => "1-0",
            Self::Draw => "1/2-1/2",
            Self::BlackWin => "0-1",
        }
    }

    pub fn parse_pgn(s: &str) -> anyhow::Result<Option<Self>> {
        match s {
            "*" => Ok(None),
            _ => Ok(Some(s.parse()?)),
        }
    }

    pub fn to_pgn(outcome: Option<WdlOutcome>) -> &'static str {
        match outcome {
            Some(o) => o.to_pgn_wdl(),
            None => "*",
        }
    }
}
impl Display for WdlOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_pgn_wdl().fmt(f)
    }
}

impl FromStr for WdlOutcome {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1-0" => Ok(Self::WhiteWin),
            "1/2-1/2" => Ok(Self::Draw),
            "0-1" => Ok(Self::BlackWin),
            s => Err(anyhow::anyhow!("{s} is not 1-0, 1/2-1/2 or 0-1")),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, PartialOrd, Ord, Hash)]
pub struct ScoreWdl {
    pub w: i32,
    pub d: i32,
    pub l: i32,
}

impl fmt::Display for ScoreWdl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "W: {}  D: {}  L: {}", self.w, self.d, self.l)
    }
}

impl FromStr for ScoreWdl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((w, d, l)) = s.split_whitespace().collect_tuple() else {
            return Err(anyhow::anyhow!("unable to parse {s} into a WDL score"));
        };
        Ok(ScoreWdl::new(w.parse()?, d.parse()?, l.parse()?))
    }
}

// Error calc from...
// http://www.open-aurec.com/wbforum/viewtopic.php?t=949

// https://www.chessprogramming.org/Match_Statistics

impl ScoreWdl {
    pub fn new(w: i32, d: i32, l: i32) -> ScoreWdl {
        ScoreWdl { w, d, l }
    }

    pub fn from_outcome(outcome: &Outcome) -> Self {
        if outcome.is_draw() {
            return ScoreWdl::new(0, 1, 0);
        } else if outcome.winning_color() == Some(Color::White) {
            return ScoreWdl::new(1, 0, 0);
        } else if outcome.winning_color() == Some(Color::Black) {
            return ScoreWdl::new(0, 0, 1);
        }
        ScoreWdl::new(0, 0, 0)
    }

    // Test page: https://www.3dkingdoms.com/chess/elo.htm
    //
    pub fn elo(&self) -> f64 {
        let score = self.w as f64 + self.d as f64 / 2.0;
        let total = self.w as f64 + self.d as f64 + self.l as f64;
        let percentage = score / total;
        Math::sigmoid_inv(percentage) * 400.
    }

    /// The likelihood of superiority (LOS) denotes how
    /// likely it would be for two players of the same strength
    /// to reach a certain result
    ///
    /// LOS = ϕ((wins - losses)/√(wins + losses))
    fn los(&self) -> f64 {
        let wins = self.w as f64;
        let losses = self.l as f64;
        Math::phi((wins - losses) / f64::sqrt(wins + losses))
    }

    /// total games played
    pub fn n(&self) -> i32 {
        self.w + self.d + self.l
    }

    /// points where: w=1 d=1/2 l=0
    pub fn points(&self) -> f64 {
        self.w as f64 + 0.5 * self.d as f64
    }

    /// at 97
    pub fn elo_error_margin(&self, confidence_level: f64) -> f64 {
        // total
        let n = self.n() as f64;

        let w = self.w as f64;
        let d = self.d as f64;
        let l = self.l as f64;

        // mean
        let μ = self.points() / n;

        // prob[win] etc
        let wp = w / n;
        let dp = d / n;
        let lp = l / n;

        // calculate deviations
        let wd = wp * (1.0 - μ).powi(2);
        let dd = dp * (0.5 - μ).powi(2);
        let ld = lp * (0.0 - μ).powi(2);

        // standard deviation
        let sigma = f64::sqrt(wd + dd + ld) / f64::sqrt(n);

        // confidence interval
        let cl = 1.0 - confidence_level;
        let ch = confidence_level;
        let μ0 = μ + Math::phi_inv(cl) * sigma;
        let μ1 = μ + Math::phi_inv(ch) * sigma;

        0.5 * (Math::sigmoid_inv(μ1) - Math::sigmoid_inv(μ0)) * 400.
    }

    // 	m_bayesElo = 200.0 * std::log10(p.pWin() / p.pLoss() *
    // 					(1.0 - p.pLoss()) / (1.0 - p.pWin()));
    // 	m_drawElo  = 200.0 * std::log10((1.0 - p.pLoss()) / p.pLoss() *
    // 					(1.0 - p.pWin()) / p.pWin());
    // }
}

impl std::ops::SubAssign for ScoreWdl {
    fn sub_assign(&mut self, o: Self) {
        self.l += o.w;
        self.d += o.d;
        self.w += o.l;
    }
}

impl std::ops::Neg for ScoreWdl {
    type Output = Self;
    fn neg(self) -> Self {
        let mut z = Self::default();
        z -= self;
        z
    }
}

impl std::ops::AddAssign for ScoreWdl {
    fn add_assign(&mut self, o: Self) {
        *self -= -o;
    }
}

impl std::ops::Add for ScoreWdl {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        let mut z = Self::default();
        z += o;
        z += self;
        z
    }
}

impl std::ops::Sub for ScoreWdl {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        self + -o
    }
}

impl std::iter::Sum for ScoreWdl {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(ScoreWdl::default(), |mut acc, x| {
            acc += x;
            acc
        })
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use crate::domain::wdl::ScoreWdl;

    #[test]
    fn test_score_wdl() {
        let mut wdl138 = ScoreWdl::new(1, 3, 8);
        assert_eq!(wdl138, "1 3 8".parse().unwrap());
        let wdl567 = ScoreWdl::new(5, 6, 7);
        assert_eq!(wdl138 + wdl567, ScoreWdl::new(6, 9, 15));
        assert_eq!(wdl138 - wdl567, ScoreWdl::new(8, 9, 13));
        wdl138 += ScoreWdl::new(100, 200, 300);
        assert_eq!(wdl138, ScoreWdl::new(101, 203, 308));
        wdl138 -= ScoreWdl::new(1, 3, 8);
        assert_eq!(wdl138, ScoreWdl::new(109, 206, 309));
        assert_eq!(-wdl138, ScoreWdl::new(309, 206, 109));
        // checked by https://www.3dkingdoms.com/chess/elo.htm
        assert_eq!(format!("{:.02}", ScoreWdl::new(217, 77, 184).elo()), "24.02");
    }
}
