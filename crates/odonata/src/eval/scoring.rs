use std::{collections::HashMap, fmt, io::Write, marker::PhantomData, ops::Mul};

use crate::{
    bits::precalc::Pawns, boards::Board, eval::weight::Weight, infra::utils::Formatting,
    other::Phase, Color,
};

use super::{
    weight::{Rounding, WeightOf},
    Feature, FeatureCategory,
};
use crate::prelude::*;

use super::hardcoded_weights::RawHardcoded;

use crate::eval::weight::Number;
use indexmap::map::IndexMap;
use num_traits::AsPrimitive;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tabled::object::Columns;

pub trait WeightVec<T: Number> {
    fn weight(&self, f: Feature) -> WeightOf<T>;
}

pub trait Scorer<T> {
    #[inline(always)]
    fn accum2(&mut self, c: Color, i: Feature, value: i32) {
        match c {
            Color::White => self.accum(i, value, 0),
            Color::Black => self.accum(i, 0, value),
        }
    }

    fn accum(&mut self, f: Feature, w: i32, b: i32);
    fn set_bits(&mut self, _i: Feature, _bits: Bitboard) {}
    fn apply_scaling(&mut self, _scaling: f32) {}
    fn total(&self) -> WeightOf<T>;
}

impl<T: Number> fmt::Debug for dyn WeightVec<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut tw = tabwriter::TabWriter::new(vec![]);
        for f in Feature::iter() {
            writeln!(
                tw,
                "{}\t{f}\t= {:>7},\t{:>7}",
                f.index(),
                self.weight(f).s(),
                self.weight(f).e()
            )
            .or(Err(fmt::Error))?;
        }
        fmt.write_str(
            &String::from_utf8(tw.into_inner().or(Err(fmt::Error))?).or(Err(fmt::Error))?,
        )?;
        Ok(())
    }
}

impl<T: Number> dyn WeightVec<T> {
    pub fn write<W: Write>(&self, mut w: W) -> anyhow::Result<()> {
        writeln!(w, "//\n// type: {}\n//\n", std::any::type_name::<T>())?;
        for f in Feature::iter() {
            writeln!(
                w,
                "\t({:>8.2},\t{:>8.2} ),\t //  {f:>30} {cat:>10}  ",
                self.weight(f).s(),
                self.weight(f).e(),
                cat = f.category().name(),
            )?;
        }
        writeln!(
            w,
            "//\n// type: (Millipawns) {}\n//\n",
            std::any::type_name::<T>()
        )?;
        for f in Feature::iter() {
            let wt = self.weight(f);
            let w3 = wt + wt + wt;
            let w10 = w3 + w3 + w3 + wt;

            writeln!(
                w,
                "\t({:>8.0},\t{:>8.0} ),\t //  {f:>30} {cat:>10}  ",
                w10.s(),
                w10.e(),
                cat = f.category().name(),
            )?;
        }
        Ok(())
    }
}

// pub(crate) struct Hardcoded<T: Number> {
//     _phantom: PhantomData<T>,
// }

// impl<T: Number> Hardcoded<T> {
//     pub const WTS: [WeightOf<T>; Feature::len()] = {
//         let a = &RawHardcoded::<T>::RAW_WTS;
//         let mut b = [WeightOf::zero(); Feature::len()];
//         let mut i = 0;
//         while i < b.len() {
//             b[i] = WeightOf::<T>::new(a[i].0, a[i].1);
//             i += 1;
//         }
//         b
//     };
// }

// impl<T: Number> WeightVec<T> for Hardcoded<T> {
//     fn weight(&self, f: Feature) -> WeightOf<T> {
//         Self::WTS[f]
//     }
// }

// impl HardcodedI32 {
//     pub const WTS: [WeightOf<i32>; Feature::len()] = {
//         let a = &RawHardcoded::<i32>::RAW_WTS;
//         let mut b = [WeightOf::new(0, 0); Feature::len()];
//         let mut i = 0;
//         while i < b.len() {
//             b[i] = WeightOf::new(a[i].0, a[i].1);
//             i += 1;
//         }
//         b
//     };
// }

pub(crate) struct Hardcoded<T: Number> {
    _phantom: PhantomData<T>,
}

