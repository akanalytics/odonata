use crate::bitboard::square::Square;
use crate::board::Board;
use crate::eval::material_balance::MaterialBalance;
use crate::eval::pst::Pst;
use crate::eval::score::Score;
use crate::eval::see::See;
use crate::eval::switches::Switches;
use crate::eval::weight::Weight;
use crate::globals::counts;
use crate::infra::component::Component;
use crate::infra::component::State;
use crate::mv::Move;
use crate::phaser::Phaser;
use crate::prelude::*;
use crate::search::node::Node;
use crate::trace::stat::{ArrayStat, Stat};
use crate::types::{Color, Piece};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use std::collections::HashMap;
use std::fmt;

use super::calc::Calc;
use super::feature::WeightsVector;


use super::scorer::ExplainScore;
use super::scorer::{TotalScore};

// https://www.chessprogramming.org/Simplified_Evaluation_Function

pub static ALL: Stat = Stat::new("ALL");
pub static QUIESCENCE: Stat = Stat::new("QUIESCENCE");
pub static MATERIAL: Stat = Stat::new("MATERIAL");
pub static POSITION: Stat = Stat::new("POSITION");
pub static MOBILITY: Stat = Stat::new("MOBILITY");
pub static SEE: Stat = Stat::new("SEE");
pub static MOVE: Stat = Stat::new("MOVE");

pub static EVAL_COUNTS: ArrayStat = ArrayStat(&[&ALL, &QUIESCENCE, &MATERIAL, &POSITION, &MOBILITY, &SEE, &MOVE]);

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PieceArray<T> {
    pub p: T,
    pub n: T,
    pub b: T,
    pub r: T,
    pub q: T,
    pub k: T,
}

impl<T> std::ops::Index<Piece> for PieceArray<T> {
    type Output = T;
    #[inline]
    fn index(&self, i: Piece) -> &Self::Output {
        [&self.p, &self.n, &self.b, &self.r, &self.q, &self.k][i.index() - 1]
    }
}

impl<T> std::ops::IndexMut<Piece> for PieceArray<T> {
    #[inline]
    fn index_mut(&mut self, p: Piece) -> &mut Self::Output {
        [&mut self.p, &mut self.n, &mut self.b, &mut self.r, &mut self.q, &mut self.k][p.index() - 1]
    }
}

use strum_macros::Display;
use strum_macros::EnumCount;
use strum_macros::EnumDiscriminants;
use strum_macros::EnumIter;
use strum_macros::IntoStaticStr;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, IntoStaticStr, EnumCount, EnumIter, Display)]
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
    EnemyRookOnPasser,
    Space,
    RammedPawns,
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

    CenterAttacks,
    UndefendedSq,
    UndefendedPiece,
    TrappedPiece,
    PartiallyTrappedPiece,
    RookOpenFile,
    QueenOpenFile,

    Fianchetto,
    BishopOutposts,
    BishopColorPawns,
    KnightForks,
    KnightOutposts,
    DoubledRooks,
    DoubledRooksOpenFile,
    EnemyPawnsOnRookRank,
    QueenEarlyDevelop,

    PawnAdjacentShield,
    PawnNearbyShield,
    KingSafetyBonus,
    OpenFilesNearKing,
    OpenFilesAdjacentKing,
    AttacksNearKing,
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

    ContemptPenalty,
    TempoBonus,
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

#[derive(Clone, Copy, PartialEq, Debug, EnumDiscriminants, IntoStaticStr, EnumCount, EnumIter, Display)]
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
            Feature::Pst(p, sq) => Attr::WinMetric2 as usize + 1 + (p.index() - 1) * Square::len() + sq.index(),
            Feature::Piece(p) => Attr::WinMetric2 as usize + 1 + Square::len() * (Piece::len() - 1) + (p.index() - 1),
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
                    for p in Piece::ALL_BAR_NONE {
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
    pub material: bool,
    pub position: bool,
    pub mobility: bool,
    pub pawn: bool,
    pub safety: bool,
    pub contempt: bool,
    pub tempo: bool,
    pub phasing: bool,
    pub mobility_phase_disable: u8,
    pub quantum: i32,

    pub min_depth_mob: u8,

    pub pst: Pst,
    // pub pmvt: Pmvt,
    pub phaser: Phaser,
    pub see: See,
    pub mb: MaterialBalance,
    pub discrete: HashMap<String, Weight>,

    #[serde(skip)]
    pub feature_weights: Vec<Weight>,

}

