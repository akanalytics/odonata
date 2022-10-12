use crate::board::Board;
use crate::catalog::Catalog;
use crate::domain::info::BareMoveVariation;
use crate::domain::SearchResults;
use crate::infra::utils::{Formatting, Uci};
use crate::movelist::strip_move_numbers;
use crate::other::outcome::Outcome;
use crate::piece::{Ply, ScoreWdl};
use crate::search::timecontrol::TimeControl;
use crate::variation::Variation;
use crate::Color;
use crate::{eval::score::Score, mv::BareMove};
use anyhow::{Context, Result};
use indexmap::{indexmap, IndexMap};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use tabled::{Style, Table, Tabled};


// https://tim-mann.org/Standard
//
// reduced export format
// STR seven tag roster
// {} or ; style comments
// % = other software
// 4.   white
// 4... black (starting move or comment)
// move suffix "!", "?", "!!", "!?", "?!", and "??"
// Numeric Annotation Glyph $2

// Tags
//
// WhiteType, BlackType human/program
//
// Termination
// * "abandoned": abandoned game.
// * "adjudication": result due to third party adjudication process.
// * "death": losing player called to greater things, one hopes.
// * "emergency": game concluded due to unforeseen circumstances.
// * "normal": game terminated in a normal fashion.
// * "rules infraction": administrative forfeit due to losing player's failure to observe either the Laws of Chess or the event regulations.
// * "time forfeit": loss due to losing player's failure to meet time control requirements.
// * "unterminated": game not terminated.
//
// PlyCount
//

// [%clk 1:05:23]
// [%emt 0:05:42]}
//[ %egt 0:05:42]}
// [%eval 250,5] meaning white is +2.5 pawn up at depth 5
// [%eval +0.25] means white is 0.25 pawn up

#[derive(Clone, Debug)]
pub struct GameHeader {
    tag_pairs: IndexMap<String, String>,
    starting_pos: Board,
    tc: TimeControl,
}

impl Default for GameHeader {
    fn default() -> Self {
        Self {
            tag_pairs: indexmap! {
                "Event".to_string() => "?".to_string(),
                "Site".to_string() => "?".to_string(),
                "Date".to_string() => "????.??.??".to_string(),
                "Round".to_string() => "?".to_string(),
                "White".to_string() => "?".to_string(),
                "Black".to_string() => "?".to_string(),
                "Result".to_string() => "*".to_string(),
            },
            starting_pos: Catalog::starting_board(),
            tc: TimeControl::DefaultTime,
        }
    }
}

impl GameHeader {
    pub fn starting_pos(&self) -> &Board {
        &self.starting_pos
    }

    pub fn time_control(&self) -> &TimeControl {
        &self.tc
    }

    pub fn player(&self, c: Color) -> &str {
        &self.tag_pairs[c.chooser_wb("White", "Black")]
    }
}

impl Uci for GameHeader {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.tag_pairs
            .iter()
            .try_for_each(|(k, v)| writeln!(f, "[{k} \"{v}\"]"))
    }

    // [Event "GRENKE Chess Classic 2019"]
    // [Site "Karlsruhe/Baden Baden GER"]
    // [Date "2019.04.20"]
    // [Round "1.2"]
    // [White "Svidler, Peter"]
    // [Black "Caruana, Fabiano"]
    // [Result "1/2-1/2"]
    // [WhiteTitle "GM"]
    // [BlackTitle "GM"]
    // [WhiteElo "2735"]
    // [BlackElo "2819"]
    // [ECO "B33"]
    // [Opening "Sicilian"]
    // [Variation "Pelikan (Lasker/Sveshnikov) variation"]
    // [WhiteFideId "4102142"]
    // [BlackFideId "2020009"]
    // [EventDate "2019.04.20"]
    // [WhiteACPL "252"]
    // [BlackACPL "141"]
    // [GameDuration "00:00:23"]
    // [GameEndTime "2022-10-08T18:49:37.228 BST"]
    // [GameStartTime "2022-10-08T18:49:13.587 BST"]
    // [PlyCount "133"]
    // [TimeControl "75+0.6"]
    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut gh = GameHeader::new();
        for line in s.lines() {
            debug!("Parsing game header line '{line}'...");
            let l = line
                .trim_start()
                .strip_prefix("[")
                .ok_or(anyhow::anyhow!("Missing '[' in pgn tag pair line '{line}'"))?;
            let (k, rest) = l
                .split_once(" ")
                .ok_or_else(|| anyhow::format_err!("No tag in line '{line}'"))?;
            let (v, rest) = rest
                .split_once("]")
                .ok_or_else(|| anyhow::format_err!("No ']' in line '{line}'"))?;
            let v = v.trim_matches('"').to_string();
            if !rest.trim().is_empty() {
                anyhow::bail!("Extraneous text found in '{line}'");
            }
            if k == "FEN" {
                gh.starting_pos = Board::parse_fen(&v)?;
            }
            if k == "TimeControl" {
                gh.tc = TimeControl::parse(&v)?;
            }
            gh.tag_pairs.insert(k.to_string(), v);
        }

        Ok(gh)
    }
}

