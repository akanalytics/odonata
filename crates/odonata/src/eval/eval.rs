use crate::{
    bits::square::Square,
    boards::Board,
    cache::lockless_hashmap::UnsharedTable,
    eval::{
        material_balance::MaterialBalance,
        pst::Pst,
        score::Score,
        see::See,
        weight::{Rounding, Weight},
        ExplainingScorer,
    },
    infra::{
        component::{Component, State},
        metric::Metrics,
    },
    mv::Move,
    other::Phaser,
    piece::{Color, Piece},
    search::node::{Counter, Event, Node},
};

use itertools::Itertools;
use once_cell::unsync::OnceCell;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use std::{collections::HashMap, fmt};

use super::{
    calc::Calc, scorer::TotalScore, scoring::Hardcoded, weight::WeightOf, Feature, Scorer,
    Softcoded, SummationScorer,
};

use super::{score::WhiteScore, scorer::ExplainScore};

// https://www.chessprogramming.org/Simplified_Evaluation_Function

use strum_macros::{Display, EnumCount, EnumDiscriminants, EnumIter, IntoStaticStr};

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum WeightsKind {
    HardcodedF64,
    HardcodedI32,
    HardcodedI32Millis,
    SoftcodedF64,
    SoftcodedF32,
    SoftcodedI32,
    Legacy,
}

#[derive(
    Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Debug, IntoStaticStr, EnumCount, EnumIter, Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum Attr {
    PawnDoubled,
    PawnDirectlyDoubled,
    PawnWeak,
    PawnIsolated,
    PawnIsolatedHalfOpen,
    SemiIsolated,
    PawnPassed,
    PawnPassedR7,
    PawnPassedR6,
    PawnPassedR5,
    PawnPassedR4,
    PassersOnRim,
    CandidatePassedPawn,
    BlockadedOpponent,
    BlockadedSelf,
    BlockadedAny,
    BlockadedPassers,
    RooksBehindPasser,
    PawnIsolatedDoubled,
    PawnDoubleAttacks,
    Space,
    RammedPawns,
    PawnDistantNeighboursR7,
    PawnDistantNeighboursR6,
    PawnDistantNeighboursR5,
    PawnConnectedR67,
    PawnConnectedR345,
    PassedConnectedR67,
    PassedConnectedR345,
    PawnDuoR67,
    PawnDuoR2345,
    PassedDuoR67,
    PassedDuoR2345,
    BackwardHalfOpen,
    Backward,
    PotentialOutpost,

    BishopPair,
    RookPair,
    CertainWinBonus,
    WinBonus,
    Closedness,

    CenterAttacks,
    DoubleAttacks,
    Controlled,
    Defended,
    Attacked,

    UndefendedSq,
    UndefendedPiece,
    DefendsOwn,
    TrappedPiece,
    PartiallyTrappedPiece,
    RookOpenFile,
    RookSemiOpenFile,
    KnightMoves1,
    KnightMoves2,
    KnightMoves3,
    KnightMoves4,
    KnightMoves5,
    KnightMoves6,
    BishopMoves1,
    BishopMoves2,
    BishopMoves3,
    BishopMoves4,
    BishopMoves5,
    BishopMoves6,
    RookMoves1,
    RookMoves2,
    RookMoves3,
    RookMoves4,
    RookMoves5,
    RookMoves6,
    QueenMoves1,
    QueenMoves2,
    QueenMoves3,
    QueenMoves4,
    QueenMoves5,
    QueenMoves6,

    KnightClosedness,
    KnightOutpost,
    KnightOutpostPawnDefended,
    KnightOutpostRookSafe,
    KnightConnected,
    KnightAttacksCenter,
    KnightTrapped,

    Fianchetto,
    DiscoveredAttsByBishop,
    RelativePinsByBishop,
    BishopOutposts,
    BishopConnected,
    BishopColorPawns,
    BishopPawnTrap,
    BishopColorRammedPawns,
    BishopFarPawns,
    BishopTrapped,
    BishopClosedness,

    RelativePinsByRook,
    RookClosedness,
    DoubledRooks,
    ConnectedRooks,
    DoubledRooksOpenFile,
    EnemyPawnsOnRookRank,
    RookTrapped,

    QueenEarlyDevelop,
    QueenOpenFile,
    QueenTrapped,

    PawnAdjacentShield,
    PawnNearbyShield,
    PawnShieldFaulty,
    KingSafetyBonus,
    StormBlocked,
    StormBlockedR3,
    StormBlockedR4,
    StormUnblocked,
    StormUnblockedR23,

    OpenFilesNearKing,
    OpenFilesAdjacentKing,
    AttacksNearKing,
    DoubleAttacksNearKing,
    MovesNearKing,
    TropismD1,
    TropismD2,
    TropismD3,
    TropismD4,
    KingTrappedOnBackRank,
    RqOnOpenFilesNearKing,
    CastlingRightsBonus,
    Uncastled,
    Checkers,
    PiecesNearKing,
    PinnedNearKing,
    PinnedFar,
    DiscoveredChecks,
    TempoBonus,
    TempoKnightForks,
    TempoSafety,
    TempoUndefendedPiece,

    ContemptPenalty,
    WinMetric1,
    WinMetric2,
}

