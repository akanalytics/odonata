use crate::{domain::BoundType, eval::score::Score};
// use crate::boards::Board;

use crate::piece::Ply;

use std::{convert::AsRef, fmt};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{AsRefStr, Display, EnumCount, EnumIter, EnumMessage};

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Display, EnumCount, EnumMessage, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "title_case")]
pub enum Histograms {
    EvalCacheNodeCount,
}

impl Histograms {
    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Display, EnumCount, EnumMessage, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "title_case")]
pub enum Timing {
    TimingSearchRoot,
    TimingEval,
    TimingMoveGen,
    TimingMakeMove,
    TimingParseEpd,
    TimingParseEpdThreaded,
    TimingReadFile,
    TimingSortMoves,
    TimingPseudoLegalAndLegal,
    TimingTtProbe,
    TimingTtStore,

    TimingQs,
    TimingQsEval,
    TimingQsSee,
    TimingQsMoveSort,

    TimimgTunerUploadPositions,
    TimingTunerSolverLbfgs,
}

impl Timing {
    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Display, EnumCount, EnumMessage, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "title_case")]
pub enum Counter {
    SearchTimeUp,
    SearchComplete,

    MakeMove,
    MoveGen,
    CalcHashBoard,
    CalcHashPawns,
    CalcHashMove,
    MatchedCounterMove,
    TtCollision,
    TtInsert,
    TtUpdate,
    TtPvOverwrite,
    TtIllegalMove,

    EvalCacheHit,
    EvalCacheMiss,
    EvalCachePercent,

    TunerGradient,
    TunerCostFunction,
    TunerMse,
}

impl Counter {
    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }
}

#[derive(
    Copy, Clone, Default, Debug, PartialEq, Eq, Display, EnumCount, EnumMessage, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "title_case")]
pub enum Event {
    #[default]
    Unknown,
    SearchTimeUp,
    SearchComplete,
    UserCancelled,

    MovePush,

    HashProbe,
    HashHit,
    PercentHashHit,
    EvalCacheHit,
    EvalCacheMiss,

    #[strum(message = "Node Counts")]
    NodeTotal,
    NodeNmpPerc,
    NodeRazorPerc,
    NodeRevFutPerc,
    NodeNullMove,
    NodeRazor,
    NodeZw,
    NodeQsLeaf,
    NodeLeafDraw,
    NodeLeafWinLoss,
    NodeLeafStalemate,
    NodeInterior,
    NodeInteriorAll,
    NodeInteriorCut,
    NodeInteriorPv,
    MeanBranchingFactor,
    EffectiveBranchingFactor,

    #[strum(message = "Eval")]
    EvalSee,
    InteriorEvalStatic,

    EndgameDraw,
    EndgameKnown,
    EndgameWinOrDraw,
    RecogImmediateDraw,
    RecogMaybeWin,
    RecogHelpmateOrDraw,

    #[strum(message = "Moves")]
    MoveCount,

    MovePawn,
    MoveBishop,
    MoveKnight,
    MoveRook,
    MoveQueen,
    MoveKing,

    // MoveStart,
    #[strum(message = "Move type")]
    MoveHash,
    // MoveInitialize,
    MoveNull,
    MoveCapture,
    MoveGoodCapture,
    MoveBadCapture,
    // MoveGoodCaptureUpfrontSorted,
    MoveKiller,
    MoveSortCounterMove,
    MovePromo,
    MoveQuiet,
    MoveOther,

    // CounterMove,
    // MoveEvasion,
    // MoveQuietOrPromo,
    // MoveQuietUnsorted,
    // MoveRemaining,
    // MoveUnsorted,
    // MoveEnd,
    DerivedRecog,
    DerivedPrunedInterior,
    PercentPrunedInterior,

    #[strum(message = "Prune node: Razoring")]
    RazorConsider,
    RazorDeclineDepth,
    RazorDeclineAlphaNumeric,
    RazorDeclineBetaNumeric,
    RazorDeclinePvNode,
    RazorDeclineInCheck,
    RazorDeclineMinOpponents,
    RazorDeclineMinPieces,
    RazorDeclineMargin,
    RazorSuccess,
    RazorD1Success,
    RazorD2Success,
    RazorD3Success,
    RazorFail,

