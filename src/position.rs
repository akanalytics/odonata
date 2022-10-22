use crate::bits::bitboard::Bitboard;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::infra::metric::Metrics;
use crate::mv::Move;
use crate::search::node::Timing;
use crate::variation::Variation;

use crate::bits::castling::CastlingRights;
use crate::infra::utils::{StringUtils, Uci};
use crate::movelist::MoveList;
use crate::piece::{Color, Ply};
use crate::tags::{Tag, Tags};
use anyhow::{anyhow, bail, Context, Result};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::prelude::ParallelIterator;
use regex::Regex;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::convert::{Into, TryFrom};

use std::fmt;

// http://jchecs.free.fr/pdf/EPDSpecification.pdf
// BRATKO https://www.stmintz.com/ccc/index.php?id=20631
// https://sites.google.com/site/strategictestsuite/about-1
// shredder test https://www.stmintz.com/ccc/index.php?id=137052
// WAC https://opensource.apple.com/source/Chess/Chess-311/sjeng/tests/wac.epd.auto.html
// http://www.talkchess.com/forum3/viewtopic.php?t=62576  WAC with scores
// https://www.chessprogramming.org/Extended_Position_Description
// http://www.talkchess.com/forum3/viewtopic.php?t=69640&start=20

// #[serde(into = "HashMap<String,String>")]
#[derive(Clone, Default, Debug, Deserialize, PartialEq)]
#[serde(try_from = "HashMap<String,String>")]
pub struct Position {
    board: Board,
    tags: Tags,
}

impl TryFrom<HashMap<String, String>> for Position {
    type Error = anyhow::Error;

    fn try_from(map: HashMap<String, String>) -> Result<Self> {
        let board_str = map.get("fen").ok_or(anyhow!("no key 'fen'"))?;
        let mut p = Position {
            board: Board::parse_fen(board_str)?,
            tags: Tags::new(),
        };
        for (k, v) in map.iter() {
            if k == "fen" {
                continue;
            }
            let tag = Tag::parse(p.board(), k, v)
                .with_context(|| format!("in tag '{}' with value '{}'", k, v))?;
            p.set(tag);
        }
        Ok(p)
    }
}

#[allow(clippy::from_over_into)]
impl Into<HashMap<String, String>> for Position {
    fn into(self) -> HashMap<String, String> {
        let mut map = self.tags.as_hash_map(self.board());
        map.insert("fen".to_string(), self.board().to_fen());
        map
    }
}

// ordered with fen first, then alphabetically by tag-key
impl Serialize for Position {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // let mut s = serializer.serialize_struct("Position", 2)?;
        // s.serialize_field("board", self.board);
        let mut m = s.serialize_map(None)?;
        m.serialize_entry("fen", &self.board().to_fen())?;
        let map = self.tags.as_hash_map(self.board());
        let mut entries = map.iter().collect::<Vec<_>>();
        entries.sort_by(|x, y| x.0.cmp(y.0)); // sort by key
        for (k, v) in entries.iter() {
            m.serialize_entry(k, &v)?;
        }
        m.end()
    }
}

impl Uci for Position {
    fn fmt_uci(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.board() == &Catalog::starting_board() {
            write!(f, "position startpos")?;
        } else {
            write!(f, "position fen {fen}", fen = self.board().to_fen())?;
        }
        let var = self.supplied_variation();
        if !var.is_empty() {
            write!(f, " moves {}", var.to_uci())?;
        }
        Ok(())
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut words = s.split_whitespace();
        let word = words.next();
        if word != Some("position") {
            bail!("expected 'position' at start of '{s}'")
        }
        let fen1 = words.next();
        let board = if fen1 == Some("startpos") {
            Catalog::starting_board()
        } else if fen1 == Some("fen") {
            let fen = words.by_ref().take(6).join(" ");
            Board::parse_fen(&fen)?
        } else {
            bail!("expected either 'fen' or 'startpos' after 'position' in '{s}'");
        };
        let moves = words.join(" ");
        let var = if let Some(var_text) = moves.strip_prefix("moves ") {
            Variation::parse_uci(&var_text.trim_start(), &board)?
        } else if moves.trim().is_empty() {
            Variation::new()
        } else {
            bail!("unexpected text '{moves}' at end of position '{s}'")
        };
        let mut pos = Position::from_board(board);
        pos.set(Tag::SuppliedVariation(var));
        Ok(pos)
    }
}

