use std::fmt::{Debug, Display};
use std::ops::{AddAssign, Mul, MulAssign, SubAssign};

use crossbeam_utils::CachePadded;
use itertools::Itertools as _;
use num_traits::MulAdd;
use odonata_base::infra::math::Quantize;
use odonata_base::infra::utils;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const N: usize = 512;

#[derive(Clone, PartialEq)]
#[repr(C)]
pub struct Vector<T> {
    pub vals: CachePadded<[T; N]>,
    size:     CachePadded<usize>,
}

impl<'de, T: Default + Copy + Deserialize<'de>> Deserialize<'de> for Vector<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v: Vec<T> = Deserialize::deserialize(d)?;
        let vector = Vector::<T>::try_from(v).map_err(|e| serde::de::Error::custom(e.to_string()))?;
        Ok(vector)
    }
}

impl<T: Serialize + Copy> Serialize for Vector<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let v: Vec<T> = self.into();
        v.serialize(ser)
    }
}

impl<T: Copy> From<&Vector<T>> for Vec<T> {
    fn from(vector: &Vector<T>) -> Self {
        vector.as_slice().into()
    }
}

impl<T: Default + Copy> TryFrom<Vec<T>> for Vector<T> {
    type Error = anyhow::Error;

    fn try_from(vec: Vec<T>) -> Result<Self, Self::Error> {
        anyhow::ensure!(vec.len() <= N);
        let mut vector = Vector::<T> {
            size: CachePadded::new(vec.len()),
            ..Vector::default()
        };
        vec.iter().enumerate().for_each(|(i, t)| vector.vals[i] = *t);
        Ok(vector)
    }
}

impl<T: Display> Debug for Vector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let typename = format!("Vector<{}>", utils::type_suffix(std::any::type_name::<T>()));
        f.debug_struct(&typename)
            .field("size", &self.size)
            .field(
                "vals",
                &format!("[{:.4}..]", self.as_slice().iter().take(5).format(",")),
            )
            .finish()
    }
}

impl<T: Default> Default for Vector<T> {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Quantize for Vector<f64> {
    type Output = Vector<i16>;

    fn quantize(&self) -> anyhow::Result<Self::Output> {
        let mut v16 = Self::Output::new(self.len());
        for i in 0..v16.len() {
            let f = self.get(i);
            if f > i16::MAX as f64 || f < i16::MIN as f64 {
                anyhow::bail!("{f} at index {i} is out of bounds");
            } else {
                *v16.set(i) = f.round() as i16;
            }
        }
        Ok(v16)
    }
}

impl Quantize for Vector<f32> {
    type Output = Vector<i16>;

    fn quantize(&self) -> anyhow::Result<Self::Output> {
        let mut v16 = Self::Output::new(self.len());
        for i in 0..v16.len() {
            let f = self.get(i);
            if f > i16::MAX as f32 || f < i16::MIN as f32 {
                anyhow::bail!("{f} at index {i} is out of bounds");
            } else {
                *v16.set(i) = f.round() as i16;
            }
        }
        Ok(v16)
    }
}

impl<T: Copy> Vector<T> {
    #[inline(always)]
    pub fn get(&self, i: usize) -> T {
        debug_assert!(i < N);
        self.vals[i]
    }
}

impl<T> Vector<T> {
    #[allow(clippy::len_without_is_empty)]
    #[inline(always)]
    pub fn len(&self) -> usize {
        *self.size
    }

    #[inline(always)]
    pub fn set(&mut self, i: usize) -> &mut T {
        debug_assert!(i < N);
        &mut self.vals[i]
    }
}

impl<T: Default> Vector<T> {
    #[inline(always)]
    pub fn new(size: usize) -> Self {
        debug_assert!(size <= N);
        Self {
            vals: CachePadded::new([(); N].map(|_| T::default())),
            size: CachePadded::new(size),
        }
    }

    #[inline(always)]
    fn once(t: T) -> Self {
        let mut v = Self {
            vals: CachePadded::new([(); N].map(|_| T::default())),
            size: CachePadded::new(1),
        };
        v.vals[0] = t;
        v
    }
}

impl<T> Vector<T> {
    #[inline(always)]
    pub fn apply_zip_mut(&mut self, rhs: &Vector<T>, mut f: impl FnMut(&mut T, &T)) {
        let n = *self.size;
        debug_assert_eq!(n, *rhs.size);
        match n {
            128 => self.vals[..128].iter_mut().zip(&*rhs.vals).for_each(|(x, y)| f(x, y)),
            256 => self.vals[..256].iter_mut().zip(&*rhs.vals).for_each(|(x, y)| f(x, y)),
            512 => self.vals[..512].iter_mut().zip(&*rhs.vals).for_each(|(x, y)| f(x, y)),
            _ => self.vals[..n].iter_mut().zip(&rhs.vals[..]).for_each(|(x, y)| f(x, y)),
        }
    }

