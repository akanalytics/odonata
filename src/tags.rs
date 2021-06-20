use crate::eval::score::Score;
use crate::mv::Move;
use crate::variation::Variation;
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
    BestMove(Move),
    Pv(Variation),
    Id(String),
    AnalysisCountDepth(u32),
    AnalysisCountSeconds(u32),
    ChessClock(Duration),
    CentipawnEvaluation(Score),
    DirectMate(u32),
    FullMoveNumber(u32),
    HalfMoveClock(u32),
    PredictedMove(Move),
    RepititionCount(u32),
    NoOp(u32),
    SuppliedMove(Move),
    SuppliedVariation(Move),
    //Timestamp(String, String),
    Perft(u8, u128),
    Comment(u8, String),
}

impl Tag {
    pub fn key(&self) -> String {
        match &self {
            Tag::BestMove(_) => "bm".to_string(),
            Tag::Pv(_) => "pv".to_string(),
            Tag::Id(_) => "id".to_string(),
            Tag::AnalysisCountDepth(_) => "acd".to_string(),
            Tag::AnalysisCountSeconds(_) => "acs".to_string(),
            Tag::ChessClock(_) => "cc".to_string(),
            Tag::CentipawnEvaluation(_) => "ce".to_string(),
            Tag::DirectMate(_) => "dm".to_string(),
            Tag::FullMoveNumber(_) => "fmvn".to_string(),
            Tag::HalfMoveClock(_) => "hmvc".to_string(),
            Tag::PredictedMove(_) => "pm".to_string(),
            Tag::RepititionCount(_) => "rc".to_string(),
            Tag::NoOp(_) => "noop".to_string(),
            Tag::SuppliedMove(_) => "sm".to_string(),
            Tag::SuppliedVariation(_) => "sv".to_string(),
            // Tag::Timestamp(_, _) => "ts".to_string(),
            Tag::Perft(depth, _count) => format!("D{}", depth),
            Tag::Comment(n, _text) => format!("c{}", n),
        }
    }

    pub fn value(&self) -> String {
        match &self {
            Tag::BestMove(mv) => mv.uci(),
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
            // Tag::Timestamp(date, time) => format!("{} {}", date, time),
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
        &self.tags[key]
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
//         let tags = self.as_hash_map();
//         let mut entries = tags.iter().collect::<Vec<_>>();
//         entries.sort();
//         for (k, v) in entries {
//             if v.is_empty() {
//                 write!(f, " {};", k)?;
//             } else if v.contains(char::is_whitespace) {
//                 write!(f, " {} \"{}\";", k, v)?;
//             } else {
//                 write!(f, " {} {};", k, v)?;
//             }
//         }
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;


    #[test]
    fn test_tags() {
        let mut tags = Tags::new();
        tags.set(Tag::AnalysisCountDepth(3));
        if let Tag::AnalysisCountDepth(u) = tags.get("&Tag::AnalysisCountDepth") {
            
        }

    }
}