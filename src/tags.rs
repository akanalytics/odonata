use crate::bits::bitboard::Bitboard;
use crate::board::Board;
use crate::infra::utils::{Displayable, Formatting};
use crate::movelist::{MoveList, ScoredMoveList};
use crate::mv::Move;
use crate::piece::Ply;
use crate::variation::Variation;
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde::{ser::SerializeMap, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
// use serde_with::{DeserializeFromStr};

// #[derive(Clone, Debug)]
// pub enum TagValue {
//     String(String),
//     DateType(String),
//     TimeType(String),
//     MoveType(Move),
//     MoveListType(MoveList),
//     IntType(i64),
// }

// impl fmt::Display for TagValue {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         Ok(())
//     }
// }




#[derive(Default, Debug, Serialize, Deserialize)]
pub struct TagUnion {
    avoid_moves: Option<Box<MoveList>>,
    best_moves: Option<Box<MoveList>>,
    best_scored_moves: Option<ScoredMoveList>,
    branching_factor: Option<f64>, 
    pv: Option<Variation>,
    id: Option<String>,
    analysis_count_depth: Option<Ply>,
    analysis_count_sel_depth: Option<Ply>,
    analysis_count_nodes: Option<u128>,
    analysis_count_seconds: Option<u32>,
    analysis_count_milli_seconds: Option<u64>,
    chess_clock: Option<Duration>,
    centipawn_evaluation: Option<i32>,
    direct_mate: Option<u32>,
    full_move_number: Option<u32>,
    half_move_clock: Option<u32>,
    predicted_move: Option<Move>,
    repitition_count: Option<u32>,
    result: Option<String>,
    no_op: Option<String>,
    supplied_move: Option<Move>,
    supplied_variation: Option<Variation>,
    squares: Option<Bitboard>,
    timestamp: Option<(String, String)>,
    perft: Option<(u8, u128)>,
    comment: Option<(u8, String)>,
}


impl TagUnion {
    pub fn parse_tag(_kind: &str, _value: &str) -> anyhow::Result<TagUnion> {

        Ok(TagUnion::default())
    } 
}







#[cfg(test)]
mod tests2 {
    use serde_json::Map;
    use serde_json::value::Value;

    use super::*;


    #[test]
    fn tags_x() {
        let mut tags = TagUnion::default();
        tags.result = Some("Hello Word".to_owned());
        let value = serde_json::to_value(tags).unwrap();
        dbg!(std::mem::size_of_val(&value));
        dbg!(&value);
        if let Value::Object(map) = &value {
            dbg!(map);
        }

        let mut map = Map::new();
        map.insert("result".to_owned(), Value::String("Hello World2".to_owned()));

        let tags2: TagUnion = serde_json::from_value(Value::Object(map)).unwrap();
        dbg!(tags2);


    }
}






















#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    None,
    AvoidMoves(MoveList),
    BestMoves(MoveList),
    BestScoredMoves(ScoredMoveList),
    BranchingFactor(f64), // 100x
    Pv(Variation),
    Id(String),
    AnalysisCountDepth(Ply),
    AnalysisCountSelDepth(Ply),
    AnalysisCountNodes(u128),
    AnalysisCountSeconds(u32),
    AnalysisCountMilliSeconds(u64),
    ChessClock(Duration),
    CentipawnEvaluation(i32),
    DirectMate(u32),
    FullMoveNumber(u32),
    HalfMoveClock(u32),
    PredictedMove(Move),
    RepititionCount(u32),
    Result(String),
    NoOp(String),
    SuppliedMove(Move),
    SuppliedVariation(Variation),
    Squares(Bitboard),
    Timestamp(String, String),
    Perft(u8, u128),
    Comment(u8, String),
}

