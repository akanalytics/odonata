use crate::{bits::Square, Piece};

use serde::{Deserialize, Serialize};
use strum_macros::{
    Display, EnumCount, EnumIter, EnumString, EnumVariantNames, FromRepr, IntoStaticStr,
};

#[derive(
    Clone,
    Copy,
    Eq,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Debug,
    IntoStaticStr,
    FromRepr,
    EnumCount,
    EnumString,
    EnumIter,
    EnumVariantNames,
    Display,
)]
#[strum(serialize_all = "title_case")] // "King Safety"
pub enum FeatureCategory {
    Material = 0,
    Imbalance,
    Initiative,
    Pawns,
    Knights,
    Bishops,
    Rooks,
    Queens,
    Mobility,
    #[strum(serialize = "King safety")]
    KingSafety,
    Threats,
    Passed,
    Space,
    Winnable,
    Other,
}

#[derive(
    Clone,
    Copy,
    Eq,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Debug,
    IntoStaticStr,
    FromRepr,
    EnumCount,
    EnumString,
    EnumIter,
    Display,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "snake_case")]
#[allow(non_camel_case_types)]
pub enum Feature {
    PawnDoubled = 0,
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

    // block copy from square
    PstP_A1,
    PstP_B1,
    PstP_C1,
    PstP_D1,
    PstP_E1,
    PstP_F1,
    PstP_G1,
    PstP_H1,
    PstP_A2,
    PstP_B2,
    PstP_C2,
    PstP_D2,
    PstP_E2,
    PstP_F2,
    PstP_G2,
    PstP_H2,
    PstP_A3,
    PstP_B3,
    PstP_C3,
    PstP_D3,
    PstP_E3,
    PstP_F3,
    PstP_G3,
    PstP_H3,
    PstP_A4,
    PstP_B4,
    PstP_C4,
    PstP_D4,
    PstP_E4,
    PstP_F4,
    PstP_G4,
    PstP_H4,
    PstP_A5,
    PstP_B5,
    PstP_C5,
    PstP_D5,
    PstP_E5,
    PstP_F5,
    PstP_G5,
    PstP_H5,
    PstP_A6,
    PstP_B6,
    PstP_C6,
    PstP_D6,
    PstP_E6,
    PstP_F6,
    PstP_G6,
    PstP_H6,
    PstP_A7,
    PstP_B7,
    PstP_C7,
    PstP_D7,
    PstP_E7,
    PstP_F7,
    PstP_G7,
    PstP_H7,
    PstP_A8,
    PstP_B8,
    PstP_C8,
    PstP_D8,
    PstP_E8,
    PstP_F8,
    PstP_G8,
    PstP_H8,

    PstN_A1,
    PstN_B1,
    PstN_C1,
    PstN_D1,
    PstN_E1,
    PstN_F1,
    PstN_G1,
    PstN_H1,
    PstN_A2,
    PstN_B2,
    PstN_C2,
    PstN_D2,
    PstN_E2,
    PstN_F2,
    PstN_G2,
    PstN_H2,
    PstN_A3,
    PstN_B3,
    PstN_C3,
    PstN_D3,
    PstN_E3,
    PstN_F3,
    PstN_G3,
    PstN_H3,
    PstN_A4,
    PstN_B4,
    PstN_C4,
    PstN_D4,
    PstN_E4,
    PstN_F4,
    PstN_G4,
    PstN_H4,
    PstN_A5,
    PstN_B5,
    PstN_C5,
    PstN_D5,
    PstN_E5,
    PstN_F5,
    PstN_G5,
    PstN_H5,
    PstN_A6,
    PstN_B6,
    PstN_C6,
    PstN_D6,
    PstN_E6,
    PstN_F6,
    PstN_G6,
    PstN_H6,
    PstN_A7,
    PstN_B7,
    PstN_C7,
    PstN_D7,
    PstN_E7,
    PstN_F7,
    PstN_G7,
    PstN_H7,
    PstN_A8,
    PstN_B8,
    PstN_C8,
    PstN_D8,
    PstN_E8,
    PstN_F8,
    PstN_G8,
    PstN_H8,