impl Hardcoded<f32> {
    pub const WTS: [WeightOf<f32>; Feature::len()] = {
        let a = &RawHardcoded::<f32>::RAW_WTS;
        let mut b = [WeightOf::new(0.0, 0.0); Feature::len()];
        let mut i = 0;
        while i < b.len() {
            b[i] = WeightOf::<f32>::new(a[i].0, a[i].1);
            i += 1;
        }
        b
    };
}
impl WeightVec<f32> for Hardcoded<f32> {
    fn weight(&self, f: Feature) -> WeightOf<f32> {
        Self::WTS[f]
    }
}

impl Hardcoded<f64> {
    pub const WTS: [WeightOf<f64>; Feature::len()] = {
        let a = &RawHardcoded::<f32>::RAW_WTS;
        let mut b = [WeightOf::new(0.0, 0.0); Feature::len()];
        let mut i = 0;
        while i < b.len() {
            b[i] = WeightOf::<f64>::new(a[i].0 as f64, a[i].1 as f64);
            i += 1;
        }
        b
    };
}
impl WeightVec<f64> for Hardcoded<f64> {
    fn weight(&self, f: Feature) -> WeightOf<f64> {
        Self::WTS[f]
    }
}

impl WeightVec<i32> for Hardcoded<i32> {
    fn weight(&self, f: Feature) -> WeightOf<i32> {
        Self::WTS[f]
    }
}

impl Hardcoded<i32> {
    pub const WTS: [WeightOf<i32>; Feature::len()] = {
        let a = &RawHardcoded::<i32>::RAW_WTS;
        let mut b = [WeightOf::new(0, 0); Feature::len()];
        let mut i = 0;
        while i < b.len() {
            b[i] = WeightOf::new(a[i].0, a[i].1);
            i += 1;
        }
        b
    };

    pub const WTS_MILLIS: [WeightOf<i32>; Feature::len()] = {
        let a = &RawHardcoded::<i32>::RAW_WTS_MILLIS;
        let mut b = [WeightOf::new(0, 0); Feature::len()];
        let mut i = 0;
        while i < b.len() {
            b[i] = WeightOf::new(a[i].0, a[i].1);
            i += 1;
        }
        b
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct Softcoded<T: Number> {
    pub wts: [WeightOf<T>; Feature::len()],
}

impl<T: Number> Default for Softcoded<T> {
    fn default() -> Self {
        Self {
            wts: [(); Feature::len()].map(|_| WeightOf::zero()),
        }
    }
}

impl<T: Number> WeightVec<T> for Softcoded<T> {
    fn weight(&self, f: Feature) -> WeightOf<T> {
        self.wts[f]
    }
}

impl<T: Number + Serialize> Serialize for Softcoded<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // let h: IndexMap<&str, WeightOf<T>> = Feature::iter()
        //     .map(|f| (f.name(), self.weight(f)))
        //     .collect();
        // let h: IndexMap<&str, [T;2]> = Feature::iter()
        //     .map(|f| (f.name(), [self.weight(f).0, self.weight(f).1]))
        //     .collect();
        // h.serialize(serializer)
        let h: IndexMap<&str, [f64; 2]> = Feature::iter()
            .map(|f| {
                (f.name(), [
                    (self.weight(f).0.as_() as f64 * 100.).round() / 100.,
                    (self.weight(f).1.as_() as f64 * 100.).round() / 100.,
                ])
            })
            .collect();
        h.serialize(serializer)
    }
}

impl<'de, T: Number + Serialize> Deserialize<'de> for Softcoded<T>
where
    f32: AsPrimitive<T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut h: IndexMap<String, [f32; 2]> = Deserialize::deserialize(deserializer)?;

        // put the map into a vec
        let mut wt_vec = Softcoded::<T>::default();
        for f in Feature::iter() {
            let wt = h.remove(f.name());
            if let Some([s, e]) = wt {
                wt_vec.wts[f.index()] = WeightOf::<T>::from_f32(s, e);
            } else {
                // warn!("feature weight for {f} not deserialized - assuming zero...");
                return Err(serde::de::Error::missing_field(f.name()));
            }
        }
        if !h.is_empty() {
            for f in h.keys() {
                return Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(&f.to_string()),
                    &"...",
                ));
                // warn!("extra feature weight for {f} found - ignoring...");
            }
        }
        Ok(wt_vec)
    }
}

#[cfg(test)]
mod tests_weights {
    use super::*;
    use crate::test_log::test;