impl Default for Eval {
    fn default() -> Self {
        let mut s = Self {
            mb: MaterialBalance::default(),
            pst: Pst::default(),
            feature_weights: Vec::new(),
            discrete: HashMap::new(),
            // pmvt: Pmvt::default(),
            phaser: Phaser::default(),
            see: See::default(),
            mobility: true,
            position: true,
            material: true,
            pawn: true,
            safety: true,
            contempt: true,
            tempo: true,
            phasing: true,
            mobility_phase_disable: 101,
            quantum: 1,
            min_depth_mob: 1,
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
                self.mb.new_game();
                self.phaser.new_game();
                self.see.new_game();
            }
            SetPosition => {
                self.mb.new_position();
                self.phaser.new_position();
                self.see.new_position();
            }
            StartSearch => {
                self.populate_feature_weights();
            }
            EndSearch => {}
            StartDepthIteration(_) => {}
        }
    }
    fn new_game(&mut self) {}

    fn new_position(&mut self) {}
}

impl fmt::Display for Eval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "[material balance]\n{}", self.mb)?;
        writeln!(f, "[phaser]\n{}", self.phaser)?;
        writeln!(f, "material         : {}", self.material)?;
        writeln!(f, "position         : {}", self.position)?;
        writeln!(f, "mobility         : {}", self.mobility)?;
        writeln!(f, "pawn             : {}", self.pawn)?;
        writeln!(f, "safety           : {}", self.safety)?;
        writeln!(f, "contempt         : {}", self.contempt)?;
        writeln!(f, "tempo            : {}", self.tempo)?;
        writeln!(f, "phasing          : {}", self.phasing)?;
        writeln!(f, "mob.phase.disable: {}", self.mobility_phase_disable)?;
        writeln!(f, "mob.min.depth    : {}", self.min_depth_mob)?;
        // writeln!(f, "undefended.piece : {}", self.undefended_piece)?;
        // writeln!(f, "undefended.sq    : {}", self.undefended_sq)?;
        // writeln!(f, "trapped.piece    : {}", self.trapped_piece)?;
        // writeln!(f, "part.trap.piece  : {}", self.partially_trapped_piece)?;
        // // writeln!(f, "defended.non.pawn: {}", self.defended_non_pawn)?;
        // writeln!(f, "castling.rights  : {}", self.castling_rights)?;
        // // writeln!(f, "pawn.shield      : {}", self.pawn_shield)?;
        // writeln!(f, "pawn.doubled     : {}", self.pawn_doubled)?;
        // writeln!(f, "pawn.passed      : {}", self.pawn_passed)?;
        // writeln!(f, "pawn.passed.r5   : {}", self.pawn_passed_r5)?;
        // writeln!(f, "pawn.passed.r6   : {}", self.pawn_passed_r6)?;
        // writeln!(f, "pawn.passed.r7   : {}", self.pawn_passed_r7)?;
        // writeln!(f, "pawn.isolated    : {}", self.pawn_isolated)?;

        // writeln!(f, "bishop pair      : {}", self.bishop_pair)?;
        // writeln!(f, "rook pair        : {}", self.rook_pair)?;
        // writeln!(f, "fianchetto       : {}", self.fianchetto)?;
        // writeln!(f, "bishop outposts  : {}", self.bishop_outposts)?;
        // writeln!(f, "knight outposts  : {}", self.knight_outposts)?;
        // writeln!(f, "knight forks     : {}", self.knight_forks)?;
        // writeln!(f, "doubled.rook     : {}", self.doubled_rooks)?;
        // writeln!(f, "doubled.rook.open: {}", self.doubled_rooks_open_file)?;

        // writeln!(f, "rook.open.file   : {}", self.rook_open_file)?;

        // writeln!(f, "queen.open.file  : {}", self.queen_open_file)?;
        // writeln!(f, "pawn.nearby      : {}", self.pawn_nearby_shield)?;
        // writeln!(f, "pawn.adjacent    : {}", self.pawn_adjacent_shield)?;
        // writeln!(f, "tropism.d1       : {}", self.tropism_d1)?;
        // writeln!(f, "tropism.d2       : {}", self.tropism_d2)?;
        // writeln!(f, "tropism.d3       : {}", self.tropism_d3)?;
        // writeln!(f, "contempt penalty : {}", self.contempt_penalty)?;
        // writeln!(f, "tempo bonus      : {}", self.tempo_bonus)?;
        writeln!(f, "eval stats\n{}", EVAL_COUNTS)?;
        // writeln!(f, "cache\n{}", self.cache)?;
        // writeln!(f, "qcache\n{}", self.qcache)?;

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
        Feature::all().iter().for_each(|f| {
            self.feature_weights[f.index()] = match f {
                Feature::Discrete(_i) => {
                    *self
                        .discrete
                        .get(&f.name())
                        .expect(&format!("Missing discrete eval param {} in {:?}", f.name(), self.discrete))
                }
                Feature::Pst(p, sq) => self.pst.pst(*p, *sq),
                Feature::Piece(p) => self.mb.piece_weights[*p],
            }
        });
    }

    pub fn set_switches(&mut self, enabled: bool) {
        self.material = enabled;
        self.position = enabled;
        self.mobility = enabled;
        self.safety = enabled;
        self.pawn = enabled;
        self.contempt = enabled;
        self.tempo = enabled;
    }

    pub fn set_position(&mut self, enabled: bool) -> Self {
        self.position = enabled;
        self.clone()
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

    pub fn w_eval_draw(&self, board: &Board, _node: &Node) -> Score {
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
        let contempt = self.weight(&Attr::ContemptPenalty.into());
        let contempt = self.contempt as i32 * board.color_us().chooser_wb(contempt, -contempt);
        Score::from_f32(contempt.interpolate(board.phase(&self.phaser)) as f32)

        // FIXME! v33
        // let signum = 1 - (node.ply % 2) * 2; // ply=0 => 1  ply=1=> -1
        // let contempt = signum * self.contempt + board.signum();
        // return Score::from_cp(contempt);
    }

    pub fn w_evaluate(&self, board: &Board, node: &Node) -> Score {
        counts::EVAL_COUNT.increment();
        let outcome = board.outcome();
        if outcome.is_game_over() {
            if outcome.is_draw() {
                return self.w_eval_draw(board, node);
            }
            if let Some(c) = outcome.winning_color() {
                return c.chooser_wb(Score::white_win(node.ply), Score::white_loss(node.ply));
            }
        }
        self.w_eval_some(board, Switches::ALL_SCORING)
    }

    // we dont care about stalemates or checkmates
    pub fn w_eval_qsearch(&self, board: &Board, _node: &Node) -> Score {
        counts::QEVAL_COUNT.increment();
        self.w_eval_some(board, Switches::ALL_SCORING | Switches::INSUFFICIENT_MATERIAL)
    }



    pub fn w_eval_explain(&self, b: &Board, _csv: bool) -> ExplainScore {
        // let mut model = Model::from_board(b, b.phase(&self.phaser), Switches::ALL_SCORING);
        // model.csv = csv;
        let mut scorer = ExplainScore::new(b.phase(&self.phaser), b.to_fen());
        Calc::score(&mut scorer, b, self, &self.phaser);
        // self.predict(&model, &mut scorer);
        scorer
    }

    pub fn w_eval_some(&self, b: &Board, mut switches: Switches) -> Score {
        if !self.position {
            switches -= Switches::POSITION;
        }
        if !self.safety {
            switches -= Switches::SAFETY;
        }
        if !self.mobility {
            switches -= Switches::MOBILITY;
        }
        if !self.pawn {
            switches -= Switches::PAWN;
        }


        // error!("===== {}",b.to_fen());
        // let model = Model::from_board(b, b.phase(&self.phaser), switches);
        let ph = b.phase(&self.phaser);
        let mut scorer = TotalScore::new(&self.feature_weights, ph);
        Calc::score(&mut scorer, b, self, &self.phaser);
        let score1 = Score::from_cp(scorer.total().interpolate(ph) as i32 / self.quantum * self.quantum);
        score1

    }


    /// the value of the capture or promotion (or both for promo capture)
    #[inline]
    pub fn eval_move_material(&self, mv: &Move) -> Weight {
        self.mb.eval_move_material(mv)
    }
}