impl Tag {
    pub const AM: &'static str = "am";
    pub const BM: &'static str = "bm";
    pub const BSM: &'static str = "Bsm"; // best scored moves "Bsm a4:+45 Nf6:-56;"
    pub const BF: &'static str = "Bf";
    pub const PV: &'static str = "pv";
    pub const ID: &'static str = "id";
    pub const ACD: &'static str = "acd";
    pub const ACSD: &'static str = "acsd";
    pub const ACN: &'static str = "acn";
    pub const ACS: &'static str = "acs";
    pub const ACMS: &'static str = "Acms";
    pub const CC: &'static str = "cc";
    pub const CE: &'static str = "ce";
    pub const C9: &'static str = "c9";
    pub const DM: &'static str = "dm";
    pub const FMVN: &'static str = "fmvn";
    pub const HMVC: &'static str = "hmvc";
    pub const PM: &'static str = "pm";
    pub const RC: &'static str = "rc";
    pub const NOOP: &'static str = "noop";
    pub const SM: &'static str = "sm";
    pub const SV: &'static str = "sv";
    pub const SQ: &'static str = "Sq";
    pub const RES: &'static str = "Res";
    pub const TS: &'static str = "ts";
    pub const DRAW_REJECT: &'static str = "draw_reject";

    pub const ATTRIBUTES: &'static [&'static str] = &[Self::ACD, Self::BM, Self::PV];

    fn parse_internal(b: &Board, key: &str, v: &str) -> Result<Tag> {
        Ok(match key {
            Self::AM => Tag::AvoidMoves(b.parse_san_movelist(v)?),
            Self::BM => Tag::BestMoves(b.parse_san_movelist(v)?),
            Self::BSM => Tag::BestScoredMoves(ScoredMoveList::parse_san(v, b)?),
            Self::BF => Tag::BranchingFactor(v.parse::<f64>()?),
            Self::PV => Tag::Pv(b.parse_san_variation(v)?),
            Self::ID => Tag::Id(v.to_string()),
            Self::ACD => Tag::AnalysisCountDepth(v.parse::<Ply>()?),
            Self::ACSD => Tag::AnalysisCountSelDepth(v.parse::<Ply>()?),
            Self::ACN => Tag::AnalysisCountNodes(v.parse::<u128>()?),
            Self::ACS => Tag::AnalysisCountSeconds(v.parse::<u32>()?),
            Self::ACMS => Tag::AnalysisCountMilliSeconds(v.parse::<u64>()?),
            Self::CC => Tag::ChessClock(Duration::new(0, 0)),
            Self::CE => Tag::CentipawnEvaluation(v.parse::<i32>()?),
            Self::DM => Tag::DirectMate(v.parse::<u32>()?),
            Self::FMVN => Tag::FullMoveNumber(v.parse::<u32>()?),
            Self::HMVC => Tag::HalfMoveClock(v.parse::<u32>()?),
            Self::PM => Tag::PredictedMove(b.parse_san_move(v)?),
            Self::RC => Tag::RepititionCount(v.parse::<u32>()?),
            Self::RES => Tag::Result(v.to_string()),
            Self::NOOP => Tag::NoOp(v.to_string()),
            Self::SM => Tag::SuppliedMove(b.parse_san_move(v)?),
            Self::SV => Tag::SuppliedVariation(b.parse_san_variation(v)?),
            Self::SQ => Tag::Squares(Bitboard::parse_squares(v)?),
            Self::TS => Tag::Timestamp("".to_string(), "".to_string()),
            _ if key.starts_with('D') => Tag::Perft(key[1..].parse::<u8>()?, v.parse::<u128>()?),
            _ if key.starts_with('c') => Tag::Comment(key[1..].parse::<u8>()?, v.to_string()),
            _ => Tag::None,
        })
    }

    pub fn parse(b: &Board, key: &str, v: &str) -> Result<Tag> {
        match Self::parse_internal(b, key, v) {
            Err(err) => Err(anyhow!("{} parsing tag '{}' from '{}'", err, key, v)),
            Ok(tag) => Ok(tag),
        }
    }

    pub fn key(&self) -> String {
        match &self {
            Tag::None => "".to_string(),
            Tag::AvoidMoves(_) => Self::AM.to_string(),
            Tag::BestMoves(_) => Self::BM.to_string(),
            Tag::BestScoredMoves(_) => Self::BSM.to_string(),
            Tag::BranchingFactor(_) => Self::BF.to_string(),
            Tag::Pv(_) => Self::PV.to_string(),
            Tag::Id(_) => Self::ID.to_string(),
            Tag::AnalysisCountDepth(_) => Self::ACD.to_string(),
            Tag::AnalysisCountSelDepth(_) => Self::ACSD.to_string(),
            Tag::AnalysisCountNodes(_) => Self::ACN.to_string(),
            Tag::AnalysisCountSeconds(_) => Self::ACS.to_string(),
            Tag::AnalysisCountMilliSeconds(_) => Self::ACMS.to_string(),
            Tag::ChessClock(_) => Self::CC.to_string(),
            Tag::CentipawnEvaluation(_) => Self::CE.to_string(),
            Tag::DirectMate(_) => Self::DM.to_string(),
            Tag::FullMoveNumber(_) => Self::FMVN.to_string(),
            Tag::HalfMoveClock(_) => Self::HMVC.to_string(),
            Tag::PredictedMove(_) => Self::PM.to_string(),
            Tag::RepititionCount(_) => Self::RC.to_string(),
            Tag::Result(_) => Self::RES.to_string(),
            Tag::NoOp(_) => Self::NOOP.to_string(),
            Tag::SuppliedMove(_) => Self::SM.to_string(),
            Tag::SuppliedVariation(_) => Self::SV.to_string(),
            Tag::Squares(_) => Self::SQ.to_string(),
            Tag::Timestamp(_, _) => Self::TS.to_string(),
            Tag::Perft(depth, _count) => format!("D{}", depth),
            Tag::Comment(n, _text) => format!("c{}", n),
        }
    }

    #[allow(clippy::useless_format)]
    pub fn value_uci(&self) -> String {
        match &self {
            Tag::None => "".to_string(),
            Tag::AvoidMoves(mvs) => mvs.uci(),
            Tag::BestMoves(mvs) => mvs.uci(),
            Tag::BestScoredMoves(mvs) => format!("{:?}", mvs),
            Tag::BranchingFactor(bf) => Formatting::decimal(2, *bf),
            Tag::Pv(variation) => variation.to_uci(),
            Tag::Id(s) => format!("{}", s),
            Tag::AnalysisCountDepth(n) => format!("{}", n),
            Tag::AnalysisCountSelDepth(n) => format!("{}", n),
            Tag::AnalysisCountNodes(n) => format!("{}", n),
            Tag::AnalysisCountSeconds(n) => format!("{}", n),
            Tag::AnalysisCountMilliSeconds(n) => format!("{}", n),
            Tag::ChessClock(_duration) => format!("{}", "na"), // FIXME!
            Tag::CentipawnEvaluation(score) => score.to_string(),
            Tag::DirectMate(n) => format!("{}", n),
            Tag::FullMoveNumber(n) => format!("{}", n),
            Tag::HalfMoveClock(n) => format!("{}", n),
            Tag::PredictedMove(mv) => mv.to_uci(),
            Tag::RepititionCount(n) => format!("{}", n),
            Tag::Result(s) => format!("{}", s),
            Tag::NoOp(vec) => format!("{:?}", vec),
            Tag::SuppliedMove(mv) => mv.to_uci(),
            Tag::SuppliedVariation(movelist) => movelist.to_uci(),
            Tag::Squares(bitboard) => bitboard.uci(),
            Tag::Timestamp(date, time) => format!("{} {}", date, time),
            Tag::Perft(_depth, count) => format!("{}", count),
            Tag::Comment(_n, text) => format!("{}", text),
        }
    }

    pub fn value(&self, b: &Board) -> String {
        match &self {
            Tag::AvoidMoves(mvs) => b.to_san_movelist(mvs),
            Tag::BestMoves(mvs) => b.to_san_movelist(mvs),
            Tag::BestScoredMoves(mvs) => mvs.to_san(b),
            Tag::Pv(variation) => b.to_san_variation(variation, None),
            Tag::PredictedMove(mv) => b.to_san(mv),
            Tag::SuppliedMove(mv) => b.to_san(mv),
            Tag::SuppliedVariation(variation) => b.to_san_variation(variation, None),
            _ => self.value_uci(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tags {
    tags: HashMap<String, Tag>,
}

impl Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.tags.len()))?;
        let mut entries = self.tags.iter().collect::<Vec<_>>();
        entries.sort_by(|x, y| x.0.cmp(y.0)); // sort by key
        for (k, v) in entries.iter() {
            map.serialize_entry(k, &v.value_uci())?;
        }
        map.end()
    }
}

