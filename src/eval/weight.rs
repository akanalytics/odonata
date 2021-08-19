use std::iter;
use std::fmt;

//
// This impl 50% faster than manipulating pairs of integers separately...
//
// eval/position           time:   [159.77 ns 160.45 ns 161.25 ns]
//                         change: [-51.630% -50.540% -49.410%] (p = 0.00 < 0.05)
//                         Performance has improved.



// essntially models the score bonus for s=start or e=end of game
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Weight(i32, i32);



impl Weight {
    #[inline]
    #[must_use]
    pub const fn new(s: i32, e: i32) -> Weight {
        Weight(s, e)
    }

    #[inline]
    pub const fn zero() -> Weight {
        Weight(0, 0)
    }

    #[inline]
    pub const fn s(self) -> i32 {
        self.0
    }

    #[inline]
    pub const fn e(self) -> i32 {
        self.1
    }

    #[inline]
    pub const fn interpolate(self, phase: i32) -> i32 {
        (self.s() * (100 - phase) + self.e() * phase) / 100
    }
}

// impl Default for Weight {
//     #[inline]
//     fn default() -> Self {
//         Weight::new(0, 0)
//     }
// }

impl fmt::Display for Weight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.s(), self.e())
    }
}


impl iter::Sum for Weight {
    #[inline]
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::new(0, 0), |a, b| a + b)
    }
}

impl std::ops::Add for Weight {
    type Output = Self;

    #[inline]
    fn add(self, o: Self) -> Self {
        Self::new(self.s() + o.s(), self.e() + o.e())
    }
}
impl std::ops::AddAssign for Weight {

    #[inline]
    fn add_assign(&mut self, o: Self) {
        self.0 += o.s();
        self.1 += o.e();
    }
}


impl std::ops::Mul<Weight> for i32 {
    type Output = Weight;

    #[inline]
    fn mul(self, o: Weight) -> Weight {
        Weight::new(self * o.s(), self * o.e())
    }
}

impl std::ops::Sub for Weight {
    type Output = Self;

    #[inline]
    fn sub(self, o: Self) -> Self {
        Self::new(self.s() - o.s(), self.e() - o.e())
    }
}

impl std::ops::Neg for Weight {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.s(), -self.e())
    }
}

// This impl 42% slower

// essntially models the score bonus for s=start or e=end of game
// #[derive(Copy, Clone, PartialEq, Eq, Debug)]
// pub struct Weight(i64);

// // #[inline]
// // pub const fn w(s: i32, e: i32) -> Weight {
// //     Weight::new(s, e)
// // }

// impl Weight {
//     #[inline]
//     pub const fn new(s: i32, e: i32) -> Weight {
//         Weight(s as i64 * (2<<30) + e as i64)
//     }

//     #[inline]
//     pub const fn s(self) -> i32 {
//         (self.0 / (2<<30)) as i32
//     }

//     #[inline]
//     pub const fn e(self) -> i32 {
//         (self.0 - self.s() as i64) as i32
//     }

//     #[inline]
//     pub const fn interpolate(self, phase: i32) -> i32 {
//         (self.s() * (100 - phase) + self.e() * phase) / 100
//     }
// }

// impl iter::Sum for Weight {
//     fn sum<I>(iter: I) -> Self
//     where
//         I: Iterator<Item = Self>,
//     {
//         iter.fold(Self::new(0, 0), |a, b| a+b)
//     }
// }

// impl std::ops::Add for Weight {
//     type Output = Self;

//     #[inline]
//     fn add(self, o: Self) -> Self {
//         Self(self.0 + o.0)
//     }
// }

// impl std::ops::Mul<Weight> for i32 {
//     type Output = Weight;

//     #[inline]
//     fn mul(self, o: Weight) -> Weight {
//         Weight(self as i64* o.0 )
//     }
// }

// impl std::ops::Sub for Weight {
//     type Output = Self;

//     #[inline]
//     fn sub(self, o: Self) -> Self {
//         Self(self.0 - o.0)
//     }
// }

// impl std::ops::Neg for Weight {
//     type Output = Self;

//     #[inline]
//     fn neg(self) -> Self {
//         Self(-self.0)
//     }
// }
