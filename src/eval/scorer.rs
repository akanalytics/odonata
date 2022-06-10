use std::{collections::HashMap, fmt, fmt::Display, io::Write};

use anyhow::Result;
use itertools::Itertools;
use tabled::{
    builder::Builder,
    object::{Columns, Segment},
    Alignment, Modify, Padding, Style,
};

use crate::{outcome::Outcome, phaser::Phase, utils::Formatting, Bitboard, Color, Piece};

use super::{eval::Feature, feature::WeightsVector, weight::Weight};

pub trait ScorerBase {
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32);
    fn accum(&mut self, c: Color, i: Feature, value: i32);
    fn set_bits(&mut self, i: Feature, bits: Bitboard);
}

#[derive(Debug)]
pub struct TotalScore<'a> {
    weights: &'a [Weight],
    _phase: Phase,
    total: Weight,
}

impl<'a> TotalScore<'a> {
    #[inline]
    pub fn new(weights: &'a Vec<Weight>, _phase: Phase) -> Self {
        Self {
            weights,
            _phase,
            total: Weight::zero(),
        }
    }

    #[inline]
    pub fn total(&self) -> Weight {
        self.total
    }
}

pub fn profile() {
    let a = vec![Weight::default(); 100];
    let mut ts = TotalScore::new(&a, Phase::default());
    let f = TotalScore::accumulate;

    f(&mut ts, Feature::Piece(Piece::Pawn), 3, 2);

    println!("{:?}", ts);
}

impl<'a> ScorerBase for TotalScore<'a> {
    #[inline]
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32) {
        self.total += (w_value as i16 - b_value as i16) * self.weights[i.index()];
    }

    #[inline]
    fn accum(&mut self, c: Color, i: Feature, value: i32) {
        match c {
            Color::White => self.accumulate(i, value, 0),
            Color::Black => self.accumulate(i, 0, value),
        }
    }

    #[inline]
    fn set_bits(&mut self, _i: Feature, _bits: Bitboard) {}
}

#[derive(Clone, Debug, Default)]
pub struct ExplainScore {
    pub outcome: Outcome,
    pub fen: String,
    pub phase: Phase,
    vec: Vec<(Feature, i32, i32, i32)>, // w-val, b-val, feature-index
    weights: Option<WeightsVector>,
    bitboards: HashMap<Feature, Bitboard>,
}

impl ExplainScore {
    pub fn new(phase: Phase, fen: String) -> Self {
        Self {
            phase,
            fen,
            ..Self::default()
        }
    }

    pub fn set_outcome(&mut self, o: Outcome) {
        self.outcome = o;
    }

    pub fn set_weights(&mut self, wts: WeightsVector) {
        self.weights = Some(wts);
    }
}

impl ScorerBase for ExplainScore {
    #[inline]
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32) {
        if w_value != 0 || b_value != 0 {
            // see if weve seen this feature before
            if let Some(index) = self.vec.iter().position(|e| e.0 == i) {
                let old_w = self.vec[index].1;
                let old_b = self.vec[index].2;
                self.vec[index] = (i, w_value + old_w, b_value + old_b, i.index() as i32);
                return;
            }
            self.vec.push((i, w_value, b_value, i.index() as i32));
        }
    }
    #[inline]
    fn accum(&mut self, c: Color, i: Feature, value: i32) {
        match c {
            Color::White => self.accumulate(i, value, 0),
            Color::Black => self.accumulate(i, 0, value),
        }
    }
    #[inline]
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

impl ExplainScore {
    pub fn dot_product(&self, wv: &WeightsVector) -> Weight {
        self.vec
            .iter()
            .map(|e| (e.1 - e.2) * *unsafe { wv.weights.get_unchecked(e.3 as usize) })
            .sum()
    }

    pub fn value(&self, i: Feature) -> i32 {
        self.vec
            .iter()
            .find(|&e| i == e.0)
            .map(|e| (e.1 - e.2))
            .unwrap_or_default()
    }

    pub fn score_for_category(&self, s: &str) -> f32 {
        match self.weights {
            None => 0.0,
            Some(ref wv) => self
                .vec
                .iter()
                .filter(|&e| s == &e.0.category())
                .map(|e| (e.1 - e.2) * wv.weights[e.0.index()]).sum::<Weight>().interpolate(self.phase)
        }
    }

    pub fn discard_balanced_features(&mut self) {
        let mut i = 0;
        while i < self.vec.len() {
            let e = &self.vec[i];
            if e.1 == e.2 {
                self.vec.remove(i);
                // your code here
            } else {
                i += 1;
            }
        }
        // self.vec.drain_filter(|e| (e.1 != e.2));
    }

    pub fn total(&self) -> Weight {
        match self.weights {
            None => self.vec.iter().map(|e| (e.1 - e.2)).sum::<i32>() * Weight::new(1.0, 1.0),
            Some(ref wv) => self
                .vec
                .iter()
                .map(|e| (e.1 - e.2) * wv.weights[e.0.index()])
                .sum(),
        }
    }