static REGEX_CR_PLUS_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\s*\n\s*"#).unwrap());

/// builder methods
impl Position {
    pub fn from_board(board: Board) -> Self {
        Self {
            board,
            ..Self::default()
        }
    }

    /// 0. Piece placement
    /// 1. Active color
    /// 2. Castling rights
    /// 3. E/P square
    /// 4. Half move clock
    /// 5. Full move counter
    ///
    // TODO! allow newlines replaced by / only in fen, thereafter just ignore
    pub fn parse_epd(epd: &str) -> Result<Self> {
        let t = Metrics::timing_start();
        // replace \n followed by whitespace with "/"
        let epd = epd.trim_start();
        let epd = REGEX_CR_PLUS_WS.replace_all(epd, "/");
        let epd = epd.as_ref();
        let words = epd.split_whitespace().collect::<Vec<_>>();
        if words.len() < 4 {
            bail!("Must specify at least 4 parts in EPD '{}'", epd);
        }
        let mut pos = Position {
            board: Board::parse_piece_placement(words[0])?,
            tags: Tags::new(),
        };
        pos.board.set_turn(Color::parse(words[1])?);
        pos.board.set_castling(CastlingRights::parse(words[2])?);
        if words[3] == "-" {
            pos.board.set_en_passant(Bitboard::EMPTY)
        } else {
            pos.board
                .set_en_passant(Bitboard::parse_square(words[3])?.as_bb())
        };

        let mut remaining = StringUtils::trim_first_n_words(epd, 4);
        if words.len() >= 6 {
            let hmvc = words[4].parse::<u16>();
            let fmvn = words[5].parse::<u16>();
            if let Ok(hmvc) = hmvc {
                if let Ok(fmvn) = fmvn {
                    pos.board.set_fifty_halfmove_clock(hmvc as i32);
                    pos.board.set_fullmove_number(fmvn as i32);
                    remaining = StringUtils::trim_first_n_words(epd, 6);
                }
            }
        }
        pos.tags = Tags::parse_tags(&pos.board, remaining)?;
        Metrics::profile(t, Timing::TimingParseEpd);
        Ok(pos)
    }

    pub fn find_by_id<'a>(id: &str, positions: &'a [Position]) -> Option<&'a Position> {
        for p in positions.iter() {
            if p.id().ok() == Some(id) {
                return Some(p);
            }
        }
        None
    }

    pub fn parse_many_epd<I>(iter: I) -> Result<Vec<Position>>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut vec = Vec::<Position>::new();
        for item in iter {
            let s = item.as_ref();
            if !s.is_empty() {
                vec.push(
                    Self::parse_epd(&s.replace('\n', " "))
                        .with_context(|| format!("in epd {}", s))?,
                );
            }
        }
        Ok(vec)
    }

    pub fn parse_many_epd_threaded<I>(par_iter: I) -> Result<Vec<Position>>
    where
        I: ParallelIterator<Item = String>,
    {
        // <<I as IntoIterator>::IntoIter as Iterator>::Item=String, {
        par_iter
            .filter_map(|line: String| {
                let s = line;
                if !s.trim_start().starts_with('#') {
                    Some(Position::parse_epd(&s).with_context(|| format!("in epd {}", s)))
                } else {
                    None
                }
            })
            .collect::<Result<Vec<Position>>>()
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.board().to_fen())?;
        self.tags.fmt(f)
    }
}

impl Position {
    pub const VERB_DATA_NORMALIZATION: &'static str = "pfdn";
    pub const VERB_GENERAL_ANALYSIS: &'static str = "pfga";
    pub const VERB_MATE_SEARCH: &'static str = "pfms";
    pub const VERB_OPERATION_PURGE: &'static str = "pfop";
    pub const VERB_TARGET_SEARCH: &'static str = "pfts";

