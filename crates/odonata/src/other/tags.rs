use crate::{
    bits::bitboard::Bitboard,
    boards::Board,
    infra::utils::Formatting,
    movelist::{MoveList, ScoredMoveList},
    mv::Move,
    piece::Ply,
    variation::{MultiVariation, Variation},
};
use anyhow::{anyhow, Context, Result};
use indexmap::map::IndexMap;
use indexmap::indexmap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
// use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, time::Duration};

/// to support EPD and PGN formats
#[derive(Default, Clone, PartialEq)]
pub struct Tags {
    // consider: #[serde(skip_serializing_if = "Option::is_none")]
    pub eco:                          Option<String>,
    pub avoid_moves:                  Option<Box<MoveList>>,
    pub best_moves:                   Option<Box<MoveList>>,
    pub ann_scored_moves:             Option<ScoredMoveList>,
    pub eng_scored_moves:             Option<ScoredMoveList>,
    pub branching_factor:             Option<f64>,
    pub pv:                           Option<Variation>,
    pub multi_pv:                     Option<MultiVariation>,
    pub id:                           Option<String>,
    pub acd:                          Option<Ply>,  /// analysis count: depth
    pub annotator_depth:              Option<Ply>,
    pub analysis_count_sel_depth:     Option<Ply>,
    pub acn:                          Option<u64>,  /// analysis count: nodes
    pub analysis_count_seconds:       Option<u32>,
    pub analysis_count_milli_seconds: Option<u64>,
    pub chess_clock:                  Option<Duration>,
    pub centipawn_evaluation:         Option<i32>,
    pub centipawn_loss:               Option<i32>,
    pub direct_mate:                  Option<u32>,
    pub full_move_number:             Option<u32>,
    pub game_move:                    Option<Move>,
    pub half_move_clock:              Option<u32>,
    pub predicted_move:               Option<Move>,
    pub repetition_count:             Option<u32>,
    pub result:                       Option<String>,
    pub no_op:                        Option<String>,
    pub supplied_move:                Option<Move>,
    pub sv:                           Option<Variation>,
    pub squares:                      Option<Bitboard>,
    pub timestamp:                    Option<String>,
    pub perfts:                       [Option<u64>; 8],
    pub comments:                     [Option<String>; 10],
}

impl fmt::Debug for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Tags2")
            .field("uci", &self.to_uci())
            .finish()
    }
}

impl fmt::Display for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_uci())
    }
}

impl Tags {
    pub const ECO: &'static str = "eco";
    pub const AM: &'static str = "am";
    pub const BM: &'static str = "bm";
    pub const ASM: &'static str = "Asm"; // ann scored moves "Asm a4:+45 Nf6:-56;" 
    pub const ESM: &'static str = "Esm"; // eng scored moves "Esm a4:+45 Nf6:-56;"
    pub const BF: &'static str = "Bf";
    pub const PV: &'static str = "pv";
    pub const MPV: &'static str = "Mpv";
    pub const ID: &'static str = "id";
    pub const ACD: &'static str = "acd"; 
    pub const AD: &'static str = "Ad";
    pub const ACSD: &'static str = "acsd";
    pub const ACN: &'static str = "acn";
    pub const ACS: &'static str = "acs";
    pub const ACMS: &'static str = "Acms";
    pub const CC: &'static str = "cc";
    pub const CE: &'static str = "ce";
    pub const CPL: &'static str = "Cpl"; // centipawn loss
    pub const C9: &'static str = "c9";
    pub const DM: &'static str = "dm";
    pub const FMVN: &'static str = "fmvn";
    pub const HMVC: &'static str = "hmvc";
    pub const PM: &'static str = "pm";
    pub const RC: &'static str = "rc";
    pub const NOOP: &'static str = "noop";
    pub const SM: &'static str = "sm";
    pub const SV: &'static str = "sv";
    pub const GM: &'static str = "Gm"; // Game move
    pub const SQ: &'static str = "Sq";
    pub const RES: &'static str = "Res";
    pub const TS: &'static str = "ts";
    pub const COMMENTS: [&'static str; 10] =
        ["c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9"];
    pub const PERFTS: [&'static str; 8] = ["D0", "D1", "D2", "D3", "D4", "D5", "D6", "D7"];
    pub const DRAW_REJECT: &'static str = "draw_reject";

    pub const ATTRIBUTES: &'static [&'static str] = &[Self::ACD, Self::BM, Self::PV];
    pub const ALL_TAGS: &'static [&'static str] = &[
        Self::ACD,
        Self::ACN,
        Self::ACS,
        Self::ACMS,
        Self::ACSD,
        Self::BF,
        Self::BM,
        Self::ASM,
        Self::CE,
        Self::PV,
        Self::SM,
    ];

