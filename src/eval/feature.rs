use std::fmt;

use itertools::Itertools;

use crate::eval::score::Score;
use crate::eval::weight::Weight;
use crate::outcome::Outcome;
use crate::phaser::Phase;

#[derive(Default, Clone, Debug)]
pub struct FeatureVector {
    pub values: Vec<(i32, usize)>,
    pub outcome: Outcome,
    pub phase: Phase,
}

impl FeatureVector {
    pub fn value(&self, index: usize) -> Option<i32> {
        self.values.iter().find(|(_val, i)| index == *i).map(|(v, _i)| *v)
    }
}

#[derive(Default, Clone)]
pub struct FeatureMatrix {
    pub feature_names: Vec<String>,
    pub feature_vectors: Vec<FeatureVector>,
}

#[derive(Default, Clone)]
pub struct WeightsVector {
    pub weights: Vec<(String, Weight)>,
}

impl FeatureMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dot_product(&self, fv: &FeatureVector, wv: &WeightsVector) -> Score {
        // debug_assert!(self.feature_names.len() == wv.weights.len());
        let mut dp = Weight::zero();
        for (feature_value, index) in &fv.values {
            dp += *feature_value * wv.weights[*index].1;
            // debug!("{index:>3} {:>27}: {} x {} ", self.feature_names[*index], feature_value, wv.weights[*index].1)
        }
        // debug!("total {dp}");
        Score::from_cp(dp.interpolate(fv.phase).round() as i32)
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

    #[test]
    fn test_feature_matrix() {
        let mut fm = FeatureMatrix::new();
        fm.feature_names = vec!["wheels".into(), "passengers".into(), "windows".into()];
        fm.feature_vectors.push(FeatureVector {
            values: vec![(4, 0), (3, 1), (5, 2)],
            outcome: Outcome::WinBlack,
            phase: Phase(30),
        }); // car
        fm.feature_vectors.push(FeatureVector {
            values: vec![(2, 0), (1, 1)],
            outcome: Outcome::WinBlack,
            phase: Phase(30),
        }); // motorbike
        println!("{}", fm);
    }
}