    PstB_A1,
    PstB_B1,
    PstB_C1,
    PstB_D1,
    PstB_E1,
    PstB_F1,
    PstB_G1,
    PstB_H1,
    PstB_A2,
    PstB_B2,
    PstB_C2,
    PstB_D2,
    PstB_E2,
    PstB_F2,
    PstB_G2,
    PstB_H2,
    PstB_A3,
    PstB_B3,
    PstB_C3,
    PstB_D3,
    PstB_E3,
    PstB_F3,
    PstB_G3,
    PstB_H3,
    PstB_A4,
    PstB_B4,
    PstB_C4,
    PstB_D4,
    PstB_E4,
    PstB_F4,
    PstB_G4,
    PstB_H4,
    PstB_A5,
    PstB_B5,
    PstB_C5,
    PstB_D5,
    PstB_E5,
    PstB_F5,
    PstB_G5,
    PstB_H5,
    PstB_A6,
    PstB_B6,
    PstB_C6,
    PstB_D6,
    PstB_E6,
    PstB_F6,
    PstB_G6,
    PstB_H6,
    PstB_A7,
    PstB_B7,
    PstB_C7,
    PstB_D7,
    PstB_E7,
    PstB_F7,
    PstB_G7,
    PstB_H7,
    PstB_A8,
    PstB_B8,
    PstB_C8,
    PstB_D8,
    PstB_E8,
    PstB_F8,
    PstB_G8,
    PstB_H8,

    PstR_A1,
    PstR_B1,
    PstR_C1,
    PstR_D1,
    PstR_E1,
    PstR_F1,
    PstR_G1,
    PstR_H1,
    PstR_A2,
    PstR_B2,
    PstR_C2,
    PstR_D2,
    PstR_E2,
    PstR_F2,
    PstR_G2,
    PstR_H2,
    PstR_A3,
    PstR_B3,
    PstR_C3,
    PstR_D3,
    PstR_E3,
    PstR_F3,
    PstR_G3,
    PstR_H3,
    PstR_A4,
    PstR_B4,
    PstR_C4,
    PstR_D4,
    PstR_E4,
    PstR_F4,
    PstR_G4,
    PstR_H4,
    PstR_A5,
    PstR_B5,
    PstR_C5,
    PstR_D5,
    PstR_E5,
    PstR_F5,
    PstR_G5,
    PstR_H5,
    PstR_A6,
    PstR_B6,
    PstR_C6,
    PstR_D6,
    PstR_E6,
    PstR_F6,
    PstR_G6,
    PstR_H6,
    PstR_A7,
    PstR_B7,
    PstR_C7,
    PstR_D7,
    PstR_E7,
    PstR_F7,
    PstR_G7,
    PstR_H7,
    PstR_A8,
    PstR_B8,
    PstR_C8,
    PstR_D8,
    PstR_E8,
    PstR_F8,
    PstR_G8,
    PstR_H8,

    PstQ_A1,
    PstQ_B1,
    PstQ_C1,
    PstQ_D1,
    PstQ_E1,
    PstQ_F1,
    PstQ_G1,
    PstQ_H1,
    PstQ_A2,
    PstQ_B2,
    PstQ_C2,
    PstQ_D2,
    PstQ_E2,
    PstQ_F2,
    PstQ_G2,
    PstQ_H2,
    PstQ_A3,
    PstQ_B3,
    PstQ_C3,
    PstQ_D3,
    PstQ_E3,
    PstQ_F3,
    PstQ_G3,
    PstQ_H3,
    PstQ_A4,
    PstQ_B4,
    PstQ_C4,
    PstQ_D4,
    PstQ_E4,
    PstQ_F4,
    PstQ_G4,
    PstQ_H4,
    PstQ_A5,
    PstQ_B5,
    PstQ_C5,
    PstQ_D5,
    PstQ_E5,
    PstQ_F5,
    PstQ_G5,
    PstQ_H5,
    PstQ_A6,
    PstQ_B6,
    PstQ_C6,
    PstQ_D6,
    PstQ_E6,
    PstQ_F6,
    PstQ_G6,
    PstQ_H6,
    PstQ_A7,
    PstQ_B7,
    PstQ_C7,
    PstQ_D7,
    PstQ_E7,
    PstQ_F7,
    PstQ_G7,
    PstQ_H7,
    PstQ_A8,
    PstQ_B8,
    PstQ_C8,
    PstQ_D8,
    PstQ_E8,
    PstQ_F8,
    PstQ_G8,
    PstQ_H8,

