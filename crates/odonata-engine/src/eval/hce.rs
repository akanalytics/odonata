use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use odonata_base::boards::Position;
use odonata_base::domain::node::{Counter, Event, Node};
use odonata_base::domain::staticeval::{EvalExplain, StaticEval};
use odonata_base::eg::endgame::EndGameScoring;
use odonata_base::infra::component::{Component, State};
use odonata_base::infra::lockless_hashmap::UnsharedTable;
use odonata_base::infra::metric::Metrics;
use odonata_base::other::{Phase, Phaser};
use odonata_base::prelude::*;
use once_cell::unsync::OnceCell;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::eval::evaluation::Evaluation;
use crate::eval::feature::{Feature, FeatureCategory};
use crate::eval::scoring::{Scorer, Softcoded, SummationScorer, WeightVec};
use crate::eval::see::See;
use crate::eval::weight::{Rounding, Weight, WeightOf};

// https://www.chessprogramming.org/Simplified_Evaluation_Function

#[derive(Clone)]
pub struct Hce {
    pub hce_file:           PathBuf,
    pub phasing:            bool,
    weights_kind:           WeightsKind,
    rounding:               Rounding,
    mobility_phase_disable: u8,
    pub quantum:            i32,
    cache_size:             usize,
    draw_scaling:           f32,
    draw_scaling_noisy:     f32,
    see:                    See,
    pub endgame:            EndGameScoring,
    pub phaser:             Phaser,
    weights_raw:            Softcoded<f64>,
    eval_cache:             UnsharedTable<Score>,
    weights_i32:            OnceCell<Softcoded<i32>>,
    weights_f64:            OnceCell<Softcoded<f64>>,
    weights_f32:            OnceCell<Softcoded<f32>>,
}

impl Default for Hce {
    fn default() -> Self {
        const DEFAULT_CACHE_SIZE: usize = 10_000;
        let hce_file = "eval.hce.toml";
        Self {
            hce_file:               hce_file.into(),
            weights_kind:           WeightsKind::SoftcodedF64,
            rounding:               Rounding::None,
            weights_raw:            Softcoded::load(hce_file).expect("unable to load default weights"),
            weights_i32:            Default::default(),
            weights_f32:            Default::default(),
            weights_f64:            Default::default(),
            draw_scaling:           1.,
            draw_scaling_noisy:     1.,
            see:                    See::default(),
            endgame:                EndGameScoring::default(),
            phaser:                 Phaser::default(),
            phasing:                true,
            mobility_phase_disable: 101,
            quantum:                1,
            cache_size:             DEFAULT_CACHE_SIZE,
            eval_cache:             UnsharedTable::with_size(DEFAULT_CACHE_SIZE),
        }
    }
}

impl Configurable for Hce {
    fn set(&mut self, p: Param) -> Result<bool> {
        if self.hce_file.set(p.get("hce_file"))? {
            self.reload_weights()?;
        }
        self.phasing.set(p.get("phasing"))?;
        self.weights_kind.set(p.get("weights_kind"))?;
        self.rounding.set(p.get("rounding"))?;
        self.mobility_phase_disable.set(p.get("mobility_phase_disable"))?;
        self.quantum.set(p.get("quantum"))?;
        self.cache_size.set(p.get("cache_size"))?;
        self.draw_scaling.set(p.get("draw_scaling"))?;
        self.draw_scaling_noisy.set(p.get("draw_scaling_noisy"))?;
        self.see.set(p.get("see"))?;
        self.endgame.set(p.get("endgame"))?;
        self.phaser.set(p.get("phaser"))?;
        self.eval_cache.set(p.get("eval_cache"))?;
        Ok(p.is_modified())
    }
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Serialize, Deserialize, EnumString)]
#[serde(deny_unknown_fields)]
enum WeightsKind {
    // HardcodedF64,
    // HardcodedI32,
    // HardcodedI32Millis,
    SoftcodedF64,
    SoftcodedF32,
    SoftcodedI32,
}

