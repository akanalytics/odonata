use crate::bitboard::Bitboard;
use crate::board::boardbuf::BoardBuf;
use crate::board::Board;
use crate::movelist::MoveList;
use crate::movelist::MoveValidator;
use crate::types::{CastlingRights, Color};
use crate::utils::StringUtils;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

// http://jchecs.free.fr/pdf/EPDSpecification.pdf

// BRATKO https://www.stmintz.com/ccc/index.php?id=20631

// https://sites.google.com/site/strategictestsuite/about-1

// shredder test https://www.stmintz.com/ccc/index.php?id=137052

// WAC https://opensource.apple.com/source/Chess/Chess-311/sjeng/tests/wac.epd.auto.html
// http://www.talkchess.com/forum3/viewtopic.php?t=62576  WAC with scores

// https://www.chessprogramming.org/Extended_Position_Description

// http://www.talkchess.com/forum3/viewtopic.php?t=69640&start=20

#[derive(Clone, Default, Debug)]
pub struct Position {
    board: Board,
    operations: HashMap<String, String>,
}

/// builder methods
impl Position {
    /// 0. Piece placement
    /// 1. Active color
    /// 2. Castling rights
    /// 3. E/P square
    /// 4. Half move clock
    /// 5. Full move counter
    pub fn parse_epd(epd: &str) -> Result<Self, String> {
        let words = epd.split_whitespace().collect::<Vec<_>>();
        if words.len() < 4 {
            return Err(format!("Must specify at least 4 parts in EPD '{}'", epd));
        }
        let mut pos = Position { board: Board::parse_piece_placement(words[0])?, operations: HashMap::new() };
        pos.board.set_turn(Color::parse(words[1])?);
        pos.board.set_castling(CastlingRights::parse(words[2])?);
        if words[3] == "-" {
            pos.board.set_en_passant(Bitboard::EMPTY)
        } else {
            pos.board.set_en_passant(Bitboard::parse_square(words[3])?)
        };

        let mut remaining = StringUtils::trim_first_n_words(epd, 4);
        if words.len() >= 6 {
            let hmvc = words[4].parse::<u16>();
            let fmvn = words[5].parse::<u16>();
            if let Ok(hmvc) = hmvc {
                if let Ok(fmvn) = fmvn {
                    pos.board.set_fifty_halfmove_clock(hmvc as i32);
                    pos.board.set_fullmove_counter(fmvn as i32);
                    remaining = StringUtils::trim_first_n_words(epd, 6);
                }
            }
        }
        pos.operations = Self::parse_operations(remaining);
        Ok(pos)
    }

    fn parse_operations(operations_str: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let ops: Vec<&str> = Self::split_into_operations(operations_str);
        for op in ops {
            let words: Vec<&str> = Self::split_into_words(op);
            debug_assert!(
                words.len() > 0,
                "no words parsing EPD operation '{}' from '{}'",
                op,
                operations_str
            );
            map.insert(words[0].to_string(), words[1..].join(" ").to_string());
        }
        map
    }

    fn split_into_operations(s: &str) -> Vec<&str> {
        let re = Regex::new(
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
        );
        // for cap in re.unwrap().captures_iter(s) {
        //     println!("{:?}", cap, cap.get(1).or(cap(get(2))) );
        // }
        let vec = re
            .unwrap()
            .captures_iter(s)
            .map(|cap| cap.get(1).or(cap.get(2)).or(cap.get(3)).unwrap().as_str())
            .collect();
        vec
    }

    fn split_into_words(s: &str) -> Vec<&str> {
        let re = Regex::new(
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
        );
        // for cap in re.clone().unwrap().captures_iter(s) {
        //      println!("{:?}", cap );
        // }
        let vec = re
            .unwrap()
            .captures_iter(s)
            .map(|cap| cap.get(1).or(cap.get(2)).or(cap.get(3)).unwrap().as_str())
            .collect();
        vec
    }