impl Default for Attr {
    fn default() -> Self {
        Attr::PawnDoubled
    }
}

impl Attr {
    pub const fn as_feature(&self) -> FeatureOld {
        FeatureOld::Discrete(*self)
    }
}

#[derive(
    Clone,
    Copy,
    Eq,
    Hash,
    PartialEq,
    Debug,
    EnumDiscriminants,
    IntoStaticStr,
    EnumCount,
    EnumIter,
    Display,
)]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(vis())]
pub enum FeatureOld {
    Discrete(Attr),
    Pst(Piece, Square),
    Piece(Piece),
}

impl From<Attr> for FeatureOld {
    fn from(a: Attr) -> Self {
        FeatureOld::Discrete(a)
    }
}

impl FeatureOld {
    pub const fn index(&self) -> usize {
        match self {
            FeatureOld::Discrete(f) => *f as usize,
            FeatureOld::Pst(p, sq) => {
                Attr::WinMetric2 as usize + 1 + (p.index()) * Square::len() + sq.index()
            }
            FeatureOld::Piece(p) => {
                Attr::WinMetric2 as usize + 1 + Square::len() * (Piece::len()) + (p.index())
            }
        }
    }

    pub const fn len() -> usize {
        FeatureOld::Piece(Piece::Queen).index() + 1
    }
    pub fn all() -> Vec<FeatureOld> {
        let mut features = Vec::<FeatureOld>::new();
        for f in FeatureOld::iter() {
            match f {
                FeatureOld::Discrete(_) => {
                    for a in Attr::iter() {
                        features.push(FeatureOld::Discrete(a));
                    }
                }
                FeatureOld::Pst(..) => {
                    for p in Piece::ALL {
                        for sq in Square::all() {
                            features.push(FeatureOld::Pst(p, sq));
                        }
                    }
                }
                FeatureOld::Piece(_) => {
                    for p in Piece::ALL_BAR_KING {
                        features.push(FeatureOld::Piece(p));
                    }
                }
            }
        }
        features
    }

