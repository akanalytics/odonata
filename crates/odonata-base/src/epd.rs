use std::fmt;
use std::ops::Index;
use std::path::Path;

use anyhow::{bail, Context, Result};
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::bits::bitboard::Bitboard;
use crate::bits::castling::CastlingRights;
use crate::bits::Square;
use crate::boards::board::BoardBuilder;
use crate::boards::Position;
use crate::catalog::Catalog;
use crate::domain::node::Timing;
use crate::infra::metric::Metrics;
use crate::infra::utils::StringUtils;
use crate::other::tags::{TagOps, Tags};
use crate::piece::{Color, Ply};
use crate::prelude::Board;
use crate::variation::Variation;

// http://jchecs.free.fr/pdf/EPDSpecification.pdf
// BRATKO https://www.stmintz.com/ccc/index.php?id=20631
// https://sites.google.com/site/strategictestsuite/about-1
// shredder test https://www.stmintz.com/ccc/index.php?id=137052
// WAC https://opensource.apple.com/source/Chess/Chess-311/sjeng/tests/wac.epd.auto.html
// http://www.talkchess.com/forum3/viewtopic.php?t=62576  WAC with scores
// https://www.chessprogramming.org/Extended_Position_Description
// http://www.talkchess.com/forum3/viewtopic.php?t=69640&start=20

// #[serde(into = "HashMap<String,String>")]
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Epd {
    starting: Board,
    played:   Variation,
    tags:     Tags,
}

impl fmt::Display for Epd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.starting.to_fen())?;
        if !self.played.is_empty() {
            write!(f, " moves {} ", self.played.to_san(&self.starting))?;
        }
        write!(f, "{}", self.tags)?; // will write trailing semicolon and preceding space
        Ok(())
    }
}

impl TagOps for Epd {
    fn tags_mut(&mut self) -> &mut Tags {
        &mut self.tags
    }

    fn tags(&self) -> &Tags {
        &self.tags
    }
}

impl Index<&str> for Epd {
    type Output = str;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).unwrap_or_else(|| panic!("No key '{key}' in EPD {self}"))
    }
}

impl Epd {
    pub fn to_epd(&self) -> String {
        format!(
            "{fen}{tags}",
            fen = self.board().to_fen(),
            tags = self.tags.to_epd(&self.board())
        )
    }

    pub fn has_extension(file: impl AsRef<Path>) -> bool {
        file.as_ref().extension().unwrap_or_default() == "epd" || file.as_ref().extension().unwrap_or_default() == "EPD"
    }

    /// current board position after any moves
    pub fn board(&self) -> Board {
        self.starting.make_moves(&self.played)
    }

    pub fn setup_board(&self) -> Board {
        self.starting.clone()
    }

    pub fn played(&self) -> Variation {
        self.played.clone()
    }

    pub fn into_pos(self) -> Position {
        Position::from_played_moves(self.starting, self.played)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        self.starting.validate()?;
        // validate thye sv before we use it to validate other fields
        self.played.validate(&self.starting)?;
        self.tags.validate(&self.board())?;
        Ok(())
    }

    pub fn int(&self, tag: &str) -> Option<i64> {
        self.tags
            .get(tag)?
            .parse::<i64>()
            .context(self.to_epd())
            .unwrap()
            .into()
    }

    pub fn bitboard(&self, tag: &str) -> Option<Bitboard> {
        Bitboard::parse_squares(self.tags.get(tag)?)
            .context(self.to_epd())
            .unwrap()
            .into()
    }

    pub fn tag(&self, tag: &str) -> Option<&str> {
        self.tags.get(tag)
    }

    pub fn set_tag(&mut self, k: &str, v: &str) {
        self.tags_mut().insert(k, v);
    }

    pub fn merge_tags_from(&mut self, other: Epd, matching: &[&str]) {
        let Epd { mut tags, .. } = other;
        let other_tags = if matching.is_empty() {
            tags
        } else {
            tags.retain(matching);
            tags
        };
        *self.tags_mut() = other_tags;
    }

    pub fn to_uci(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        if self.board() == Catalog::starting_board() {
            write!(s, "position startpos").unwrap();
        } else {
            write!(s, "position fen {fen}", fen = self.setup_board().to_fen()).unwrap();
        }
        let var = self.played();
        if !var.is_empty() {
            write!(s, " moves {}", var.to_uci()).unwrap();
        }
        s
    }