#[derive(Default, Debug)]
struct ExplainVector {
    color:        Option<Color>,
    board:        Board,
    items:        HashMap<Feature, i16>,
    bitboards:    HashMap<Feature, Bitboard>,
    phase:        Phase,
    draw_scaling: f32,
}

impl ExplainVector {
    pub fn to_eval_explain(explains: &[Self], weights: &[Weight]) -> EvalExplain {
        use std::fmt::Write;
        let mut s = String::new();
        let mut e = EvalExplain::default();
        let mut total = [0.0; 6];
        for fc in FeatureCategory::iter() {
            let mut f_scores = [0.0; 6];
            writeln!(&mut s, "| {fc:<30} |{:>17}|{:>17}|{:>17}|", "", "", "").unwrap();

            for f in fc.features() {
                let (w, b, tot) = (
                    explains[0].items.get(&f).cloned().unwrap_or_default(),
                    explains[1].items.get(&f).cloned().unwrap_or_default(),
                    explains[2].items.get(&f).cloned().unwrap_or_default(),
                );
                let wt = weights[f.index()];
                let s0 = w as f64 * wt.s() as f64 / 100.0;
                let s1 = w as f64 * wt.e() as f64 / 100.0;
                let s2 = b as f64 * wt.s() as f64 / 100.0;
                let s3 = b as f64 * wt.e() as f64 / 100.0;
                let s4 = tot as f64 * wt.s() as f64 / 100.0;
                let s5 = tot as f64 * wt.e() as f64 / 100.0;

                if (w, b, tot) != (0, 0, 0) {
                    writeln!(
                        &mut s,
                        "|   {f:<28} | {w:>3} {s0:5.2} {s1:5.2} | {b:>3} {s2:5.2} {s3:5.2} | {tot:>3} {s4:5.2} {s5:5.2} |"
                    )
                    .unwrap();
                    let scores = [s0, s1, s2, s3, s4, s5];
                    (0..6).for_each(|i| f_scores[i] += scores[i]);
                    (0..6).for_each(|i| total[i] += scores[i]);
                }
            }
            e.scores.insert(fc.to_string(), f_scores);
        }
        *e.scores.entry("Total".to_string()).or_default() = total;
        let ex = &explains[0];
        let mut ai = String::new();
        writeln!(&mut ai, "{s}\n\n").unwrap();
        writeln!(&mut ai, "phase   : {}%", ex.phase).unwrap();
        writeln!(&mut ai, "balance : {}", ex.board.material().balance()).unwrap();
        writeln!(&mut ai, "{:#}", ex.board.to_diagram()).unwrap();
        writeln!(&mut ai, "end of hce explain").unwrap();
        e.additional_info = ai;
        e
    }
}

impl Scorer<f64> for ExplainVector {
    fn accum(&mut self, f: Feature, w: i32, b: i32) {
        let d = match self.color {
            None => (w - b) as i16,
            Some(Color::White) => w as i16,
            Some(Color::Black) => b as i16,
        };

        if d != 0 {
            *self.items.entry(f).or_default() += d;
        }
    }

    fn total(&self) -> WeightOf<f64> {
        unreachable!();
    }

    fn set_bits(&mut self, f: Feature, bits: Bitboard) {
        self.bitboards.insert(f, bits);
    }

    fn apply_scaling(&mut self, _scaling: f32) {}
}

impl StaticEval for Hce {
    fn static_eval(&self, p: &Position) -> Score {
        self.w_eval_some(p.board())
    }

    fn piece_material_eval(&self, p: Piece, b: &Board) -> f64 {
        self.soft_coded_f64()
            .weight(Feature::material(p))
            .interpolate(b.phase(&self.phaser))
    }

    fn eval_move_see(&self, mv: Move, b: &Board) -> Score {
        Score::from_cp(self.see.eval_move_see(b, mv))
    }