    pub fn name(&self) -> String {
        use crate::eval::eval::FeatureOld::*;
        match self {
            Pst(p, sq) => format!("{}.{}", p.to_lower_char(), sq.uci()),
            Piece(p) => format!("{}", p.to_lower_char()),
            Discrete(f) => f.to_string(),
        }
    }
    pub fn category(&self) -> String {
        match self {
            FeatureOld::Discrete(x) if x <= &Attr::Backward => "Pawn".to_string(),
            FeatureOld::Discrete(x) if x <= &Attr::WinBonus => "Material".to_string(),
            FeatureOld::Discrete(x) if x <= &Attr::QueenOpenFile => "Mobility".to_string(),
            FeatureOld::Discrete(x) if x <= &Attr::QueenEarlyDevelop => "Position".to_string(),
            FeatureOld::Discrete(x) if x <= &Attr::DiscoveredChecks => "Safety".to_string(),
            FeatureOld::Piece(_) => "Material".to_string(),
            FeatureOld::Pst(..) => "Position".to_string(),
            _ => "Tempo".to_string(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct WeightsVector {
    pub weights: Vec<Weight>,
    pub names:   Vec<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eval {
    pub phasing:                bool,
    pub weights_kind:           WeightsKind,
    rounding:                   Rounding,
    pub mobility_phase_disable: u8,
    pub quantum:                i32,
    cache_size:                 usize,
    pub draw_scaling:           f32,
    pub draw_scaling_noisy:     f32,

    pub pst:          Pst,
    pub phaser:       Phaser,
    pub see:          See,
    pub mb:           MaterialBalance,
    pub discrete:     HashMap<String, Weight>,
    pub weights_raw:  Softcoded<f64>,
    pub weights_raw2: Softcoded<f64>,
    eval_cache:       UnsharedTable<WhiteScore>,

    #[serde(skip)]
    pub feature_weights: Vec<Weight>,

    #[serde(skip)]
    pub weights_i32: OnceCell<Softcoded<i32>>,

    #[serde(skip)]
    pub weights_f64: OnceCell<Softcoded<f64>>,

    #[serde(skip)]
    pub weights_f32: OnceCell<Softcoded<f32>>,
}

impl Default for Eval {
    fn default() -> Self {
        const DEFAULT_CACHE_SIZE: usize = 10_000;
        let mut s = Self {
            mb:                     MaterialBalance::default(),
            weights_kind:           WeightsKind::HardcodedF64,
            rounding:               Rounding::None,
            pst:                    Pst::default(),
            weights_raw:            Default::default(),
            weights_raw2:           Default::default(),
            weights_i32:            Default::default(),
            weights_f32:            Default::default(),
            weights_f64:            Default::default(),
            feature_weights:        Vec::new(),
            draw_scaling:           1.,
            draw_scaling_noisy:     1.,
            discrete:               HashMap::new(),
            phaser:                 Phaser::default(),
            see:                    See::default(),
            phasing:                true,
            mobility_phase_disable: 101,
            quantum:                1,
            cache_size:             DEFAULT_CACHE_SIZE,
            eval_cache:             UnsharedTable::with_size(DEFAULT_CACHE_SIZE),
        };
        for f in FeatureOld::all() {
            s.discrete.insert(f.name(), Weight::zero());
        }
        s.populate_feature_weights();
        s
    }
}

impl Component for Eval {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => {
                self.eval_cache = UnsharedTable::with_size(self.cache_size);
                self.populate_feature_weights();
                self.mb.new_game();
                self.phaser.new_game();
                self.see.new_game();
                // self.pawn_cache.clear();
                self.eval_cache.clear();
            }
            SetPosition => {
                self.mb.new_position();
                self.phaser.new_position();
                self.see.new_position();
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

impl fmt::Display for Eval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "cache size       : {}", self.cache_size)?;
        writeln!(f, "eval_cache       : {}", self.eval_cache)?;
        writeln!(f, "draw scaling     : {}", self.draw_scaling)?;
        writeln!(f, "rounding         : {}", self.rounding)?;
        writeln!(f, "weights kind     : {}", self.weights_kind)?;
        writeln!(
            f,
            "utilization (‰)  : {}",
            self.eval_cache.hashfull_per_mille()
        )?;
        writeln!(f, "[material balance]\n{}", self.mb)?;
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

// builder methods
impl Eval {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn populate_feature_weights(&mut self) {
        info!("Populating feature weights");
        self.feature_weights
            .resize(FeatureOld::len(), Weight::zero());
        for f in &FeatureOld::all() {
            self.feature_weights[f.index()] = match f {
                FeatureOld::Discrete(_i) => *self.discrete.get(&f.name()).unwrap_or_else(|| {
                    panic!(
                        "Missing discrete eval param {} in {:?}",
                        f.name(),
                        self.discrete
                    )
                }),
                FeatureOld::Pst(p, sq) => self.pst.pst(*p, *sq),
                FeatureOld::Piece(p) => self.mb.piece_weights[*p],
            }
        }
    }
}

impl Eval {
    pub fn weights_vector(&self) -> WeightsVector {
        WeightsVector {
            names:   FeatureOld::all().iter().map(|f| f.name()).collect_vec(),
            weights: self.feature_weights.clone(),
        }
    }

    pub(crate) fn weight(&self, f: &FeatureOld) -> Weight {
        self.feature_weights[f.index()]
    }

    pub fn set_weight(&mut self, f: FeatureOld, wt: Weight) {
        self.feature_weights[f.index()] = wt;
    }

    pub fn w_tempo_adjustment(&self, us: Color) -> Weight {
        // axiom: we're white
        // white to move => advantage, black to move means white has a disadvantage
        let tempo_bonus = self.weight(&Attr::TempoBonus.into());

        if us == Color::White {
            tempo_bonus
        } else {
            -tempo_bonus
        }
    }

    pub fn w_eval_draw(&self, board: &Board, node: &Node) -> WhiteScore {
        // draw score is +ve for playing a stronger opponent (we want a draw), neg for weaker
        //
        //  Engine Col   |  search ply   |  value to searcher   | Score to white
        //     W               0                   +ve               +ve
        //     B               0                   +ve               -ve
        //     W               1 (oppo B)          -ve               +ve (a bonus to white opponet)
        //     B               1 (oppo W)          -ve               -ve
        // board.color_us() == Color::White => maximising
        // +ve contempt => +ve score => aim for draw => opponent stronger than us
        // board.color_us() == Color::Black => minimising
        // +ve contempt => -ve score => aim for draw => opponent stronger than us
        let contempt_weight = self.weight(&Attr::ContemptPenalty.into());
        let mut contempt_pov =
            Score::from_f32(contempt_weight.interpolate(board.phase(&self.phaser)));
        if (node.ply % 2) == 1 {
            contempt_pov = -contempt_pov;
        }
        board.white_score(contempt_pov)
    }

    fn w_evaluate_with_outcome(&self, board: &Board, n: &Node) -> WhiteScore {
        let outcome = board.outcome();
        if outcome.is_game_over() {
            if outcome.is_draw() {
                return self.w_eval_draw(board, n);
            }
            if let Some(c) = outcome.winning_color() {
                if board.color_us() == c {
                    return board.white_score(Score::we_win_in(n.ply));
                } else {
                    return board.white_score(Score::we_lose_in(n.ply));
                }
            }
        }
        self.w_eval_some(board)
    }

    fn soft_coded_i32(&self) -> &Softcoded<i32> {
        let soft = self.weights_i32.get_or_init(|| {
            let mut w = Softcoded::default();
            w.wts = self
                .weights_raw
                .wts
                .map(|w| WeightOf::cast_from(w, self.rounding));
            w
        });
        soft
    }

    fn soft_coded_f64(&self) -> &Softcoded<f64> {
        let soft = self.weights_f64.get_or_init(|| {
            let mut w = Softcoded::default();
            w.wts = self
                .weights_raw
                .wts
                .map(|w| WeightOf::cast_from(w, self.rounding));
            w
        });
        soft
    }

    fn soft_coded_f32(&self) -> &Softcoded<f32> {
        let soft = self.weights_f32.get_or_init(|| {
            let mut w = Softcoded::default();
            w.wts = self
                .weights_raw
                .wts
                .map(|w| WeightOf::cast_from(w, self.rounding));
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
        use crate::eval::scoring::WeightVec;
        (self.soft_coded_f64() as &dyn WeightVec<f64>).write(w)?;
        Ok(())
    }

    pub fn w_eval_explain(&self, b: &Board) -> impl fmt::Display {
        let ph = b.phase(&self.phaser);

        match self.weights_kind {
            WeightsKind::SoftcodedF64 | WeightsKind::SoftcodedF32 => {
                let soft_coded = self.soft_coded_f64().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| soft_coded[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                format!("{scorer}")
            }
            WeightsKind::SoftcodedI32 => {
                let soft_coded = self.soft_coded_i32().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| soft_coded[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                format!("{scorer}")
            }
            WeightsKind::HardcodedF64 => {
                let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| Hardcoded::<f64>::WTS[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                format!("{scorer}")
            }
            WeightsKind::HardcodedI32 => {
                let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| Hardcoded::<i32>::WTS[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                format!("{scorer}")
            }
            WeightsKind::HardcodedI32Millis => {
                let mut scorer = ExplainingScorer::new(b.clone(), ph, |f| Hardcoded::<i32>::WTS[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                format!("{scorer}")
            }
            WeightsKind::Legacy => {
                let mut scorer = ExplainScore::new(ph, self.draw_scaling, b.to_fen());
                scorer.set_weights(self.weights_vector());
                Calc::new(b).score(&mut scorer, b);
                format!("{scorer:#}")
            }
        }
    }

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

    pub fn w_eval_no_cache(&self, b: &Board) -> WhiteScore {
        let cp = match self.weights_kind {
            WeightsKind::SoftcodedF64 => {
                let softcoded: &[WeightOf<f64>; Feature::len()] = &self.soft_coded_f64().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = SummationScorer::new(
                    #[inline(always)]
                    |f| softcoded[f],
                );
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph) as i32
            }
            WeightsKind::SoftcodedF32 => {
                let softcoded: &[WeightOf<f32>; Feature::len()] = &self.soft_coded_f32().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = SummationScorer::new(|f| softcoded[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph) as i32
            }
            WeightsKind::SoftcodedI32 => {
                let soft_coded = &self.soft_coded_i32().wts; // outside of closure - to avoid repeated once_cell checks
                let mut scorer = SummationScorer::new(|f| soft_coded[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph)
            }
            WeightsKind::HardcodedF64 => {
                let mut scorer = SummationScorer::new(|f| Hardcoded::<f64>::WTS[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph) as i32
            }
            WeightsKind::HardcodedI32 => {
                let mut scorer = SummationScorer::new(|f| Hardcoded::<i32>::WTS[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph)
            }
            WeightsKind::HardcodedI32Millis => {
                let mut scorer = SummationScorer::new(|f| Hardcoded::<i32>::WTS_MILLIS[f]);
                crate::eval::evaluation::Eval.eval(b, &mut scorer);
                let ph = b.phase(&self.phaser);
                scorer.total().interpolate(ph) / 10
            }
            WeightsKind::Legacy => {
                let ph = b.phase(&self.phaser);
                let mut scorer = TotalScore::new(&self.feature_weights, self.draw_scaling, ph);
                Calc::new(b).score(&mut scorer, b);
                return WhiteScore(Score::from_cp(
                    scorer.total().interpolate(ph) as i32 / self.quantum * self.quantum,
                ));
            }
        };

        WhiteScore(Score::from_cp(cp / self.quantum * self.quantum))
    }

    pub fn w_old_eval_no_cache(&self, b: &Board) -> WhiteScore {
        let ph = b.phase(&self.phaser);
        let mut scorer = TotalScore::new(&self.feature_weights, self.draw_scaling, ph);
        Calc::new(&b).score(&mut scorer, b);
        WhiteScore(Score::from_cp(
            scorer.total().interpolate(ph) as i32 / self.quantum * self.quantum,
        ))
    }

    fn w_eval_some(&self, b: &Board) -> WhiteScore {
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

    /// the value of the capture or promotion (or both for promo capture)
    #[inline]
    pub fn eval_move_material(&self, mv: Move, b: &Board) -> Weight {
        self.mb.eval_move_material(mv, b)
    }
}

impl Board {
    #[inline]
    pub fn pov_score(&self, ws: WhiteScore) -> Score {
        // avoid multiply on a possible mate score
        self.color_us().chooser_wb(ws.0, -ws.0)
    }

    #[inline]
    pub fn white_score(&self, pov_score: Score) -> WhiteScore {
        // avoid multiply on a possible mate score
        WhiteScore(self.color_us().chooser_wb(pov_score, -pov_score))
    }

    #[inline]
    pub fn eval_draw(&self, eval: &mut Eval, nd: &Node) -> Score {
        self.pov_score(eval.w_eval_draw(self, nd))
    }

    #[inline]
    pub fn eval_move_see(&self, eval: &Eval, mv: Move) -> Score {
        Score::from_cp(eval.see.eval_move_see(self, mv))
    }

    #[inline]
    pub fn eval_move_material(&self, eval: &Eval, mv: Move) -> Score {
        // FIXME! far too slow (-7 ELO)
        Score::from_cp(
            eval.eval_move_material(mv, self)
                .interpolate(self.phase(&eval.phaser)) as i32,
        )
    }

    #[inline]
    pub fn eval_with_outcome(&self, eval: &Eval, nd: &Node) -> Score {
        self.pov_score(eval.w_evaluate_with_outcome(self, nd))
    }

    #[inline]
    pub fn static_eval(&self, eval: &Eval) -> Score {
        // profile_fn!(board.eval_some);
        // let _g = hprof::enter("eval some");
        self.pov_score(eval.w_eval_some(self))
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::{
        catalog::Catalog,
        infra::{black_box, profiler::*},
        search::engine::ThreadedSearch,
        test_log::test,
        Algo, Position,
    };
    use anyhow::Result;
    use toml;

    #[test]
    fn test_feature_index() {
        assert_eq!(FeatureOld::Discrete(Attr::PawnDoubled).index(), 0);
        assert_eq!(FeatureOld::Discrete(Attr::PawnDirectlyDoubled).index(), 1);
        let last = FeatureOld::Discrete(Attr::WinMetric2).index();
        let first_sq = FeatureOld::Pst(Piece::Pawn, Square::A1).index();
        let last_sq = FeatureOld::Pst(Piece::King, Square::H8).index();
        let first_piece = FeatureOld::Piece(Piece::Pawn).index();
        let last_piece = FeatureOld::Piece(Piece::King).index();
        assert_eq!(first_sq, last + 1);
        assert_eq!(last_sq, first_sq + 6 * 64 - 1);
        assert_eq!(first_piece, last_sq + 1);
        assert_eq!(last_piece, first_piece + 6 - 1);

        assert_eq!(
            FeatureOld::Discrete(Attr::CenterAttacks).name(),
            "center_attacks"
        );
        assert_eq!(FeatureOld::Pst(Piece::Pawn, Square::A1).name(), "p.a1");
        // assert!(Feature::all().len() > 64 * 6 + 6 + Feature::WinBonus.index());
    }

    #[test]
    fn eval_serde_test() -> Result<()> {
        let eval = Eval::default();
        info!("\n{}", toml::to_string_pretty(&eval)?);
        // info!("{:#?}", v);
        // info!("\n{}", toml::to_string_pretty(&SimpleScorer::default()).unwrap());
        Ok(())
    }

    #[test]
    fn test_draw_scaling() {
        fn score_for(s: &str, draw_scaling: f32) -> ExplainScore {
            let eng = ThreadedSearch::new();
            let phr = Phaser::default();
            let pos = Position::parse_epd(s).unwrap();
            let mut sc = ExplainScore::new(
                pos.board().phase(&phr),
                draw_scaling,
                format!("{:#}", pos.board().to_string()),
            );
            sc.set_weights(eng.algo.eval.weights_vector());
            Calc::new(&pos.board()).score(&mut sc, pos.board());
            sc
        }
        let drawish = r"
            ........
            ........
            ........
            R.K.....
            ........
            ........
            ........
            .....n.k w KQkq - 1 1";

        let winnish = r"
            ........
            ........
            R.......
            R.K.....
            ........
            ........
            ........
            .....n.k w KQkq - 1 1";

        let unscaled = score_for(drawish, 1.).total();
        let half = score_for(drawish, 0.25).total();
        assert_eq!(0.25 * unscaled, half);
        info!("{unscaled} {half}");

        let unscaled = score_for(winnish, 1.).total();
        let half = score_for(winnish, 0.5).total();
        assert_eq!(1. * unscaled, half);
        info!("{unscaled} {half}");
    }

    #[test]
    fn test_eval_bug1() {
        let pos = &Catalog::bratko_kopec()[0];
        let b = pos.board();
        let mut eval = Eval::default();
        eval.mb.enabled = false;
        let explain = eval.w_eval_explain(b);
        println!("{}", explain);
    }

    // #[test]
    // fn test_pawn_cache() {
    //     let mut eng = Algo::new();
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
        let algo = Algo::new();
        algo.eval.write_weights(std::io::stdout()).unwrap()
    }

    #[test]
    fn test_eval_indexing() {
        let mut eng = ThreadedSearch::new();
        eng.algo.eval.populate_feature_weights();
        for i in FeatureOld::all().iter() {
            let wt = eng.algo.eval.weight(i);
            println!("{} {:<20} = {}", i.index(), i.name(), wt);
        }
        for se in 0..=1 {
            for p in Piece::ALL {
                println!("Piece {p}.. s/e={se}");
                for r in 0..8 {
                    print!("r{r} ");
                    for c in 0..8 {
                        let sq = Square::from_xy(c, r);
                        let f = FeatureOld::Pst(p, sq);
                        let wt = eng.algo.eval.weight(&f);
                        let w = if se == 0 { wt.s() } else { wt.e() };
                        print!("{w:>5} ", w = (w as i32).to_string());
                    }
                    println!();
                }
            }
        }
    }

    #[test]
    fn bench_eval() {
        let mut eval = Eval::default();
        eval.mb.enabled = false;
        let mut prof = PerfProfiler::new("bench_eval".into());
        let node = Node::root(0);
        let mut total_score = 0;
        for pos in Catalog::win_at_chess() {
            let b = pos.board();
            prof.start();
            let score = b.white_score(b.eval_with_outcome(&eval, &node));
            prof.stop();
            println!("{:>6.0} {}", score.as_white_cp(), pos);
            total_score = total_score + score.as_white_cp();
        }
        println!("{:>6.0} {:<}", total_score, "total");
    }

    #[test]
    fn print_weights() {
        let eng = Algo::new();
        let eval = &eng.eval;
        for &f in Feature::all()[0..10].iter() {
            let hardcoded_i32 = |f: Feature| Hardcoded::<i32>::WTS[f.index()];
            let softcoded_i32 = |f: Feature| &eval.soft_coded_i32().wts[f.index()];
            println!("feature = {fn:<30} sc i32 = {sc:<10} hc i32 = {hc:<10}", 
                fn = f.name(), sc = softcoded_i32(f).to_string(), hc = hardcoded_i32(f).to_string());
        }
    }

    #[ignore]
    #[test]
    fn profile_eval() {
        let positions = Catalog::win_at_chess();
        for _ in 0..150000 {
            let mut eval = Eval::default();
            eval.mb.enabled = false;
            let node = Node::root(0);
            let mut total_score = 0;
            for pos in &positions {
                let b = pos.board();
                let score = b.white_score(b.eval_with_outcome(&eval, &node));
                total_score = total_score + score.as_white_cp();
                // println!("{:>6.0} {}", score.as_i16(), pos);
            }
            black_box(total_score);
        }
    }
}