    pub fn parse_uci(s: &str) -> anyhow::Result<Self> {
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
            Variation::parse_uci(var_text.trim_start(), &board)?
        } else if moves.trim().is_empty() {
            Variation::new()
        } else {
            bail!("unexpected text '{moves}' at end of position '{s}'")
        };
        Ok(Epd::from_var(board, var))
    }

    pub fn starting_pos() -> Epd {
        Self {
            starting: Board::starting_pos(),
            played:   Variation::new(),
            tags:     Tags::new(),
        }
    }

    pub fn from_var(starting: Board, played: Variation) -> Self {
        Self {
            starting,
            played,
            ..Self::default()
        }
    }
    pub fn from_board(board: Board) -> Self {
        Self {
            starting: board,
            ..Self::default()
        }
    }

    /// returns null if random sequence resulted in early mate/stalemate
    pub fn play_random_moves<R: Rng + ?Sized>(&self, ply: Ply, r: &mut R) -> Option<Variation> {
        let mut pos = Position::from_epd(self.clone());
        for _p in 0..ply {
            if pos.outcome().is_game_over() {
                // println!("Restarted from {pos} {}", pos.board());
                return None;
            }
            let moves = pos.board().legal_moves().iter().cloned().collect_vec();
            let mv = moves.choose(r).expect("no valid moves");
            pos.push_move(*mv);
        }
        Some(pos.search_variation())
    }

    /// 0. Piece placement
    /// 1. Active color
    /// 2. Castling rights
    /// 3. E/P square
    /// 4. Half move clock
    /// 5. Full move counter
    // TODO! allow newlines replaced by / only in fen, thereafter just ignore
    pub fn parse_epd(epd: &str) -> Result<Self> {
        let t = Metrics::timing_start();
        // replace \n followed by whitespace with "/"
        // let epd = epd.trim_start();
        // let epd = REGEX_CR_PLUS_WS.replace_all(epd, "/");
        // let epd = epd.as_ref();
        let (rest, board) = if let Some(s) = epd.strip_prefix("startpos") {
            (s.trim_start(), Board::starting_pos())
        } else {
            let words = epd.split_whitespace().collect::<Vec<_>>();
            if words.len() < 4 {
                bail!("must specify at least 4 parts in EPD '{}'", epd);
            }
            let mut bb = BoardBuilder::parse_piece_placement(words[0])?;
            bb.set_turn(Color::parse(words[1])?);
            bb.set_castling(CastlingRights::parse(words[2])?);
            if words[3] == "-" {
                bb.set_ep_square(None);
            } else {
                bb.set_ep_square(Some(Square::parse(words[3])?));
            };

            let mut remaining = StringUtils::trim_first_n_words(epd, 4);
            if words.len() >= 6 {
                let hmvc = words[4].parse::<u16>();
                let fmvn = words[5].parse::<u16>();
                if let Ok(hmvc) = hmvc {
                    if let Ok(fmvn) = fmvn {
                        bb.set_halfmove_clock(hmvc);
                        bb.set_fullmove_number(fmvn);
                        remaining = StringUtils::trim_first_n_words(epd, 6);
                    }
                }
            }
            (remaining, bb.try_build()?)
        };
        let (rest, played) = Self::parse_moves(&board, rest)?;
        // use root board
        let tags = Tags::parse_epd_tags(&board.make_moves(&played), rest)?;
        Metrics::profile(t, Timing::TimingParseEpd);
        Ok(Self {
            starting: board,
            played,
            tags,
        })
    }

    fn parse_moves<'a>(starting: &Board, s: &'a str) -> Result<(&'a str, Variation)> {
        let Some(s) = s.strip_prefix("moves") else {
            return Ok((s.trim_start(), Variation::new()));
        };
        if let Some((var, rest)) = s.split_once(';') {
            Ok((rest.trim(), Variation::parse_san(var, starting)?))
        } else {
            Ok(("", Variation::parse_san(s, starting)?))
        }
    }

    pub fn find_by_id<'a>(id: &str, epds: &'a [Epd]) -> Option<&'a Epd> {
        for p in epds.iter() {
            if let Some(pos_id) = p.tag("id") {
                if pos_id == id {
                    return Some(p);
                }
            }
        }
        None
    }

    pub fn parse_many_epd(iter: impl IntoIterator<Item = impl AsRef<str>>) -> Result<Vec<Epd>> {
        let mut vec = Vec::<Epd>::new();
        for item in iter {
            let s = item.as_ref();
            let multiline = s.contains('\n');
            let comment = s.trim_start().starts_with('#');
            if comment && !multiline {
                continue;
            }
            if !s.trim().is_empty() {
                vec.push(Self::parse_epd(s.replace('\n', " ").trim_start()).with_context(|| format!("in EPD '{s}'"))?);
            }
        }
        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng as _;
    use rand_chacha::ChaChaRng;
    use test_log::test;

    use super::*;
    use crate::infra::utils::read_file;
    use crate::other::tags::EpdOps as _;
    use crate::prelude::*;

    #[test]
    fn test_epd_parse1() {
        // tags already ASCII ordered
        let str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 acd 1000; bm e4; draw_reject; id \"TEST CASE.1\";";
        let epd = Epd::parse_epd(str).unwrap();
        assert_eq!(epd.int("acd"), Some(1000));
        assert_eq!(epd.tag("bm"), Some("e4"));
        assert_eq!(epd.tag("draw_reject"), Some(""));
        assert_eq!(epd.tag("id"), Some("TEST CASE.1"));
        let mut opcodes = epd.tags().keys().collect_vec();
        opcodes.sort();
        assert_eq!(opcodes.into_iter().collect_vec(), vec![
            "acd",
            "bm",
            "draw_reject",
            "id"
        ]);
        assert_eq!(epd.to_string(), str, "{epd:?}");
    }

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
        let epds = Epd::parse_many_epd(strs)?;
        assert_eq!(epds.len(), 2);
        assert_eq!(epds[0].var("pv").unwrap().len(), 3);
        assert_eq!(epds[0].tag("c0"), Some("Henry Buckle vs NN, London, 1840"));
        assert_eq!(epds[1].var("pv").unwrap().len(), 3);
        Ok(())
    }
    #[test]
    fn test_epd_uci() -> Result<()> {
        let epd = Epd::parse_uci("position startpos")?;
        assert_eq!(epd.board(), Catalog::starting_board());
        assert_eq!(epd.played(), Variation::new());

        // a7a6 on board of [starting pos + a2a3]
        let mut pos = epd.into_pos();
        pos.push_moves_str("a2a3")?;
        let s2 = "position fen ".to_string() + &pos.board().to_fen() + " moves a7a6";
        let epd_a2a3_e7e6 = Epd::parse_uci(&s2)?;
        assert_eq!(epd_a2a3_e7e6.played().to_uci(), "a7a6");
        Ok(())
    }

    #[test]
    fn test_epd_basics() -> Result<()> {
        let mut pos = Epd::starting_pos();
        pos.set_tag("bm", "e4");
        assert_eq!(pos.moves("bm").unwrap().to_string(), "e2e4");

        let mut pos = Epd::from_board(Catalog::starting_board());
        pos.set_tag("bm", "e4, c4, a4");
        pos.set_tag("pv", "e4 e5 d3");
        assert_eq!(pos.moves("bm").unwrap().to_string(), "e2e4, c2c4, a2a4");
        assert_eq!(pos.var("pv").unwrap().to_string(), "e2e4.e7e5.d2d3");
        Ok(())
    }

    #[test]
    fn test_epd_custom() {
        let mut epd = Epd::starting_pos();
        epd.set_tag("Sq", "e4 e5 e6");
        assert_eq!(epd.bitboard("Sq"), Some(Square::E4 | Square::E5 | Square::E6));

        epd.set_tag("Sq", "");
        assert_eq!(epd.bitboard("Sq").unwrap(), Bitboard::empty());
    }

    #[test]
    fn test_epd_parse() -> Result<()> {
        assert_eq!(Epd::parse_epd("startpos")?, Epd::from_board(Board::starting_pos()));
        assert_eq!(Epd::parse_epd("startpos;")?, Epd::from_board(Board::starting_pos()));
        assert_eq!(
            Epd::parse_epd("startpos moves")?,
            Epd::from_board(Board::starting_pos())
        );
        assert_eq!(
            Epd::parse_epd("startpos moves;")?,
            Epd::from_board(Board::starting_pos())
        );
        assert_eq!(
            Epd::parse_epd("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?,
            Epd::from_board(Board::starting_pos())
        );
        let mut pos = Position::starting_pos();
        pos.push_moves_str("e2e4")?;
        let epd_e2e4 = Epd {
            starting: Board::starting_pos(),
            played:   pos.search_variation().clone(),
            tags:     Tags::new(),
        };
        assert_eq!(Epd::parse_epd("startpos moves e2e4")?, epd_e2e4);
        assert_eq!(Epd::parse_epd("startpos moves e4")?, epd_e2e4);
        assert_eq!(Epd::parse_epd("startpos moves e8").is_err(), true);

        let s1 = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 id 'test1'; bm e4 d4;";
        let s2 = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 id 'test2'; am a3 h3;";
        let mut epd1 = Epd::parse_epd(s1).unwrap();
        let epd2 = Epd::parse_epd(s2).unwrap();
        assert_eq!(epd1.tag("id").unwrap(), "test1");
        assert_eq!(epd1.moves("bm").unwrap().to_san(&epd1.board()), "e4 d4");
        epd1.merge_tags_from(epd2.clone(), &["pv"]);
        assert_eq!(epd1.tag("am").is_none(), true);

        epd1.merge_tags_from(epd2, &["am"]);
        assert_eq!(epd1.tag("am").is_some(), true);
        Ok(())
    }

    #[test]
    fn test_epd_file_parse() -> Result<()> {
        // let positions = Position::parse_epd_file("../odonata-extras/epd/quiet-labeled.epd")?;
        let positions = Epd::parse_many_epd(read_file("../../ext/epd/com15.epd")?)?;
        for p in positions {
            println!(">> {}", p);
        }
        Ok(())
    }

    #[test]
    fn test_random_epd() {
        let mut rand = ChaChaRng::seed_from_u64(1);
        let mut n_game_over = 0;
        for _i in 0..1000 {
            let var = Epd::starting_pos().play_random_moves(50, &mut rand);
            match var {
                Some(var) => println!("{var}"),
                None => {
                    println!("game over");
                    n_game_over += 1;
                }
            }
        }
        println!("Game over count {n_game_over}");
    }
}
