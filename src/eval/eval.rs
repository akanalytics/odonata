use crate::bits::square::Square;
use crate::board::Board;
use crate::cache::lockless_hashmap::VecCache;
use crate::eval::material_balance::MaterialBalance;
use crate::eval::pst::Pst;
use crate::eval::score::Score;
use crate::eval::see::See;
use crate::eval::weight::Weight;
use crate::infra::component::Component;
use crate::infra::component::State;
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::phaser::Phaser;
use crate::piece::{Color, Piece};
use crate::search::node::Counter;
use crate::search::node::Event;
use crate::search::node::Node;

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;

use super::calc::Calc;
use super::feature::WeightsVector;

use super::score::WhiteScore;
use super::scorer::ExplainScore;
use super::scorer::TotalScore;

// https://www.chessprogramming.org/Simplified_Evaluation_Function

use strum_macros::Display;
use strum_macros::EnumCount;
use strum_macros::EnumDiscriminants;
use strum_macros::EnumIter;
use strum_macros::IntoStaticStr;

#[derive(
    Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Debug, IntoStaticStr, EnumCount, EnumIter, Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum Attr {
    PawnDoubled,
    PawnDirectlyDoubled,
    PawnIsolated,
    SemiIsolated,
    PawnPassed,
    PawnPassedR7,
    PawnPassedR6,
    PawnPassedR5,
    PawnPassedR4,
    PassersOnRim,
    CandidatePassedPawn,
    Blockaded,
    BlockadedPassers,
    RooksBehindPasser,
    PawnIsolatedDoubled,
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

    BishopPair,
    RookPair,
    WinBonus,
    Closedness,

    CenterAttacks,
    DoubleAttacks,
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
    BishopOutposts,
    BishopColorPawns,
    BishopColorRammedPawns,
    BishopFarPawns,
    BishopTrapped,
    BishopClosedness,

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
    CastlingRights,
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
    pub const fn as_feature(&self) -> Feature {
        Feature::Discrete(*self)
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
pub enum Feature {
    Discrete(Attr),
    Pst(Piece, Square),
    Piece(Piece),
}

impl From<Attr> for Feature {
    fn from(a: Attr) -> Self {
        Feature::Discrete(a)
    }
}

impl Feature {
    pub const fn index(&self) -> usize {
        match self {
            Feature::Discrete(f) => *f as usize,
            Feature::Pst(p, sq) => {
                Attr::WinMetric2 as usize + 1 + (p.index()) * Square::len() + sq.index()
            }
            Feature::Piece(p) => {
                Attr::WinMetric2 as usize + 1 + Square::len() * (Piece::len()) + (p.index())
            }
        }
    }

    pub const fn len() -> usize {
        Feature::Piece(Piece::Queen).index() + 1
    }
    pub fn all() -> Vec<Feature> {
        let mut features = Vec::<Feature>::new();
        for f in Feature::iter() {
            match f {
                Feature::Discrete(_) => {
                    for a in Attr::iter() {
                        features.push(Feature::Discrete(a));
                    }
                }
                Feature::Pst(_, _) => {
                    for p in Piece::ALL {
                        for sq in Square::all() {
                            features.push(Feature::Pst(p, sq));
                        }
                    }
                }
                Feature::Piece(_) => {
                    for p in Piece::ALL_BAR_KING {
                        features.push(Feature::Piece(p));
                    }
                }
            }
        }
        features
    }

    pub fn name(&self) -> String {
        use crate::eval::eval::Feature::*;
        match self {
            Pst(p, sq) => format!("{}.{}.{}", self, p.to_lower_char(), sq.uci()),
            Piece(p) => format!("{}.{}", self, p.to_lower_char()),
            Discrete(f) => f.to_string(),
        }
    }
    pub fn category(&self) -> String {
        match self {
            Feature::Discrete(x) if x <= &Attr::Backward => "Pawn".to_string(),
            Feature::Discrete(x) if x <= &Attr::WinBonus => "Material".to_string(),
            Feature::Discrete(x) if x <= &Attr::QueenOpenFile => "Mobility".to_string(),
            Feature::Discrete(x) if x <= &Attr::QueenEarlyDevelop => "Position".to_string(),
            Feature::Discrete(x) if x <= &Attr::DiscoveredChecks => "Safety".to_string(),
            Feature::Piece(_) => "Material".to_string(),
            Feature::Pst(_, _) => "Position".to_string(),
            _ => "Tempo".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Eval {
    pub phasing: bool,
    pub mobility_phase_disable: u8,
    pub quantum: i32,
    cache_size: usize,

    pub pst: Pst,
    pub phaser: Phaser,
    pub see: See,
    pub mb: MaterialBalance,
    pub discrete: HashMap<String, Weight>,

    #[serde(skip)]
    cache: VecCache<WhiteScore>,

    #[serde(skip)]
    node_counts: Vec<Cell<u64>>,

    #[serde(skip)]
    node_count: Cell<u64>,

    #[serde(skip)]
    pub feature_weights: Vec<Weight>,
}

impl Default for Eval {
    fn default() -> Self {
        const DEFAULT_CACHE_SIZE: usize = 10_000;
        let mut s = Self {
            mb: MaterialBalance::default(),
            pst: Pst::default(),
            feature_weights: Vec::new(),
            discrete: HashMap::new(),
            phaser: Phaser::default(),
            see: See::default(),
            phasing: true,
            mobility_phase_disable: 101,
            quantum: 1,
            cache_size: DEFAULT_CACHE_SIZE,
            cache: VecCache::with_size(DEFAULT_CACHE_SIZE),
            node_counts: vec![Cell::new(0); DEFAULT_CACHE_SIZE],
            node_count: Cell::new(0),
        };
        for f in Feature::all() {
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
                self.cache = VecCache::with_size(self.cache_size);
                self.node_counts = vec![Cell::new(0); self.cache_size];
                self.populate_feature_weights();
                self.mb.new_game();
                self.phaser.new_game();
                self.see.new_game();
            }
            SetPosition => {
                self.mb.new_position();
                self.phaser.new_position();
                self.see.new_position();
            }
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => {}
        }
    }
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl fmt::Display for Eval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "cache size       : {}", self.cache_size)?;
        writeln!(f, "utilization (‰)  : {}", self.cache.hashfull_per_mille())?;
        writeln!(f, "[material balance]\n{}", self.mb)?;
        writeln!(f, "[phaser]\n{}", self.phaser)?;
        writeln!(f, "phasing          : {}", self.phasing)?;
        writeln!(f, "mob.phase.disable: {}", self.mobility_phase_disable)?;
        writeln!(
            f,
            "wts: {} {} {}",
            self.feature_weights[0], self.feature_weights[1], self.feature_weights[2],
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
        self.feature_weights.resize(Feature::len(), Weight::zero());
        for f in &Feature::all() {
            self.feature_weights[f.index()] = match f {
                Feature::Discrete(_i) => *self.discrete.get(&f.name()).expect(&format!(
                    "Missing discrete eval param {} in {:?}",
                    f.name(),
                    self.discrete
                )),
                Feature::Pst(p, sq) => self.pst.pst(*p, *sq),
                Feature::Piece(p) => self.mb.piece_weights[*p],
            }
        }
    }
}

impl Eval {
    pub fn weights_vector(&self) -> WeightsVector {
        WeightsVector {
            names: Vec::new(),
            weights: self.feature_weights.clone(),
        }
    }

    pub(crate) fn weight(&self, f: &Feature) -> Weight {
        self.feature_weights[f.index()]
    }

    pub fn set_weight(&mut self, f: Feature, wt: Weight) {
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

    pub fn w_eval_explain(&self, b: &Board) -> ExplainScore {
        let mut scorer = ExplainScore::new(b.phase(&self.phaser), b.to_fen());
        scorer.set_weights(self.weights_vector());
        Calc::new(&b).score(&mut scorer, b, self, &self.phaser);
        scorer
    }

    fn w_eval_no_cache(&self, b: &Board) -> WhiteScore {
        let ph = b.phase(&self.phaser);
        let mut scorer = TotalScore::new(&self.feature_weights, ph);
        Calc::new(&b).score(&mut scorer, b, self, &self.phaser);
        WhiteScore(Score::from_cp(
            scorer.total().interpolate(ph) as i32 / self.quantum * self.quantum,
        ))
    }

    fn w_eval_some(&self, b: &Board) -> WhiteScore {
        if self.cache_size == 0 {
            return self.w_eval_no_cache(b);
        }
        #[cfg(not(feature = "remove_metrics"))]
        self.node_count.set(self.node_count.get() + 1);

        let key = b.hash() as usize % self.cache_size;
        if let Some(score) = self.cache.probe(key, b.hash()) {
            Metrics::incr(Counter::EvalCacheHit);
            Metrics::incr_node(
                &Node {
                    ply: b.ply(),
                    ..Node::default()
                },
                Event::EvalCacheHit,
            );

            #[cfg(not(feature = "remove_metrics"))]
            Metrics::add_value(
                self.node_count.get() - self.node_counts[key].get(),
                crate::search::node::Histograms::EvalCacheNodeCount,
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
            self.cache.store(key, b.hash(), s);

            #[cfg(not(feature = "remove_metrics"))]
            self.node_counts[key].set(self.node_count.get());

            s
        }
    }

    /// the value of the capture or promotion (or both for promo capture)
    #[inline]
    pub fn eval_move_material(&self, mv: &Move) -> Weight {
        self.mb.eval_move_material(mv)
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
    pub fn eval_move_material(&self, eval: &Eval, mv: &Move) -> Score {
        // FIXME! far too slow (-7 ELO)
        Score::from_cp(
            eval.eval_move_material(mv)
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
    use crate::catalog::Catalog;
    use crate::infra::black_box;
    use crate::infra::profiler::*;
    use crate::search::engine::Engine;
    use crate::test_log::test;
    use anyhow::Result;
    use toml;

    #[test]
    fn test_feature_index() {
        assert_eq!(Feature::Discrete(Attr::PawnDoubled).index(), 0);
        assert_eq!(Feature::Discrete(Attr::PawnDirectlyDoubled).index(), 1);
        let last = Feature::Discrete(Attr::WinMetric2).index();
        let first_sq = Feature::Pst(Piece::Pawn, Square::A1).index();
        let last_sq = Feature::Pst(Piece::King, Square::H8).index();
        let first_piece = Feature::Piece(Piece::Pawn).index();
        let last_piece = Feature::Piece(Piece::King).index();
        assert_eq!(first_sq, last + 1);
        assert_eq!(last_sq, first_sq + 6 * 64 - 1);
        assert_eq!(first_piece, last_sq + 1);
        assert_eq!(last_piece, first_piece + 6 - 1);

        assert_eq!(
            Feature::Discrete(Attr::CenterAttacks).name(),
            "center_attacks"
        );
        assert_eq!(Feature::Pst(Piece::Pawn, Square::A1).name(), "pst.p.a1");
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
    fn test_eval_bug1() {
        let pos = &Catalog::bratko_kopec()[0];
        let b = pos.board();
        let mut eval = Eval::default();
        eval.mb.enabled = false;
        let explain = eval.w_eval_explain(b);
        println!("{}", explain);
    }

    #[test]
    fn test_eval_indexing() {
        let eng = Engine::new();
        for i in Feature::all().iter() {
            let wt = eng.algo.eval.weight(i);
            println!("{} {:<20} = {}", i.index(), i.name(), wt);
        }
    }

    #[test]
    fn bench_eval() {
        let mut eval = Eval::default();
        eval.mb.enabled = false;
        let mut prof = Profiler::new("bench_eval".into());
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