    pub const ENGINE_RESULTS: &'static [&'static str] = &[
        Self::PV,
        Self::MPV,
        Self::SM,
        Self::BM,
        Self::CE,
        Self::BF,
        Self::CPL,
    ];

    pub const ANNOTATOR_RESULTS: &'static [&'static str] = &[Self::ASM, Self::CPL];

    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn keys(&self) -> Vec<&'static str> {
        self.as_hash_map_uci().keys().cloned().collect_vec()
    }

    pub fn calculate_without_engine(&mut self, desired: &[&str]) {
        // MPV => PV
        if desired.contains(&Tags::PV) && self.pv.is_none() {
            if let Some(mpv) = &self.multi_pv {
                self.pv = mpv.first().map(|vs| vs.var);
            }
        }

        // PV => BM
        if desired.contains(&Tags::BM) && self.best_moves.is_none() {
            if let Some(pv) = &self.pv {
                if let Some(mv) = pv.first() {
                    self.best_moves = Some(Box::new(MoveList::from([mv])))
                }
            }
        }

        // ASM => BM
        if desired.contains(&Tags::BM) && self.best_moves.is_none() {
            if let Some(asm) = &self.ann_scored_moves {
                if let Some(mv) = asm.best_move() {
                    self.best_moves = Some(Box::new(MoveList::from([mv])))
                }
            }
        }

        // BM & ASM => CPL
        if desired.contains(&Tags::CPL) && self.centipawn_loss.is_none() {
            if let Some(asm) = &self.ann_scored_moves {
                if let Some(bm) = &self.best_moves {
                    if let Some(&bm1) = bm.iter().nth(0) {
                        self.centipawn_loss = asm.centipawn_loss(bm1).map(|sc| sc.as_i16() as i32);
                    }
                }
            }
        }
    }

    pub fn requires_engine(current: &[&str], desired: &[&str]) -> bool {
        for &key in desired.iter().filter(|t| !current.contains(&t)) {
            // if can be derived - dont worry
            if key == Tags::CE && current.contains(&Tags::ESM)
                || key == Tags::BM && current.contains(&Tags::PV)
                || key == Tags::ACS && current.contains(&Tags::ACMS)
                || key == Tags::SM && current.contains(&Tags::PV)
                || key == Tags::SM && current.contains(&Tags::BM)
                || key == Tags::PV && current.contains(&Tags::MPV)
                || key == Tags::BM && current.contains(&Tags::MPV)
                || key == Tags::SM && current.contains(&Tags::MPV)
                || key == Tags::CPL && current.contains(&Tags::BM) && current.contains(&Tags::ESM)
            {
                continue;
            }

            if Self::ENGINE_RESULTS.contains(&key) {
                return true;
            }
            if Self::ANNOTATOR_RESULTS.contains(&key) {
                return true;
            }
        }
        false
    }

    /// filters on key
    pub fn filter(self, f: impl Fn(&str) -> bool) -> Tags {
        let mut t = self;
        t.annotator_depth = t.annotator_depth.filter(|_| f("Ad"));
        t.analysis_count_milli_seconds = t.analysis_count_milli_seconds.filter(|_| f("Acms"));
        t.branching_factor = t.branching_factor.filter(|_| f("Bf"));
        t.ann_scored_moves = t.ann_scored_moves.filter(|_| f("Asm"));
        t.eng_scored_moves = t.eng_scored_moves.filter(|_| f("Esm"));
        t.multi_pv = t.multi_pv.filter(|_| f("Mpv"));
        t.result = t.result.filter(|_| f("Res"));
        t.squares = t.squares.filter(|_| f("Sq"));
        t.avoid_moves = t.avoid_moves.filter(|_| f("am"));
        t.acd = t.acd.filter(|_| f("acd"));
        t.acn = t.acn.filter(|_| f("acn"));
        t.analysis_count_sel_depth = t.analysis_count_sel_depth.filter(|_| f("acsd"));
        t.analysis_count_seconds = t.analysis_count_seconds.filter(|_| f("acs"));
        t.best_moves = t.best_moves.filter(|_| f("bm"));
        t.chess_clock = t.chess_clock.filter(|_| f("cc"));
        t.centipawn_evaluation = t.centipawn_evaluation.filter(|_| f("ce"));
        t.centipawn_loss = t.centipawn_loss.filter(|_| f("Cpl"));
        t.direct_mate = t.direct_mate.filter(|_| f("dm"));
        t.eco = t.eco.filter(|_| f("eco"));
        t.full_move_number = t.full_move_number.filter(|_| f("fmvn"));
        t.game_move = t.game_move.filter(|_| f("Gm"));
        t.half_move_clock = t.half_move_clock.filter(|_| f("hmvc"));
        t.id = t.id.filter(|_| f("id"));
        t.predicted_move = t.predicted_move.filter(|_| f("pm"));
        t.pv = t.pv.filter(|_| f("pv"));
        t.repetition_count = t.repetition_count.filter(|_| f("rc"));
        t.no_op = t.no_op.filter(|_| f("noop"));
        t.supplied_move = t.supplied_move.filter(|_| f("sm"));
        t.sv = t.sv.filter(|_| f("sv"));
        t.timestamp = t.timestamp.filter(|_| f("ts"));
        (0..t.perfts.len()).for_each(|i| t.perfts[i] = t.perfts[i].filter(|_| f(Self::PERFTS[i])));
        (0..t.comments.len())
            .for_each(|i| t.comments[i] = t.comments[i].clone().filter(|_| f(Self::COMMENTS[i])));
        t
    }

    pub fn parse_epd_tags(board: &Board, tags_str: &str) -> Result<Tags> {
        let mut tags = Tags::new();
        let ops: Vec<&str> = Self::split_into_tags(tags_str);
        for op in ops {
            let words: Vec<&str> = Tags::split_into_words(op);
            debug_assert!(
                !words.is_empty(),
                "no words parsing EPD operation '{}' from '{}'",
                op,
                tags_str
            );
            let tag = Tags::parse_single_tag(board, words.join(" ").as_str())?;
            tags.fill_from(tag);
        }
        Ok(tags)
    }

    pub fn parse_single_tag(b: &Board, v: &str) -> anyhow::Result<Tags> {
        let words: Vec<&str> = Self::split_into_words(v);
        match Self::parse_key_and_tag(b, words[0], words[1..].join(" ").as_str()) {
            Err(err) => Err(anyhow!("{} parsing tag '{}' from '{}'", err, words[0], v)),
            Ok(tag) => Ok(tag),
        }
    }
    fn parse_key_and_tag(b: &Board, key: &str, v: &str) -> anyhow::Result<Tags> {
        let mut tags = Tags::default();
        match key {
            "eco" => tags.eco = Some(v.to_string()),
            "am" => tags.avoid_moves = Some(b.parse_san_movelist(v)?.into()),
            "bm" => tags.best_moves = Some(b.parse_san_movelist(v)?.into()),
            "Asm" | "Bsm" => tags.ann_scored_moves = Some(ScoredMoveList::parse_san(v, b)?),
            "Esm" => tags.eng_scored_moves = Some(ScoredMoveList::parse_san(v, b)?),
            "Bf" => tags.branching_factor = Some(v.parse::<f64>()?),
            "pv" => tags.pv = Some(b.parse_san_variation(v)?),
            "Mpv" => tags.multi_pv = Some(MultiVariation::parse_san(v, b)?),
            "id" => tags.id = Some(v.to_string()),
            "acd" => tags.acd = Some(v.parse::<Ply>()?),
            "Ad" => tags.annotator_depth = Some(v.parse::<Ply>()?),
            "acsd" => tags.analysis_count_sel_depth = Some(v.parse::<Ply>()?),
            "acn" => tags.acn = Some(v.parse::<u64>()?),
            "acs" => tags.analysis_count_seconds = Some(v.parse::<u32>()?),
            "Acms" => tags.analysis_count_milli_seconds = Some(v.parse::<u64>()?),
            "cc" => tags.chess_clock = Some(Duration::new(0, 0)),
            "ce" => tags.centipawn_evaluation = Some(v.parse::<i32>()?),
            "Cpl" => tags.centipawn_loss = Some(v.parse::<i32>()?),
            "dm" => tags.direct_mate = Some(v.parse::<u32>()?),
            "fmvn" => tags.full_move_number = Some(v.parse::<u32>()?),
            "Gm" => tags.game_move = Some(b.parse_san_move(v)?),
            "hmvc" => tags.half_move_clock = Some(v.parse::<u32>()?),
            "pm" => tags.predicted_move = Some(b.parse_san_move(v)?),
            "rc" => tags.repetition_count = Some(v.parse::<u32>()?),
            "Res" => tags.result = Some(v.to_string()),
            "noop" => tags.no_op = Some(v.to_string()),
            "sm" => tags.supplied_move = Some(b.parse_san_move(v)?),
            "sv" => tags.sv = Some(b.parse_san_variation(v)?),
            "Sq" => tags.squares = Some(Bitboard::parse_squares(v)?),
            "ts" => tags.timestamp = Some(v.to_string()),
            _ if key.starts_with('D') => {
                tags.perfts[key[1..].parse::<usize>()?] = Some(v.parse::<u64>()?)
            }
            _ if key.starts_with('c') => {
                tags.comments[key[1..].parse::<usize>()?] = Some(v.to_string())
            }
            _ => anyhow::bail!("unknown tag {key}"),
        }
        Ok(tags)
    }

    /// like Option.or()
    /// combines two tags with shortcircuit
    /// i.e. other.field only used if self.field == None
    pub fn fill_from(&mut self, mut other: Tags) {
        self.eco = self.eco.take().or(other.eco);
        self.avoid_moves = self.avoid_moves.take().or(other.avoid_moves);
        self.best_moves = self.best_moves.take().or(other.best_moves);
        self.ann_scored_moves = self.ann_scored_moves.take().or(other.ann_scored_moves);
        self.eng_scored_moves = self.eng_scored_moves.take().or(other.eng_scored_moves);
        self.branching_factor = self.branching_factor.take().or(other.branching_factor);
        self.pv = self.pv.take().or(other.pv);
        self.multi_pv = self.multi_pv.take().or(other.multi_pv);
        self.id = self.id.take().or(other.id);
        self.acd = self.acd.take().or(other.acd);
        self.annotator_depth = self.annotator_depth.take().or(other.annotator_depth);
        self.analysis_count_sel_depth = self
            .analysis_count_sel_depth
            .take()
            .or(other.analysis_count_sel_depth);
        self.acn = self.acn.take().or(other.acn);
        self.analysis_count_seconds = self
            .analysis_count_seconds
            .take()
            .or(other.analysis_count_seconds);
        self.analysis_count_milli_seconds = self
            .analysis_count_milli_seconds
            .take()
            .or(other.analysis_count_milli_seconds);
        self.chess_clock = self.chess_clock.take().or(other.chess_clock);
        self.centipawn_evaluation = self
            .centipawn_evaluation
            .take()
            .or(other.centipawn_evaluation);
        self.centipawn_loss = self.centipawn_loss.take().or(other.centipawn_loss);
        self.direct_mate = self.direct_mate.take().or(other.direct_mate);
        self.full_move_number = self.full_move_number.take().or(other.full_move_number);
        self.game_move = self.game_move.take().or(other.game_move);
        self.half_move_clock = self.half_move_clock.take().or(other.half_move_clock);
        self.predicted_move = self.predicted_move.take().or(other.predicted_move);
        self.repetition_count = self.repetition_count.take().or(other.repetition_count);
        self.result = self.result.take().or(other.result);
        self.no_op = self.no_op.take().or(other.no_op);
        self.supplied_move = self.supplied_move.take().or(other.supplied_move);
        self.sv = self.sv.take().or(other.sv);
        self.squares = self.squares.take().or(other.squares);
        self.timestamp = self.timestamp.take().or(other.timestamp);
        (0..self.perfts.len())
            .for_each(|i| self.perfts[i] = self.perfts[i].take().or(other.perfts[i].take()));
        (0..self.comments.len())
            .for_each(|i| self.comments[i] = self.comments[i].take().or(other.comments[i].take()));
    }

    /// convert tag fields into their annotator equivalent
    pub fn annotator_equivalents(fields: &[&'static str]) -> Vec<&'static str> {
        fields
            .iter()
            .map(|&s| match s {
                "Asm" => "Esm",
                "sv" => "pv",
                "sm" => "bm",
                "acd" => "Ad",
                x => x,
            })
            .collect()
        // Tags {
        //     annotator_depth: self.acd,
        //     sv: self.pv,
        //     ann_scored_moves: self.eng_scored_moves,
        //     multi_pv: self.multi_pv,
        //     ..Self::default()
        // }
    }

    pub fn validate(&self, b: &Board) -> anyhow::Result<()> {
        // self.annotator_depth.as_ref().map(|v| v.validate()).transpose().with_context(|| "Ad")?;
        //  self.analysis_count_milli_seconds.as_ref().map(|v| v.validate()).transpose().with_context(|| "Acms")?;
        //  self.branching_factor.as_ref().map(|v| v.validate()).transpose().with_context(|| "Bf")?;
        self.ann_scored_moves
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .with_context(|| "Asm")?;
        self.eng_scored_moves
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .with_context(|| "Esm")?;
        self.multi_pv
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .with_context(|| "Mpv")?;
        // self.result.as_ref().map(|v| v.validate()).transpose().with_context(|| "Res")?;
        // self.squares.as_ref().map(|v| v.validate()).transpose().with_context(|| "Sq")?;
        self.avoid_moves
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .with_context(|| "am")?;
        //  self.acd.as_ref().map(|v| v.validate()).transpose().with_context(|| "acd")?;
        //  self.analysis_count_nodes.as_ref().map(|v| v.validate()).transpose().with_context(|| "acn")?;
        //  self.analysis_count_sel_depth.as_ref().map(|v| v.validate()).transpose().with_context(|| "acsd")?;
        //  self.analysis_count_seconds.as_ref().map(|v| v.validate()).transpose().with_context(|| "acs")?;
        self.best_moves
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .with_context(|| "bm")?;
        //  self.chess_clock.as_ref().map(|v| v.validate()).transpose().with_context(|| "cc")?;
        //  self.centipawn_evaluation.as_ref().map(|v| v.validate()).transpose().with_context(|| "ce")?;
        //  self.direct_mate.as_ref().map(|v| v.validate()).transpose().with_context(|| "dm")?;
        //  self.eco.as_ref().map(|v| v.validate()).transpose().with_context(|| "eco")?;
        //  self.full_move_number.as_ref().map(|v| v.validate()).transpose().with_context(|| "fmvn")?;
        //  self.half_move_clock.as_ref().map(|v| v.validate()).transpose().with_context(|| "hmvc")?;
        //  self.id.as_ref().map(|v| v.validate()).transpose().with_context(|| "id")?;
        self.predicted_move
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .map_err(anyhow::Error::msg)
            .with_context(|| "pm")?;
        self.pv
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .with_context(|| "pv")?;
        //  self.repetition_count.as_ref().map(|v| v.validate()).transpose().with_context(|| "rc")?;
        //  self.no_op.as_ref().map(|v| v.validate()).transpose().with_context(|| "noop")?;
        self.supplied_move
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .map_err(anyhow::Error::msg)
            .with_context(|| "sm")?;

        self.game_move
            .as_ref()
            .map(|v| v.validate(b))
            .transpose()
            .map_err(anyhow::Error::msg)
            .with_context(|| "Gm")?;
        // we dont validate sv as it is used to reach the board state
        // self.sv
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .with_context(|| "sv")?;
        //  self.timestamp.as_ref().map(|v| v.validate()).transpose().with_context(|| "ts")?;

        Ok(())
    }

    // pub fn take_first(&mut self) -> Option<Tags2> {
    //     let mut res = Self::default();
    //     match self {
    //         // s if s.pv.is_some() => res.pv = s.pv.take(),
    //         Self {
    //             sv: v @ Some(_), ..
    //         } => res.sv = v.take(),
    //         Self {
    //             pv: v @ Some(_), ..
    //         } => res.pv = v.take(),
    //         // s if s.supplied_variation.is_some() => res.supplied_variation = s.supplied_variation.take(),
    //         _ => return None,
    //     };
    //     Some(res)
    // }

    // pub fn take_first_key(&mut self) -> Option<&str> {
    //     match self {
    //         s if s.pv.is_some() => Some("pv"),
    //         Self { pv: Some(_), .. } => Some("pv"),
    //         _ => return None,
    //     }
    // }
    pub fn to_hash_map(self, b: &Board) -> HashMap<String, String> {
        self.as_hash_map_san(b)
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    pub fn as_hash_map_san(&self, b: &Board) -> IndexMap<&'static str, String> {
        let san_map: IndexMap<&str, Option<String>> = indexmap! {
            "pv" => self.pv.as_ref().map(|v| v.to_san(b)),
            "Asm" => self.ann_scored_moves.as_ref().map(|v| v.to_san(b)),
            "sv" => self.sv.as_ref().map(|v| v.to_san(b)),
            "Mpv" => self.multi_pv.as_ref().map(|v| v.to_san(b)),
            "Gm" => self.game_move.as_ref().map(|mv| mv.to_san(b)),
            "Esm" => self.eng_scored_moves.as_ref().map(|v| v.to_san(b)),
            "bm" => self.best_moves.as_ref().map(|v| MoveList::from(*v.clone()).to_san(b)),
            "pm" => self.predicted_move.as_ref().map(|mv| mv.to_san(b)),
            "sm" => self.supplied_move.as_ref().map(|mv| mv.to_san(b)),
        };
        let san_map: IndexMap<&str, String> = san_map
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect();

        let mut map = self.as_hash_map_uci();
        map.extend(san_map);
        map
    }

    #[must_use]
    pub fn as_hash_map_uci(&self) -> IndexMap<&'static str, String> {
        let mut map: IndexMap<&str, Option<String>> = indexmap! {
            "Ad" => self.annotator_depth.as_ref().map(|n| format!("{}", n)),
            "Acms" => self.analysis_count_milli_seconds.as_ref().map(|n| format!("{n}")),
            "Bf" => self.branching_factor.as_ref().map(|bf| Formatting::decimal(2, *bf)),
            "Asm" => self.ann_scored_moves.as_ref().map(|v| v.to_uci()),
            "Esm" => self.eng_scored_moves.as_ref().map(|v| v.to_uci()),
            "Mpv" => self.multi_pv.as_ref().map(|v| v.to_uci()),
            "Res" => self.result.as_ref().map(|s| format!("{s}")),
            "Sq" => self.squares.as_ref().map(|bb| bb.uci()),
            "am" => self.avoid_moves.as_ref().map(|v| MoveList::from(*v.clone()).to_uci()),
            "acd" => self.acd.as_ref().map(|n| format!("{n}")),
            "acn" => self.acn.as_ref().map(|n| format!("{n}")),
            "acsd" => self.analysis_count_sel_depth.as_ref().map(|n| format!("{n}")),
            "acs" => self.analysis_count_seconds.as_ref().map(|n| format!("{n}")),
            "bm" => self.best_moves.as_ref().map(|v| MoveList::from(*v.clone()).to_uci()),
            "cc" => self.chess_clock.as_ref().map(|_| format!("{}", "na")), // FIXME: chess clock tag
            "ce" => self.centipawn_evaluation.as_ref().map(|sc| sc.to_string()),
            "Cpl" => self.centipawn_loss.as_ref().map(|sc| sc.to_string()),
            "dm" => self.direct_mate.as_ref().map(|n| format!("{n}")),
            "eco" => self.eco.as_ref().map(ToString::to_string),
            "fmvn" => self.full_move_number.as_ref().map(|n| format!("{n}")),
            "Gm" => self.game_move.as_ref().map(Move::to_uci),
            "hmvc" => self.half_move_clock.as_ref().map(|n| format!("{n}")),
            "id" => self.id.as_ref().map(|id| format!("{id}")),
            "pm" => self.predicted_move.as_ref().map(|mv| mv.to_uci()),
            "pv" => self.pv.as_ref().map(Variation::to_uci),
            "rc" => self.repetition_count.as_ref().map(|n| format!("{n}")),
            "noop" => self.no_op.as_ref().map(|_| format!("{}", "")),
            "sm" => self.supplied_move.as_ref().map(|mv| mv.to_uci()),
            "sv" => self.sv.as_ref().map(|mvs| mvs.to_uci()),
            "ts" => self.timestamp.as_ref().map(|v| format!("{v}")),
        };
        for i in 0..self.perfts.len() {
            map.insert(Self::PERFTS[i], self.perfts[i].map(|v| v.to_string()));
        }
        for i in 0..self.comments.len() {
            map.insert(Self::COMMENTS[i], self.comments[i].clone());
        }
        map.into_iter()
            .filter_map(|(k, v)| match v {
                Some(v) => Some((k, v)),
                _ => None,
            })
            .collect()
    }

    pub fn to_pgn(&self, b: &Board) -> String {
        let mut strings = vec![];
        for (k, v) in self.as_hash_map_san(b).iter() {
            if v.is_empty() {
                strings.push(format!("[%{k}]"));
            } else if v.contains(char::is_whitespace) {
                strings.push(format!("[%{k} \"{v}\"]"));
            } else {
                strings.push(format!("[%{k} {v}]"));
            }
        }
        strings.join(" ")
    }

    pub fn to_epd(&self, b: &Board) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for (k, v) in self.as_hash_map_san(b).iter() {
            if v.is_empty() {
                write!(s, " {};", k).unwrap();
            } else if v.contains(char::is_whitespace) {
                write!(s, " {} \"{}\";", k, v).unwrap();
            } else {
                write!(s, " {} {};", k, v).unwrap();
            }
        }
        s
    }

    pub fn to_uci(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for (k, v) in self.as_hash_map_uci().iter() {
            if v.is_empty() {
                write!(s, " {};", k).unwrap();
            } else if v.contains(char::is_whitespace) {
                write!(s, " {} \"{}\";", k, v).unwrap();
            } else {
                write!(s, " {} {};", k, v).unwrap();
            }
        }
        s
    }
    fn split_into_words(s: &str) -> Vec<&str> {
        REGEX_SPLIT_WORDS
            .captures_iter(s)
            .map(|cap| {
                cap.get(1)
                    .or_else(|| cap.get(2))
                    .or_else(|| cap.get(3))
                    .unwrap()
                    .as_str()
            })
            .collect()
    }

    fn split_into_tags(s: &str) -> Vec<&str> {
        REGEX_SPLIT_TAGS
            .captures_iter(s)
            .map(|cap| {
                cap.get(1)
                    .or_else(|| cap.get(2))
                    .or_else(|| cap.get(3))
                    .unwrap()
                    .as_str()
            })
            .collect()
    }
}

