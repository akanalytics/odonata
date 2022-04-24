use itertools::Itertools;
use std::fmt;

use crate::eval::weight::Weight;
use crate::outcome::Outcome;
use crate::phaser::Phase;

#[derive(Default, Clone, Debug)]
pub struct Sparse {
    index: u16,
    value: i16,
}

impl Sparse {
    #[inline(always)]
    pub const fn new(value: i16, index: u16) -> Self {
        Sparse { index, value }
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index as usize
    }

    #[inline]
    pub const fn value(&self) -> i16 {
        self.value
    }
}

#[derive(Default, Clone, Debug)]
pub struct FeatureVector {
    pub values: Vec<Sparse>,
    pub outcome: Outcome,
    pub phase: Phase,
}

impl FeatureVector {
    #[inline]
    pub fn value(&self, index: usize) -> Option<i32> {
        self.values
            .iter()
            .find(|&sparse| index == sparse.index() as usize)
            .map(|sparse| sparse.value() as i32)
    }
}

#[derive(Default, Clone)]
pub struct FeatureMatrix {
    pub feature_names: Vec<String>,
    pub feature_vectors: Vec<FeatureVector>,
}

#[derive(Default, Clone)]
pub struct WeightsVector {
    pub weights: Vec<Weight>,
    pub names: Vec<String>,
}

impl fmt::Display for WeightsVector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        assert!(self.weights.len() == self.names.len());
        for (i, weight) in self.weights.iter().enumerate() {
            writeln!(f, "{} = {}", self.names[i], weight)?
        }
        Ok(())
    }
}

impl FeatureMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn dot_product(&self, fv: &FeatureVector, wv: &WeightsVector) -> f32 {
        // debug_assert!(self.feature_names.len() == wv.weights.len());
        let mut s = 0.0;
        let mut e = 0.0;
        for sparse in &fv.values {
            // dp += *feature_value * unsafe{ wv.weights.get_unchecked(*index as usize) }.1.s();
            let w = unsafe { wv.weights.get_unchecked(sparse.index()) };
            s += sparse.value() as f32 * w.s();
            e += sparse.value() as f32 * w.e();
            // debug!("{index:>3} {:>27}: {} x {} ", self.feature_names[*index], feature_value, wv.weights[*index].1)
        }

        // let (s, e) = fv.values.iter().fold((0.0_f32, 0.0_f32), |(ss, se), sparse| {
        //     let (ws, we) = unsafe { wv.weights.get_unchecked(sparse.index() as usize) }.as_tuple();
        //     (ss + sparse.value() as f32 * ws, se + sparse.value() as f32 * we)
        // });

        // debug!("total {dp}");
        // Score::from_cp(dp.interpolate(fv.phase).round() as i32)
        Weight::from_f32(s, e).interpolate(fv.phase)
    }
}

impl fmt::Debug for FeatureMatrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FeatureMatrix")
            .field("feature_names", &self.feature_names.iter().enumerate().collect_vec())
            .field("#features", &self.feature_vectors.len())
            .finish()
    }
}

trait CsvDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

impl CsvDisplay for FeatureMatrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.feature_names.iter().join(","))?;
        for r in self.feature_vectors.iter() {
            for (c, _name) in self.feature_names.iter().enumerate() {
                match r.value(c as usize) {
                    Some(v) => write!(f, "{},", v),
                    None => write!(f, "0,"),
                }?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl fmt::Display for FeatureMatrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Count feature vectors: {}", self.feature_vectors.len())?;
        for r in -1..self.feature_names.len() as i32 {
            if r == -1 {
                write!(f, "{:>27}", "")?;
            } else {
                write!(f, "{:>27}", self.feature_names[r as usize])?;
            }
            for c in 0..std::cmp::min(3, self.feature_vectors.len()) {
                if r == -1 {
                    write!(f, "{:>6}", c)?;
                    continue;
                }
                let fv = &self.feature_vectors[c];
                match fv.value(r as usize) {
                    Some(v) => write!(f, "{:>6}", v),
                    None => write!(f, "{:>6}", "-"),
                }?;
            }
            writeln!(f)?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_log::test;

    fn create_feature_matrix() -> FeatureMatrix {
        let mut fm = FeatureMatrix::new();
        fm.feature_names = vec!["wheels".into(), "passengers".into(), "windows".into()];
        fm.feature_vectors.push(FeatureVector {
            values: Default::default(),
            outcome: Outcome::WinBlack,
            phase: Phase(30),
        }); // car
        fm.feature_vectors[0].values.push(Sparse::new(4, 0));
        fm.feature_vectors[0].values.push(Sparse::new(3, 1));
        fm.feature_vectors[0].values.push(Sparse::new(5, 2));

        fm.feature_vectors.push(FeatureVector {
            values: Default::default(),
            outcome: Outcome::WinBlack,
            phase: Phase(30),
        }); // motorbike
        fm.feature_vectors[1].values.push(Sparse::new(2, 0));
        fm.feature_vectors[1].values.push(Sparse::new(1, 1));
        fm
    }

    #[test]
    fn test_feature_matrix() {
        let fm = create_feature_matrix();
        println!("{}", fm);
    }

    fn test_csv_feature_matrix() {
        let _fm = create_feature_matrix();
        let buf = String::new();
        // let mut formatter = std::fmt::Formatter::new(&mut buf);
        // fmt::Display::fmt(self, &mut formatter)
        //         .expect("a Display implementation returned an error unexpectedly");
        println!("to sting csv:\n{buf}");
    }
}