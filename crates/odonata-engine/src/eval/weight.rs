use std::ops::{Index, IndexMut};
use std::{fmt, iter};

use num_traits::{AsPrimitive, Num};
use odonata_base::other::Phase;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum_macros::{Display, EnumString};

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Serialize, Deserialize, EnumString)]
#[serde(deny_unknown_fields)]
pub enum Rounding {
    Truncate,
    Round,
    None,
}
// This impl 50% faster than manipulating pairs of integers separately...
//
// eval/position           time:   [159.77 ns 160.45 ns 161.25 ns]
//                         change: [-51.630% -50.540% -49.410%] (p = 0.00 < 0.05)
//                         Performance has improved.

pub trait Number
where
    Self:
        AsPrimitive<f32> + Copy + num_traits::Num + Default + std::ops::AddAssign + Into<f64> + fmt::Display + 'static,
{
}

impl Number for f64 {}
impl Number for f32 {}
impl Number for i32 {}
impl Number for i16 {}

// essntially models the score bonus for s=start or e=end of game
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct WeightOf<T>(pub T, pub T);

impl<T: Copy> Copy for WeightOf<T> {}

pub type Weight = WeightOf<f32>;
// pub type Weight = crate::eval::weight3::WeightOf<f32>;

impl<T: Number> WeightOf<T> {
    pub fn cast_from<S: Number>(w: WeightOf<S>, rounding: Rounding) -> Self
    where
        T: 'static + Copy + Num,
        S: AsPrimitive<T>,
        f32: AsPrimitive<T>,
    {
        let wt = w.to_weight_f32(rounding);
        Self::from_f32(wt.s(), wt.e())
    }
}

impl<T: Number> WeightOf<T>
where
    T: AsPrimitive<f32>,
{
    pub fn to_weight_f32(&self, rounding: Rounding) -> WeightOf<f32> {
        let s = self.0.as_();
        let e = self.1.as_();
        match rounding {
            Rounding::Truncate => WeightOf::<f32>::from_f32(s.trunc(), e.trunc()),
            Rounding::Round => WeightOf::<f32>::from_f32(s.round(), e.round()),
            Rounding::None => WeightOf::<f32>(s.as_(), e.as_()),
        }
    }
}

// private
#[derive(Serialize, Deserialize)]
struct WeightOfHelper<T> {
    s: T,
    e: T,
}

impl<T: Number> Serialize for WeightOf<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        WeightOfHelper {
            s: self.0.as_(),
            e: self.1.as_(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WeightOf<i32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|WeightOfHelper { s, e }| WeightOf::<i32>::from_f32(s, e))
    }
}

impl<'de> Deserialize<'de> for WeightOf<f32> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|WeightOfHelper { s, e }| WeightOf::<f32>(s, e))
    }
}

impl<'de> Deserialize<'de> for WeightOf<f64> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|WeightOfHelper { s, e }| WeightOf::<f64>(s, e))
    }
}

impl WeightOf<i32> {
    #[inline]
    #[must_use]
    pub const fn from_i32(s: i32, e: i32) -> WeightOf<i32> {
        WeightOf(s, e)
    }
}

impl WeightOf<f32> {
    #[inline]
    #[must_use]
    pub const fn from_i32(s: i32, e: i32) -> WeightOf<f32> {
        WeightOf(s as f32, e as f32)
    }
}

impl<T> WeightOf<T>
where
    T: 'static,
    // i32: AsPrimitive<T>,
{
    #[inline]
    #[must_use]
    pub const fn new(s: T, e: T) -> WeightOf<T> {
        WeightOf(s, e)
    }
}

// impl<T> WeightOf<T>
// where
//     T: 'static + Copy + Num,
//     i32: AsPrimitive<T>,
// {
//     #[inline]
//     #[must_use]
//     pub const fn from_f32(s: T, e: T) -> WeightOf<T> {
//         WeightOf(s, e)
//     }
// }

impl<T: Sized> WeightOf<T>
where
    T: Copy + Num,
{
    #[inline]
    pub fn zero() -> WeightOf<T> {
        WeightOf(T::zero(), T::zero())
    }

    #[inline]
    pub const fn s(self) -> T {
        self.0
    }

    #[inline]
    pub const fn e(self) -> T {
        self.1
    }

    #[inline]
    pub fn as_tuple(self) -> (T, T) {
        (self.0, self.1)
    }
}

impl<T> Index<usize> for WeightOf<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.0,
            1 => &self.1,
            _ => unreachable!(),
        }
    }
}