    #[rustfmt::skip]
    #[inline(always)]
    pub fn apply_zip2_mut(&mut self, rhs1: &Vector<T>, rhs2: &Vector<T>, mut f: impl FnMut(&mut T, &T, &T)) {
        let n = *self.size;
        debug_assert_eq!(n, *rhs1.size);
        debug_assert_eq!(n, *rhs2.size);
        match n {
            128 => self.vals[..128].iter_mut().zip(&*rhs1.vals).zip(&*rhs2.vals).for_each(|((x, y), z)| f(x, y, z)),
            256 => self.vals[..256].iter_mut().zip(&*rhs1.vals).zip(&*rhs2.vals).for_each(|((x, y), z)| f(x, y, z)),
            512 => self.vals[..512].iter_mut().zip(&*rhs1.vals).zip(&*rhs2.vals).for_each(|((x, y), z)| f(x, y, z)),
            _ => self.vals[..n].iter_mut().zip(&rhs1.vals[..]).zip(&rhs2.vals[..]).for_each(|((x, y), z)| f(x, y, z)),
        }
    }

    #[inline(always)]
    pub fn apply_zip(&self, rhs: &Vector<T>, mut f: impl FnMut(&T, &T)) {
        let n = *self.size;
        debug_assert_eq!(n, *rhs.size);
        match n {
            128 => self.vals[..128].iter().zip(&rhs.vals[..128]).for_each(|(x, y)| f(x, y)),
            256 => self.vals[..256].iter().zip(&rhs.vals[..256]).for_each(|(x, y)| f(x, y)),
            512 => self.vals[..512].iter().zip(&rhs.vals[..512]).for_each(|(x, y)| f(x, y)),
            _ => self.vals[..n].iter().zip(&rhs.vals[..]).for_each(|(x, y)| f(x, y)),
        }
    }

    #[inline(always)]
    pub fn apply_mut(&mut self, f: impl FnMut(&mut T)) {
        let n = *self.size;
        match n {
            128 => self.vals[..128].iter_mut().for_each(f),
            256 => self.vals[..256].iter_mut().for_each(f),
            512 => self.vals[..512].iter_mut().for_each(f),
            _ => self.vals[..n].iter_mut().for_each(f),
        }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        &self.vals[0..*self.size]
    }

    #[inline(always)]
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        &mut self.vals[0..*self.size]
    }
}

impl<T> Mul<T> for &Vector<T>
where
    T: Copy + MulAssign,
{
    type Output = Vector<T>;

    #[inline(always)]
    fn mul(self, m: T) -> Self::Output {
        let mut res = self.clone();
        res.apply_mut(
            #[inline(always)]
            |x| *x *= m,
        );
        res
    }
}

impl<T: AddAssign + Copy + Mul<Output = T> + MulAdd<Output = T>> Vector<T> {
    #[inline(always)]
    pub fn mul_add_assign(&mut self, m: T, rhs: &Vector<T>) {
        self.apply_zip_mut(
            rhs,
            #[inline(always)]
            |x, y| *x = m.mul_add(*y, *x),
        );
    }
}

impl<T: AddAssign + Copy> AddAssign<&Vector<T>> for Vector<T> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: &Vector<T>) {
        self.apply_zip_mut(
            rhs,
            #[inline(always)]
            |x, y| *x += *y,
        );
    }
}

impl<T: SubAssign + Copy> SubAssign<&Vector<T>> for Vector<T> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: &Vector<T>) {
        self.apply_zip_mut(
            rhs,
            #[inline(always)]
            |x, y| *x -= *y,
        );
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::infra::profiler::PerfProfiler;
    use test_log::test;

    use super::*;

    #[test]
    fn bench_vector() {
        let mut perf_get = PerfProfiler::new("vector<f64>.get");

        let mut v1_f64 = Vector::<f64>::new(512);
        let v2_f64 = Vector::new(512);
        let mut perf_add_f64 = PerfProfiler::new("vector<f64>.add_assign");
        let mut perf_zip_f64 = PerfProfiler::new("vector<f64>.apply_zip");
        let mut perf_muladd_f64 = PerfProfiler::new("vector<f64>.mul_add");

        let mut v1_f32 = Vector::<f32>::new(512);
        let v2_f32 = Vector::new(512);
        let mut perf_add_f32 = PerfProfiler::new("vector<f32>.add_assign");
        let mut perf_zip_f32 = PerfProfiler::new("vector<f32>.apply_zip");
        let mut perf_muladd_f32 = PerfProfiler::new("vector<f32>.mul_add");

        perf_get.bench(|| v1_f64.get(3));

        perf_add_f64.bench(|| v1_f64.add_assign(&v2_f64));
        perf_zip_f64.bench(|| v1_f64.apply_zip_mut(&v2_f64, |x, y| *x += y * y));
        perf_muladd_f64.bench(|| v1_f64.apply_zip_mut(&v2_f64, |x, y| *x = 5.0 * *x + y));

        perf_add_f32.bench(|| v1_f32.add_assign(&v2_f32));
        perf_zip_f32.bench(|| v1_f32.apply_zip_mut(&v2_f32, |x, y| *x += y * y));
        perf_muladd_f32.bench(|| v1_f32.apply_zip_mut(&v2_f32, |x, y| *x = 5.0 * *x + y));
    }
}
