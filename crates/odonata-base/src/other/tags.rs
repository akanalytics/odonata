use std::collections::HashMap;
use std::fmt::{self, Display};

use anyhow::{Context as _, Result};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::domain::score::Score;
use crate::domain::wdl::WdlOutcome;
use crate::movelist::ScoredMoveList;
use crate::prelude::{Board, Move};
use crate::variation::Variation;
use crate::{Epd, MoveList};

pub trait TagOps: Display {
    fn tags_mut(&mut self) -> &mut Tags;
    fn tags(&self) -> &Tags;

    fn get(&self, key: &str) -> Option<&str> {
        self.tags().get(key)
    }

    fn ensure(&self, key: &str) -> Result<()> {
        self.ensure_one_of(&[key])
    }

    fn ensure_one_of(&self, keys: &[&str]) -> Result<()> {
        match self.find(keys) {
            Some(_) => Ok(()),
            _ => anyhow::bail!("Expected {} in {self}", keys.join(",")),
        }
    }

    fn find(&self, keys: &[&str]) -> Option<&str> {
        for k in keys {
            let v = self.get(k);
            if v.is_some() {
                return v;
            }
        }
        None
    }

    fn res(&self) -> Option<WdlOutcome> {
        self.get(Tags::RES).and_then(|s| s.parse().ok())
    }

    /// score is from the stm's pov
    ///
    /// Opcode "ce": centipawn evaluation
    /// The opcode "ce" indicates the evaluation of the indicated position in centipawn
    /// units. It takes a single operand, an optionally signed integer that gives an evaluation
    /// of the position from the viewpoint of the active player; i.e., the player with the move.
    fn score(&self) -> Option<Score> {
        self.tags()
            .get(Tags::CE)
            .and_then(|s| s.parse().ok())
            .map(Score::from_cp)
    }

    fn score_from_tag(&self, tag: &str) -> Option<Score> {
        self.get(tag).and_then(|s| s.parse().ok()).map(Score::from_cp)
    }

    /// score is from the stm's pov
    fn set_score(&mut self, pov_score: Score) {
        self.tags_mut().set(Tags::CE, &pov_score.as_i16().to_string());
    }

    fn is_book_move(&self) -> bool {
        self.get("eco").is_some()
    }
}

pub trait EpdOps: TagOps {
    fn board(&self) -> Board;

    fn scored_move_list(&self) -> Option<ScoredMoveList> {
        let bsm = self.find(&[Tags::BSM, Tags::ASM, Tags::ESM])?;
        ScoredMoveList::parse_san(bsm, &self.board()).ok()
    }

    /// from ce, Bsm, Asm, Esm. Empty Bsm => None
    fn score_any(&self) -> Option<Score> {
        if let Some(ce) = self.get(Tags::CE) {
            return ce.parse().ok().map(Score::from_cp);
        }
        if let Some(bsm) = self.find(&[Tags::BSM, Tags::ASM, Tags::ESM]) {
            let bsm = ScoredMoveList::parse_san(bsm, &self.board()).ok()?;
            return bsm.best_score();
        }
        None
    }

    fn var(&self, tag: &str) -> Option<Variation> {
        self.board()
            .parse_san_variation(self.get(tag)?)
            .with_context(|| self.to_string())
            .unwrap()
            .into()
    }

    fn mv(&self, tag: &str) -> Option<Move> {
        self.board()
            .parse_san_move(self.get(tag)?)
            .with_context(|| self.to_string())
            .unwrap()
            .into()
    }

    fn moves(&self, tag: &str) -> Option<MoveList> {
        self.board()
            .parse_san_movelist(self.get(tag)?)
            .context(self.to_string())
            .unwrap()
            .into()
    }

    fn best_move(&self) -> Option<Move> {
        self.mv(Tags::SM)
            .or_else(|| self.moves(Tags::BM).and_then(|ml| ml.iter().next().copied()))
    }

