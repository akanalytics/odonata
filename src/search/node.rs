use crate::eval::score::Score;
// use crate::board::Board;
use serde::{Deserialize, Serialize};

use crate::types::Ply;

use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{Display, EnumCount, EnumIter};
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Display, Deserialize, EnumCount, EnumIter)]
pub enum Event {
    Unknown,
    UserCancelled,
    TimeUp,

    HashProbe,
    HashHit,
    PercentHashHit,

    RecogImmediateDraw,
    RecogMaybeWin,
    RecogHelpmateOrDraw,
    DerivedRecog,

    PruneRazor,
    PruneStandingPat,
    PruneNullMovePrune,
    DerivedPrunedInterior,
    PercentPrunedInterior,

    PruneFutilityD0,
    PruneFutilityD1,
    PruneFutilityD2,
    PruneFutilityD3,

    Lmp,
    Lmr,
    Pvs,
    Extension,

    Aspiration0,
    Aspiration1,
    Aspiration2,
    Aspiration3,
    AspirationN,
    AspirationFailLow,
    AspirationFailHigh,
    DerivedAspiration,
    PercentAspiration1,

    NodeLeafQuietEval,
    NodeLeafDraw,
    NodeLeafWinLoss,
    NodeLeafStalemate,
    DerivedLeaf,
    PercentBranchingFactor,

    NodeInterior,
    NodeInteriorAll,
    NodeInteriorCut,
    NodeInteriorPv,

    Moves,
    MoveStart,
    MoveHash,
    MoveInitialize,
    MoveNull,
    MoveGoodCapture,
    MoveGoodCaptureUpfrontSorted,
    MoveKiller,
    MovePromo,
    MoveEvasion,
    MoveQuiet,
    MoveQuietUnsorted,
    MoveBadCapture,
    MoveRemaining,
    MoveUnsorted,
    MoveCapture,
    MoveEnd,

    Clock,
    NodeTypeQuiesce,
    NodeTypeZw,
    PvsReSearch,
    LmrReSearch,
    PercentPvsReSearch,
    PercentLmrReSearch,
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
    pub fn name(&self) -> String {
        self.to_string()
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
        write!(f, "{} D:{} a:{} b:{}", self.ply, self.depth, self.alpha, self.beta)
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
        self.alpha.is_numeric() && self.beta.is_numeric() && self.alpha.as_i16() + 1 == self.beta.as_i16()
    }

    #[inline]
    pub fn is_pv(&self) -> bool {
        !self.is_zw()
    }

    #[inline]
    pub fn is_qs(&self) -> bool {
        self.depth <= 0
    }
}
