use statrs::function::erf;

pub struct Math;

pub trait Quantize {
    type Output;
    fn quantize(&self) -> anyhow::Result<Self::Output>;
}



impl Math {
    #[inline]
    pub fn crelu(x: f64) -> f64 {
        x.clamp(0., 1.)
    }

    #[inline]
    pub fn grad_crelu(x: f64) -> f64 {
        if 0.0 < x && x < 1.0 {
            1.0
        } else {
            0.0
        }
    }

    #[inline]
    pub fn logistic_sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + f64::exp(-x))
    }

    pub fn sigmoid_inv(p: f64) -> f64 {
        -f64::ln(1.0 / p - 1.0) / f64::ln(10.0)
    }

    // https://en.wikipedia.org/wiki/Logistic_function#Derivative
    #[inline]
    pub fn grad_logistic_sigmoid(x: f64) -> f64 {
        Self::logistic_sigmoid(x) * (1.0 - Self::logistic_sigmoid(x))
    }

    // 1 / [1 + exp(-300 / k=200)]
    //

    // https://www.chessprogramming.org/Pawn_Advantage,_Win_Percentage,_and_Elo
    #[inline]
    pub fn win_probability_from_cp_and_k(centipawns: f32, k: f32) -> f32 {
        1.0 / (1.0 + 10_f32.powf(-centipawns / (k * 100.0)))
    }

    pub fn logistic_steepness_to_base10(steepness: f64) -> f64 {
        f64::ln(10.0) * steepness
    }

    pub fn ordinal_logistic_sigmoid_f64(x: f64) -> f64 {
        let epsilon = 1e-5_f64;
        let x = x / 10.;
        let lambda = 1_f64;
        let y = match x {
            x if x > 0.518 / lambda => Self::logistic_sigmoid(x),
            x if x < -0.518 / lambda => Self::logistic_sigmoid(x),
            x if x > 0.0 => 0.5 + 1.0 / lambda * 1.0 / (1.0 + f64::exp(1. / (lambda * x + epsilon))),
            x if x < 0.0 => 0.5 - 1.0 / lambda * 1.0 / (1.0 + f64::exp(1. / (-lambda * x + epsilon))),
            _ => 0.0,
        };
        if y.is_nan() {
            panic!("x was {x} y is nan");
        }
        y
    }

    /// erf = 2 / sqrt(pi) * integral[0..x] exp(-t^2) dt
    ///
    /// https://en.wikipedia.org/wiki/Error_function
    ///
    /// erf[ a / (sigma *sqrt(2)) ] = proba that sample lies in (-a, a)
    ///
    /// Prob(x in [l1, l2]) is   1/2 x [ erf (l2-mean) / sqrt(2)sigma - erf (l1-mean) / sqrt(2)sigma
    pub fn erf(x: f64) -> f64 {
        erf::erf(x)
        // libm::erf(x)
    }

    /// inverse of error function
    ///
    /// erf_inv(-1) = -∞
    /// erf_inv( 0) =  0
    /// erf_inv(-1) = +∞
    pub fn erf_inv(x: f64) -> f64 {
        erf::erf_inv(x)
    }

    ///  phi(x)     = 0.5 + 0.5 * erf(x / sqrt(2))
    ///  phi_inv(p) = sqrt(2) * erfinv(2 * p - 1.0)
    pub fn phi_inv(p: f64) -> f64 {
        f64::sqrt(2.) * Self::erf_inv(2. * p - 1.)
    }

    pub fn phi(p: f64) -> f64 {
        0.5 + 0.5 * Self::erf(p / f64::sqrt(2.0))
    }
}