impl Tags {
    pub fn new() -> Self {
        Tags::default()
    }
    pub fn as_hash_map(&self, b: &Board) -> HashMap<String, String> {
        let mut map = HashMap::<String, String>::new();
        for (k, t) in self.tags.iter() {
            map.insert(k.clone(), t.value(b));
        }
        map
    }

    pub fn get(&self, key: &str) -> &Tag {
        let ov = self.tags.get(key);
        match ov {
            Some(v) => v,
            None => &Tag::None,
        }
    }

    pub fn remove(&mut self, key: &str) {
        self.tags.remove(key);
    }

    // pub fn set_str(&mut self, key: &str, value: &str) -> Result<(), String> {
    //     self.tags.insert(key.to_string(), value.to_string());
    //     self
    // }

    pub fn set(&mut self, tag: Tag) {
        self.tags.insert(tag.key(), tag);
    }

    // pub fn set_all(&mut self, map: &HashMap<String, String>) {
    //     for (k, v) in map {
    //         self.tags.insert(k.clone(), v.clone());
    //     }
    // }

    // pub fn to_pgn(&self) -> String {
    //     unreachable!();
    // }
    //     let ce = self.get(&Tag::CentipawnEvaluation(Score::from_cp(0)).key()).ok();
    //     let acd = self.get(&Tag::AnalysisCountDepth(0).key()).ok();
    //     if let Some(ce) = ce {
    //         if let Some(acd) = acd {
    //             let ce = ce.replace("cp", "").trim().parse::<i32>();
    //             if let Ok(ce) = ce {
    //                 return format!(" {{ {:.02}/{} }}", ce as f32 / 100., acd);
    //             }
    //         }
    //     }
    //     "".to_string()
    //     // format!("{:?}", self)
    // }