    StandingPatSuccess,

    #[strum(message = "Prune node: Null Move")]
    NmpConsider,
    NmpDeclineDepth,
    NmpDeclinePvNode,
    NmpDeclineMateBound,
    NmpDeclineEvalNumeric,
    NmpDeclineEvalMargin,
    NmpDeclineZugzwang,
    NmpDeclineInCheck,
    NmpDeclineSuccessive,
    NmpDeclineRecursive,
    NmpAttempt,
    NmpSuccess,
    NmpFail,
    NmpBadSkip,
    NmpGoodSkip,
    NmpBadNullSearch,
    NmpGoodNullSearch,

    #[strum(message = "Prune node: Mate distance")]
    MateDistConsider,
    MateDistSuccess,

    #[strum(message = "Prune node: Rev Fut Prune")]
    RevFutConsider,
    RevFutDeclineExt,
    RevFutDeclineAlphaMateBound,
    RevFutDeclineBetaMateBound,
    RevFutDeclineEvalMateBound,
    RevFutDeclineZugzwang,
    RevFutDeclineInCheck,
    RevFutDeclineMaxDepth,
    RevFutDeclineFwWindow,
    RevFutDeclineMinPieces,
    RevFutFail,
    RevFutSuccess,

    #[strum(message = "Prune move: Futility Prune")]
    FutilityConsider,
    FutilityDeclineExt,
    FutilityDeclineFirstMove,
    FutilityDeclineMateBound,
    FutilityDeclineGivesCheck,
    FutilityDeclineInCheck,
    FutilityDeclineDiscoverer,
    FutilityDeclinePawnMaxRank,
    FutilityDeclineMaxDepth,
    FutilityDeclineFwWindow,
    FutilityDeclineMinPieces,
    FutilityFail,
    FutilitySuccess,
    FutilitySuccessRemaining,
    FutilityD0,
    FutilityD1,
    FutilityD2,
    FutilityD3,

    #[strum(message = "Prune move: Late Move Prune")]
    LmpConsider,
    LmpDeclineMateBound,
    LmpDeclineMoveNum,
    LmpDeclineFormula,
    LmpDeclineGivesCheck,
    LmpDeclineInCheck,
    LmpDeclineMaxDepth,
    LmpDeclineFwWindow,
    LmpSuccess,

    #[strum(message = "Late Move Reduce")]
    LmrConsider,
    LmrDeclineCapture,
    LmrDeclineKiller,
    LmrDeclinePawnRank,
    Lmr,
    LmrD1,
    LmrD2,
    LmrD3,
    LmrD4,
    LmrDN,

    Pvs,
    Extension,

    #[strum(message = "Aspiration")]
    AspirationNone,
    Aspiration1,
    Aspiration2,
    Aspiration3,
    AspirationN,
    AspirationFailLow,
    AspirationFailHigh,
    DerivedAspiration,
    PercentAspiration1,

    #[strum(message = "PVS")]
    SearchFwFd,
    SearchZwFd,
    SearchZwRd,
    ReSearchFwFd,
    ReSearchZwFd,

    NodeLeafQuietEval,
    DerivedLeaf,
    PercentBranchingFactor,

    #[strum(message = "Hash table")]
    TtHitNode,
    TtHitEvalNode,
    TtMissEvalNode,
    TtProbeNode,
    TtHitRate,
    TtStoreNode,

    TtCut,
    TtAll,
    TtPv,

    DurationIterEst,
    DurationIterAllotted,
    DurationIterActual,

    AlphaRaised,
    MoveScoreLow,
    MoveScoreHigh,

    Clock,
    NodeTypeQuiesce,
    NodeTypeZw,
    PvsReSearch,
    LmrReSearch,
    PercentPvsReSearch,
    PercentLmrReSearch,