    #[test]
    fn test_weights_basics() {
        let wt_vec1 = Softcoded::<f32> {
            wts: Hardcoded::<f32>::WTS,
        };
        let s = toml::to_string(&wt_vec1).unwrap();
        let wt_vec2: Softcoded<f32> = toml::from_str(&s).unwrap();
        assert_eq!(wt_vec1, wt_vec2);
        println!("f32:\n{s}");

        let wt_vec1 = Softcoded::<i32> {
            wts: Hardcoded::<f32>::WTS.map(|w| WeightOf::<i32>::cast_from(w, Rounding::Round)),
        };
        let s = toml::to_string(&wt_vec1).unwrap();
        let wt_vec2: Softcoded<i32> = toml::from_str(&s).unwrap();
        assert_eq!(wt_vec1, wt_vec2);
        println!("i32:\n");
        for f in Feature::iter() {
            println!(
                "\t({:>4},\t{:>4} ),\t // {f}",
                wt_vec1.weight(f).s(),
                wt_vec1.weight(f).e(),
            );
        }
        println!();

        // one item missing
        let bishop_pair_name = Feature::BishopPair.name();
        let s2 = s
            .lines()
            .filter(|l| !l.contains(bishop_pair_name))
            .join("\n");
        let res = toml::from_str::<Softcoded<i32>>(&s2);
        let err = res.unwrap_err();
        let msg = err.message();
        assert!(msg.contains("missing"), "{}", msg);
        assert!(msg.contains(bishop_pair_name), "{}", msg);

        // one item extra and invalid
        let mut s3 = vec![];
        for l in s.lines() {
            s3.push(l.to_string());
            if l.contains(bishop_pair_name) {
                let l = l.replace(bishop_pair_name, "turkey");
                s3.push(l);
            }
        }
        let s3 = s3.join("\n");
        let res = toml::from_str::<Softcoded<i32>>(&s3);
        let err = res.unwrap_err();
        let msg = err.message();
        assert!(msg.contains("invalid"), "{}", msg);
        assert!(msg.contains("turkey"), "{}", msg);
    }
}

#[derive(Clone, Default, Debug)]
pub struct SummationScorer<T: Number, W> {
    pub total: WeightOf<T>,
    pub wts:   W,
}

#[derive(Clone, Default, Debug)]
pub struct CategoryScorer<T: Number, W = ()> {
    pub totals: [(WeightOf<T>, WeightOf<T>, WeightOf<T>); FeatureCategory::len()],
    pub wts:    W,
}

#[derive(Clone, Debug)]
struct Item<T> {
    w:  i32,
    b:  i32,
    f:  Feature,
    wt: WeightOf<T>,
}

#[derive(Clone, Debug)]
pub struct ExplainingScorer<T, W> {
    items:     Vec<Item<T>>,
    bitboards: HashMap<Feature, Bitboard>,
    wts:       W,
    phase:     Phase,
    board:     Board,
}

impl<T, W> ExplainingScorer<T, W>
where
    W: FnMut(Feature) -> WeightOf<T>,
    T: Number,
{
    pub fn new(board: Board, phase: Phase, wts: W) -> Self {
        Self {
            wts,
            items: Vec::new(),
            bitboards: HashMap::new(),
            phase,
            board,
        }
    }
}

// // BETTER OFF AS A FEATURE VECTOR
// impl< 'a, T, W> ExplainingScorer<T, W>
// where
//     W: FnMut(Feature) -> WeightOf<T> + 'a,
//     T: Number,
// {
//     pub fn write_csv<WR: Write>(
//         iter: impl Iterator<Item = &'a ExplainingScorer<T,W>>,
//         f: &mut WR,
//     ) -> anyhow::Result<i32> {
//         writeln!(
//             f,
//             "{},phase,outcome,ce,fen",
//             Feature::iter().map(|f| f.name()).join(",")
//         )?;
//         let mut count = 0;
//         for r in iter {
//             count += 1;
//             for c in &Feature::all() {
//                 let (w, b) = r.feature_counts(*c);
//                 write!(f, "{},", w - b)?;
//             }
//             writeln!(
//                 f,
//                 "{},{},{},{}",
//                 r.phase,
//                 0, // FIXME outcome not stored
//                 // r.outcome.as_win_fraction(),
//                 0,
//                 r.board.to_fen()
//             )?;
//         }
//         Ok(count)
//     }

// }

impl<T: Number, W> fmt::Display for ExplainingScorer<T, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", ExplainScoreLegacy::from(self))?;
        Ok(())
    }
}

impl<T: Number, W> ExplainingScorer<T, W> {
    pub fn feature_counts(&self, f: Feature) -> (i32, i32) {
        self.items
            .iter()
            .find(|&it| f == it.f)
            .map(|e| (e.w, e.b))
            .unwrap_or_default()
    }