    pub fn to_pgn(&self) -> String {
        format!(
            "{fen}{tags}",
            fen = self.board().to_fen(),
            tags = self.tags.to_pgn(self.board())
        )
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    /// consumes the supplied variation, and applies it to the board
    pub fn make_moves(&mut self) {
        self.board = self.board().make_moves_old(self.supplied_variation());
        self.tags.remove(Tag::SV);
    }

    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    // pub fn as_hash_map(&self) -> HashMap<String, String> {
    //     self.tags.as_hash_map()
    // }

    pub fn tags(&self) -> &Tags {
        &self.tags
    }

    pub fn tags_mut(&mut self) -> &mut Tags {
        &mut self.tags
    }

    pub fn set(&mut self, tag: Tag) {
        self.tags.set(tag);
    }

    pub fn set_operation(&mut self, key: &str, value: &str) -> Result<()> {
        self.tags.set(Tag::parse(&self.board, key, value)?);
        Ok(())
    }

    pub const ACD: &'static str = "acd";
    pub const BM: &'static str = "bm";
    pub const SM: &'static str = "sm";
    pub const SQ: &'static str = "Sq";
    pub const DM: &'static str = "dm";
    pub const CE: &'static str = "ce";
    pub const DRAW_REJECT: &'static str = "draw_reject";
    pub const ID: &'static str = "id";
    pub const PV: &'static str = "pv";

    pub const ATTRIBUTES: &'static [&'static str] = &[Self::ACD, Self::BM, Self::PV];

    // FIXME - other EPD tags

    pub fn try_get(&self, key: &str) -> Result<&Tag> {
        Ok(self.tags.get(key))
    }

    pub fn tag(&self, key: &str) -> &Tag {
        self.tags.get(key)
    }

    pub fn pv(&self) -> Result<Variation> {
        if let Tag::Pv(v) = self.tag(Tag::PV) {
            Ok(v.clone())
        } else {
            panic!("Expected pv tag in EPD {}", self);
        }
    }

    pub fn supplied_variation(&self) -> &Variation {
        if let Tag::SuppliedVariation(ref v) = self.tag(Tag::SV) {
            v
        } else {
            Variation::empty()
        }
    }

    pub fn branching_factor(&self) -> f64 {
        if let Tag::BranchingFactor(bf) = self.tag(Tag::BF) {
            *bf
        } else {
            0.0
        }
    }

    pub fn bm(&self) -> Result<MoveList, String> {
        if let Tag::BestMoves(ml) = self.tag(Tag::BM) {
            Ok(ml.clone())
        } else {
            Err("Not good".into())
        }
    }

    pub fn sm(&self) -> Result<Move> {
        if let Tag::SuppliedMove(mv) = self.tag(Tag::SM) {
            Ok(*mv)
        } else {
            Err(anyhow!("Not good sm"))
        }
    }

    pub fn sq(&self) -> Result<Bitboard> {
        if let Tag::Squares(s) = self.tag(Tag::SQ) {
            Ok(*s)
        } else {
            Err(anyhow!("Not good sq"))
        }
    }

    pub fn ce(&self) -> Result<i32> {
        if let Tag::CentipawnEvaluation(ce) = self.tag(Tag::CE) {
            Ok(*ce)
        } else {
            Err(anyhow!("Not good ce"))
        }
    }

    // acd analysis count depth [3]
    pub fn acd(&self) -> Result<Ply> {
        if let Tag::AnalysisCountDepth(acd) = self.tag(Tag::ACD) {
            Ok(*acd)
        } else {
            Err(anyhow!("Not good acd"))
        }
    }

    // acd analysis count depth [3]
    pub fn acn(&self) -> Result<u128> {
        if let Tag::AnalysisCountNodes(n) = self.tag(Tag::ACN) {
            Ok(*n)
        } else {
            Err(anyhow!("Not good"))
        }
    }

    pub fn dm(&self) -> Result<u32> {
        if let Tag::DirectMate(dm) = self.tag(Tag::DM) {
            Ok(*dm)
        } else {
            Err(anyhow!("Not good"))
        }
    }

    pub fn id(&self) -> Result<&str> {
        if let Tag::Id(id) = self.tag(Tag::ID) {
            Ok(id)
        } else {
            Err(anyhow!("Not good"))
        }
    }

    // pub fn set_id(&mut self, id: &str) -> &mut Self {
    //     self.set_operation(Self::ID, id)
    // }

    // pub fn draw_reject(&self) -> bool {
    //     self.tags.as_hash_map().get(Self::DRAW_REJECT).is_some()
    // }

