use crate::eval::score::Score;
// use crate::board::Board;

use crate::piece::Ply;

use std::convert::AsRef;
use std::fmt;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{AsRefStr, EnumMessage};
use strum_macros::{Display, EnumCount, EnumIter};

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
    TimingSortMoves,
    TimingPseudoLegalAndLegal,
    TimingTtProbe,
    TimingTtStore,

    TimingQs,
    TimingQsEval,
    TimingQsSee,
    TimingQsMoveSort,
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

}

impl Counter {
    #[inline]
    pub fn index(&self) -> usize {
        *self as usize
    }
}


#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Display, EnumCount, EnumMessage, EnumIter, AsRefStr,
)]
#[strum(serialize_all = "title_case")]
pub enum Event {
    Unknown,
    SearchTimeUp,
    SearchComplete,
    UserCancelled,

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
    NodeZw,
    NodeLeafQs,
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
    EvalStatic,

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
    NmpDeclineBetaNumeric,
    NmpDeclineEvalNumeric,
    NmpDeclineEvalMargin,
    NmpDeclineMaterial,
    NmpDeclineInCheck,
    NmpDeclineSuccessive,
    NmpDeclineRecursive,
    NmpAttempt,
    NmpSuccess,
    NmpFail,

    #[strum(message = "Prune node: Mate distance")]
    MateDistConsider,
    MateDistSuccess,

    #[strum(message = "Prune node: Rev Fut Prune")]
    RevFutConsider,
    RevFutDeclineExt,
    RevFutDeclineMateBound,
    RevFutDeclineZugzwang,
    RevFutDeclineInCheck,
    RevFutDeclineMaxDepth,
    RevFutDeclineFwWindow,
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
    FutilityFail,
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
    SearchZwRd,
    SearchZwFd,
    ReSearchZwFd,
    ReSearchFwFd,

    NodeLeafQuietEval,
    DerivedLeaf,
    PercentBranchingFactor,

    #[strum(message = "Hash table")]
    TtHitNode,
    TtHitEvalNode,
    TtProbeNode,
    TtHitRate,
    TtStoreNode,

    TtCut,
    TtAll,
    TtPv,



    DurationIterEst,
    DurationIterAllotted,
    DurationIterActual,

    Clock,
    NodeTypeQuiesce,
    NodeTypeZw,
    PvsReSearch,
    LmrReSearch,
    PercentPvsReSearch,
    PercentLmrReSearch,

    #[strum(message = "QS")]
    NodeQs,
    NodeQsZw,
    NodeQsInterior,
    NodeQsCut,
    NodeQsPv,
    NodeQsAll,
    NodeQsAllVeryLow,    
    NodeQsInCheck,
    QsEvalStatic,
    QsTtProbe,
    QsTtHit,
    QsStandingPatPrune,
    QsMoveCount,
    QsMoveCountAtPvNode,
    QsMoveCountAtCutNode,
    QsSeePruneMove,
    QsDeltaPruneMove,
    QsDeltaPruneNode,

}

impl Default for Event {
    fn default() -> Self {
        Event::Unknown
    }
}

impl Event {
    #[inline]
    pub const fn len() -> usize {
        Event::COUNT as usize
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

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub ply: Ply,
    pub depth: Ply,
    pub alpha: Score,
    pub beta: Score,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} D:{} a:{} b:{}",
            self.ply, self.depth, self.alpha, self.beta
        )
    }
}

impl Node {
    #[inline]
    pub fn root(depth: Ply) -> Node {
        Node {
            ply: 0,
            depth,
            alpha: -Score::INFINITY,
            beta: Score::INFINITY,
        }
    }

    #[inline]
    pub fn new_child(&self) -> Node {
        Node {
            alpha: -self.beta,
            depth: self.depth - 1,
            beta: -self.alpha,
            ply: self.ply + 1,
        }
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.ply == 0
    }

    #[inline]
    pub fn is_zw(&self) -> bool {
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

}