    PstK_A1,
    PstK_B1,
    PstK_C1,
    PstK_D1,
    PstK_E1,
    PstK_F1,
    PstK_G1,
    PstK_H1,
    PstK_A2,
    PstK_B2,
    PstK_C2,
    PstK_D2,
    PstK_E2,
    PstK_F2,
    PstK_G2,
    PstK_H2,
    PstK_A3,
    PstK_B3,
    PstK_C3,
    PstK_D3,
    PstK_E3,
    PstK_F3,
    PstK_G3,
    PstK_H3,
    PstK_A4,
    PstK_B4,
    PstK_C4,
    PstK_D4,
    PstK_E4,
    PstK_F4,
    PstK_G4,
    PstK_H4,
    PstK_A5,
    PstK_B5,
    PstK_C5,
    PstK_D5,
    PstK_E5,
    PstK_F5,
    PstK_G5,
    PstK_H5,
    PstK_A6,
    PstK_B6,
    PstK_C6,
    PstK_D6,
    PstK_E6,
    PstK_F6,
    PstK_G6,
    PstK_H6,
    PstK_A7,
    PstK_B7,
    PstK_C7,
    PstK_D7,
    PstK_E7,
    PstK_F7,
    PstK_G7,
    PstK_H7,
    PstK_A8,
    PstK_B8,
    PstK_C8,
    PstK_D8,
    PstK_E8,
    PstK_F8,
    PstK_G8,
    PstK_H8,

    MaterialPawn,
    MaterialKnight,
    MaterialBishop,
    MaterialRook,
    MaterialQueen,
}

// impl<T> std::ops::Index<Feature> for [T] {
//     type Output = T;
//     #[inline(always)]
//     fn index(&self, i: Feature) -> &Self::Output {
//         #[cfg(not(all(not(feature = "unchecked_indexing"), debug_assertions)))]
//         unsafe {
//             &self.get_unchecked(i.index())
//         }

//         #[cfg(all(not(feature = "unchecked_indexing"), debug_assertions))]
//         &self[(i.index())]
//     }
// }

// needs arrays to effectibely inline const array access
impl<const N: usize, T> std::ops::Index<Feature> for [T; N] {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: Feature) -> &Self::Output {
        #[cfg(not(all(not(feature = "unchecked_indexing"), debug_assertions)))]
        unsafe {
            &self.get_unchecked(i.index())
        }

        #[cfg(all(not(feature = "unchecked_indexing"), debug_assertions))]
        &self[i.index()]
    }
}

impl FeatureCategory {
    pub const fn index(&self) -> usize {
        *self as usize
    }

    pub const fn from_index(idx: usize) -> Self {
        match Self::from_repr(idx) {
            Some(f) => f,
            _ => panic!("FeatureCategory from index out of range"),
        }
    }

    pub fn name(&self) -> &'static str {
        self.into()
    }

    pub const fn len() -> usize {
        use strum::EnumCount;
        Self::COUNT
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        <Self as strum::IntoEnumIterator>::iter()
    }

    pub fn all_names() -> &'static [&'static str] {
        <Self as strum::VariantNames>::VARIANTS
    }

    pub fn from_name(name: &str) -> Self {
        use std::str::FromStr;
        Self::from_str(name).unwrap_or_else(|_| panic!("no FeatureCategory enum for '{name}'"))
    }

    pub fn all() -> Vec<Self> {
        Self::iter().collect()
    }
}

impl Feature {
    pub const fn index(&self) -> usize {
        *self as usize
    }

    // pub const fn from_index(idx: usize) -> Self {
    //     match Self::from_repr(idx) {
    //         Some(f) => f,
    //         _ => panic!("Feature from index out of range"),
    //     }
    // }

    pub const fn from_index(idx: usize) -> Self {
        const LOOKUP: [Feature; Feature::len()] = {
            let mut lookup = [Feature::MaterialPawn; Feature::len()];
            let mut i = 0;
            while i < Feature::len() {
                let opt_f = Feature::from_repr(i);
                match opt_f {
                    Some(f) => lookup[i] = f,
                    None => {
                        unreachable!();
                    }
                }
                i += 1;
            }
            lookup
        };
        LOOKUP[idx]
    }

    pub fn try_from_index(idx: usize) -> anyhow::Result<Self> {
        Self::from_repr(idx).ok_or_else(|| anyhow::anyhow!("index {idx} out of range for Feature"))
    }

    pub const fn index_pst(base: Feature, p: Piece, sq: Square) -> usize {
        base.index() + p.index() * Square::len() + sq.index()
    }

