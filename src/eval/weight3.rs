use std::iter;
use std::fmt;
use num_traits::{Num, AsPrimitive};
use serde::{Deserialize, Serialize};

use crate::utils::Formatting;
use serde::{ Serializer, Deserializer};


//
// This impl 50% faster than manipulating pairs of integers separately...
//
// eval/position           time:   [159.77 ns 160.45 ns 161.25 ns]
//                         change: [-51.630% -50.540% -49.410%] (p = 0.00 < 0.05)
//                         Performance has improved.



// essntially models the score bonus for s=start or e=end of game
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct WeightOf<T>(T, T, T) where T:Copy + Num;

// pub type Weight = WeightOf<f32>;





// private
#[derive(Serialize, Deserialize)]
struct WeightOfHelper<T> {
    s: T,
    #[serde(default)]
    m: Option<T>,
    e: T,
}


impl<T: Copy + Num + Serialize> Serialize for WeightOf<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WeightOfHelper::<T> { s: self.0, m: Some(self.1), e: self.2 }.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for WeightOf<T>
where 
T: Copy + Num + Default+ Deserialize<'de> + 'static,
i32: AsPrimitive<T>  {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .map(|WeightOfHelper::<T> { s, m, e }| WeightOf::<T>(s, m.unwrap_or((s + e)/2.as_()), e))
    }
}



impl WeightOf<i32>  {

    #[inline]
    #[must_use]
    pub const fn from_i32(s: i32, e: i32) -> WeightOf<i32> {
        WeightOf(s as i32, e as i32, e as i32)
    }
}

impl WeightOf<f32>  {

    #[inline]
    #[must_use]
    pub const fn from_i32(s: i32, e: i32) -> WeightOf<f32> {
        WeightOf(s as f32, e as f32, e as f32)
    }
}

impl<T> WeightOf<T> where T:'static + Copy + Num, i32: AsPrimitive<T>  {
    

    #[inline]
    #[must_use]
    pub fn new(s: T, m: T, e: T) -> WeightOf<T> {
        WeightOf(s, m, e)
    }
}


impl<T: Sized> WeightOf<T> where T: Copy + Num {
    #[inline]
    pub fn zero() -> WeightOf<T> {
        WeightOf(T::zero(), T::zero(), T::zero())
    }

    #[inline]
    pub fn s(self) -> T {
        self.0
    }

    #[inline]
    pub fn m(self) -> T {
        self.1
    }

    #[inline]
    pub fn e(self) -> T {
        self.2
    }

}
// impl<T> WeightOf<T> where T: Copy + Num + std::ops::Sub<T, Output=T> + std::ops::Div<i32, Output=T>, i32: std::ops::Mul<T, Output=T>  {
impl<T> WeightOf<T> where T: 'static + Copy + Num, i32: AsPrimitive<T>  {
    #[inline]
    pub fn interpolate(self, phase: i32) -> T {
        if phase <= 50 {
            ((50 - phase).as_() * self.s() + phase.as_() * self.m()) / 50.as_()
        } else {
            ((50 - (phase-50)).as_() * self.m() + (phase-50).as_() * self.e()) / 50.as_()
        }
    }
}

impl<T> WeightOf<T> where T: 'static + Copy + Num, f32: AsPrimitive<T>   {
    #[inline]
    #[must_use]
    pub fn from_f32(s: f32, e: f32) -> WeightOf<T> {
        WeightOf(s.as_(), e.as_(), e.as_())
    }
    

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

impl<T> fmt::Display for WeightOf<T> where T: Copy + Num + fmt::Display + Into<f64> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", Formatting::decimal(2, self.s()), Formatting::decimal(2, self.m()), Formatting::decimal(2, self.e()))
    }
}


impl<T> iter::Sum for WeightOf<T> where T: 'static + Copy + Num{
    #[inline]
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::zero(), |a, b| a + b)
    }
}

impl<T> std::ops::Add for WeightOf<T> where T:'static + Copy + Num {
    type Output = Self;

    #[inline]
    fn add(self, o: Self) -> Self {
        Self(self.s() + o.s(), self.m() + o.m(), self.e() + o.e())
    }
}
impl<T> std::ops::AddAssign for WeightOf<T> where T: Copy + Num + std::ops::AddAssign { 

    #[inline]
    fn add_assign(&mut self, o: Self) {
        self.0 += o.s();
        self.1 += o.m();
        self.2 += o.e();
    }
}


impl<T> std::ops::Mul<WeightOf<T>> for i32 where T: 'static + Copy + Num, i32: AsPrimitive<T> {
    type Output = WeightOf<T>;

    #[inline]
    fn mul(self, o: WeightOf<T>) -> WeightOf<T> {
        WeightOf::<T>(self.as_() * o.s(), self.as_() * o.m(), self.as_() * o.e())
    }
}

// impl<T> std::ops::Mul<WeightOf<T>> for i32 where T: Copy + Num, i32: std::ops::Mul<T,Output=T>  {
//     type Output = WeightOf<T>;

//     #[inline]
//     fn mul(self, o: WeightOf<T>) -> WeightOf<T> {
//         WeightOf::<T>::new(self.as_() * o.s(), self * o.e())
//     }
// }

impl<T> std::ops::Sub for WeightOf<T> where T: Copy + Num + std::ops::Sub {
    type Output = Self;

    #[inline]
    fn sub(self, o: Self) -> Self {
        Self(self.s() - o.s(), self.m() - o.m(), self.e() - o.e())
    }
}

impl<T> std::ops::Neg for WeightOf<T> where T: Copy + Num + std::ops::Neg<Output=T> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self(-self.s(), -self.m(),  -self.e())
    }
}









#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use toml;
    

    #[test]
    fn weight_serde_test() {
        info!("{}", toml::to_string(&WeightOf::<f32>::default()).unwrap());
        // info!("{}", toml::to_string_pretty(&Weight::default()).unwrap());
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