    fn centipawn_loss(&self) -> Option<i16> {
        let sm = self.best_move()?;
        let loss = self.scored_move_list()?.centipawn_loss(sm)?;
        loss.cp()
    }
}

impl EpdOps for Epd {
    fn board(&self) -> Board {
        self.board()
    }
}

/// to support EPD and PGN formats
#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tags(HashMap<String, String>);

impl fmt::Debug for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Tags").field("0", &self.0).finish()
    }
}

impl fmt::Display for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_epd(&Board::default())) // BUG
    }
}

impl TagOps for Tags {
    fn tags_mut(&mut self) -> &mut Tags {
        self
    }

    fn tags(&self) -> &Tags {
        self
    }
}

impl Tags {
    // http://www.saremba.de/chessgml/standards/pgn/pgn-complete.htm
    pub const VERB_DATA_NORMALIZATION: &'static str = "pfdn";
    pub const VERB_GENERAL_ANALYSIS: &'static str = "pfga";
    pub const VERB_MATE_SEARCH: &'static str = "pfms";
    pub const VERB_OPERATION_PURGE: &'static str = "pfop";
    pub const VERB_TARGET_SEARCH: &'static str = "pfts";

    pub const ECO: &'static str = "eco";
    pub const AM: &'static str = "am";
    pub const BM: &'static str = "bm";
    pub const ASM: &'static str = "Asm"; // ann scored moves "Asm a4:+45 Nf6:-56;"
    pub const BSM: &'static str = "Bsm"; // best scored moves "Bsm a4:+45 Nf6:-56;"
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
    pub const PC: &'static str = "Pc"; // pgn "PlyCount" - total game moves from pgn
    pub const CC: &'static str = "cc";
    pub const CE: &'static str = "ce";
    pub const CPL: &'static str = "Cpl"; // centipawn loss
    pub const C9: &'static str = "c9";
    pub const DM: &'static str = "dm";
    pub const FMVN: &'static str = "fmvn";
    pub const HMVC: &'static str = "hmvc";
    pub const PM: &'static str = "pm";
    pub const RC: &'static str = "rc"; // repetition count
    pub const NOOP: &'static str = "noop";
    pub const SM: &'static str = "sm";
    pub const SV: &'static str = "sv";
    pub const GM: &'static str = "Gm"; // Game move
    pub const SQ: &'static str = "Sq";
    pub const RES: &'static str = "Res";
    pub const TS: &'static str = "ts";
    pub const COMMENTS: [&'static str; 10] = ["c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9"];
    pub const PERFTS: [&'static str; 8] = ["D0", "D1", "D2", "D3", "D4", "D5", "D6", "D7"];
    pub const DRAW_REJECT: &'static str = "draw_reject";

    pub const ATTRIBUTES: &'static [&'static str] = &[Self::ACD, Self::BM, Self::PV];

    const ENGINE_RESULTS: &'static [&'static str] =
        &[Self::PV, Self::MPV, Self::SM, Self::BM, Self::CE, Self::BF, Self::CPL];