    // pub fn validate(&self) -> Result<()> {
    //     for &k in Self::ATTRIBUTES {
    //         if let Some(_) = self.as_hash_map().get(k) {
    //             match k {
    //                 Self::ACD => {
    //                     self.acd()?;
    //                 }
    //                 Self::BM => {
    //                     self.bm()?;
    //                 }
    //                 Self::ID => {
    //                     self.id()?;
    //                 }
    //                 Self::DRAW_REJECT => {
    //                 }
    //                 Self::DM => {
    //                     self.bm()?;
    //                 }
    //                 Self::SQ => {
    //                     self.sq()?;
    //                 }
    //                 Self::PV => {
    //                     self.pv()?;
    //                 }
    //                 _ => {}
    //             }
    //         }
    //     }
    //     Ok(())
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::globals::constants::*;
    use crate::infra::utils::read_file;
    use anyhow::Result;

    // FIXME!!!!
    // #[test]
    // fn test_epd_parse() -> Result<(), String> {
    //     // tags already ASCII ordered
    //     let str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 acd 1000; bm e4; draw_reject; id \"TEST CASE.1\";";
    //     let pos = Position::parse_epd(str)?;
    //     assert_eq!(pos.get("acd").ok(), Some("1000"));
    //     assert_eq!(pos.get(Position::BM).ok(), Some("e4"));
    //     assert_eq!(pos.get("draw_reject").ok(), Some(""));
    //     assert_eq!(pos.id().ok(), Some("TEST CASE.1"));
    //     let mut opcodes = pos.as_hash_map().keys().cloned().collect::<Vec<_>>();
    //     opcodes.sort();
    //     assert_eq!(
    //         opcodes.iter().map(|s| s.as_str()).collect::<Vec::<_>>(),
    //         vec!["acd", "bm", "draw_reject", "id"]
    //     );
    //     assert_eq!(pos.to_string(), str);
    //     Ok(())
    // }

    #[test]
    fn test_epd_parse_many() -> Result<()> {
        let strs = &[
            "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1
            pv 1. Nf6+ gxf6 2. Bxf7#;
            c0 'Henry Buckle vs NN, London, 1840';
            c1 'http://wtharvey.com/m8n2.txt';",
            "r3k2r/p3bpp1/2q1p1b1/1ppPP1B1/3n3P/5NR1/PP2NP2/K1QR4 b kq - 0 1
            pv 1. .. Nb3+ 2. axb3 Qa6#;
            c0 'Alexander Areshchenko vs Sergei Zhigalko, Kiev, 6/6/2013';",
        ];
        let epds = Position::parse_many_epd(strs)?;
        assert_eq!(epds.len(), 2);
        assert_eq!(epds[0].pv()?.len(), 3);
        assert_eq!(
            epds[0].tag("c0"),
            &Tag::Comment(0, "Henry Buckle vs NN, London, 1840".to_string())
        );
        assert_eq!(epds[1].pv()?.len(), 3);
        Ok(())
    }
    #[test]
    fn test_position_uci() -> Result<()> {
        let pos = Position::parse_uci("position startpos")?;
        assert_eq!(pos.board(), &Catalog::starting_board());
        assert_eq!(pos.supplied_variation(), Variation::empty());

        // a7a6 on board of [starting pos + a2a3]
        let bd2 = pos.board.make_move(&pos.board().parse_uci_move("a2a3")?);
        let s2 = "position fen ".to_string() + &bd2.to_fen() + " moves a7a6";
        let pos2 = Position::parse_uci(&s2)?;
        assert_eq!(pos2.supplied_variation().to_uci(), "a7a6");
        Ok(())
    }

    #[test]
    fn test_parse_multiline_epd() -> Result<()> {
        let s = r"
        K....... 
        PPP.....
        ........
        ........
        ........
        ........
        ppppp...
        rnbqk... w KQkq - 0 1";
        let pos = Position::parse_epd(s)?;
        assert_eq!(
            pos.board().to_fen(),
            "K7/PPP5/8/8/8/8/ppppp3/rnbqk3 w KQkq - 0 1"
        );
        Ok(())
    }

    #[test]
    fn test_pos_basics() -> Result<()> {
        let mut pos = Position::default();
        *pos.board_mut() = Board::parse_fen(Catalog::STARTING_POSITION_FEN).unwrap();
        pos.set_operation(Position::BM, "e4")?;
        assert_eq!(pos.bm().unwrap().to_string(), "e2e4");

        let mut pos = Position {
            board: Catalog::starting_board(),
            tags: Tags::default(),
        };
        pos.set_operation(Position::BM, "e4, c4, a4")?;
        pos.set_operation(Position::PV, "e4, e5, d3")?;
        assert_eq!(pos.bm().unwrap().to_string(), "e2e4, c2c4, a2a4");
        assert_eq!(pos.pv().unwrap().to_string(), "e2e4, e7e5, d2d3");
        Ok(())
    }