    pub fn parse_many_epd<I>(iter: I) -> Result<Vec<Position>, String>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut vec = Vec::<Position>::new();
        for item in iter {
            let s = item.as_ref();
            if !s.is_empty() {
                vec.push(Self::parse_epd(s)?);
            }
        }
        Ok(vec)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.board().to_fen())?;
        let ops = self.operations();
        let mut entries = ops.iter().collect::<Vec<_>>();
        entries.sort();
        for (k, v) in entries {
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

impl Position {
    pub const VERB_DATA_NORMALIZATION: &'static str = "pfdn";
    pub const VERB_GENERAL_ANALYSIS: &'static str = "pfga";
    pub const VERB_MATE_SEARCH: &'static str = "pfms";
    pub const VERB_OPERATION_PURGE: &'static str = "pfop";
    pub const VERB_TARGET_SEARCH: &'static str = "pfts";

    pub fn board(&self) -> &Board {
        &self.board
    }
    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn operations(&self) -> HashMap<String, String> {
        self.operations.clone()
    }
    pub fn set_operation(&mut self, key: &str, value: &str) {
        self.operations.insert(key.to_string(), value.to_string());
    }

    pub const ACD: &'static str = "acd";
    pub const BM: &'static str = "bm";
    pub const DM: &'static str = "dm";
    pub const DRAW_REJECT: &'static str = "draw_reject";
    pub const ID: &'static str = "id";
    pub const PV: &'static str = "pv";

    pub const ATTRIBUTES: &'static [&'static str] = &[Self::ACD, Self::BM, Self::PV];

    // FIXME - other EPD operations

    pub fn get(&self, key: &str) -> Result<&str, String> {
        self.operations.get(key).map(|s: &String| s.as_str()).ok_or(format!("No attribute '{}'", key))
    }

    pub fn pv(&self) -> Result<MoveList, String> {
        self.board.parse_san_moves(self.get(Self::PV)?)
    }

    pub fn bm(&self) -> Result<MoveList, String> {
        self.board.parse_san_choices(self.get(Self::BM)?)
    }

    // acd analysis count depth [3]
    pub fn acd(&self) -> Result<u32, String> {
        self.get(Self::ACD)?.parse::<u32>().map_err(|e| e.to_string())
    }

    pub fn dm(&self) -> Result<u32, String> {
        self.get(Self::DM)?.parse::<u32>().map_err(|e| e.to_string())
    }

    pub fn id(&self) -> Result<&str, String> {
        self.get(Self::ID)
    }

    pub fn draw_reject(&self) -> bool {
        self.operations.get(Self::DRAW_REJECT).is_some()
    }

    pub fn validate(&self) -> Result<(), String> {
        for &k in Self::ATTRIBUTES {
            if let Some(_) = self.operations().get(k) {
                match k {
                    Self::ACD => {
                        self.acd()?;
                    }
                    Self::BM => {
                        self.bm()?;
                    }
                    Self::ID => {
                        self.id()?;
                    }
                    Self::DRAW_REJECT => {}
                    Self::DM => {
                        self.bm()?;
                    }
                    Self::PV => {
                        self.pv()?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn test_split_into_operations() {
        let vec = Position::split_into_operations(r#"cat"meo;w";"mouse";"toad;;;;;;" ;zebra;"#);
        assert_eq!(vec, vec!["cat\"meo;w\"", "\"mouse\"", "\"toad;;;;;;\" ", "zebra"]);

        let vec = Position::split_into_operations(r#"cat'meo;w';'mouse';'toad;;;;;;' ;zebra;"#);
        assert_eq!(vec, vec!["cat\'meo;w\'", "\'mouse\'", "\'toad;;;;;;\' ", "zebra"]);

        let vec = Position::split_into_operations(r#";cat;mouse;toad;;;;;;sheep;zebra"#);
        assert_eq!(vec, vec!["cat", "mouse", "toad", "sheep"]);

        // FIXME! OK, but not desirable (unmatched quote parsing)
        let vec = Position::split_into_operations(r#";ca"t;mouse;"#);
        assert_eq!(vec, vec!["t", "mouse"]);
        // let vec = split_on_regex("cat;mat;sat;");
        // assert_eq!(vec, vec!["cat;", "mat;", "sat;"], "cat;mat;sat;");
        // let vec = split_on_regex("cat \"hello\";mat;sat;");
        // assert_eq!(vec, vec!["cat \"hello\";", "mat;", "sat;"], "cat;mat;sat;");
    }

    #[test]
    fn test_split_words() {
        let vec = Position::split_into_words(r#"bm e4"#);
        assert_eq!(vec, vec!["bm", "e4"]);

        let vec = Position::split_into_words(r#"id "my name is bob""#);
        assert_eq!(vec, vec!["id", "my name is bob"]);

        let vec = Position::split_into_words(r#"id 'my name is bob'"#);
        assert_eq!(vec, vec!["id", "my name is bob"]);
    }

    #[test]
    fn test_epd_parse() -> Result<(), String> {
        // operations already ASCII ordered
        let str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 acd 1000; bm e4; draw_reject; id \"TEST CASE.1\";";
        let epd = Position::parse_epd(str)?;
        assert_eq!(epd.get("acd").ok(), Some("1000"));
        assert_eq!(epd.get(Position::BM).ok(), Some("e4"));
        assert_eq!(epd.get("draw_reject").ok(), Some(""));
        assert_eq!(epd.id().ok(), Some("TEST CASE.1"));
        let mut opcodes = epd.operations().keys().cloned().collect::<Vec<_>>();
        opcodes.sort();
        assert_eq!(
            opcodes.iter().map(|s| s.as_str()).collect::<Vec::<_>>(),
            vec!["acd", "bm", "draw_reject", "id"]
        );
        assert_eq!(epd.to_string(), str);
        Ok(())
    }

    #[test]
    fn test_epd_parse_many() -> Result<(), String> {
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
        assert_eq!(epds[1].pv()?.len(), 3);
        Ok(())
    }

    #[test]
    fn test_epd_basics() {
        let mut epd = Position::default();
        *epd.board_mut() = Board::parse_fen(Catalog::STARTING_POSITION_FEN).unwrap();
        epd.set_operation(Position::BM, "e4");
        assert_eq!(epd.bm().unwrap().to_string(), "e2e4");

        let mut epd = Position { board: Catalog::starting_position(), operations: HashMap::default() };
        epd.set_operation(Position::BM, "e4, c4, a4");
        epd.set_operation(Position::PV, "e4, e5, d3");
        assert_eq!(epd.bm().unwrap().to_string(), "e2e4, c2c4, a2a4");
        assert_eq!(epd.pv().unwrap().to_string(), "e2e4, e7e5, d2d3");
    }
}

// Custom operations
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

//     // tcgs telecommunication game selector
//     // tcri telecommunication receiver identification
//     // tcsi telecommunication sender identification
//     // v0 variation name (primary, also v1 though v9)
//     v: [String;10]
// }