impl<T> IndexMut<usize> for WeightOf<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.0,
            1 => &mut self.1,
            _ => unreachable!(),
        }
    }
}
// impl<T> WeightOf<T> where T: Copy + Num + std::ops::Sub<T, Output=T> + std::ops::Div<i32, Output=T>, i32: std::ops::Mul<T, Output=T>  {
impl<T> WeightOf<T>
where
    T: 'static + Copy + Num,
    i32: AsPrimitive<T>,
{
    #[inline]
    pub fn interpolate(self, phase: Phase) -> T {
        ((100 - phase.0).as_() * self.s() + phase.0.as_() * self.e()) / 100.as_()
    }
}

impl<T: Number> WeightOf<T>
where
    f32: AsPrimitive<T>,
{
    #[inline]
    #[must_use]
    pub fn from_f32(s: f32, e: f32) -> Self {
        WeightOf(s.as_(), e.as_())
    }
}

impl<T: Number> WeightOf<T>
where
    f64: AsPrimitive<T>,
{
    #[inline]
    #[must_use]
    pub fn from_f64(s: f64, e: f64) -> Self {
        WeightOf(s.as_(), e.as_())
    }

    // pub fn norm_l2(&self) -> f64 {
    //     f64::sqrt(self.s().into() * self.s().into() + self.e().into() * self.e().into())
    // }

    // pub fn norm_l1(&self) -> f64 {
    //     self.s().into().abs() + self.e().into().abs()
    // }
}

// impl<T> WeightOf<T> where T: Copy + Num + std::ops::Div<i32, Output=T>, i32: std::ops::Mul<T, Output=T>  {
//     #[inline]
//     pub fn interpolate(self, phase: i32) -> T {
//         ((100 - phase) * self.s() + phase * self.e()) / 100
//     }
// }
// impl Default for WeightOf {
//     #[inline]
//     fn default() -> Self {
//         WeightOf::new(0, 0)
//     }
// }

impl<T> fmt::Display for WeightOf<T>
where
    T: fmt::Display + Number,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // let precision = match f.precision() {
        //     Some(p) => p,
        //     None => 2,
        // };
        f.write_str("(")?;
        self.s().fmt(f)?;
        f.write_str(", ")?;
        self.e().fmt(f)?;
        f.write_str(")")?;

        //     "({}, {})",
        //     Formatting::decimal(precision, self.s()),
        //     Formatting::decimal(precision, self.e())
        // )
        // write!(f, "({}, {})", self.0, self.1)
        Ok(())
    }
}

// impl fmt::Display for WeightOf<f32> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         let precision = match f.precision() {
//             Some(p) => p,
//             None => 2,
//         };
//         write!(
//             f,
//             "({}, {})",
//             Formatting::decimal(precision, self.s()),
//             Formatting::decimal(precision, self.e())
//         )
//     }
// }

impl<T> iter::Sum for WeightOf<T>
where
    T: 'static + Copy + Num,
{
    #[inline]
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::zero(), |a, b| a + b)
    }
}

impl<T> std::ops::Add for WeightOf<T>
where
    T: 'static + Copy + Num,
{
    type Output = Self;

    #[inline]
    fn add(self, o: Self) -> Self {
        Self(self.s() + o.s(), self.e() + o.e())
    }
}

// impl<T> std::ops::AddAssign for WeightOf<T>
// where
//     T: Copy + Num + std::ops::AddAssign,
// {
//     #[inline]
//     fn add_assign(&mut self, o: Self) {
//         self.0 += o.s();
//         self.1 += o.e();
//     }
// }

impl<T: num_traits::Num + std::ops::AddAssign + Copy + Default> std::ops::AddAssign for WeightOf<T> {
    #[inline]
    fn add_assign(&mut self, o: Self) {
        self.0 += o.s();
        self.1 += o.e();
    }
}

// impl<T> std::ops::Mul<WeightOf<T>> for i32
// where
//     T: 'static + Copy + Num,
//     i32: AsPrimitive<T>,
// {
//     type Output = WeightOf<T>;

//     #[inline]
//     fn mul(self, o: WeightOf<T>) -> WeightOf<T> {
//         WeightOf::<T>(self.as_() * o.s(), self.as_() * o.e())
//     }
// }

impl std::ops::Mul<WeightOf<f64>> for i32 {
    type Output = WeightOf<f64>;

    #[inline]
    fn mul(self, o: WeightOf<f64>) -> WeightOf<f64> {
        // num_traits::ops::mul_add::MulAdd;
        WeightOf::<f64>(self as f64 * o.s(), self as f64 * o.e())
    }
}

impl std::ops::Mul<WeightOf<f32>> for i32 {
    type Output = WeightOf<f32>;

    #[inline]
    fn mul(self, o: WeightOf<f32>) -> WeightOf<f32> {
        // num_traits::ops::mul_add::MulAdd;
        WeightOf::<f32>(self as f32 * o.s(), self as f32 * o.e())
    }
}

impl std::ops::Mul<WeightOf<i32>> for i32 {
    type Output = WeightOf<i32>;

