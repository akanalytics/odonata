use std::fmt;

use crate::{phaser::Phase, outcome::Outcome, utils::Formatting};

use super::{weight::Weight, switches::Switches, feature::{FeatureVector, WeightsVector}, score::Score};

pub trait Scorer {
    // fn set_multiplier(&mut self, m: i32);
    fn annotate(&mut self, annotation: &str);
    fn material(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn position(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn pawn(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn mobility(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn safety(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn tempo(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn contempt(&mut self, attr: &str, w_value: i32, b_value: i32, score: Weight);
    fn interpolate_and_scale(&mut self, attr: &str);
    fn total(&self) -> Weight;
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
}

pub enum ReportLine {
    Header,
    Body,
}


impl ExplainScorer {
    pub fn new() -> Self {
        Self {
            delegate: ModelScore::new(),
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

    pub fn feature_vector(&self, o: Outcome) -> FeatureVector {
        let mut fv = FeatureVector::default();
        fv.phase = self.phase();
        fv.outcome = o;
        let mut index = 0;
        for (i, _sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![&self.mat, &self.pos, &self.mob, &self.paw, &self.saf, &self.con, &self.tem][i];
            for (attr, w, b, wt) in vec {
                let (_attr, w, b, _wt) = (attr, *w, *b, *wt);
                // sparse!
                if (w - b) != 0 {
                    fv.values.push((w - b, index));
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
            let vec = vec![&self.mat, &self.pos, &self.mob, &self.paw, &self.saf, &self.con, &self.tem][i];
            for (attr, w, b, wt) in vec {
                let (attr, _w, _b, _wt) = (attr, *w, *b, *wt);
                v.push(attr.clone());
            }
        }
        v
    }

    pub fn weights_vector(&self) -> WeightsVector {
        let mut weights_vec = WeightsVector::default();
        for (i, _sw) in Switches::all_scoring().iter().enumerate() {
            let vec = vec![&self.mat, &self.pos, &self.mob, &self.paw, &self.saf, &self.con, &self.tem][i];
            for (attr, w, b, wt) in vec {
                let (attr, _w, _b, wt) = (attr, *w, *b, *wt);
                weights_vec.weights.push((attr.clone(), wt));
            }
        }
        weights_vec
    }
}

impl Scorer for ExplainScorer {
    #[inline]
    fn annotate(&mut self, _annotation: &str) {}

    // fn set_multiplier(&mut self, mult: i32) {
    //     self.delegate.set_multiplier(mult);
    // }
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
    fn total(&self) -> Weight {
        self.delegate.total()
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
        Self {
            ..Self::default()
        }
    }

    pub fn as_f32(&self) -> f32 {
        self.interpolated
    }

    pub fn as_score(&self) -> Score {
        Score::from_cp(self.interpolated.round() as i32)
    }
}

impl Scorer for ModelScore {
    #[inline]
    fn annotate(&mut self, _annotation: &str) {}

    // #[inline]
    // fn set_multiplier(&mut self, mult: i32) {
    //     self.mult = mult;
    // }

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
    fn total(&self) -> Weight {
        self.material + self.position + self.pawn + self.mobility + self.safety + self.tempo + self.contempt
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
