use std::{fmt, fmt::Display, io::Write, collections::HashMap};

use anyhow::Result;
use comfy_table::{presets, Cell, CellAlignment, Row, Table};
use itertools::Itertools;

use crate::{outcome::Outcome, phaser::Phase, utils::Formatting, Bitboard, Color};

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

impl<'a> ScorerBase for TotalScore<'a> {
    #[inline]
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32) {
        self.total += (w_value - b_value) * self.weights[i.index()];
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
    vec: Vec<(Feature, i32, i32, i32)>,
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
        if c == Color::White {
            self.accumulate(i, value, 0);
        } else {
            self.accumulate(i, 0, value);
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

    pub fn value(&self, i: Feature) -> Option<i32> {
        self.vec.iter().find(|&e| i == e.0).map(|e| (e.1 - e.2) as i32)
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
            Some(ref wv) => self.vec.iter().map(|e| (e.1 - e.2) * wv.weights[e.0.index()]).sum(),
        }
    }

    pub fn write_csv<'a, W: Write>(iter: impl Iterator<Item = &'a ExplainScore>, f: &mut W) -> Result<i32> {
        writeln!(f, "{},phase,outcome,ce,fen", Feature::all().iter().map(|f| f.name()).join(","))?;
        let mut count = 0;
        for r in iter {
            count += 1;
            for c in &Feature::all() {
                match r.value(*c) {
                    Some(v) => write!(f, "{},", v),
                    None => write!(f, "0,"),
                }?;
            }
            writeln!(f, "{},{},{},{}", r.phase, r.outcome.as_win_fraction(), 0, r.fen)?;
        }
        Ok(count)
    }
}

// TODO! Move to 'tabled' crate
impl Display for ExplainScore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn fp(decimal: f32) -> Cell {
            Cell::new(Formatting::decimal(2, decimal).to_string())
        }

        fn int(int: i32) -> Cell {
            Cell::new(int.to_string())
        }

        let mut tab = Table::new();
        let row = Row::from(vec!["attr", "w#", "w mg", "w eg", "int", "mg", "eg", "b#", "b mg", "b eg", "wt"]);
        tab.load_preset(presets::ASCII_BORDERS_ONLY_CONDENSED).set_header(row);
        tab.column_iter_mut()
            .skip(1)
            .for_each(|c| c.set_cell_alignment(CellAlignment::Right));
        tab.get_column_mut(1).unwrap().set_padding((1, 2));
        tab.get_column_mut(4).unwrap().set_padding((1, 2));
        tab.get_column_mut(7).unwrap().set_padding((1, 2));
        let mut tot = Weight::zero();
        let mut grand_tot = Weight::zero();
        let mut iter = self.vec.iter().peekable();
        while let Some((i, w, b, _index)) = iter.next() {
            let wt = if let Some(ref wv) = self.weights {
                wv.weights[i.index()]
            } else {
                Weight::new(1.0, 1.0)
            };
            let mut row = Row::new();
            row.add_cell(Cell::new(i.name()));
            row.add_cell(int(*w));
            row.add_cell(fp((*w * wt).s()));
            row.add_cell(fp((*w * wt).e()));
            row.add_cell(fp((*w * wt).interpolate(self.phase)));
            tot += (w - b) * wt;
            row.add_cell(fp((*w * wt).s() - (*b * wt).s()));
            row.add_cell(fp((*w * wt).s() - (*b * wt).e()));
            row.add_cell(int(*b));
            row.add_cell(fp((*b * wt).s()));
            row.add_cell(fp((*b * wt).e()));
            row.add_cell(Cell::new(wt));
            tab.add_row(row);
            if let Some((j, _, _, _)) = iter.peek() {
                if i.category() == j.category() {
                    continue;
                }
            }
            let mut row = Row::new();
            row.add_cell(Cell::new(i.category()));
            row.add_cell("".into()).add_cell("".into()).add_cell("".into());
            row.add_cell(fp((tot).interpolate(self.phase)));
            row.add_cell(fp(tot.s()));
            row.add_cell(fp(tot.e()));
            tab.add_row(row);
            grand_tot += tot;
            tot = Weight::zero();
            tab.add_row(Row::new()); // blank row
        }
        let mut row = Row::new();
        row.add_cell(Cell::new("Total"));
        row.add_cell("".into()).add_cell("".into()).add_cell("".into());
        row.add_cell(fp((grand_tot).interpolate(self.phase)));
        row.add_cell(fp(grand_tot.s()));
        row.add_cell(fp(grand_tot.e()));
        tab.add_row(row);
        tab.fmt(f)?;
        f.write_str(&self.fen)?;
        if f.alternate() {
            let mut tab = Table::new();
            for y in &self.bitboards.iter().chunks(5) {
                let mut row = Row::new(); 
                for (i, bb) in y {
                    row.add_cell(format!("{}\n{bb:#}",i.name()).into());
                }
                tab.add_row(row);
            }
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
    use comfy_table::{Cell, Color, Table};
    // use crate::utils::StringUtils;

    #[test]
    fn test_explain() {
        let positions = Catalog::bratko_kopec();
        let end_games = Catalog::end_games();
        let mut eng = Engine::new();
        let eval = &mut eng.algo.eval;
        eval.populate_feature_weights();
        let phaser = Phaser::default();
        let mut table = Table::new();
        table.set_header(vec!["old", "TotalScore", "ExplainScore"]);
        for pos in positions.iter().chain(end_games.iter()) {
            let b = pos.board();

            let mut scorer2 = TotalScore::new(&eval.feature_weights, b.phase(&phaser));
            Calc::score(&mut scorer2, &b, &eval, &phaser);

            let mut scorer3 = ExplainScore::new(b.phase(&phaser), pos.board().to_fen());
            scorer3.set_weights(eval.weights_vector());
            Calc::score(&mut scorer3, &b, &eval, &phaser);

            table.add_row(vec![scorer2.total().to_string(), scorer3.to_string()]);
            if scorer2.total().to_string() != scorer3.total().to_string() {
                table.add_row(vec![format!("{:.6}", scorer2.total()), format!("{:.6}", scorer3.total())]);
                table.add_row(vec![Cell::new("Fail!").bg(Color::Red)]);
                break;
            }
        }
        println!("{table}");
    }
}