    pub fn category_string(&self) -> &str {
        use Feature::*;
        match *self {
            x if x <= Backward => "Pawn",
            x if x <= WinBonus => "Material",
            x if x <= QueenOpenFile => "Mobility",
            x if x <= QueenEarlyDevelop => "Position",
            x if x <= DiscoveredChecks => "Safety",
            x if x >= PstP_A1 && x <= PstK_H8 => "Pst",
            x if x >= MaterialPawn && x <= MaterialQueen => "Material",
            _ => "Tempo",
        }
    }

    pub fn is_pst(&self) -> bool {
        (Feature::PstP_A1.index()..=Feature::PstK_H8.index()).contains(&self.index())
    }

    pub const fn category(&self) -> FeatureCategory {
        const LOOKUP: [FeatureCategory; Feature::len()] = {
            use Feature::*;
            let mut lookup = [FeatureCategory::Initiative; Feature::len()];
            let mut f = 0;
            while f < Feature::len() {
                lookup[f] = match f {
                    x if x <= Backward.index() => FeatureCategory::Pawns,
                    x if x <= WinBonus.index() => FeatureCategory::Imbalance,
                    x if x <= QueenOpenFile.index() => FeatureCategory::Mobility,
                    x if x <= QueenEarlyDevelop.index() => FeatureCategory::Threats,
                    x if x <= DiscoveredChecks.index() => FeatureCategory::KingSafety,
                    x if x >= PstP_A1.index() && x <= PstK_H8.index() => FeatureCategory::Mobility,
                    x if x >= MaterialPawn.index() && x <= MaterialQueen.index() => {
                        FeatureCategory::Material
                    }
                    _ => FeatureCategory::Initiative,
                };
                f += 1;
            }
            lookup
        };
        LOOKUP[self.index()]
    }
    // pub const fn len_pst() -> isize {
    //     (Piece::len() * Square::len()) as isize
    // }

    #[must_use]
    pub const fn len() -> usize {
        use strum::EnumCount;
        Self::COUNT
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        <Self as strum::IntoEnumIterator>::iter()
    }

    #[must_use]
    pub fn all() -> Vec<Feature> {
        Self::iter().collect()
    }

    #[must_use]
    pub fn from_name(name: &str) -> Self {
        use std::str::FromStr;
        Self::from_str(name).unwrap_or_else(|_| panic!("no Feature enum for '{name}'"))
    }

    pub fn try_from_name(name: &str) -> anyhow::Result<Self> {
        use std::str::FromStr;
        Ok(Self::from_str(name)?)
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        self.into()
    }
}

// #[derive(
//     Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord, Debug, IntoStaticStr, FromRepr, EnumCount,
// )]
// pub enum PhaseKind {
//     Start = 0,
//     End   = 1,
// }

// pub struct FeaturePhase(pub Feature, pub u8);

// impl FeaturePhase {
//     pub const fn index(&self) -> usize {
//         self.0.index() * PhaseKind::COUNT + self.1 as usize
//     }

//     pub fn name(&self) -> String {
//         self.0.name().to_string() + [".s", ".e"][self.1 as usize]
//     }
// }

#[cfg(test)]
mod tests_feature {
    use super::*;
    use crate::test_log::test;

    #[test]
    fn test_feature_basics() {
        use Feature::*;
        use FeatureCategory::*;
        assert_eq!(CenterAttacks.name(), "center_attacks");
        assert_eq!(Feature::from_index(CenterAttacks.index()), CenterAttacks);
        assert_eq!(Feature::from_name("center_attacks"), CenterAttacks);
        assert_eq!(Feature::try_from_name("invalid").is_err(), true);
        assert_eq!(Feature::try_from_index(10000).is_err(), true);
        assert_eq!(TropismD2.index() - TropismD1.index(), 1);

        assert_eq!(KingSafety.name(), "King safety");
        assert_eq!(Initiative.name(), "Initiative");

        pub fn test_index_pst(base: Feature, p: Piece, sq: Square) -> usize {
            let index = base.index() + p.index() * Square::len() + sq.index();
            debug_assert!(
                index < Feature::len(),
                "index {index} > len {len} for base {base} p {p} sq {sq}",
                index = index,
                len = Feature::len(),
                base = base.index(),
                p = p.index(),
                sq = sq.index()
            );
            index
        }

        for &p in &Piece::ALL {
            for sq in Square::all() {
                assert!(test_index_pst(Feature::PstP_A1, p, sq) > 0);
                let _f = Feature::from_index(Feature::index_pst(Feature::PstP_A1, p, sq));
            }
        }
    }
}