    fn move_material_eval(&self, mv: Move, b: &Board) -> Score {
        // FIXME! far too slow (-7 ELO)
        let mut wt = WeightOf::zero();
        if let Some(cap) = mv.capture_piece(b) {
            wt += self.soft_coded_f64().weight(Feature::material(cap));
        }
        if let Some(promo) = mv.promo_piece() {
            wt += self.soft_coded_f64().weight(Feature::material(promo))
                - self.soft_coded_f64().weight(Feature::material(Piece::Pawn));
        }
        let score = wt.interpolate(b.phase(&self.phaser));
        Score::from_f64(score)
    }

    fn static_eval_explain(&self, pos: &Position) -> EvalExplain {
        let mut vec = vec![];
        // White, Black and then Total
        for color in [Some(Color::White), Some(Color::Black), None] {
            let mut explain = ExplainVector {
                color,
                board: pos.board().clone(),
                phase: pos.board().phase(&self.phaser),
                ..Default::default()
            };
            Evaluation.eval(pos.board(), &mut explain);
            vec.push(explain);
        }
        ExplainVector::to_eval_explain(&vec, &self.feature_weights())
    }

    fn new_game(&mut self) {
        self.eval_cache = UnsharedTable::with_size(self.cache_size);
        // self.mb.new_game();
        // self.pawn_cache.clear();
        self.eval_cache.clear();
    }

    fn eval_draw(&self, _b: &Board, _ply: Ply) -> Score {
        Score::from_cp(0)
    }
}

impl Component for Hce {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => {
                self.eval_cache = UnsharedTable::with_size(self.cache_size);
                // self.mb.new_game();
                self.phaser.new_game();
                // self.pawn_cache.clear();
                self.eval_cache.clear();
            }
            SetPosition => {
                // self.mb.new_position();
                self.phaser.new_position();
            }
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => {}
            Shutdown => {}
        }
    }
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl fmt::Display for Hce {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.hce_file.file_name().unwrap_or_default().to_string_lossy())?;
        Ok(())
    }
}

impl fmt::Debug for Hce {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "cache size       : {}", self.cache_size)?;
        writeln!(f, "eval_cache       : {}", self.eval_cache)?;
        writeln!(f, "draw scaling     : {}", self.draw_scaling)?;
        writeln!(f, "rounding         : {}", self.rounding)?;
        writeln!(f, "weights kind     : {}", self.weights_kind)?;
        writeln!(f, "utilization (‰)  : {}", self.eval_cache.hashfull_per_mille())?;
        // writeln!(f, "[material balance]\n{}", self.mb)?;
        writeln!(f, "[phaser]\n{}", self.phaser)?;
        writeln!(f, "phasing          : {}", self.phasing)?;
        writeln!(f, "mob.phase.disable: {}", self.mobility_phase_disable)?;
        writeln!(
            f,
            "weights raw: {} {} {}",
            self.weights_raw.wts[0], self.weights_raw.wts[1], self.weights_raw.wts[2],
        )?;
        Ok(())
    }
}

impl Hce {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reload_weights(&mut self) -> Result<()> {
        self.weights_raw = Softcoded::load(&self.hce_file)
            .context(format!("unable to load weights from {}", self.hce_file.display()))?;
        Ok(())
    }

    // pub fn configure(settings: HashMap<String, String>) -> anyhow::Result<Self> {
    //     Config::new()
    //         .resource("eval.hce.toml")
    //         .props(settings)
    //         .env_var_props("ODONATA")
    //         .allow_override_files()
    //         .deserialize_node("eval")
    // }

    pub fn soft_coded_i32(&self) -> &Softcoded<i32> {
        let soft = self.weights_i32.get_or_init(|| {
            let mut w = Softcoded::default();
            w.wts = self.weights_raw.wts.map(|w| WeightOf::cast_from(w, self.rounding));
            w
        });
        soft
    }

    pub fn soft_coded_f64(&self) -> &Softcoded<f64> {
        let soft = self.weights_f64.get_or_init(|| {
            let mut w = Softcoded::default();
            w.wts = self.weights_raw.wts.map(|w| WeightOf::cast_from(w, self.rounding));
            w
        });
        soft
    }

    pub fn soft_coded_f32(&self) -> &Softcoded<f32> {
        let soft = self.weights_f32.get_or_init(|| {
            let mut w = Softcoded::default();
            w.wts = self.weights_raw.wts.map(|w| WeightOf::cast_from(w, self.rounding));
            w
        });
        soft
    }

