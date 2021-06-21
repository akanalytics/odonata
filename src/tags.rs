use crate::mv::Move;
use crate::variation::Variation;
use crate::movelist::MoveList;
use crate::types::Ply;
use crate::board::Board;
use crate::bitboard::bitboard::Bitboard;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

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




#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    None,
    BestMove(MoveList),
    Pv(Variation),
    Id(String),
    AnalysisCountDepth(Ply),
    AnalysisCountSeconds(u32),
    ChessClock(Duration),
    CentipawnEvaluation(i32),
    DirectMate(u32),
    FullMoveNumber(u32),
    HalfMoveClock(u32),
    PredictedMove(Move),
    RepititionCount(u32),
    NoOp(String),
    SuppliedMove(Move),
    SuppliedVariation(Variation),
    Squares(Bitboard),
    Timestamp(String, String),
    Perft(u8, u128),
    Comment(u8, String),
}

impl Tag {

    pub const BM: &'static str = "bm";
    pub const PV: &'static str = "pv";
    pub const ID: &'static str = "id";
    pub const ACD: &'static str = "acd";
    pub const ACS: &'static str = "acs";
    pub const CC: &'static str = "cc";
    pub const CE: &'static str = "ce";
    pub const DM: &'static str = "dm";
    pub const FMVN: &'static str = "fmvn";
    pub const HMVC: &'static str = "hmvc";
    pub const PM: &'static str = "pm";
    pub const RC: &'static str = "rc";
    pub const NOOP: &'static str = "noop";
    pub const SM: &'static str = "sm";
    pub const SV: &'static str = "sv";
    pub const SQ: &'static str = "Sq";
    pub const TS: &'static str = "ts";
    pub const DRAW_REJECT: &'static str = "draw_reject";