impl Board {
    #[inline]
    fn signum(&self) -> i32 {
        self.color_us().chooser_wb(1, -1)
    }

    #[inline]
    pub fn eval_qsearch(&self, eval: &mut Eval, nd: &Node) -> Score {
        QUIESCENCE.increment();
        self.signum() * eval.w_eval_qsearch(self, nd)
    }

    #[inline]
    pub fn eval_draw(&self, eval: &mut Eval, nd: &Node) -> Score {
        self.signum() * eval.w_eval_draw(self, nd)
    }

    #[inline]
    pub fn eval_move_see(&self, eval: &Eval, mv: Move) -> Score {
        SEE.increment();
        Score::from_cp(eval.see.eval_move_see(self, mv))
    }

    #[inline]
    pub fn eval_move_material(&self, eval: &Eval, mv: &Move) -> Score {
        MOVE.increment();
        // FIXME! far too slow (-7 ELO)
        Score::from_cp(eval.eval_move_material(mv).interpolate(self.phase(&eval.phaser)) as i32)
    }

    #[inline]
    pub fn eval(&self, eval: &Eval, nd: &Node) -> Score {
        ALL.increment();
        self.signum() * eval.w_evaluate(self, nd)
    }

    #[inline]
    pub fn eval_some(&self, eval: &Eval, sw: Switches) -> Score {
        profile_fn!(board.eval_some);
        // let _g = hprof::enter("eval some");
        ALL.increment();
        self.signum() * eval.w_eval_some(self, sw)
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::infra::profiler::Profiler;
    use crate::search::engine::Engine;
    use crate::test_log::test;
    use anyhow::Result;
    use iai::black_box;
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

        assert_eq!(Feature::Discrete(Attr::CenterAttacks).name(), "center_attacks");
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
        let explain = eval.w_eval_explain(b, false);
        println!("{}", explain);
    }