impl GameHeader {
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Default, Debug)]
pub struct GameMove {
    mv: BareMove,
    sr: SearchResults,
    tc: TimeControl,
    comment: String,
}

static REGEX_MOVE_AND_COMMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)         # x flag to allow whitespace and comments
    ((\d)+\.)?(\s)*(\.\.\s)?      # digits a '.' and then whitespace and optionally ".."
    (?P<move>\S+)
    ((\s)* \{  (?P<comment>[^\}]+)  \} \s*)?              # word(move) space {comment} space
    "#,
    )
    .unwrap()
});

fn match_move_and_comment(s: &str) -> anyhow::Result<(&str, Option<&str>, &str)> {
    let caps = REGEX_MOVE_AND_COMMENT
        .captures(&s)
        .ok_or_else(|| anyhow::anyhow!("Unable to parse '{}' as a move and comment", s))?;
    let mv = caps
        .name("move")
        .with_context(|| format!("expected a move in '{s}'"))?
        .as_str();
    let comment = caps.name("comment").map(|m| m.as_str());
    // 0th capture is entire group
    let rest = &s[caps.get(0).unwrap().end()..];
    Ok((mv, comment, rest))
}

fn fmt_moves_uci(moves: &[GameMove], bd: &Board, f: &mut fmt::Formatter) -> fmt::Result {
    let mut bd = bd.clone();
    for (i, gm) in moves.iter().enumerate() {
        let mv = bd.augment_move(gm.mv);
        if bd.color_us() == Color::White {
            write!(f, "{fmvn}. ", fmvn = bd.fullmove_number())?;
        }
        if i == 0 && bd.color_us() == Color::Black {
            write!(f, "..")?;
        }
        write!(f, "{san} ", san = bd.to_san(&mv))?;
        if !gm.comment.is_empty() {
            write!(f, "{{{comment}}} ", comment = gm.comment)?;
        }
        if bd.color_us() == Color::Black {
            writeln!(f)?;
        }
        bd = bd.make_move(&mv);
    }
    Ok(())
}

fn parse_moves_uci(bd: &Board, s: &str) -> anyhow::Result<(Vec<GameMove>, Outcome)> {
    let mut bd = bd.clone();
    let mut vec = vec![];
    for line in s.lines() {
        let line = strip_move_numbers(line);
        let mut line = line.as_str();
        loop {
            debug!("Game moves parsing line: '{line}'");
            let (mv, comment, rest) = match_move_and_comment(&line)?;
            if let Ok(outcome) = Outcome::try_from_pgn(mv) {
                return Ok((vec, outcome));
            }
            let mv = bd.parse_san_move(mv)?;
            bd = bd.make_move(&mv);
            let gm = GameMove {
                mv: mv.to_inner(),
                sr: SearchResults::default(),
                tc: TimeControl::default(),
                comment: comment.unwrap_or_default().to_string(),
            };
            vec.push(gm);
            line = rest.trim();
            if line.is_empty() {
                break;
            }
        }
    }
    Ok((vec, Outcome::Unterminated))
}