    pub const ATTRIBUTES: &'static [&'static str] = &[Self::ACD, Self::BM, Self::PV];



    pub fn parse(b: &Board, key: &str, v: &str) -> Result<Tag, String> {
        Ok(match key {
            Self::BM => Tag::BestMove(b.parse_san_choices(v)?),
            Self::PV => Tag::Pv(b.parse_san_moves(v)?),
            Self::ID => Tag::Id(v.to_string()) ,
            Self::ACD => Tag::AnalysisCountDepth(v.parse::<Ply>().map_err(|e| e.to_string())?) ,
            Self::ACS => Tag::AnalysisCountSeconds(v.parse::<u32>().map_err(|e| e.to_string())?) ,
            Self::CC => Tag::ChessClock(Duration::new(0, 0)),
            Self::CE => Tag::CentipawnEvaluation(v.parse::<i32>().map_err(|e| e.to_string())?),
            Self::DM => Tag::DirectMate(v.parse::<u32>().map_err(|e| e.to_string())?),
            Self::FMVN => Tag::FullMoveNumber(v.parse::<u32>().map_err(|e| e.to_string())?),
            Self::HMVC => Tag::HalfMoveClock(v.parse::<u32>().map_err(|e| e.to_string())?),
            Self::PM => Tag::PredictedMove(b.parse_san_move(v)?),
            Self::RC => Tag::RepititionCount(v.parse::<u32>().map_err(|e| e.to_string())?),
            Self::NOOP => Tag::NoOp(v.to_string()),
            Self::SM => Tag::SuppliedMove(b.parse_san_move(v)?) ,
            Self::SV => Tag::SuppliedVariation(b.parse_san_moves(v)?) ,
            Self::SQ => Tag::Squares(Bitboard::parse_squares(v)?),
            Self::TS => Tag::Timestamp("".to_string(), "".to_string()),
            _ if key.starts_with('D') => Tag::Perft( key[1..].parse::<u8>().map_err(|e| e.to_string())?, v.parse::<u128>().map_err(|e| e.to_string())?),
            _ if key.starts_with('c') => Tag::Comment( key[1..].parse::<u8>().map_err(|e| e.to_string())?, v.to_string()),
            _ => Tag::None,

        })
    }



    pub fn key(&self) -> String {
        match &self {
            Tag::None => "".to_string(),
            Tag::BestMove(_) => Self::BM.to_string(),
            Tag::Pv(_) => Self::PV.to_string(),
            Tag::Id(_) => Self::ID.to_string(),
            Tag::AnalysisCountDepth(_) => Self::ACD.to_string(),
            Tag::AnalysisCountSeconds(_) => Self::ACS.to_string(),
            Tag::ChessClock(_) => "cc".to_string(),
            Tag::CentipawnEvaluation(_) => Self::CE.to_string(),
            Tag::DirectMate(_) => Self::DM.to_string(),
            Tag::FullMoveNumber(_) => "fmvn".to_string(),
            Tag::HalfMoveClock(_) => "hmvc".to_string(),
            Tag::PredictedMove(_) => "pm".to_string(),
            Tag::RepititionCount(_) => "rc".to_string(),
            Tag::NoOp(_) => "noop".to_string(),
            Tag::SuppliedMove(_) => Self::SM.to_string(),
            Tag::SuppliedVariation(_) => Self::SV.to_string(),
            Tag::Squares(_) => Self::SQ.to_string(),
            Tag::Timestamp(_, _) => Self::TS.to_string(),
            Tag::Perft(depth, _count) => format!("D{}", depth),
            Tag::Comment(n, _text) => format!("c{}", n),
        }
    }

    pub fn value(&self) -> String {
        match &self {
            Tag::None => "".to_string(),
            Tag::BestMove(mvs) => mvs.uci(),
            Tag::Pv(variation) => variation.uci(),
            Tag::Id(s) => format!("{}", s),
            Tag::AnalysisCountDepth(n) => format!("{}", n),
            Tag::AnalysisCountSeconds(n) => format!("{}", n),
            Tag::ChessClock(_duration) => format!("{}", "na"), // FIXME!
            Tag::CentipawnEvaluation(score) => score.to_string(),
            Tag::DirectMate(n) => format!("{}", n),
            Tag::FullMoveNumber(n) => format!("{}", n),
            Tag::HalfMoveClock(n) => format!("{}", n),
            Tag::PredictedMove(mv) => mv.uci(),
            Tag::RepititionCount(n) => format!("{}", n),
            Tag::NoOp(vec) => format!("{:?}", vec),
            Tag::SuppliedMove(mv) => mv.uci(),
            Tag::SuppliedVariation(movelist) => movelist.uci(),
            Tag::Squares(bitboard) => bitboard.uci(),
            Tag::Timestamp(date, time) => format!("{} {}", date, time),
            Tag::Perft(_depth, count) => format!("{}", count),
            Tag::Comment(_n, text) => format!("{}", text),
        }
    }
}





#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tags {
    tags: HashMap<String, Tag>,
}

impl Tags {


    pub fn new() -> Self {
        Tags::default()
    }
    pub fn as_hash_map(&self) -> HashMap<String, String> {
        HashMap::<String, String>::new()
    }

    pub fn get(&self, key: &str) -> &Tag {
        let ov = self.tags.get(key);
        match ov {
            Some(v) => v,
            None => &Tag::None,
        }
    }


    pub fn remove(&mut self, key: &str)  {
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

    pub fn to_pgn(&self) -> String {
        unreachable!();
    }
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
}

//
//  key1; key2; key3; key4 ABCD; key5 12345; key6 "ABC;DEF";
//
impl fmt::Display for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut entries = self.tags.iter().collect::<Vec<_>>();
        entries.sort_by(|x,y| x.0.cmp(&y.0));  // sort by key
        for (k, t) in entries {
            let v = t.value();
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



#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_tag_comment() {
        let mut tags = Tags::new();
        
        tags.set(Tag::Comment(0, "Hello".into()));
        tags.set(Tag::Comment(1, "World".into()));
        assert_eq!(tags.get("c0").value(), "Hello");
        assert_eq!(tags.get("c1").value(), "World");
        let b = Board::default();
        assert_eq!(Tag::parse(&b, "c0", "Hello World"), Ok(Tag::Comment(0, "Hello World".to_string())));
    
    }
    
    #[test]
    fn test_tags() {
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
        assert_eq!(tags.to_string(), " acd 3; acs 4; id \"Hello World\";");
    }
}