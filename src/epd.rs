use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::types::{Color, CastlingRights};
use crate::bitboard::Bitboard;
use crate::board::boardbuf::BoardBuf;
use crate::movelist::MoveValidator;
use crate::catalog::Catalog;
use crate::movelist::{Move, MoveList};
use crate::outcome::GameEnd;
use crate::outcome::Outcome;
use crate::utils::StringUtils;
use crate::search::algo::Algo;
use std::fmt;
use std::time;
use std::collections::HashMap;
use regex::Regex;


// http://jchecs.free.fr/pdf/EPDSpecification.pdf

// BRATKO https://www.stmintz.com/ccc/index.php?id=20631

// https://sites.google.com/site/strategictestsuite/about-1

// shredder test https://www.stmintz.com/ccc/index.php?id=137052

// WAC https://opensource.apple.com/source/Chess/Chess-311/sjeng/tests/wac.epd.auto.html
// http://www.talkchess.com/forum3/viewtopic.php?t=62576  WAC with scores

// https://www.chessprogramming.org/Extended_Position_Description

// http://www.talkchess.com/forum3/viewtopic.php?t=69640&start=20


#[derive(Clone, Default, Debug)]
pub struct Epd { 
    board: Board,
    attributes: HashMap<String,String>,
}

    
/// builder methods
impl Epd {
    /// 0. Piece placement
    /// 1. Active color
    /// 2. Castling rights
    /// 3. E/P square
    /// 4. Half move clock
    /// 5. Full move counter
    pub fn parse(epd: &str) -> Result<Self, String> {
        
        let words = epd.split_whitespace().collect::<Vec<_>>();
        if words.len() < 4 {
            return Err(format!("Must specify at least 4 parts in EPD '{}'", epd));
        }
        let mut pos = Epd { 
            board: Board::parse_piece_placement(words[0])?,
            attributes: HashMap::new(),
        };
        pos.board.set_turn(Color::parse(words[1])?);
        pos.board.set_castling(CastlingRights::parse(words[2])?);
        if words[3] == "-" { 
            pos.board.set_en_passant(Bitboard::EMPTY) 
        } else { 
            pos.board.set_en_passant(Bitboard::parse_square(words[3])?) 
        };

        let mut _remaining;
        if words.len() >= 6 {
            let hmvc = words[4].parse::<u16>();
            let fmvn = words[5].parse::<u16>();
            if let Ok(hmvc) = hmvc {
                if let Ok(fmvn) = fmvn {
                    pos.board.set_fifty_halfmove_clock(hmvc as i32);
                    pos.board.set_fullmove_counter(fmvn as i32);
                    _remaining = StringUtils::trim_first_n_words(epd, 6);
                }
            }
            _remaining = StringUtils::trim_first_n_words(epd, 4);
        }
        Ok(pos)
    }
}       
        // // parse the attributes
        // Regex re = Regex::new(r#"([^"]+|"(?:\\"|[^"])+");"#


impl Epd {

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

    pub fn attributes(&self) -> HashMap<String,String> {
        self.attributes.clone()
    }
    
    pub fn set_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    pub const ACD: &'static str = "acd";
    pub const BM: &'static str = "bm";
    pub const DM: &'static str = "dm";
    pub const PV: &'static str = "pv";

    pub const ATTRIBUTES: &'static [&'static str] = &[
        Self::ACD, 
        Self::BM,
        Self::PV,
    ];


    // FIXME - other EPD attributes

    pub fn get(&self, key: &str) -> Result<&String, String> {
        self.attributes.get(key).ok_or(format!("No attribute '{}'", key))
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

    pub fn validate(&self) -> Result<(), String> {
        for &k in Self::ATTRIBUTES {
            if let Some(_) = self.attributes().get(k) {
                match k {
                    Self::ACD => { self.acd()?; },
                    Self::BM => { self.bm()?; },
                    Self::DM => { self.bm()?; },
                    Self::PV => { self.pv()?; },
                    _ => {}
                }
            }
        
        }
        Ok(())
    }
}








fn split_on_regex(s: &str) -> Vec<&str> {
    let re = Regex::new(
        r#"(?x)
        (
            [^";]*
            "
            [^"]*
            "
            [^";]*
        )
        ;
        
        |
        
        ([^;"]+)
        ;
        "#
          // |("[^"]*")
        //   r#"(?x)
        //   (?:
        //   ([^"]*);
        //   |
        //   (?:"[^"]*");
        //   )+
        //   "#
      
    );        
    // let splits: Vec<_> = re.unwrap().split(s).into_iter().collect();
    for cap in re.unwrap().captures_iter(s) {
        println!("{:?}", cap);
    }
    // let splits: Vec<_> = re.unwrap().captures_iter(s).map(|mat| mat.as_str()).collect();
    vec![""]        
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_split_on_regex() {
        let vec = split_on_regex(r#"cat"he;llo";MAT;"tiny"; book  ; "test;;;;;;" ; ENDING;"#);
        assert_eq!(vec, vec!["cat;", "MAT;"], "cat;mat;");
        // let vec = split_on_regex("cat;mat;sat;");
        // assert_eq!(vec, vec!["cat;", "mat;", "sat;"], "cat;mat;sat;");
        // let vec = split_on_regex("cat \"hello\";mat;sat;");
        // assert_eq!(vec, vec!["cat \"hello\";", "mat;", "sat;"], "cat;mat;sat;");
    }


    #[test]
    fn test_epd_basics() {
        let mut epd = Epd::default();
        *epd.board_mut() = Board::parse_fen(Catalog::STARTING_POSITION_FEN).unwrap();
        epd.set_attribute(Epd::BM, "e4");
        assert_eq!(epd.bm().unwrap().to_string(), "e2e4");

        let mut epd = Epd { board: Catalog::starting_position(), attributes: HashMap::default() };
        epd.set_attribute(Epd::BM, "e4, c4, a4");
        epd.set_attribute(Epd::PV, "e4, e5, d3");
        assert_eq!(epd.bm().unwrap().to_string(), "e2e4, c2c4, a2a4");
        assert_eq!(epd.pv().unwrap().to_string(), "e2e4, e7e5, d2d3");
    }
}






    // Custom attributes
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