    pub fn write_csv<'a, W: Write>(
        iter: impl Iterator<Item = &'a ExplainScore>,
        f: &mut W,
    ) -> Result<i32> {
        writeln!(
            f,
            "{},phase,outcome,ce,fen",
            Feature::all().iter().map(|f| f.name()).join(",")
        )?;
        let mut count = 0;
        for r in iter {
            count += 1;
            for c in &Feature::all() {
                write!(f, "{},", r.value(*c))?;
            }
            writeln!(
                f,
                "{},{},{},{}",
                r.phase,
                r.outcome.as_win_fraction(),
                0,
                r.fen
            )?;
        }
        Ok(count)
    }
}

// TODO! Move to 'tabled' crate
impl Display for ExplainScore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn fp(decimal: f32) -> String {
            Formatting::decimal(2, decimal)
        }

        fn int(int: i32) -> String {
            int.to_string()
        }

        let mut builder = Builder::default().set_columns([
            "attr", "w#", "w mg", "w eg", "int", "mg", "eg", "b#", "b mg", "b eg", "wt",
        ]);
        let style = Style::github_markdown().top('-').bottom('-').top('-');

        let mut tot = Weight::zero();
        let mut grand_tot = Weight::zero();
        let mut iter = self.vec.iter().peekable();
        while let Some((i, w, b, _index)) = iter.next() {
            let wt = if let Some(ref wv) = self.weights {
                wv.weights[i.index()]
            } else {
                Weight::new(1.0, 1.0)
            };
            let mut row = vec![];
            row.push(i.name());

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
            builder = builder.add_record(row);
            if let Some((j, _, _, _)) = iter.peek() {
                if i.category() == j.category() {
                    continue;
                }
            }
            let mut row = vec![];
            row.push(i.category());
            row.push("".into());
            row.push("".into());
            row.push("".into());
            row.push(fp((tot).interpolate(self.phase)));
            row.push(fp(tot.s()));
            row.push(fp(tot.e()));
            builder = builder.add_record(row);
            grand_tot += tot;
            tot = Weight::zero();
            builder = builder.add_record(vec![""]); // blank row
        }
        let mut row = vec![];
        row.push("Total".to_owned());
        row.push("".into());
        row.push("".into());
        row.push("".into());
        row.push(fp((grand_tot).interpolate(self.phase)));
        row.push(fp(grand_tot.s()));
        row.push(fp(grand_tot.e()));
        builder = builder.add_record(row);
        let mut tab = builder.build();
        tab = tab
            .with(Modify::new(Segment::all()).with(Alignment::right()))
            .with(Modify::new(Columns::single(1)).with(Padding::new(4, 1, 0, 0)))
            .with(Modify::new(Columns::single(4)).with(Padding::new(4, 1, 0, 0)))
            .with(Modify::new(Columns::single(7)).with(Padding::new(4, 1, 0, 0)))
            .with(style);
        tab.fmt(f)?;
        f.write_str(&self.fen)?;

        if f.alternate() {
            let mut builder = Builder::new();
            for y in &self.bitboards.iter().chunks(5) {
                let mut row = vec![];
                for (i, bb) in y {
                    row.push(format!("{}\n{bb:#}", i.name()));
                }
                builder = builder.add_record(row);
            }
            let tab = builder.build();
            tab.fmt(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::eval::calc::Calc;
    use crate::phaser::Phaser;
    use crate::search::engine::Engine;
    use crate::test_log::test;
    // use crate::utils::StringUtils;

    #[test]
    fn test_explain() {
        let positions = Catalog::bratko_kopec();
        let end_games = Catalog::end_games();
        let mut eng = Engine::new();
        let eval = &mut eng.algo.eval;
        eval.populate_feature_weights();
        let phaser = Phaser::default();
        let mut builder = Builder::new().set_columns(["old", "TotalScore", "ExplainScore"]);
        for pos in positions.iter().chain(end_games.iter()) {
            let b = pos.board();

            let mut scorer2 = TotalScore::new(&eval.feature_weights, b.phase(&phaser));
            Calc::score(&mut scorer2, &b, &eval, &phaser);

            let mut scorer3 = ExplainScore::new(b.phase(&phaser), pos.board().to_fen());
            scorer3.set_weights(eval.weights_vector());
            Calc::score(&mut scorer3, &b, &eval, &phaser);

            builder = builder.add_record([scorer2.total().to_string(), scorer3.to_string()]);
            if scorer2.total().to_string() != scorer3.total().to_string() {
                builder = builder
                    .add_record([
                        format!("{:.6}", scorer2.total()),
                        format!("{:.6}", scorer3.total()),
                    ])
                    .add_record(["Fail!", ""]);
                break;
            }
        }
        let table = builder.build();
        println!("{table}");
    }
}