    pub fn feature_weights(&self) -> Vec<WeightOf<f32>> {
        self.soft_coded_f64()
            .wts
            .map(|wt| WeightOf::<f32>::cast_from(wt, Rounding::None))
            .to_vec()
    }

    pub fn write_weights<W: std::io::Write>(&self, w: W) -> anyhow::Result<()> {
        (self.soft_coded_f64() as &dyn WeightVec<f64>).write(w)?;
        Ok(())
    }

    // pub fn w_eval_explain(&self, b: &Board) -> impl fmt::Display {
    //     let ph = b.phase(&self.phaser);

    //     match self.weights_kind {
    //         WeightsKind::SoftcodedF64 | WeightsKind::SoftcodedF32 => {
    //             let soft_coded = self.soft_coded_f64().wts; // outside of closure - to avoid repeated once_cell checks
    //             let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| soft_coded[f]);
    //             Evaluation.eval(b, &mut scorer);
    //             format!("{scorer}")
    //         }
    //         WeightsKind::SoftcodedI32 => {
    //             let soft_coded = self.soft_coded_i32().wts; // outside of closure - to avoid repeated once_cell checks
    //             let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| soft_coded[f]);
    //             Evaluation.eval(b, &mut scorer);
    //             format!("{scorer}")
    //         }
    //         WeightsKind::HardcodedF64 => {
    //             let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| Hardcoded::<f64>::WTS[f]);
    //             Evaluation.eval(b, &mut scorer);
    //             format!("{scorer}")
    //         }
    //         WeightsKind::HardcodedI32 => {
    //             let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| Hardcoded::<i32>::WTS[f]);
    //             Evaluation.eval(b, &mut scorer);
    //             format!("{scorer}")
    //         }
    //         WeightsKind::HardcodedI32Millis => {
    //             let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| Hardcoded::<i32>::WTS[f]);
    //             Evaluation.eval(b, &mut scorer);
    //             format!("{scorer}")
    //         }
    //     }
    // }

    // pub fn init(&mut self) {
    //     self.weights_f32 = || -> Softcoded<f32> {
    //         let mut w = Softcoded::default();
    //         w.wts = self
    //             .weights_raw
    //             .wts
    //             .map(|w| WeightOf::cast_from(w, self.rounding));
    //         w
    //     }()
    // }

    // let soft = self.weights_f32.get_or_init(|| {
    //     let mut w = Softcoded::default();
    //     w.wts = self
    //         .weights_raw
    //         .wts
    //         .map(|w| WeightOf::cast_from(w, self.rounding));
    //     w