    #[inline]
    fn mul(self, o: WeightOf<i32>) -> WeightOf<i32> {
        WeightOf::<i32>(self * o.s(), self * o.e())
    }
}

impl std::ops::Mul<WeightOf<f32>> for f32 {
    type Output = WeightOf<f32>;

    #[inline]
    fn mul(self, o: WeightOf<f32>) -> WeightOf<f32> {
        WeightOf::<f32>(self * o.s(), self * o.e())
    }
}

impl std::ops::Mul<WeightOf<f32>> for i16 {
    type Output = WeightOf<f32>;

    #[inline]
    fn mul(self, o: WeightOf<f32>) -> WeightOf<f32> {
        WeightOf::<f32>(self as f32 * o.s(), self as f32 * o.e())
    }
}

impl std::ops::Mul<WeightOf<i16>> for i32 {
    type Output = WeightOf<i16>;

    #[inline]
    fn mul(self, o: WeightOf<i16>) -> WeightOf<i16> {
        WeightOf::<i16>(self as i16 * o.s(), self as i16 * o.e())
    }
}

// impl<T> std::ops::Mul<WeightOf<T>> for i32 where T: Copy + Num, i32: std::ops::Mul<T,Output=T>  {
//     type Output = WeightOf<T>;

//     #[inline]
//     fn mul(self, o: WeightOf<T>) -> WeightOf<T> {
//         WeightOf::<T>::new(self.as_() * o.s(), self * o.e())
//     }
// }

impl<T> std::ops::Sub for WeightOf<T>
where
    T: Copy + Num + std::ops::Sub,
{
    type Output = Self;

    #[inline]
    fn sub(self, o: Self) -> Self {
        Self(self.s() - o.s(), self.e() - o.e())
    }
}

impl<T> std::ops::Neg for WeightOf<T>
where
    T: Copy + Num + std::ops::Neg<Output = T>,
{
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self(-self.s(), -self.e())
    }
}

// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use odonata_base::infra::profiler::PerfProfiler;
    use test_log::test;
    use tracing::info;

    use super::*;

    #[test]
    fn test_basic_weight() {
        assert_eq!(WeightOf::<i32>::new(-10, 11).to_string(), "(-10, 11)");
        assert_eq!(WeightOf::new(-10.1_f32, 11.6).to_string(), "(-10.1, 11.6)");
        assert_eq!(format!("{:.2}", WeightOf::new(-10.1_f32, 11.6)), "(-10.10, 11.60)");
    }

    #[test]
    fn weight_serde_test() {
        info!("{}", toml::to_string(&Weight::default()).unwrap());
        // info!("{}", toml::to_string_pretty(&Weight::default()).unwrap());
    }

    #[test]
    fn bench_weight() {
        let w = black_box(Weight::new(5.5, 6.6));
        PerfProfiler::new("weight::new").bench(|| Weight::new(32.5, 56.9));
        PerfProfiler::new("weight::add").bench(|| w + Weight::new(black_box(32.5), 56.9));
        PerfProfiler::new("weight::mul").bench(|| 3_i32 * Weight::new(black_box(32.5), 56.9));
        PerfProfiler::new("weight::s/e").bench(|| w.s() + w.e());
    }
}

// This impl 42% slower

// essntially models the score bonus for s=start or e=end of game
// #[derive(Copy, Clone, PartialEq, Eq, Debug)]

// pub struct WeightOf(i64);

// // #[inline]
// // pub const fn w(s: i32, e: i32) -> WeightOf {
// //     WeightOf::new(s, e)
// // }

// impl WeightOf {
//     #[inline]
//     pub const fn new(s: i32, e: i32) -> WeightOf {
//         WeightOf(s as i64 * (2<<30) + e as i64)
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

// impl iter::Sum for WeightOf {
//     fn sum<I>(iter: I) -> Self
//     where
//         I: Iterator<Item = Self>,
//     {
//         iter.fold(Self::new(0, 0), |a, b| a+b)
//     }
// }

// impl std::ops::Add for WeightOf {
//     type Output = Self;

//     #[inline]
//     fn add(self, o: Self) -> Self {
//         Self(self.0 + o.0)
//     }
// }

// impl std::ops::Mul<WeightOf> for i32 {
//     type Output = WeightOf;

//     #[inline]
//     fn mul(self, o: WeightOf) -> WeightOf {
//         WeightOf(self as i64* o.0 )
//     }
// }

// impl std::ops::Sub for WeightOf {
//     type Output = Self;

//     #[inline]
//     fn sub(self, o: Self) -> Self {
//         Self(self.0 - o.0)
//     }
// }

// impl std::ops::Neg for WeightOf {
//     type Output = Self;

//     #[inline]
//     fn neg(self) -> Self {
//         Self(-self.0)
//     }
// }