    const ANNOTATOR_RESULTS: &'static [&'static str] = &[Self::ASM, Self::CPL];

    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(k: &str, v: &str) -> Self {
        let mut t = Tags::new();
        t.set(k, v);
        t
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(|s| s.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn remove(&mut self, key: &str) {
        self.0.remove(key);
    }

    pub fn retain(&mut self, keys: &[&str]) {
        self.0.retain(|k, _v| keys.contains(&k.as_str()));
    }

    pub fn insert(&mut self, k: &str, v: &str) {
        self.0.insert(k.to_string(), v.to_string());
    }

    pub fn set(&mut self, k: &str, v: &str) {
        self.0.insert(k.to_string(), v.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn calculate_without_engine(&mut self, _desired: &[&str]) {
        // BUG
        // MPV => PV
        // if desired.contains(&Tags::PV) && self.pv.is_none() {
        //     if let Some(mpv) = &self.multi_pv {
        //         self.pv = mpv.first().map(|vs| vs.var);
        //     }
        // }

        // // PV => BM
        // if desired.contains(&Tags::BM) && self.best_moves.is_none() {
        //     if let Some(pv) = &self.pv {
        //         if let Some(mv) = pv.first() {
        //             self.best_moves = Some(Box::new(MoveList::from([mv])))
        //         }
        //     }
        // }

        // // ASM => BM
        // if desired.contains(&Tags::BM) && self.best_moves.is_none() {
        //     if let Some(asm) = &self.ann_scored_moves {
        //         if let Some(mv) = asm.best_move() {
        //             self.best_moves = Some(Box::new(MoveList::from([mv])))
        //         }
        //     }
        // }

        // // BM & ASM => CPL
        // if desired.contains(&Tags::CPL) && self.centipawn_loss.is_none() {
        //     if let Some(asm) = &self.ann_scored_moves {
        //         if let Some(bm) = &self.best_moves {
        //             if let Some(&bm1) = bm.iter().nth(0) {
        //                 self.centipawn_loss = asm.centipawn_loss(bm1).map(|sc| sc.as_i16() as i32);
        //             }
        //         }
        //     }
        // }
    }

    pub fn requires_engine(current: &[&str], desired: &[&str]) -> bool {
        for &key in desired.iter().filter(|t| !current.contains(t)) {
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

    pub fn parse_epd_tags(_board: &Board, tags_str: &str) -> Result<Tags> {
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
            tags.0.insert(words[0].to_string(), words[1..].join(" "));
        }
        Ok(tags)
    }

    pub fn parse_single_tag(_b: &Board, v: &str) -> Result<Tags> {
        let words: Vec<&str> = Self::split_into_words(v);
        let mut tags = Tags::new();
        tags.set(words[0], &words[1..].join(" "));
        Ok(tags)
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

    pub fn validate(&self, _b: &Board) -> anyhow::Result<()> {
        // self.annotator_depth.as_ref().map(|v| v.validate()).transpose().with_context(|| "Ad")?;
        //  self.analysis_count_milli_seconds.as_ref().map(|v| v.validate()).transpose().with_context(|| "Acms")?;
        //  self.branching_factor.as_ref().map(|v| v.validate()).transpose().with_context(|| "Bf")?;
        // self.ann_scored_moves
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .with_context(|| "Asm")?;
        // self.eng_scored_moves
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .with_context(|| "Esm")?;
        // self.multi_pv
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .with_context(|| "Mpv")?;
        // // self.result.as_ref().map(|v| v.validate()).transpose().with_context(|| "Res")?;
        // // self.squares.as_ref().map(|v| v.validate()).transpose().with_context(|| "Sq")?;
        // self.avoid_moves
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .with_context(|| "am")?;
        // //  self.acd.as_ref().map(|v| v.validate()).transpose().with_context(|| "acd")?;
        // //  self.analysis_count_nodes.as_ref().map(|v| v.validate()).transpose().with_context(|| "acn")?;
        // //  self.analysis_count_sel_depth.as_ref().map(|v| v.validate()).transpose().with_context(|| "acsd")?;
        // //  self.analysis_count_seconds.as_ref().map(|v| v.validate()).transpose().with_context(|| "acs")?;
        // self.best_moves
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .with_context(|| "bm")?;
        // //  self.chess_clock.as_ref().map(|v| v.validate()).transpose().with_context(|| "cc")?;
        // //  self.centipawn_evaluation.as_ref().map(|v| v.validate()).transpose().with_context(|| "ce")?;
        // //  self.direct_mate.as_ref().map(|v| v.validate()).transpose().with_context(|| "dm")?;
        // //  self.eco.as_ref().map(|v| v.validate()).transpose().with_context(|| "eco")?;
        // //  self.full_move_number.as_ref().map(|v| v.validate()).transpose().with_context(|| "fmvn")?;
        // //  self.half_move_clock.as_ref().map(|v| v.validate()).transpose().with_context(|| "hmvc")?;
        // //  self.id.as_ref().map(|v| v.validate()).transpose().with_context(|| "id")?;
        // self.predicted_move
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .map_err(anyhow::Error::msg)
        //     .with_context(|| "pm")?;
        // self.pv
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .with_context(|| "pv")?;
        // //  self.repetition_count.as_ref().map(|v| v.validate()).transpose().with_context(|| "rc")?;
        // //  self.no_op.as_ref().map(|v| v.validate()).transpose().with_context(|| "noop")?;
        // self.supplied_move
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .map_err(anyhow::Error::msg)
        //     .with_context(|| "sm")?;

        // self.game_move
        //     .as_ref()
        //     .map(|v| v.validate(b))
        //     .transpose()
        //     .map_err(anyhow::Error::msg)
        //     .with_context(|| "Gm")?;
        // // we dont validate sv as it is used to reach the board state
        // // self.sv
        // //     .as_ref()
        // //     .map(|v| v.validate(b))
        // //     .transpose()
        // //     .with_context(|| "sv")?;
        // //  self.timestamp.as_ref().map(|v| v.validate()).transpose().with_context(|| "ts")?;

        Ok(())
    }

    pub fn extend(&mut self, t: Tags) {
        self.0.extend(t.0)
    }

    pub fn to_pgn(&self, _b: &Board) -> String {
        let mut strings = vec![];
        for (k, v) in self.iter() {
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

    /// with preceeding space if tags present
    pub fn to_epd(&self, _b: &Board) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for (k, v) in self.0.iter().sorted() {
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

    // pub fn to_uci(&self) -> String {
    //     use std::fmt::Write;
    //     let mut s = String::new();
    //     for (k, v) in self.as_hash_map_uci().iter() {
    //         if v.is_empty() {
    //             write!(s, " {};", k).unwrap();
    //         } else if v.contains(char::is_whitespace) {
    //             write!(s, " {} \"{}\";", k, v).unwrap();
    //         } else {
    //             write!(s, " {} {};", k, v).unwrap();
    //         }
    //     }
    //     s
    // }
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
    // use serde_json::{value::Value, Map};
    use std::mem::size_of;

    use test_log::test;

    use super::*;

    // #[test]
    // fn tags_basics() {
    //     let b = &Board::starting_pos();
    //     let mut tags = Tags::new();
    //     tags.pv = Some("e4 e5 d4 b8c6".var(b));
    //     tags.sv = Some("e4".var(b));
    //     tags.acd = Some(3);
    //     tags.eco = Some("".to_string());
    //     tags.id = None;
    //     let map2: IndexMap<&str, String> = indexmap! {
    //         "acd" => "3".to_string(),
    //         "pv" => "e2e4 e7e5 d2d4 b8c6".to_string(),
    //         "eco" => "".to_string(),
    //         "sv" => "e2e4".to_string(),
    //     };
    //     let map1 = tags.as_hash_map_uci();
    //     assert_eq!(map1, map2); // contents match
    //     assert!(map1.iter().ne(map2.iter())); // ordering different

    //     let map3: IndexMap<&str, String> = indexmap! {
    //         "acd" => "3".to_string(),
    //         "eco" => "".to_string(),
    //         "pv" => "e2e4 e7e5 d2d4 b8c6".to_string(),
    //         "sv" => "e2e4".to_string(),
    //     };
    //     assert_equal(map1.iter(), map3.iter()); // ordering same

    //     // UCI - san format
    //     assert_eq!(
    //         tags.to_uci(),
    //         " acd 3; eco; pv \"e2e4 e7e5 d2d4 b8c6\"; sv e2e4;"
    //     );

    //     // HashMap - san format
    //     let map1s = tags.as_hash_map_san(b);
    //     let map3s: IndexMap<&str, String> = indexmap! {
    //         "acd" => "3".to_string(),
    //         "eco" => "".to_string(),
    //         "pv" => "e4 e5 d4 Nc6".to_string(),
    //         "sv" => "e4".to_string(),
    //     };
    //     assert_equal(map1s.iter(), map3s.iter()); // ordering same

    //     // PGN - san format
    //     assert_eq!(
    //         tags.to_pgn(b),
    //         "[%acd 3] [%eco] [%pv \"e4 e5 d4 Nc6\"] [%sv e4]"
    //     );

    //     // EPD - san format
    //     assert_eq!(tags.to_epd(b), " acd 3; eco; pv \"e4 e5 d4 Nc6\"; sv e4;");

    //     let pred = |s: &str| ["acd", "Asm", "sv", "c1"].contains(&s);
    //     assert_eq!(tags.clone().filter(pred).to_epd(b), " acd 3; sv e4;");
    //     assert_eq!(tags.clone().filter(|_| false).to_epd(b), "");

    //     tags.comments[0] = Some("0".to_string());
    //     tags.comments[1] = Some("1".to_string());
    //     assert_eq!(tags.clone().filter(pred).comments[0], None);
    //     assert_eq!(tags.clone().filter(pred).comments[1], Some("1".to_string()));
    //     assert_eq!(tags.clone().filter(pred).to_epd(b), " acd 3; sv e4; c1 1;");
    //     // JSON
    //     //  json parsing of Tags - failing due to nulls
    //     // assert_eq!(
    //     //     jsonrpc_core::to_string(&tags).unwrap(),
    //     //     r#"{"acd":"3", "eco":"", "pv":"e4 e5 d4 Nc6", "sv":"e4"}"#
    //     // );
    // }

    // // #[test]
    // fn test_tags_memory() {
    //     let mut tags = Tags::default();
    //     tags.analysis_count_seconds = Some(4);
    //     tags.acd = Some(3);
    //     tags.comments[0] = Some(String::from("Hello World2"));
    //     tags.id = Some(String::from("Hello World"));
    //     let _vec = vec![tags; 10_000_000];
    //     std::thread::sleep(Duration::from_secs_f32(100.0));
    // }

    // #[test]
    // fn test_parsing_tags() {
    //     let b = &Board::default();

    //     let tags = Tags::parse_single_tag(b, "acd 3").unwrap();
    //     assert_eq!(tags, Tags {
    //         acd: Some(3),
    //         ..Tags::default()
    //     });

    //     let mut tags = Tags::new();
    //     tags.comments[0] = Some("Hello World".to_string());
    //     assert_eq!(
    //         Tags::parse_key_and_tag(b, "c0", "Hello World").unwrap(),
    //         tags
    //     );
    // }

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
        assert_eq!(vec, vec!["cat\"meo;w\"", "\"mouse\"", "\"toad;;;;;;\" ", "zebra"]);

        let vec = Tags::split_into_tags(r#"cat'meo;w';'mouse';'toad;;;;;;' ;zebra;"#);
        assert_eq!(vec, vec!["cat\'meo;w\'", "\'mouse\'", "\'toad;;;;;;\' ", "zebra"]);

        let vec = Tags::split_into_tags(r#";cat;mouse;toad;;;;;;sheep;zebra"#);
        assert_eq!(vec, vec!["cat", "mouse", "toad", "sheep"]);

        // OK, but not desirable (unmatched quote parsing)
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

    #[test]
    fn test_parse_epd_tags() {
        let s = r#"acd 1000; bm e4; ce 123; draw_reject; id "TEST CASE.1";"#;
        let tags = Tags::parse_epd_tags(&Board::starting_pos(), s).unwrap();
        assert_eq!(tags.get("acd"), Some("1000"));
        assert_eq!(tags.get("bm"), Some("e4"));
        assert_eq!(tags.score(), Some(Score::from_cp(123)));
        assert_eq!(tags.get("draw_reject"), Some(""));
        assert_eq!(tags.get("id"), Some("TEST CASE.1"));
        assert_eq!(format!("{tags}"), " ".to_string() + s);
    }

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
