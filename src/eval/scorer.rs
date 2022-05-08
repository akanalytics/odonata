use std::{fmt, fmt::Display, io::Write};

use anyhow::Result;
use comfy_table::{presets, Cell, CellAlignment, Row, Table};
use itertools::Itertools;

use crate::{outcome::Outcome, phaser::Phase, utils::Formatting, Color};

use super::{
    eval::Feature,
    feature::{FeatureVector, Sparse, WeightsVector},
    score::Score,
    switches::Switches,
    weight::Weight,
};

pub trait ScorerBase {
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32, wt: Weight);
    fn accum(&mut self, c: Color, i: Feature, value: i32, score: Weight);
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
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32, _wt: Weight) {
        self.total += (w_value - b_value) * self.weights[i.index()];
    }

    #[inline]
    fn accum(&mut self, c: Color, i: Feature, value: i32, wt: Weight) {
        match c {
            Color::White => self.accumulate(i, value, 0, wt),
            Color::Black => self.accumulate(i, 0, value, wt),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ExplainScore {
    pub outcome: Outcome,
    pub fen: String,
    pub phase: Phase,
    vec: Vec<(Feature, i32, i32, i32)>,
    weights: Option<WeightsVector>,
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
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32, _wt: Weight) {
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
    fn accum(&mut self, c: Color, i: Feature, value: i32, score: Weight) {
        if c == Color::White {
            self.accumulate(i, value, 0, score);
        } else {
            self.accumulate(i, 0, value, score);
        }
    }
}

impl ExplainScore {
    pub fn dot_product(&self, wv: &WeightsVector) -> Weight {
        self.vec.iter().map(|e| (e.1 - e.2) * *unsafe { wv.weights.get_unchecked(e.3 as usize) }).sum()
    }

    fn value(&self, i: Feature) -> Option<i32> {
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
        Ok(())
    }
}

pub trait Scorer: ScorerBase {
    // fn set_multiplier(&mut self, m: i32);
    fn annotate(&mut self, annotation: &str);
    fn csv(&self) -> bool;
    fn material(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn position(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn pawn(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn mobility(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn safety(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn tempo(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn contempt(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn interpolate_and_scale(&mut self, attr: &str);
    fn set_phase(&mut self, p: Phase);
    fn phase(&self) -> Phase;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExplainScorer {
    mat: Vec<(String, i32, i32, Weight)>,
    pos: Vec<(String, i32, i32, Weight)>,
    paw: Vec<(String, i32, i32, Weight)>,
    mob: Vec<(String, i32, i32, Weight)>,
    saf: Vec<(String, i32, i32, Weight)>,
    tem: Vec<(String, i32, i32, Weight)>,
    con: Vec<(String, i32, i32, Weight)>,
    delegate: ModelScore,
    fen: String,
    pub csv: bool,
}

pub enum ReportLine {
    Header,
    Body,
}

impl ExplainScorer {
    pub fn new(fen: String, csv: bool) -> Self {
        Self {
            fen,
            delegate: ModelScore::new(),
            pos: Vec::with_capacity(32),
            mat: Vec::with_capacity(6),
            csv,
            ..Self::default()
        }
    }

    pub fn as_score(&self) -> Score {
        self.delegate.as_score()
    }

    pub fn as_csv(&self, line: ReportLine, consolidated: bool) -> String {
        let mut output = String::with_capacity(1000);
        for (i, _sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![&self.mat, &self.pos, &self.mob, &self.paw, &self.saf, &self.con, &self.tem][i];
            for (attr, w, b, wt) in vec {
                let (attr, w, b, _wt) = (attr, *w, *b, *wt);
                if consolidated {
                    let field = match line {
                        ReportLine::Header => format!("{}", attr.replace(" ", "_")),
                        ReportLine::Body => (w - b).to_string(),
                    };
                    output.push_str(&field);
                    output.push(',');
                } else {
                    let field_s = match line {
                        ReportLine::Header => format!("{}.s", attr.replace(" ", "_")),
                        ReportLine::Body => {
                            format!("{}", (w - b) as f32 * (100 - self.delegate.phase.0) as f32 / 100.0)
                        }
                    };
                    output.push_str(&field_s);
                    output.push_str(", ");

                    let field_e = match line {
                        ReportLine::Header => format!("{}.e", attr.replace(" ", "_")),
                        ReportLine::Body => format!("{}", (w - b) as f32 * self.delegate.phase.0 as f32 / 100.0),
                    };
                    output.push_str(&field_e);
                    output.push_str(", ");
                }
            }
        }
        output
    }

    pub fn into_feature_vector(self, o: Outcome) -> FeatureVector {
        let mut fv = FeatureVector::default();
        fv.phase = self.phase();
        fv.outcome = o;
        fv.fen = self.fen;
        let mut index = 0;
        for (i, _sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![&self.mat, &self.mob, &self.paw, &self.saf, &self.con, &self.tem, &self.pos][i];
            for (attr, w, b, wt) in vec {
                let (_attr, w, b, _wt) = (attr, *w, *b, *wt);
                // sparse!
                if (w - b) != 0 {
                    fv.values.push(Sparse::new((w - b).try_into().unwrap(), index));
                }
                index += 1;
            }
        }
        fv
    }
    // fv.push(attr.replace(" ", "_")),
    pub fn feature_names(&self) -> Vec<String> {
        let mut v = Vec::new();
        for (i, _sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![&self.mat, &self.mob, &self.paw, &self.saf, &self.con, &self.tem, &self.pos][i];
            for (attr, w, b, wt) in vec {
                let (attr, _w, _b, _wt) = (attr, *w, *b, *wt);
                v.push(attr.replace(" ", "_"));
            }
        }
        v
    }

    pub fn weights_vector(&self) -> WeightsVector {
        let mut weights_vec = WeightsVector::default();
        for (i, _sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![&self.mat, &self.mob, &self.paw, &self.saf, &self.con, &self.tem, &self.pos][i];
            for (attr, w, b, wt) in vec {
                let (attr, _w, _b, wt) = (attr, *w, *b, *wt);
                weights_vec.weights.push(wt);
                weights_vec.names.push(attr.to_owned());
            }
        }
        weights_vec
    }
    #[inline]
    pub fn total(&self) -> Weight {
        self.delegate.total()
    }
}

impl ScorerBase for ExplainScorer {
    // fn set_multiplier(&mut self, mult: i32) {
    //     self.delegate.set_multiplier(mult);
    // }

    #[inline]
    fn accum(&mut self, c: Color, i: Feature, value: i32, score: Weight) {
        if c == Color::White {
            self.accumulate(i, value, 0, score);
            // self.delegate.accum(c, i, value,score);
        } else {
            self.accumulate(i, 0, value, score);
            // self.delegate.accum(c, i, value, score);
        }
    }

    #[inline]
    fn accumulate(&mut self, i: Feature, w_value: i32, b_value: i32, score: Weight) {
        let cat = match i.category().as_str() {
            "Pawn" => &mut self.paw,
            "Material" => &mut self.mat,
            "Position" => &mut self.pos,
            "Safety" => &mut self.saf,
            "Mobility" => &mut self.mob,
            _ => &mut self.tem,
        };
        // if i.index() >= cat.len() {
        //     cat.resize(i.index() + 1, (String::new(), 0, 0, Weight::zero()));
        // }
        // let old_w = cat[i.index()].1;
        // let old_b = cat[i.index()].2;
        // cat[i.index()] = (i.name(), w_value + old_w, b_value + old_b, score);

        if let Some(index) = cat.iter().position(|e| e.0 == i.name()) {
            let old_w = cat[index].1;
            let old_b = cat[index].2;
            cat[index] = (i.name(), w_value + old_w, b_value + old_b, score);
        } else {
            cat.push((i.name(), w_value, b_value, score));
        }
        self.delegate.accumulate(i, w_value, b_value, score);
    }
}

impl Scorer for ExplainScorer {
    #[inline]
    fn annotate(&mut self, _annotation: &str) {}

    fn csv(&self) -> bool {
        self.csv
    }
    #[inline]
    fn material(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.mat.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.material(_attr, w_value, b_value, score);
    }
    #[inline]
    fn position(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.pos.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.position(_attr, w_value, b_value, score);
    }
    #[inline]
    fn pawn(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.paw.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.pawn(_attr, w_value, b_value, score);
    }

    #[inline]
    fn mobility(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.mob.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.mobility(_attr, w_value, b_value, score);
    }

    #[inline]
    fn safety(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.saf.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.safety(_attr, w_value, b_value, score);
    }

    #[inline]
    fn tempo(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.tem.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.tempo(_attr, w_value, b_value, score);
    }

    #[inline]
    fn contempt(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.con.push((_attr.to_string(), w_value, b_value, score));
        self.delegate.contempt(_attr, w_value, b_value, score);
    }

    #[inline]
    fn interpolate_and_scale(&mut self, _attr: &str) {
        self.delegate.interpolate_and_scale(_attr);
    }

    #[inline]
    fn set_phase(&mut self, p: Phase) {
        self.delegate.phase = p;
    }

    #[inline]
    fn phase(&self) -> Phase {
        self.delegate.phase()
    }
}

impl fmt::Display for ExplainScorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn pad<T: Into<f64>>(t: T) -> String {
            Formatting::decimal(2, t)
        }

        writeln!(
            f,
            "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
            "attr", "w", "w mg", "w eg", "int", "mg", "eg", "b", "b mg", "b eg", "wt"
        )?;
        for (i, sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![&self.mat, &self.pos, &self.mob, &self.paw, &self.saf, &self.con, &self.tem][i];
            for (attr, w, b, wt) in vec {
                let (attr, w, b, wt) = (attr, *w, *b, *wt);
                if w != 0 || b != 0 {
                    writeln!(
                        f,
                        "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                        attr,
                        w,
                        pad((w * wt).s()),
                        pad((w * wt).e()),
                        pad(((w * wt) - (b * wt)).interpolate(self.phase())),
                        pad((w * wt).s() - (b * wt).s()),
                        pad((w * wt).e() - (b * wt).e()),
                        b,
                        pad((b * wt).s()),
                        pad((b * wt).e()),
                        wt.to_string()
                    )?;
                }
            }
            if true {
                // !sw.intersects(Switches::TEMPO | Switches::CONTEMPT) {
                let attr = sw.name();
                let wwt: Weight = vec.iter().map(|&(_, w, _b, wt)| w * wt).sum();
                let bwt: Weight = vec.iter().map(|&(_, _w, b, wt)| b * wt).sum();
                let twt: Weight = vec.iter().map(|&(_, w, b, wt)| w * wt - b * wt).sum();
                writeln!(
                    f,
                    "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                    "", "-----", "-----", "-----", "-----", "-----", "-----", "-----", "-----", "-----", ""
                )?;
                writeln!(
                    f,
                    "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                    attr,
                    "",
                    pad(wwt.s()),
                    pad(wwt.e()),
                    pad(twt.interpolate(self.phase())),
                    pad(twt.s()),
                    pad(twt.e()),
                    "",
                    pad(bwt.s()),
                    pad(bwt.e()),
                    ""
                )?;
                writeln!(
                    f,
                    "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
                    "", "", "", "", "", "", "", "", "", "", ""
                )?;
            }
        }
        writeln!(
            f,
            "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
            "", "-----", "-----", "-----", "=====", "-----", "-----", "-----", "-----", "-----", "=========="
        )?;
        writeln!(
            f,
            "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |      Phase{:>3} %",
            "EVALUATION",
            "",
            "",
            "",
            pad(self.total().interpolate(self.phase())),
            pad(self.total().s()),
            pad(self.total().e()),
            "",
            "",
            "",
            self.phase().0
        )?;
        writeln!(
            f,
            "{:>27} | {:>7} {:>7} {:>7} | {:>7}  {:>7} {:>7} | {:>7} {:>7} {:>7} |   {:<15}",
            "", "", "", "", "=====", "-----", "-----", "", "", "", "=========="
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelScore {
    phase: Phase,
    drawish: i32,
    material: Weight,
    position: Weight,
    pawn: Weight,
    mobility: Weight,
    safety: Weight,
    tempo: Weight,
    contempt: Weight,
    interpolated: f32,
}

impl ModelScore {
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    pub fn as_f32(&self) -> f32 {
        self.interpolated
    }

    pub fn as_score(&self) -> Score {
        Score::from_cp(self.interpolated as i32)
        // Score::from_cp(self.interpolated.round() as i32)
    }

    #[inline]
    pub fn total(&self) -> Weight {
        self.material + self.position + self.pawn + self.mobility + self.safety + self.tempo + self.contempt
    }
}

impl ScorerBase for ModelScore {
    fn accum(&mut self, c: Color, _i: Feature, value: i32, score: Weight) {
        if c == Color::White {
            self.tempo += (value - 0) * score;
        } else {
            self.tempo += (0 - value) * score;
        }
    }

    fn accumulate(&mut self, _i: Feature, w_value: i32, b_value: i32, score: Weight) {
        self.tempo += (w_value - b_value) * score;
    }
}

impl Scorer for ModelScore {
    #[inline]
    fn annotate(&mut self, _annotation: &str) {}

    // #[inline]
    // fn set_multiplier(&mut self, mult: i32) {
    //     self.mult = mult;
    // }

    fn csv(&self) -> bool {
        false
    }

    #[inline]
    fn material(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.material += (w_value - b_value) * score;
    }
    #[inline]
    fn position(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.position += (w_value - b_value) * score;
    }
    #[inline]
    fn pawn(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.pawn += (w_value - b_value) * score;
    }
    #[inline]
    fn mobility(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.mobility += (w_value - b_value) * score;
    }

    #[inline]
    fn safety(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.safety += (w_value - b_value) * score;
    }

    #[inline]
    fn tempo(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.tempo += (w_value - b_value) * score;
    }

    #[inline]
    fn contempt(&mut self, _attr: &str, w_value: i32, b_value: i32, score: Weight) {
        self.contempt += (w_value - b_value) * score;
    }

    #[inline]
    fn interpolate_and_scale(&mut self, _attr: &str) {
        self.interpolated = self.total().interpolate(self.phase) as f32 * (100 - 0 * self.drawish) as f32 / 100.0;
    }

    #[inline]
    fn set_phase(&mut self, p: Phase) {
        self.phase = p;
    }

    #[inline]
    fn phase(&self) -> Phase {
        self.phase
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::eval::calc::Calc;
    use crate::eval::model::Model;
    use crate::eval::scorer::ExplainScorer;
    use crate::eval::switches::Switches;
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
            let mut scorer1 = ExplainScorer::new(b.to_fen(), false);
            scorer1.set_phase(b.phase(&phaser));
            let mut model = Model::from_board(b, b.phase(&phaser), Switches::ALL_SCORING);
            model.csv = false;
            eval.predict(&model, &mut scorer1);

            let mut scorer2 = TotalScore::new(&eval.feature_weights, b.phase(&phaser));
            Calc::score(&mut scorer2, &b, &eval, &phaser);

            let mut scorer3 = ExplainScore::new(b.phase(&phaser), pos.board().to_fen());
            scorer3.set_weights(eval.weights_vector());
            Calc::score(&mut scorer3, &b, &eval, &phaser);

            table.add_row(vec![scorer1.to_string(), scorer2.total().to_string(), scorer3.to_string()]);
            if scorer1.total().to_string() != scorer2.total().to_string() || scorer2.total().to_string() != scorer3.total().to_string() {
                table.add_row(vec![
                    format!("{:.6}", scorer1.total()),
                    format!("{:.6}", scorer2.total()),
                    format!("{:.6}", scorer3.total()),
                ]);
                table.add_row(vec![Cell::new("Fail!").bg(Color::Red)]);
                break;
            }
        }
        println!("{table}");
    }
}