    pub fn parse_tags(board: &Board, tags_str: &str) -> Result<Tags> {
        let mut tags = Tags::new();
        let ops: Vec<&str> = Self::split_into_tags(tags_str);
        for op in ops {
            let words: Vec<&str> = Self::split_into_words(op);
            debug_assert!(
                !words.is_empty(),
                "no words parsing EPD operation '{}' from '{}'",
                op,
                tags_str
            );
            let tag = Tag::parse(board, words[0], words[1..].join(" ").as_str())?;
            // map.insert.to_string(), words[1..].join(" ").to_string());
            tags.set(tag);
        }
        Ok(tags)
    }

    fn fmt_pgn(&self, f: &mut fmt::Formatter, b: &Board) -> fmt::Result {
        let mut entries = self.tags.iter().collect::<Vec<_>>();
        entries.sort_by(|x, y| x.0.cmp(y.0)); // sort by key
        for (k, t) in entries {
            let v = t.value(b);
            if v.is_empty() {
                write!(f, " {};", k)?;
            } else if v.contains(char::is_whitespace) {
                write!(f, " {} \"{}\";", k, v)?;
            } else {
                write!(f, " {} {};", k, v)?;
            }
        }
        Ok(())
    }

    pub fn display_pgn<'a>(&'a self, b: &'a Board) -> impl fmt::Display + 'a {
        Displayable(|f| self.fmt_pgn(f, b))
    }