#[derive(Clone, Default, Debug)]
pub struct Game {
    pub game_id: u32,
    header: GameHeader,
    moves: Vec<GameMove>,
    comment: String,
    outcome: Outcome,
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "#,best move,depth,seldepth,")?;
        for (i, gm) in self.moves.iter().enumerate() {
            writeln!(f, "{i}")?;
            writeln!(f, "{}", gm.sr.best_move().unwrap_or_default())?;
        }
        Ok(())
    }
}

// struct CsvWriter;

// impl CsvWriter {
//     fn export(mut w: impl Write, v: serde_json::Value) -> Result<()> {
//         for (k, _v) in v.as_object().unwrap().iter() {
//             write!(w, "{},", k)?;
//         }
//         writeln!(w)?;
//         for (_k, v) in v.as_object().unwrap().iter() {
//             write!(w, "{},", v)?;
//         }
//         writeln!(w)?;
//         Ok(())
//     }
// }

impl Game {
    pub fn fmt_pgn(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.header().fmt_uci(f)?;
        writeln!(f)?;
        if !self.comment.is_empty() {
            writeln!(f, "{{{comment}}}", comment = self.comment)?;
        }
        fmt_moves_uci(&self.moves, &self.header.starting_pos(), f)?;
        writeln!(f, " {outcome}", outcome = self.outcome.as_pgn())?;
        writeln!(f)?;
        Ok(())
    }

    fn strip_bom(s: &str) -> &str {
        if s.starts_with("\u{feff}") {
            &s[3..]
        } else {
            s
        }
    }

    pub fn parse_pgn(s: &str) -> PgnParser {
        PgnParser {
            lines: Self::strip_bom(s).lines(),
            n_line: 0,
            err: Ok(()),
        }
    }
}

pub struct PgnParser<'a> {
    lines: std::str::Lines<'a>,
    n_line: i32,
    err: Result<()>,
}

impl<'a> PgnParser<'a> {
    fn next_unfused(&mut self) -> Result<Option<Game>> {
        let mut header = vec![];
        while let Some(line) = self.lines.next() {
            if line.trim().is_empty() && !header.is_empty() {
                // blank line after finding some header
                break;
            }
            debug!("Header: '{line}'");
            if !line.trim().is_empty() {
                header.push(line);
            }
        }
        if header.is_empty() {
            return Ok(None);
        }

        let mut body = vec![];
        let mut comment = "";
        while let Some(line) = self.lines.next() {
            if line.trim().is_empty() {
                break;
            }
            if body.is_empty() && line.starts_with("{") {
                debug!("Comment: '{line}'");
                comment = line.trim_start_matches('{').trim_end_matches('}');
            } else {
                debug!("Body: '{line}'");
                body.push(line);
            }
        }
        if body.is_empty() {
            anyhow::bail!("Didnt find body");
        }
        let header = GameHeader::parse_uci(&header.join("\n"))?;
        debug!("Parsed game header: {header:?}");

        let starting_pos = header.starting_pos();
        let body_text = body.join("\n");
        let (moves, outcome) = parse_moves_uci(starting_pos, &body_text)?;
        let game = Game {
            game_id: 0,
            header,
            comment: comment.to_string(),
            moves,
            outcome,
        };
        Ok(Some(game))
    }
}

impl<'a> Iterator for PgnParser<'a> {
    type Item = Result<Game>;

    fn next(&mut self) -> Option<Result<Game>> {
        if self.err.is_err() {
            return None;
        }
        self.next_unfused().transpose()
    }
}

