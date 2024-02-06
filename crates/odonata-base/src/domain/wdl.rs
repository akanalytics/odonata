use std::{fmt, str::FromStr};

use itertools::Itertools;
use statrs::function::erf;

use crate::{other::outcome::Outcome, Color};

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

    pub fn sigmoid_inv(p: f32) -> f32 {
        -400.0 * f32::ln(1.0 / p - 1.0) / f32::ln(10.0)
    }

    // Test page: https://www.3dkingdoms.com/chess/elo.htm
    //
    pub fn elo(&self) -> f32 {
        let score = self.w as f32 + self.d as f32 / 2.0;
        let total = self.w as f32 + self.d as f32 + self.l as f32;
        let percentage = score / total;
        Self::sigmoid_inv(percentage)
    }

    /// erf = 2 / sqrt(pi) * integral[0..x] exp(-t^2) dt
    ///
    /// https://en.wikipedia.org/wiki/Error_function
    ///
    /// erf[ a / (sigma *sqrt(2)) ] = proba that sample lies in (-a, a)
    ///
    /// Prob(x in [l1, l2]) is   1/2 x [ erf (l2-mean) / sqrt(2)sigma - erf (l1-mean) / sqrt(2)sigma
    fn erf(x: f32) -> f32 {
        erf::erf(x as f64) as f32
        // libm::erf(x)
    }

    /// inverse of error function
    ///
    /// erf_inv(-1) = -∞
    /// erf_inv( 0) =  0
    /// erf_inv(-1) = +∞
    fn erf_inv(x: f32) -> f32 {
        erf::erf_inv(x as f64) as f32
    }

    ///  phi(x)     = 0.5 + 0.5 * erf(x / sqrt(2))
    ///  phi_inv(p) = sqrt(2) * erfinv(2 * p - 1.0)
    fn phi_inv(p: f32) -> f32 {
        f32::sqrt(2.) * Self::erf_inv(2. * p - 1.)
    }

    fn phi(p: f32) -> f32 {
        0.5 + 0.5 * Self::erf(p / f32::sqrt(2.0))
    }

    /// The likelihood of superiority (LOS) denotes how
    /// likely it would be for two players of the same strength
    /// to reach a certain result
    ///
    /// LOS = ϕ((wins - losses)/√(wins + losses))
    fn los(&self) -> f32 {
        let wins = self.w as f32;
        let losses = self.l as f32;
        Self::phi((wins - losses) / f32::sqrt(wins + losses))
    }

    /// total games played
    pub fn n(&self) -> i32 {
        self.w + self.d + self.l
    }

    /// points where: w=1 d=1/2 l=0
    pub fn points(&self) -> f32 {
        self.w as f32 + 0.5 * self.d as f32
    }

    /// at 97
    pub fn elo_error_margin(&self, confidence_level: f32) -> f32 {
        // total
        let n = self.n() as f32;

        let w = self.w as f32;
        let d = self.d as f32;
        let l = self.l as f32;

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
        let sigma = f32::sqrt(wd + dd + ld) / f32::sqrt(n);

        // confidence interval
        let cl = 1.0 - confidence_level;
        let ch = confidence_level;
        let μ0 = μ + Self::phi_inv(cl) * sigma;
        let μ1 = μ + Self::phi_inv(ch) * sigma;

        0.5 * (Self::sigmoid_inv(μ1) - Self::sigmoid_inv(μ0))
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
    use crate::domain::wdl::ScoreWdl;
    use test_log::test;

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
        assert_eq!(
            format!("{:.02}", ScoreWdl::new(217, 77, 184).elo()),
            "24.02"
        );
    }
}