    // #[test]
    // fn test_serde()  -> Result<()> {
    //     let mut pos = Position { board: Catalog::starting_position(), tags: Tags::default() };
    //     pos.set_operation(Position::BM, "e4, c4, a4")?;
    //     pos.set_operation(Position::PV, "e4, e5, d3")?;
    //     assert_eq!(pos.bm().unwrap().to_string(), "e2e4, c2c4, a2a4");
    //     assert_eq!(pos.pv().unwrap().to_string(), "e2e4, e7e5, d2d3");
    //     Ok(())
    // }

    #[test]
    fn test_pos_custom() -> Result<()> {
        let mut pos = Position::default();
        *pos.board_mut() = Board::parse_fen(Catalog::STARTING_POSITION_FEN).unwrap();
        pos.set_operation(Position::SQ, "e4 e5 e6")?;
        assert_eq!(pos.sq().unwrap(), e4 | e5 | e6);

        pos.set_operation(Position::SQ, "")?;
        assert_eq!(pos.sq().unwrap(), Bitboard::empty());
        Ok(())
    }

    #[test]
    fn test_serde() -> Result<()> {
        let mut pos = Position::from_board(Catalog::starting_board());
        pos.set_operation(Position::BM, "e4")?;
        assert_eq!(
            serde_json::to_string(&pos).unwrap(),
            r#"{"fen":"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1","bm":"e4"}"#
        );
        assert_eq!(
            serde_json::from_str::<Position>(
                r#"{"bm":"e2e4","fen":"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"}"#
            )
            .unwrap(),
            pos
        );
        Ok(())
    }

    #[test]
    fn test_parse_epd_file() -> Result<()> {
        // let positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled.epd")?;
        let positions = Position::parse_many_epd(read_file("../odonata-extras/epd/com15.epd")?)?;
        for p in positions {
            println!(">> {}", p);
        }
        Ok(())
    }
}

// Custom tags
// STS score
// Perft
//

//     attribute:

//     // acd analysis count depth [3]
//     acd: u32,

//     // acn analysis count nodes
//     acn: u64,

//     // acs analysis count seconds
//     acs: u32,

//     // am avoid move(s) - Each operand is a SAN move; they appear in ASCII orde
//     am: MoveList,

//     // bm best move(s)
//     bm: MoveList,
//
//     // c0 comment (primary, also c1 though c9)
//     c: [String;10],

//     // chess clock values
//     cc: (Duration, Duration),

//     // ce centipawn evaluation - from perspective of active player
//     ce: i32,

//     // dm direct mate fullmove count
//     dm: u32,

//     // draw_accept accept a draw offer
//     draw_accept: bool,

//     // draw_claim claim a draw
//     draw_claim: bool,

//     // draw_offer offer a draw
//     draw_offer: bool,

//     // draw_reject reject a draw offer
//     draw_reject: bool,

//     // eco Encyclopedia of Chess Openings opening code
//     eco: String,

//     // fmvn fullmove number
//     fmvn: u16,

//     // hmvc halfmove clock
//     hmvc: u16,

//     // id position identification
//     id: String,

//     // nic New In Chess opening code
//     nic String,

//     // noop no operation
//     noop: bool,

//     // pm predicted move
//     pm: Move;

//     // pv predicted variation
//     pv: MoveList,

//     // rc repetition count
//     rc: u16,

//     // resign game resignation
//     // sm supplied move
//     sm: Move,

//     sv: supplied variation
//     hv: history variation

// https://www.stmintz.com/ccc/index.php?id=314948
// https://www.stmintz.com/ccc/index.php?id=314898
// om opponents mate http://www.talkchess.com/forums/1/message.html?314978

//     Custom (Andy added!)
//     squares. A series of squares in uci format.
//     Useful for identifying attackers, pinned pieces etc
//     Sq: Squares,

//     // tcgs telecommunication game selector
//     // tcri telecommunication receiver identification
//     // tcsi telecommunication sender identification
//     // v0 variation name (primary, also v1 though v9)
//     v: [String;10]
// }