    pub fn item_group_by(&mut self) {
        let mut map: IndexMap<Feature, Item<T>> = IndexMap::new();
        for i in &self.items {
            if i.w != 0 || i.b != 0 {
                if let Some(existing) = map.get(&i.f) {
                    map.insert(i.f, Item {
                        f:  i.f,
                        w:  existing.w + i.w,
                        b:  existing.b + i.b,
                        wt: i.wt,
                    });
                } else {
                    map.insert(i.f, i.clone());
                };
            }
        }
        let mut vec = map.into_values().collect_vec();
        // sort by category then feature index order
        vec.sort_by(|i1, i2| {
            i1.f.category_string()
                .cmp(i2.f.category_string())
                .then(i1.f.cmp(&i2.f))
        });
        self.items = vec;
    }

    pub fn discard_balanced_features(&mut self) {
        let mut i = 0;
        while i < self.items.len() {
            let e = &self.items[i];
            if e.w == e.b {
                self.items.remove(i);
                // no incr with remove
            } else {
                i += 1;
            }
        }
    }
}

impl<T: Number, R, W> SummationScorer<T, W>
where
    W: Fn(Feature) -> R,
{
    pub fn new(wts: W) -> Self {
        Self {
            total: WeightOf::zero(),
            wts,
        }
    }
}

impl<T: Number, R, W> CategoryScorer<T, W>
where
    W: Fn(Feature) -> R,
{
    pub fn new(wts: W) -> Self {
        Self {
            totals: Default::default(),
            wts,
        }
    }
}

impl<T: Number + fmt::Display, W> fmt::Display for CategoryScorer<T, W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, &(w, b, t)) in self.totals.iter().enumerate() {
            if (w, b, t)
                == (
                    WeightOf::<T>::zero(),
                    WeightOf::<T>::zero(),
                    WeightOf::<T>::zero(),
                )
            {
                continue;
            }
            let name = FeatureCategory::all_names()[i];
            writeln!(f, "{name:>15} \t{w:6.0}\t{b:6.0}\t{t:6.0}")?;
        }
        Ok(())
    }
}

// impl<T: Number> Default for CategoryScorer<T, ()> {
//     fn default() -> Self {
//         Self {
//             totals: Default::default(),
//             wts:    Default::default(),
//         }
//     }
// }

// impl<T: Number + fmt::Display, W> fmt::Display for ExplainingScorer<T, W> {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         for i in &self.items {
//             writeln!(f, "{fn:<20}\t{w}\t{b}\t{wt}", fn = i.f.name(), w = i.w, b = i.b, wt = i.wt)?;
//         }
//         Ok(())
//     }
// }

pub struct ExplainScoreLegacy {
    board:        Board,
    items:        Vec<(String, String, i32, i32, WeightOf<f32>)>,
    bitboards:    Vec<(String, Bitboard)>,
    phase:        Phase,
    draw_scaling: f32,
}

impl ExplainScoreLegacy {
    fn from<T: Number, W>(value: &ExplainingScorer<T, W>) -> Self {
        Self {
            board:        value.board.clone(),
            draw_scaling: 1.0,
            phase:        value.phase,
            bitboards:    value
                .bitboards
                .iter()
                .map(|(k, v)| (k.name().to_string(), *v))
                .collect(),
            items:        item_group_by(&value.items)
                .iter()
                .map(|it| {
                    (
                        it.f.name().to_string(),
                        it.f.category_string().to_string(),
                        it.w,
                        it.b,
                        WeightOf::<f32>::cast_from(it.wt, Rounding::None),
                    )
                })
                .collect(),
        }
    }
}

fn item_group_by<T: Number>(items: &Vec<Item<T>>) -> Vec<Item<T>> {
    let mut map: IndexMap<Feature, Item<T>> = IndexMap::new();
    for i in items {
        if i.w != 0 || i.b != 0 {
            if let Some(existing) = map.get(&i.f) {
                map.insert(i.f, Item {
                    f:  i.f,
                    w:  existing.w + i.w,
                    b:  existing.b + i.b,
                    wt: i.wt,
                });
            } else {
                map.insert(i.f, i.clone());
            };
        }
    }
    let mut vec = map.into_values().collect_vec();
    // sort by category then feature index order
    vec.sort_by(|i1, i2| {
        i1.f.category_string()
            .cmp(i2.f.category_string())
            .then(i1.f.cmp(&i2.f))
    });
    vec
}