static REGEX_SPLIT_TAGS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)
        ([^";]*  
            " [^"]* "   # a quoted string (possibly containing ";")
        [^";]*
        );
        |
        ([^';]*  
            ' [^']* '   # a quoted string (possibly containing ";")
        [^';]*
        );
        |
        ([^;"']+)        # an opcode and operand(s) without any quotes 
        ;
        "#,
    )
    .unwrap()
});

static REGEX_SPLIT_WORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)
        (?:
            [^"\s]*  
            " ([^"]*) "    # a double quoted string (possibly containing whitespace)
            [^"\s]*
        )(?:$|\s)|
        (?:
            [^'\s]*  
            ' ([^']*) '    # a single quoted string (possibly containing whitespace)
            [^'\s]*
        )(?:$|\s)
        |
        ([^\s"']+)        # an opcode/operand without any quotes 
        (?:$|\s)"#,
    )
    .unwrap()
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use indexmap::map::IndexMap;
    use itertools::assert_equal;
    // use serde_json::{value::Value, Map};
    use std::mem::size_of;
    use test_log::test;

    #[test]
    fn tags_basics() {
        let b = &Board::starting_pos();
        let mut tags = Tags::new();
        tags.pv = Some("e4 e5 d4 b8c6".var(b));
        tags.sv = Some("e4".var(b));
        tags.acd = Some(3);
        tags.eco = Some("".to_string());
        tags.id = None;
        let map2: IndexMap<&str, String> = indexmap! {
            "acd" => "3".to_string(),
            "pv" => "e2e4 e7e5 d2d4 b8c6".to_string(),
            "eco" => "".to_string(),
            "sv" => "e2e4".to_string(),
        };
        let map1 = tags.as_hash_map_uci();
        assert_eq!(map1, map2); // contents match
        assert!(map1.iter().ne(map2.iter())); // ordering different

        let map3: IndexMap<&str, String> = indexmap! {
            "acd" => "3".to_string(),
            "eco" => "".to_string(),
            "pv" => "e2e4 e7e5 d2d4 b8c6".to_string(),
            "sv" => "e2e4".to_string(),
        };
        assert_equal(map1.iter(), map3.iter()); // ordering same

        // UCI - san format
        assert_eq!(
            tags.to_uci(),
            " acd 3; eco; pv \"e2e4 e7e5 d2d4 b8c6\"; sv e2e4;"
        );

        // HashMap - san format
        let map1s = tags.as_hash_map_san(b);
        let map3s: IndexMap<&str, String> = indexmap! {
            "acd" => "3".to_string(),
            "eco" => "".to_string(),
            "pv" => "e4 e5 d4 Nc6".to_string(),
            "sv" => "e4".to_string(),
        };
        assert_equal(map1s.iter(), map3s.iter()); // ordering same

        // PGN - san format
        assert_eq!(
            tags.to_pgn(b),
            "[%acd 3] [%eco] [%pv \"e4 e5 d4 Nc6\"] [%sv e4]"
        );

        // EPD - san format
        assert_eq!(tags.to_epd(b), " acd 3; eco; pv \"e4 e5 d4 Nc6\"; sv e4;");

        let pred = |s: &str| ["acd", "Asm", "sv", "c1"].contains(&s);
        assert_eq!(tags.clone().filter(pred).to_epd(b), " acd 3; sv e4;");
        assert_eq!(tags.clone().filter(|_| false).to_epd(b), "");

        tags.comments[0] = Some("0".to_string());
        tags.comments[1] = Some("1".to_string());
        assert_eq!(tags.clone().filter(pred).comments[0], None);
        assert_eq!(tags.clone().filter(pred).comments[1], Some("1".to_string()));
        assert_eq!(tags.clone().filter(pred).to_epd(b), " acd 3; sv e4; c1 1;");
        // JSON
        // FIXME: json parsing of Tags - failing due to nulls
        // assert_eq!(
        //     jsonrpc_core::to_string(&tags).unwrap(),
        //     r#"{"acd":"3", "eco":"", "pv":"e4 e5 d4 Nc6", "sv":"e4"}"#
        // );
    }

    // #[test]
    fn test_tags_memory() {
        let mut tags = Tags::default();
        tags.analysis_count_seconds = Some(4);
        tags.acd = Some(3);
        tags.comments[0] = Some(String::from("Hello World2"));
        tags.id = Some(String::from("Hello World"));
        let _vec = vec![tags; 10_000_000];
        std::thread::sleep(Duration::from_secs_f32(100.0));
    }

    #[test]
    fn test_parsing_tags() {
        let b = &Board::default();

        let tags = Tags::parse_single_tag(b, "acd 3").unwrap();
        assert_eq!(tags, Tags {
            acd: Some(3),
            ..Tags::default()
        });

        let mut tags = Tags::new();
        tags.comments[0] = Some("Hello World".to_string());
        assert_eq!(
            Tags::parse_key_and_tag(b, "c0", "Hello World").unwrap(),
            tags
        );
    }

    #[test]
    fn test_split_words() {
        let vec = Tags::split_into_words(r#"bm e4"#);
        assert_eq!(vec, vec!["bm", "e4"]);

        let vec = Tags::split_into_words(r#"id "my name is bob""#);
        assert_eq!(vec, vec!["id", "my name is bob"]);

        let vec = Tags::split_into_words(r#"id 'my name is bob'"#);
        assert_eq!(vec, vec!["id", "my name is bob"]);
    }

    #[test]
    fn test_split_into_tags() {
        let vec = Tags::split_into_tags(r#"cat"meo;w";"mouse";"toad;;;;;;" ;zebra;"#);
        assert_eq!(vec, vec![
            "cat\"meo;w\"",
            "\"mouse\"",
            "\"toad;;;;;;\" ",
            "zebra"
        ]);

        let vec = Tags::split_into_tags(r#"cat'meo;w';'mouse';'toad;;;;;;' ;zebra;"#);
        assert_eq!(vec, vec![
            "cat\'meo;w\'",
            "\'mouse\'",
            "\'toad;;;;;;\' ",
            "zebra"
        ]);

        let vec = Tags::split_into_tags(r#";cat;mouse;toad;;;;;;sheep;zebra"#);
        assert_eq!(vec, vec!["cat", "mouse", "toad", "sheep"]);

        // FIXME! OK, but not desirable (unmatched quote parsing)
        let vec = Tags::split_into_tags(r#";ca"t;mouse;"#);
        assert_eq!(vec, vec!["t", "mouse"]);
        // let vec = split_on_regex("cat;mat;sat;");
        // assert_eq!(vec, vec!["cat;", "mat;", "sat;"], "cat;mat;sat;");
        // let vec = split_on_regex("cat \"hello\";mat;sat;");
        // assert_eq!(vec, vec!["cat \"hello\";", "mat;", "sat;"], "cat;mat;sat;");
    }

    // #[ignore]
    // #[test]
    // fn tags_x() {
    //     let mut tags = Tags::default();
    //     tags.result = Some("Hello Word".to_owned());
    //     let value = serde_json::to_value(tags).unwrap();
    //     dbg!(std::mem::size_of_val(&value));
    //     dbg!(&value);
    //     if let Value::Object(map) = &value {
    //         dbg!(map);
    //     }

    //     let mut map = Map::new();
    //     map.insert(
    //         "result".to_owned(),
    //         Value::String("Hello World2".to_owned()),
    //     );

    //     let tags2: Tags = serde_json::from_value(Value::Object(map)).unwrap();
    //     dbg!(tags2);
    // }

    fn compare(s1: usize, s2: usize, s: &str) {
        if s1 != s2 {
            warn!("{s} actual {s1} != expected {s2}");
        }
    }

    // RUST_LOG=warn cargo t test_tag_sizes --nocapture
    #[test]
    fn test_tag_sizes() {
        warn!("Testing sizes...");
        compare(size_of::<Tags>(), 432, "TagUnion");
    }
}