    pub fn to_pgn(&self, b: &Board) -> String {
        format!("{}", self.display_pgn(b))
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
}

impl fmt::Display for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut entries = self.tags.iter().collect::<Vec<_>>();
        entries.sort_by(|x, y| x.0.cmp(y.0)); // sort by key
        for (k, t) in entries {
            let v = t.value_uci();
            if v.is_empty() {
                write!(f, " {};", k)?;
            } else if v.contains(char::is_whitespace) {
                write!(f, " {} \"{}\";", k, v)?;
            } else {
                write!(f, " {} {};", k, v)?;
            }
        }
        Ok(())
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

//
//  key1; key2; key3; key4 ABCD; key5 12345; key6 "ABC;DEF";
//

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_split_into_tags() {
        let vec = Tags::split_into_tags(r#"cat"meo;w";"mouse";"toad;;;;;;" ;zebra;"#);
        assert_eq!(
            vec,
            vec!["cat\"meo;w\"", "\"mouse\"", "\"toad;;;;;;\" ", "zebra"]
        );

        let vec = Tags::split_into_tags(r#"cat'meo;w';'mouse';'toad;;;;;;' ;zebra;"#);
        assert_eq!(
            vec,
            vec!["cat\'meo;w\'", "\'mouse\'", "\'toad;;;;;;\' ", "zebra"]
        );

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
    fn test_tag_comment() {
        let mut tags = Tags::new();

        tags.set(Tag::Comment(0, "Hello".into()));
        tags.set(Tag::Comment(1, "World".into()));
        assert_eq!(tags.get("c0").value_uci(), "Hello");
        assert_eq!(tags.get("c1").value_uci(), "World");
        let b = Board::default();
        assert_eq!(
            Tag::parse(&b, "c0", "Hello World").unwrap(),
            Tag::Comment(0, "Hello World".to_string())
        );
    }

    #[test]
    fn test_tags() {
        dbg!(std::mem::size_of::<TagUnion>());
        dbg!(std::mem::size_of::<MoveList>());
        dbg!(std::mem::size_of::<Variation>());
        dbg!(std::mem::size_of::<Duration>());
        dbg!(std::mem::size_of::<u128>());
        dbg!(std::mem::size_of::<ScoredMoveList>());
        dbg!(std::mem::size_of::<Option<Box<ScoredMoveList>>>());
        dbg!(std::mem::size_of::<Vec<bool>>());
        dbg!(std::mem::size_of::<Option<Vec<bool>>>());

        let mut tags = Tags::new();
        tags.remove(Tag::BM);
        assert_eq!(tags.get(Tag::BM), &Tag::None);

        tags.set(Tag::AnalysisCountDepth(3));
        assert_eq!(tags.get(Tag::ACD), &Tag::AnalysisCountDepth(3));
        if let Tag::AnalysisCountDepth(d) = tags.get(Tag::ACD) {
            assert_eq!(d, &3);
        } else {
            assert!(false);
        }
        assert_eq!(tags.to_string(), " acd 3;");
        tags.set(Tag::AnalysisCountSeconds(4));
        tags.set(Tag::Id("Hello World".to_string()));
        tags.set(Tag::Comment(1, "Hello World2".to_string()));
        assert_eq!(
            tags.to_string(),
            " acd 3; acs 4; c1 \"Hello World2\"; id \"Hello World\";"
        );
        assert_eq!(
            jsonrpc_core::to_string(&tags).unwrap(),
            r#"{"acd":"3","acs":"4","c1":"Hello World2","id":"Hello World"}"#
        );
    }
}