impl fmt::Display for ExplainScoreLegacy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use tabled::{builder::Builder, object::Segment, Alignment, Modify, Padding, Style};
        fn fp(decimal: f32) -> String {
            Formatting::decimal(2, decimal)
        }

        fn int(int: i32) -> String {
            int.to_string()
        }

        let mut builder = Builder::default();
        builder.set_columns([
            "attr", "w#", "w mg", "w eg", "int", "mg", "eg", "b#", "b mg", "b eg", "wt",
        ]);
        let style = Style::markdown().top('-').bottom('-').top('-');

        writeln!(f, "Phase: {}", self.phase)?;

        let mut tot = Weight::zero();
        let mut grand_tot = Weight::zero();
        let mut iter = self.items.iter().peekable();
        while let Some((i, cat, w, b, wt)) = iter.next() {
            let wt = *wt;
            // let wt = if let Some(ref wv) = self.weights {
            //     wv.weights[i.index()]
            // } else {
            //     Weight::new(1.0, 1.0)
            // };
            let mut row = vec![];
            row.push(i.to_string());

            row.push(int(*w));
            row.push(fp((*w * wt).s()));
            row.push(fp((*w * wt).e()));

            row.push(fp(((w - b) * wt).interpolate(self.phase)));
            tot += (w - b) * wt;
            row.push(fp((*w * wt).s() - (*b * wt).s()));
            row.push(fp((*w * wt).e() - (*b * wt).e()));

            row.push(int(*b));
            row.push(fp((*b * wt).s()));
            row.push(fp((*b * wt).e()));

            row.push(wt.to_string());
            builder.add_record(row);
            if let Some((_, next_cat, ..)) = iter.peek() {
                if cat == next_cat {
                    continue;
                }
            }
            let mut row = vec![];
            row.push(cat.to_string());
            row.push("".into());
            row.push("".into());
            row.push("".into());
            row.push(fp((tot).interpolate(self.phase)));
            row.push(fp(tot.s()));
            row.push(fp(tot.e()));
            builder.add_record(row);
            grand_tot += tot;
            tot = Weight::zero();
            builder.add_record(vec![""]); // blank row
        }
        let mut row = vec![];
        row.push("Total".to_owned());
        row.push("".into());
        row.push("".into());
        row.push("".into());
        row.push(fp((grand_tot).interpolate(self.phase)));
        row.push(fp(grand_tot.s()));
        row.push(fp(grand_tot.e()));
        builder.add_record(row);

        let mut row = vec![];
        row.push("Scaling".to_owned());
        row.push("".into());
        row.push("".into());
        row.push("".into());
        row.push(fp(self.draw_scaling));
        row.push("".into());
        row.push("".into());
        builder.add_record(row);

        let mut row = vec![];
        row.push("Scaled Total".to_owned());
        row.push("".into());
        row.push("".into());
        row.push("".into());
        row.push(fp((self.draw_scaling * grand_tot).interpolate(self.phase)));
        row.push(fp(self.draw_scaling * grand_tot.s()));
        row.push(fp(self.draw_scaling * grand_tot.e()));
        builder.add_record(row);

        let mut tab = builder.build();
        tab = tab
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(1)).with(Padding::new(4, 1, 0, 0)))
            .with(Modify::new(Columns::single(4)).with(Padding::new(4, 1, 0, 0)))
            .with(Modify::new(Columns::single(7)).with(Padding::new(4, 1, 0, 0)))
            .with(style);
        tab.fmt(f)?;
        // writeln!(f, "\n{}", &self.fen)?;

        if f.alternate() {
            let mut builder = Builder::new();
            for y in &self.bitboards.iter().chunks(5) {
                let mut row = vec![];
                for (i, bb) in y {
                    row.push(format!("{}\n{bb:#}", i));
                }
                builder.add_record(row);
            }
            let tab = builder.build();
            tab.fmt(f)?;
            writeln!(f)?;

            writeln!(f, "Pawns:")?;
            let b = &self.board;
            let pawns = Pawns::new(b.pawns() & b.white(), b.pawns() & b.black());
            writeln!(f, "{pawns}")?;
        }

        Ok(())
    }
}

impl<W: Fn(Feature) -> WeightOf<f64>> Scorer<f64> for SummationScorer<f64, W>
// where
//     i32: Mul<WeightOf<f64>, Output = WeightOf<f64>>,
{
    #[inline(always)]
    fn accum(&mut self, f: Feature, w: i32, b: i32) {
        self.total += (w - b) * (self.wts)(f);
    }
    fn total(&self) -> WeightOf<f64> {
        self.total
    }
}