    pub fn w_eval_no_cache(&self, b: &Board) -> Score {
        let cp = match self.weights_kind {
            WeightsKind::SoftcodedF64 => {
                let softcoded: &[WeightOf<f64>; Feature::len()] = &self.soft_coded_f64().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = SummationScorer::new(
                    #[inline(always)]
                    |f| softcoded[f],
                );
                Evaluation.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph) as i32
            }
            WeightsKind::SoftcodedF32 => {
                let softcoded: &[WeightOf<f32>; Feature::len()] = &self.soft_coded_f32().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = SummationScorer::new(|f| softcoded[f]);
                Evaluation.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph) as i32
            }
            WeightsKind::SoftcodedI32 => {
                let soft_coded = &self.soft_coded_i32().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = SummationScorer::new(|f| soft_coded[f]);
                Evaluation.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph)
            } /* WeightsKind::HardcodedF64 => {
               *     let mut scorer = SummationScorer::new(|f| Hardcoded::<f64>::WTS[f]);
               *     Evaluation.eval(b, &mut scorer);
               *     let ph = b.phase(&self.phaser);
               *     scorer.total().interpolate(ph) as i32
               * }
               * WeightsKind::HardcodedI32 => {
               *     let mut scorer = SummationScorer::new(|f| Hardcoded::<i32>::WTS[f]);
               *     Evaluation.eval(b, &mut scorer);
               *     let ph = b.phase(&self.phaser);
               *     scorer.total().interpolate(ph)
               * }
               * WeightsKind::HardcodedI32Millis => {
               *     let mut scorer = SummationScorer::new(|f| Hardcoded::<i32>::WTS_MILLIS[f]);
               *     Evaluation.eval(b, &mut scorer);
               *     let ph = b.phase(&self.phaser);
               *     scorer.total().interpolate(ph) / 10
               * } */
        };

        Score::from_white_cp(cp / self.quantum * self.quantum, b.turn())
    }

    fn w_eval_some(&self, b: &Board) -> Score {
        if self.cache_size == 0 {
            return self.w_eval_no_cache(b);
        }

        if let Some(score) = self.eval_cache.probe(b.hash()) {
            Metrics::incr(Counter::EvalCacheHit);
            Metrics::incr_node(
                &Node {
                    ply: b.ply(),
                    ..Node::default()
                },
                Event::EvalCacheHit,
            );
            score
        } else {
            Metrics::incr(Counter::EvalCacheMiss);
            Metrics::incr_node(
                &Node {
                    ply: b.ply(),
                    ..Node::default()
                },
                Event::EvalCacheMiss,
            );
            let s = self.w_eval_no_cache(b);
            self.eval_cache.store(b.hash(), s);
            s
        }
    }

    // /// the value of the capture or promotion (or both for promo capture)
    // #[inline]
    // pub fn eval_move_material_board(&self, mv: Move, b: &Board) -> Weight {
    //     self.mb.eval_move_material_mb(mv, b)
    // }
}

#[cfg(test)]

mod tests {
    use std::hint::black_box;

    use odonata_base::catalog::Catalog;
    use odonata_base::infra::profiler::*;
    use test_log::test;

    use super::*;

    // #[test]
    // fn eval_serde_test() -> Result<()> {
    //     let eval = Hce::default();
    //     info!("\n{}", toml::to_string_pretty(&eval)?);
    //     // info!("{:#?}", v);
    //     // info!("\n{}", toml::to_string_pretty(&SimpleScorer::default()).unwrap());
    //     Ok(())
    // }

    // #[test]
    // fn test_draw_scaling() {
    //     fn score_for(s: &str, draw_scaling: f32) -> ExplainScore {
    //         let eng = ThreadedSearch::new();
    //         let phr = Phaser::default();
    //         let pos = Position::parse_epd(s).unwrap();
    //         let mut sc = ExplainScore::new(
    //             pos.board().phase(&phr),
    //             draw_scaling,
    //             format!("{:#}", pos.board().to_string()),
    //         );
    //         sc.set_weights(eng.algo.eval.weights_vector());
    //         Calc::new(&pos.board()).score(&mut sc, pos.board());
    //         sc
    //     }
    //     let drawish = r"
    //         ........
    //         ........
    //         ........
    //         R.K.....
    //         ........
    //         ........
    //         ........
    //         .....n.k w KQkq - 1 1";

    //     let winnish = r"
    //         ........
    //         ........
    //         R.......
    //         R.K.....
    //         ........
    //         ........
    //         ........
    //         .....n.k w KQkq - 1 1";

    //     let unscaled = score_for(drawish, 1.).total();
    //     let half = score_for(drawish, 0.25).total();
    //     assert_eq!(0.25 * unscaled, half);
    //     info!("{unscaled} {half}");

    //     let unscaled = score_for(winnish, 1.).total();
    //     let half = score_for(winnish, 0.5).total();
    //     assert_eq!(1. * unscaled, half);
    //     info!("{unscaled} {half}");
    // }

    #[test]
    fn test_eval_bug1() {
        let epd = Catalog::bratko_kopec()[0].clone();
        let pos = Position::from_epd(epd);
        let eval = Hce::default();
        // eval.mb.enabled = false;
        let explain = eval.static_eval_explain(&pos);
        println!("{}", explain);
    }