impl Game {
    // fn play(&mut self, eng: &mut dyn Engine ) -> anyhow::Result<SearchResults> {
    //     let position = self.position_at(ply);
    //     let sr = eng.search(position, game.time_control(ply));
    //     game.ply += 1;
    //     sr
    // }
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    // for non-fisher, its the (non-changing) time control from the game
    // for fischer, we take the time control of {ply - 1} and subtract off
    // the move time for {ply - 1}
    // pub fn time_control(&self, ply: Ply) -> TimeControl {
    //     let game_tc = self.header().time_control();
    //     if let TimeControl::Fischer(game_rt) = game_tc {
    //         if ply == 0 { TimeControl::Fischer(*game_rt) }
    //         else {
    //             let last = self.moves[ply as usize - 1];
    //             if let TimeControl::Fischer(rt) = last.tc {  // kibitzer time
    //                 let move_time = last.sr.time_millis; // kibitzer time
    //                 let time_left = rt.less_move_millis(color, move_time);
    //                 let moves_left = time_left.moves_to_go - 1;
    //                 if moves_left == 0 {
    //                     moves_to_go = game_rt.moves_to_go;
    //                     set color time again;
    //                 }

    //                 *last
    //             } else {
    //                 panic!("Game is Fisher, last move isn't")
    //             }
    //         }
    //     } else {
    //         game_tc.clone()
    //     }
    // }

    pub fn clear_moves(&mut self) {
        self.moves.clear();
    }

    pub fn header(&self) -> &GameHeader {
        &self.header
    }

    fn variation(&self) -> BareMoveVariation {
        BareMoveVariation::default()
    }

    pub fn set_starting_pos(&mut self, board: Board) -> &mut Self {
        // self.board = pos.supplied_variation().apply_to(pos.board());
        self.header.starting_pos = board;
        self
    }

    pub fn export<W: Write>(&self, mut w: W) -> Result<()> {
        #[derive(Tabled)]
        struct Row {
            id: usize,
            depth: Ply,
            seldepth: Ply,
            time_millis: u64,
            nodes_k: u64,
            nps_k: u64,
            branching_factor: f32,
            hashfull: String,
            mv: BareMove,
            score_pov: Score,
            our_time_secs: f32,
            their_time_secs: f32,
            moves_to_go: u16,
            pv: BareMoveVariation,
        }

        if !self.moves.is_empty() {
            writeln!(
                w,
                "{}",
                Table::new(self.moves.iter().enumerate().map(|(i, gm)| {
                    let mut row = Row {
                        id: i,
                        depth: gm.sr.depth,
                        seldepth: gm.sr.seldepth,
                        time_millis: gm.sr.time_millis,
                        nodes_k: gm.sr.nodes / 1000,
                        nps_k: gm.sr.nps / 1000,
                        branching_factor: gm.sr.bf,
                        hashfull: format!("{}%", gm.sr.hashfull_per_mille / 10),
                        mv: gm.sr.best_move().unwrap_or_default(),
                        score_pov: gm.sr.score(),
                        pv: gm.sr.pv(),
                        our_time_secs: 0.0,
                        their_time_secs: 0.0,
                        moves_to_go: 0,
                    };
                    if let TimeControl::Fischer(rt) = gm.tc {
                        row.our_time_secs = rt.our_time_and_inc().0.as_secs_f32();
                        row.their_time_secs = rt.their_time_and_inc().0.as_secs_f32();
                        row.moves_to_go = rt.moves_to_go;
                    };
                    row
                }))
                .with(Style::markdown())
            )?;
        }
        Ok(())
    }

    pub fn capture_missing_moves(&mut self, var: &Variation) {
        for (i, gm) in self.moves.iter().enumerate() {
            if i < var.len() {
                // we have already captured this move - check its correct
                let existing_mv = gm.mv;
                let new_mv = var[i].to_inner();
                debug_assert!(existing_mv == new_mv, "record_variation: (exising move #{i}) {existing_mv} != {new_mv} (from variation {var})");
            }
        }
        for mv in var.moves().skip(self.moves.len()) {
            self.moves.push(GameMove {
                mv: mv.to_inner(),
                ..GameMove::default()
            });
        }
    }

    pub fn record_search(&mut self, sr: SearchResults, tc: TimeControl) {
        self.moves.push(GameMove {
            mv: sr.best_move().unwrap_or_default(),
            sr,
            tc,
            ..GameMove::default()
        });
    }
}

#[derive(Clone, Debug, Default)]
pub struct GameStats {
    players: HashMap<String, ScoreWdl>,
}

