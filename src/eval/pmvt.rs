use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::piece::Piece;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::fmt;

use super::pst::PstHelper;

#[derive(Clone)]
pub struct Pmvt {
    pub enabled: bool,
    mv: [[Weight; 20]; Piece::len()],
}

impl Default for Pmvt {
    fn default() -> Self {
        Self {
            enabled: true,
            mv: [[Weight::default(); 20]; Piece::len()],
        }
    }
}

impl Serialize for Pmvt {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut h = PstHelper::default();
        for (i, &p) in Piece::ALL_BAR_NONE.iter().enumerate() {
            let map = &mut [&mut h.p, &mut h.n, &mut h.b, &mut h.r, &mut h.q, &mut h.k][i];
            for i in 0..20 {
                map.insert(i.to_string(), self.mv[p][i]);
            }
        }
        h.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Pmvt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let h: PstHelper = Deserialize::deserialize(deserializer)?;
        let mut pmvt = Pmvt::default();
        for (i, &p) in Piece::ALL_BAR_NONE.iter().enumerate() {
            let map = [&h.p, &h.n, &h.b, &h.r, &h.q, &h.k][i];
            for (k, &v) in map.iter() {
                let i: usize = k.parse().map_err(serde::de::Error::custom)?;
                pmvt.mv[p][i] = v;
            }
        }
        Ok(pmvt)
    }
}

impl Component for Pmvt {
    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_position(&mut self) {}
}

impl fmt::Display for Pmvt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        for &p in &Piece::ALL_BAR_NONE {
            for phase in ["s", "e"] {
                writeln!(f, "PMVT: {}.{}", p, phase)?;
                for i in 0..20 {
                    let wt = self.mv[p][i];
                    let score = if phase == "s" { wt.s() } else { wt.e() };
                    let s = format!("{:>4}", score);
                    write!(f, "{:>6},", s)?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Pmvt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Pst")
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl Pmvt {
    pub fn _new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn _w_eval_mob(&self, p: Piece, count: i32) -> Weight {
        self.mv[p][std::cmp::min(count, 19) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::engine::Engine;
    use test_log::test;

    #[test]
    fn pmvt_serde_test() {
        let pmvt = Pmvt::default();
        let text = toml::to_string(&pmvt).unwrap();
        info!("toml\n{}", text);
        eprintln!("toml\n{}", text);
        let pmvt2: Pmvt = toml::from_str(&text).unwrap();
        eprintln!("from toml\n{}", pmvt2);
    }

    #[test]
    fn test_pmvt() {
        let pmvt = Pmvt::default();
        info!("{}", pmvt);
        let _eng = Engine::new();
        // info!("{}", eng.algo.eval.pmvt);
    }

    #[test]
    fn pmvt_config() {
        // let mut eng = Engine::default();
        // let _text = toml::to_string(&eng).unwrap();
        // eng.configment("eval.pmvt.p.a2", "{ s=6.5, e=7.6 }").unwrap();
        // eng.configment("eval.pmvt.p.a2.s", "6.5").unwrap();
        // eng.configment("eval.pmvt.p.a2.e", "7.5").unwrap();
        // let _text = toml::to_string(&eng).unwrap();
        // eprintln!("toml\n{}", text);
        // let lookup = c1.weight("eval.pmvt.p.a2", &Weight::from_i32(1, 1));
        // assert_eq!(eng.algo.eval.pmvt.pmvt(Piece::Pawn, Square::A2).s(), Weight::from_f32(6.5,7.5).s());
        // assert_eq!(eng.algo.eval.pmvt.pmvt(Piece::Pawn, Square::A2).e(), Weight::from_f32(6.5,7.5).e());
        // eng.configment("eval.pmvt.p.a2.e", "8.5").unwrap();
        // assert_eq!(eng.algo.eval.pmvt.pmvt(Piece::Pawn, Square::A2).e(), Weight::from_f32(6.5,8.5).e());
    }
}