    // #[test]
    // fn test_pawn_cache() {
    //     let mut eng = ThreadedSearch::new();
    //     let pos = Catalog::starting_position();
    //     let tc = TimeControl::Depth(11);
    //     eng.search(pos, tc).unwrap();
    //     println!(
    //         "hit_rate = {}%, cache_full = {}%% hits = {} misses = {} collisions = {}",
    //         eng.eval.pawn_cache.cache_hits_percent().dp(3),
    //         eng.eval.pawn_cache.hashfull_per_mille(),
    //         eng.eval.pawn_cache.hits.get(),
    //         eng.eval.pawn_cache.misses.get(),
    //         eng.eval.pawn_cache.collisions.get(),
    //     );

    //     let pos = Catalog::starting_position();
    //     let tc = TimeControl::Depth(11);
    //     eng.search(pos, tc).unwrap();
    //     println!(
    //         "hit_rate = {}%, cache_full = {}%% hits = {} misses = {} collisions = {}",
    //         eng.eval.pawn_cache.cache_hits_percent().dp(3),
    //         eng.eval.pawn_cache.hashfull_per_mille(),
    //         eng.eval.pawn_cache.hits.get(),
    //         eng.eval.pawn_cache.misses.get(),
    //         eng.eval.pawn_cache.collisions.get(),
    //     );

    //     let pos = Catalog::starting_position();
    //     let tc = TimeControl::Depth(11);
    //     eng.search(pos, tc).unwrap();
    //     println!(
    //         "hit_rate = {}%, cache_full = {}%% hits = {} misses = {} collisions = {}",
    //         eng.eval.pawn_cache.cache_hits_percent().dp(3),
    //         eng.eval.pawn_cache.hashfull_per_mille(),
    //         eng.eval.pawn_cache.hits.get(),
    //         eng.eval.pawn_cache.misses.get(),
    //         eng.eval.pawn_cache.collisions.get(),
    //     );
    // }

    #[test]
    fn test_write_weights() {
        let eval = Hce::new();
        eval.write_weights(std::io::stdout()).unwrap()
    }

    #[test]
    fn bench_eval() {
        let eval = Hce::default();
        // eval.mb.enabled = false;
        let mut prof = PerfProfiler::new("bench_eval");
        let _node = Node::root(0);
        let mut total_w_score = 0;
        for epd in Catalog::win_at_chess() {
            let pos = Position::from_epd(epd);
            prof.start();
            let score = eval.static_eval(&pos);
            prof.stop();
            println!("{:>6.0} {}", score.as_white(pos.board().turn()).0, pos);
            total_w_score += score.as_white(pos.board().turn()).0.as_i16() as i32;
        }
        // prof.set_iters(Catalog::win_at_chess().len() as u64);
        println!("{:>6.0} {:<}", total_w_score, "total");
    }

    #[test]
    fn print_weights() {
        let eval = Hce::new();
        for &f in Feature::all()[0..10].iter() {
            // let hardcoded_i32 = |f: Feature| Hardcoded::<i32>::WTS[f.index()];
            let softcoded_i32 = |f: Feature| &eval.soft_coded_i32().wts[f.index()];
            println!("feature = {fn:<30} sc i32 = {sc:<10} ", 
                fn = f.name(), sc = softcoded_i32(f).to_string(), );
        }
    }

    #[test]
    fn test_eval_explain_hce() {
        let eval = Hce::new();
        let pos = Position::from_epd(Catalog::test_position());
        let explain = eval.static_eval_explain(&pos);
        println!("{explain}");
        println!("{explain:#}");
    }

    #[ignore]
    #[test]
    fn profile_eval() {
        let positions = Catalog::win_at_chess();
        for _ in 0..150000 {
            let eval = Hce::default();
            // eval.mb.enabled = false;
            let _node = Node::root(0);
            let mut total_w_score = 0;
            for epd in &positions {
                let pos = Position::from_epd(epd.clone());
                let score = eval.static_eval(&pos);
                total_w_score += score.as_white(pos.board().turn()).0.as_i16();
                // println!("{:>6.0} {}", score.as_i16(), pos);
            }
            black_box(total_w_score);
        }
    }
}