impl GameStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, player: &str, wdl: &ScoreWdl) {
        if let Some(score) = self.players.get_mut(player) {
            *score += *wdl;
        } else {
            self.players.insert(player.to_string(), *wdl);
        }
    }

    pub fn include(&mut self, g: &Game) {
        self.add(g.header().player(Color::White), &g.outcome.as_wdl());
        self.add(g.header().player(Color::Black), &-g.outcome.as_wdl());
    }
}

impl fmt::Display for GameStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Tabled)]
        struct Row<'a> {
            id: usize,
            player: &'a str,
            played: i32,
            won: i32,
            drawn: i32,
            lost: i32,
            points: String,
            elo: String,
        }
        let mut kv_vec = self.players.iter().collect_vec();
        // @todo
        kv_vec.sort_by_cached_key(|(_k,v)| -v.elo() as i64);
        writeln!(
            f,
            "{}",
            Table::new(kv_vec.iter().enumerate().map(|(id, (player, wdl))| {
                let row = Row {
                    id,
                    player,
                    played: wdl.total(),
                    won: wdl.w,
                    drawn: wdl.d,
                    lost: wdl.l,
                    elo: Formatting::decimal(1, wdl.elo()),
                    points: Formatting::decimal(1, wdl.points()),
                };
                row
            }))
            .with(Style::markdown())
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::SearchResults, infra::utils::Displayable};
    use test_log::test;

    #[test]
    fn test_game() {
        let _sr = SearchResults::default();
        // println!("{}", Table::new(vec![sr]).to_string())
    }

    #[test]
    fn parse_move_and_comment() {
        let (mv, c, rest) = match_move_and_comment("a4").unwrap();
        assert_eq!(mv, "a4");
        assert_eq!(c, None);
        assert_eq!(rest, "");

        let (mv, c, rest) = match_move_and_comment("a5 {+0.37/14 4.4s}").unwrap();
        assert_eq!(mv, "a5");
        assert_eq!(c, Some("+0.37/14 4.4s"));
        assert_eq!(rest, "");

        let (mv, c, rest) = match_move_and_comment("a5  { my comment  } ").unwrap();
        assert_eq!(mv, "a5");
        assert_eq!(c, Some(" my comment  "));
        assert_eq!(rest, "");

        let (mv, c, rest) = match_move_and_comment("a5  { my comment  } blob").unwrap();
        assert_eq!(mv, "a5");
        assert_eq!(c, Some(" my comment  "));
        assert_eq!(rest, "blob");
    }

    #[test]
    fn parse_game_string() {
        let s = r###"[Event "1_tc=75+0.6"]
        [Site "?"]
        [Date "2022.10.08"]
        [Round "1"]
        [White "0.6.23:"]
        [Black "0.6.24:"]
        [Result "1/2-1/2"]
        [PlyCount "325"]
        [TimeControl "75+0.6"]
        
        1. e4 {book} c5 {book} 2. Nf3 {book} d6 {book} 3. d4 {book} cxd4 {book}
        4. Nxd4 {book} Nf6 {book} 5. Nc3 {book} a6 {book} 6. Bc4 {+0.37/14 4.4s} 

        [Event "My event"]

        1. e4 {book} c5 {book} 2. Nf3 {book} d6 {book} 3. d4 {book} cxd4 {book}

        "###;
        for game in Game::parse_pgn(s) {
            // println!("{game:#?}");
            match game {
                Ok(game) => println!("{}", Displayable(|f| game.fmt_pgn(f))),
                Err(e) => println!("{e}"),
            }
        }
    }

    #[test]
    fn parse_game_file() {
        let s =
            // std::fs::read_to_string("../odonata-extras/output/games/tourney-26283.pgn").unwrap();
            std::fs::read_to_string("../odonata-extras/pgn/96th Amateur D7 (ChessOK-Pre2022HQ.cgb).pgn").unwrap();
        let mut stats = GameStats::new();
        for game in Game::parse_pgn(&s) {
            match game {
                Ok(game) => {
                    // println!("{}", Displayable(|f| game.fmt_pgn(f)));
                    stats.include(&game);
                }
                Err(e) => panic!("{e}"),
            }
        }
        println!("{stats}");
    }
}