    #[strum(message = "QS")]
    NodeQs,
    NodeQsLeafZw,
    QsMoveGen,
    QsCatCutMoveEval,
    QsCatNodePv,
    QsCatAll,
    QsCatAllCp200,
    NodeQsInCheck,
    QsEvalStatic,
    QsTtProbe,
    QsTtHit,
    QsAlphaRaised,
    QsMoveScoreLow,
    QsCatCutStandingPat,
    QsStandingPatAlphaRaised,
    QsMoveCount,
    QsCountMovesAtAllNode,
    QsCountMovesAtPvNode,
    QsCountMovesAtCutNode,
    QsMovePruneSee,
    QsMovePruneSeeMargin,
    QsMovePruneDelta,
    QsCatAllDeltaPruneNode,
    QsCatInsufficientMaterial,
}

impl Event {
    #[inline]
    pub const fn len() -> usize {
        Event::COUNT
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.as_ref()
        // toml::to_string(self).unwrap()
    }

    #[inline]
    pub fn from(i: usize) -> Self {
        Event::iter().nth(i).unwrap()
    }

    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }
}

use bitflags::bitflags;

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct SearchType : u8 {
        const ZeroWindow = 1 << 0;
        const FullWindow = 1 << 1;
        const NullMove = 1 << 2;
        const Razor = 1 << 4;
        const ReSearch = 1 << 4;
        const End = 1 << 5;
    }
}

impl Default for SearchType {
    fn default() -> Self {
        SearchType::FullWindow
    }
}


impl fmt::Display for SearchType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use bitflags::parser::to_writer;
        let mut s = String::new();
        to_writer(self, &mut s)?;
        f.write_str(&s)?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub zw:    SearchType,
    pub ply:   Ply,
    pub depth: Ply,
    pub alpha: Score,
    pub beta:  Score,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "P:{} D:{} a:{} b:{} {}",
            self.ply, self.depth, self.alpha, self.beta, self.zw
        )
    }
}

impl Node {
    #[inline]
    pub fn root(depth: Ply) -> Node {
        Node {
            zw: SearchType::FullWindow,
            ply: 0,
            depth,
            alpha: -Score::INFINITY,
            beta: Score::INFINITY,
        }
    }

    #[inline]
    pub fn new_child(&self) -> Node {
        Node {
            zw:    self.zw,
            alpha: -self.beta,
            depth: self.depth - 1,
            beta:  -self.alpha,
            ply:   self.ply + 1,
        }
    }

    #[inline]
    pub fn ext(&self, ext: Ply) -> Node {
        Node {
            depth: self.depth + ext,
            ..*self
        }
    }

    #[inline]
    pub fn zw(&self) -> Node {
        Node {
            zw: SearchType::ZeroWindow,
            alpha: self.beta + Score::from_cp(-1),
            ..*self
        }
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.ply == 0
    }

    #[inline]
    pub fn is_null_move(&self) -> bool {
        self.zw.contains(SearchType::NullMove)
    }

    #[inline]
    pub fn is_razor(&self) -> bool {
        self.zw.contains(SearchType::Razor)
    }

    #[inline]
    pub fn is_zw(&self) -> bool {
        // self.is_fw_equal_zw()
        self.alpha.is_numeric() && self.beta.is_numeric() && self.zw.contains(SearchType::ZeroWindow)
    }

    // cases when zw and fw are actually the same
    pub fn is_fw_equal_zw(&self) -> bool {
        self.alpha.is_numeric()
            && self.beta.is_numeric()
            && self.alpha.as_i16() + 1 == self.beta.as_i16()
    }

    #[inline]
    pub fn is_fw(&self) -> bool {
        !self.is_zw()
    }

    #[inline]
    pub fn is_qs(&self) -> bool {
        self.depth <= 0
    }

    pub fn node_type(&self, score: Score) -> BoundType {
        match score {
            s if s <= self.alpha => BoundType::UpperAll,
            s if s >= self.beta => BoundType::LowerCut,
            _ => BoundType::ExactPv, // alpha < score < beta
        }
    }
}

#[cfg(test)]
mod tests {
    use strum::EnumCount;
    use test_log::test;

    use crate::search::node::Event;

    #[test]
    fn event() {
        println!("{}", Event::COUNT);
    }
}