    #[test]
    fn test_eval_indexing() {
        let mut eng = Engine::new();
        eng.algo.eval.populate_feature_weights();
        for i in Feature::all().iter() {
            let wt = eng.algo.eval.weight(i);
            println!("{} {:<20} = {}", i.index(), i.name(), wt);
        }
    }

    #[test]
    fn prof_eval() {
        let mut eval = Eval::default();
        eval.mb.enabled = false;
        let mut prof = Profiler::new("bench_eval".into());
        let node = Node::root(0);
        let mut total_score = Score::zero();
        for pos in Catalog::win_at_chess() {
            let b = pos.board();
            prof.start();
            let score = b.signum() * b.eval(&eval, &node);
            prof.stop();
            total_score = total_score + score;
            println!("{:>6.0} {}", score.as_i16(), pos);
        }
        println!("{:>6.0} {}", total_score.as_i16(), "total");
    }

    #[ignore]
    #[test]
    fn bench_eval() {
        let positions = Catalog::win_at_chess();
        for _ in 0..150000 {
            let mut eval = Eval::default();
            eval.mb.enabled = false;
            let node = Node::root(0);
            let mut total_score = Score::zero();
            for pos in &positions {
                let b = pos.board();
                let score = b.signum() * b.eval(&eval, &node);
                total_score = total_score + score;
                // println!("{:>6.0} {}", score.as_i16(), pos);
            }
            black_box(total_score);
        }
    }
}