impl<W: Fn(Feature) -> WeightOf<i16>> Scorer<i16> for SummationScorer<i16, W>
// where
//     i32: Mul<WeightOf<i16>, Output = WeightOf<i16>>,
{
    #[inline(always)]
    fn accum(&mut self, f: Feature, w: i32, b: i32) {
        self.total += (w - b) * (self.wts)(f);
    }
    fn total(&self) -> WeightOf<i16> {
        self.total
    }
}

impl<W: Fn(Feature) -> WeightOf<f32>> Scorer<f32> for SummationScorer<f32, W>
// where
// i32: Mul<WeightOf<f32>, Output = WeightOf<f32>>,
{
    #[inline(always)]
    fn accum(&mut self, f: Feature, w: i32, b: i32) {
        self.total += (w - b) * (self.wts)(f);
    }
    fn total(&self) -> WeightOf<f32> {
        self.total
    }
}

impl<W: Fn(Feature) -> WeightOf<i32>> Scorer<i32> for SummationScorer<i32, W>
// where
//     i32: Mul<WeightOf<i32>, Output = WeightOf<i32>>,
{
    #[inline(always)]
    fn accum(&mut self, f: Feature, w: i32, b: i32) {
        self.total += (w - b) * (self.wts)(f);
    }
    fn total(&self) -> WeightOf<i32> {
        self.total
    }
}

// impl<T: Number, W: Fn(Feature) -> WeightOf<T>> Scorer<T> for SummationScorer<T, W>
// where
//     i32: Mul<WeightOf<T>, Output = WeightOf<T>>,
// {
//     fn accum(&mut self, f: Feature, w: i32, b: i32) {
//         self.total += (w - b) * (self.wts)(f);
//     }
//     fn total(&self) -> WeightOf<T> {
//         self.total
//     }
// }

// impl<W: Fn(Feature) -> WeightOf<f64>> Scorer for SummationScorer<f64, W> {
//     fn accum(&mut self, f: Feature, w: i32, b: i32) {
//         self.total += (w - b) * (self.wts)(f);
//     }
// }

// impl<W: Fn(Feature) -> WeightOf<i32>> Scorer for SummationScorer<i32, W> {
//     fn accum(&mut self, f: Feature, w: i32, b: i32) {
//         self.total += (w - b) * (self.wts)(f);
//     }
// }

impl<T: Number, W: Fn(Feature) -> WeightOf<T>> Scorer<T> for CategoryScorer<T, W>
where
    i32: Mul<WeightOf<T>, Output = WeightOf<T>>,
{
    #[inline(always)]
    fn accum(&mut self, f: Feature, w: i32, b: i32) {
        let wt = (self.wts)(f);
        self.totals[f.category().index()].0 += w * wt;
        self.totals[f.category().index()].1 += b * wt;
    }
    fn total(&self) -> WeightOf<T> {
        let w: WeightOf<T> = self.totals.iter().map(|(w, _b, _t)| w).cloned().sum();
        let b: WeightOf<T> = self.totals.iter().map(|(_w, b, _t)| b).cloned().sum();
        w - b
    }
}

// function_eval(b, SummationScorer(||));

// impl Eval<i32> for SummationScorer<i32> {
//     // fn accum(&mut self, f: Feature, w: i32, b: i32, wt: &mut impl Fn(Feature) -> WeightOf<i32>) {
//     //     self.total += (w - b) * wt(f);
//     // }
//     fn set_bits(&mut self, _i: Feature, _bits: Bitboard) {}
// }

impl<T: Number, W: FnMut(Feature) -> WeightOf<T>> Scorer<T> for ExplainingScorer<T, W> {
    #[inline(always)]
    fn accum(&mut self, f: Feature, w: i32, b: i32) {
        let wt = (self.wts)(f);
        if w != 0 || b != 0 {
            self.items.push(Item { w, b, f, wt });
        }
    }

    fn total(&self) -> WeightOf<T> {
        self.items.iter().map(|it| it.wt).sum()
    }

    fn set_bits(&mut self, i: Feature, bits: Bitboard) {
        if bits.any() {
            if let Some(v) = self.bitboards.get_mut(&i) {
                v.insert(bits);
            } else {
                self.bitboards.insert(i, bits);
            }
        }
    }
}
