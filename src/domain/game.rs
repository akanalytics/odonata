use crate::board::Board;
use crate::catalog::Catalog;
use crate::domain::info::BareMoveVariation;
use crate::domain::SearchResults;
use crate::infra::utils::{Formatting, Uci};
use crate::movelist::strip_move_numbers;
use crate::other::outcome::Outcome;
use crate::piece::{Ply};
use crate::search::timecontrol::{TimeControl, RemainingTime};
use crate::tags::Tag;
use crate::variation::Variation;
use crate::{Color, Position};
use crate::{eval::score::Score, mv::BareMove};
use anyhow::{Context, Result};
use indexmap::{indexmap, IndexMap};
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt;
use std::io::{Write, Read, BufReader, BufRead};
use std::time::Duration;
use tabled::{Style, Table, Tabled};

use super::Player;
use super::info::Info;

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
// [TimeControl "40/960:40/960:40/960"]
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
                Self::EVENT.to_string() => "?".to_string(),
                Self::SITE.to_string() => "?".to_string(),
                Self::DATE.to_string() => "????.??.??".to_string(),
                Self::ROUND.to_string() => "?".to_string(),
                Self::WHITE.to_string() => "?".to_string(),
                Self::BLACK.to_string() => "?".to_string(),
                Self::RESULT.to_string() => "*".to_string(),
            },
            starting_pos: Catalog::starting_board(),
            tc: TimeControl::DefaultTime,
        }
    }
}

impl GameHeader {
    pub const EVENT: &str  = "Event";
    pub const SITE: &str  = "Site";
    pub const DATE: &str  = "Date";
    pub const ROUND: &str  = "Round";
    pub const WHITE: &str  = "White";
    pub const BLACK: &str  = "Black";
    pub const RESULT: &str  = "Result";

    pub fn starting_pos(&self) -> &Board {
        &self.starting_pos
    }

    pub fn description(&self) -> String {
        let event = self.get(Self::EVENT).unwrap_or_default();
        let round = self.get(Self::ROUND).unwrap_or_default();
        let white = self.get(Self::WHITE).unwrap_or_default();
        let black = self.get(Self::BLACK).unwrap_or_default();
        let result = self.get(Self::RESULT).unwrap_or_default();
        format!("evt:'{event}' rnd:{round} '{white}' vs '{black}' {result}")
    }

    pub fn time_control(&self) -> &TimeControl {
        &self.tc
    }

    pub fn set_starting_pos(&mut self, board: Board){
        self.set("FEN", board.to_fen());
        self.starting_pos = board;
    }

    pub fn set_time_control(&mut self, tc: TimeControl){
        match tc { 
            TimeControl::FischerMulti{ .. } => self.set("TimeControl", tc.to_string()),
            _ => self.set("TimeControlOption",   tc.to_string()),
        }
        self.tc = tc;
    }

    pub fn player(&self, c: Color) -> &str {
        &self.tag_pairs[c.chooser_wb("White", "Black")]
    }

    pub fn set_player(&mut self, c: Color, p: &Player) {
        self.set(c.chooser_wb("White", "Black"), p.name.to_string());
        if let Some(elo) = p.elo {
            self.set(c.chooser_wb("WhiteElo", "BlackElo"), elo.to_string());
        }
    }

    pub fn set(&mut self, key: &str, value: String) {
        self.tag_pairs.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.tag_pairs.get(key).map(|v| v.as_str())
    }

}

impl Uci for GameHeader {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.tag_pairs
            .iter()
            .try_for_each(|(k, v)| writeln!(f, "[{k} \"{v}\"]"))
    }



    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut gh = GameHeader::new();
        for line in s.lines() {
            debug!("Parsing game header line '{line}'...");
            let line = Game::strip_bom(line);
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
                gh.tc = TimeControl::parse_pgn(&v)?;
            }
            gh.set(k, v);
        }

        Ok(gh)
    }
}

impl GameHeader {
    fn new() -> Self {
        Self::default()
    }
}


// pov {-16.50/18 0.11s, Black wins by adjudication}
// [%clk 1:05:23]
// [%emt 0:05:42]}
//[ %egt 0:05:42]}
// [%eval 250,5] meaning white is +2.5 pawn up at depth 5
// [%eval +0.25] means white is 0.25 pawn up
// [TimeControl "40/7200:3600"]
// {[%clk 1:55:21]} <-- until next reset
// {[%egt 1:25:42]} <-- elkapsed game time hh:mm::ss
// {[%mct 17:10:42]} <-- mechanical clock time
//
#[derive(Clone, Default, Debug)]
pub struct GameMove {
    mv: BareMove,
    emt: Duration,
    sr: Option<SearchResults>,
    comment: Option<String>,
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

impl GameMove {


    pub fn elapsed_move_time(&self) -> Duration {
        self.emt
    }


    pub fn info(&self) -> Info {
        if let Some(ref sr) = self.sr {
            if let Some(info) = sr.infos.last() {
                return info.clone();
            }
        }
        Info::new()
    }

    fn format_comments(&self) -> String {
        let mut s = String::new();
        if false {
            s += &format!("[%emt {emt}] ", emt = Formatting::hhmmss(self.elapsed_move_time()));
            if let Some(d) = self.info().depth {
                if let Some(score) = self.info().score {
                    if let Some(eval) = score.to_pgn() {
                        s += &format!("[%eval {eval},{d} ] ");
                    }
                }
            }
        } else {
            if let Some(d) = self.info().depth {
                if let Some(score) = self.info().score {
                    if let Some(eval) = score.to_pgn() {
                        s += &format!("{eval}/{d} ");
                    }
                }
            }
            s += &format!("{emt:.3}s", emt = self.elapsed_move_time().as_secs_f64());
        }
        s
    }

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

    fn fmt_moves_pgn(moves: &[GameMove], bd: &Board, f: &mut fmt::Formatter) -> fmt::Result {
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
            if let Some(comment) = &gm.comment {
                write!(f, "{{{c}, {comment}}} ", c = gm.format_comments())?;

            } else {
                write!(f, "{{{c}}} ", c = gm.format_comments())?;
            }
            if bd.color_us() == Color::Black {
                writeln!(f)?;
            }
            bd = bd.make_move(&mv);
        }
        Ok(())
    }

    fn parse_moves_pgn(bd: &Board, s: &str) -> anyhow::Result<(Vec<GameMove>, Outcome)> {
        let mut bd = bd.clone();
        let mut vec = vec![];
        for line in s.lines() {
            let line = strip_move_numbers(line);
            let mut line = line.as_str();
            loop {
                debug!("Game moves parsing line: '{line}'");
                let (mv, comment, rest) = Self::match_move_and_comment(&line)?;
                if let Ok(outcome) = Outcome::try_from_pgn(mv) {
                    return Ok((vec, outcome));
                }
                let mv = bd.parse_san_move(mv)?;
                bd = bd.make_move(&mv);
                let gm = GameMove {
                    mv: mv.to_inner(),
                    comment: comment.map(|s| s.to_string()),
                    .. GameMove::default()
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
            writeln!(f, "{}", gm.mv)?;
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
        GameMove::fmt_moves_pgn(&self.moves, &self.header.starting_pos(), f)?;
        writeln!(f, " {outcome}", outcome = self.outcome.as_pgn())?;
        writeln!(f)?;
        Ok(())
    }

    fn strip_bom(s: &str) -> &str {
        s.trim_start_matches("\u{feff}") 
    }

    pub fn parse_pgn<R: Read>(r: R) -> PgnParser<BufReader<R>> {
        PgnParser {
            lines: BufReader::new(r).lines(),
            n_line: 0,
            err: Ok(()),
        }
    }

    pub fn parse_pgn_string(s: &str) -> PgnParser<BufReader<&[u8]>> {
        let s = Self::strip_bom(s);
        PgnParser {
            lines: BufReader::new(s.as_bytes()).lines(),
            n_line: 0,
            err: Ok(()),
        }
    }
}

pub struct PgnParser<B> {
    lines: std::io::Lines<B>,
    n_line: i32,
    err: Result<()>,
}

impl<B: BufRead> PgnParser<B> {
    fn next_unfused(&mut self) -> Result<Option<Game>> {
        let mut header = vec![];
        while let Some(line) = self.lines.next() {
            let line = line?;
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
        let mut comment = String::new();
        while let Some(line) = self.lines.next() {
            let line = line?;
            if line.trim().is_empty() {
                break;
            }
            if body.is_empty() && line.starts_with("{") {
                debug!("Comment: '{line}'");
                comment = line.trim_start_matches('{').trim_end_matches('}').to_string();
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
        let (moves, outcome) = GameMove::parse_moves_pgn(starting_pos, &body_text)?;
        let game = Game {
            game_id: 0,
            header,
            comment,
            moves,
            outcome,
        };
        Ok(Some(game))
    }
}

impl<R: BufRead> Iterator for PgnParser<R> {
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

    pub fn with_time_control(tc: TimeControl) -> Self {
        let mut g = Game::new();
        g.header_mut().set_time_control(tc);
        g
    }

    pub fn var(&self) -> BareMoveVariation {
        let mut var = BareMoveVariation::new();
        self.moves.iter().for_each(|gm| var.push(gm.mv));
        var
    }

    pub fn total_emt_for_ply(&self, ply: usize) -> Duration {
        self.moves.iter().take(ply).rev().step_by(2).map(|gm| gm.elapsed_move_time()).sum()
    }

    // 0 <= ply <= len
    // ply = 0 is before first move
    // ply = 1 before second move
    // ply = len(game) = after last move
    pub fn starting_pos_for(&self, ply: usize) -> Position {
        let board = self.header.starting_pos();
        let mut pos = Position::from_board(board.clone());
        let var = self.var().take(ply);
        let var = Variation::from_inner(&var, board);
        pos.set(Tag::SuppliedVariation(var));
        pos
    }

    pub fn board_for_ply(&self, ply: usize) -> Board {
        let pos = self.starting_pos_for(ply);
        pos.board().make_moves_old(pos.supplied_variation())
    }


        // let color = self.header.starting_pos().color_us();
        // if ply % 2 == 0 {
        //     color
        // } else {
        //     color.opposite()
        // }


    pub fn time_control_for_ply(&self, ply: usize) -> TimeControl {
        match self.header().time_control() {
            TimeControl::FischerMulti { moves, secs, inc } => {
                let time = Duration::from_secs_f32(secs + ((ply/2) as i32/ moves) as f32 * secs + (ply/2) as f32 * inc);
                let our_color = self.board_for_ply(ply).color_us();
                let their_elapsed = self.total_emt_for_ply(ply);
                let our_elapsed = self.total_emt_for_ply(ply.saturating_sub(1));
                let rt = RemainingTime {
                    our_color,
                    wtime: time - our_color.chooser_wb(our_elapsed, their_elapsed),
                    btime: time - our_color.chooser_wb(their_elapsed, our_elapsed),
                    winc: Duration::from_secs_f32(*inc),
                    binc: Duration::from_secs_f32(*inc),
                    moves_to_go: (moves - (ply/2) as i32 % moves) as u16,
                };
                TimeControl::UciFischer(rt)
            },
            other => other.clone(),
        }
        // let game_tc = self.header().time_control();
        // if let TimeControl::Fischer(game_rt) = game_tc {
        //     if ply == 0 { TimeControl::Fischer(*game_rt) }
        //     else {
        //         let last = self.moves[ply as usize - 1];
        //         if let TimeControl::Fischer(rt) = last.tc {  // kibitzer time
        //             let move_time = last.sr.time_millis; // kibitzer time
        //             let time_left = rt.less_move_millis(color, move_time);
        //             let moves_left = time_left.moves_to_go - 1;
        //             if moves_left == 0 {
        //                 moves_to_go = game_rt.moves_to_go;
        //                 set color time again;
        //             }

        //             *last
        //         } else {
        //             panic!("Game is Fisher, last move isn't")
        //         }
        //     }
        // } else {
        //     game_tc.clone()
        // }
    }

    pub fn clear_moves(&mut self) {
        self.moves.clear();
    }

    pub fn header(&self) -> &GameHeader {
        &self.header
    }

    pub fn header_mut(&mut self) -> &mut GameHeader {
        &mut self.header
    }


    pub fn export<W: Write>(&self, mut w: W) -> Result<()> {
        #[derive(Tabled, Default)]
        struct Row {
            fmvn: usize,
            wb: Color,
            ply: usize,
            depth: Ply,
            seldepth: Ply,
            time_millis: u64,
            nodes_k: u64,
            nps_k: u64,
            branching_factor: f64,
            hashfull: String,
            mv: BareMove,
            score_pov: Score,
            our_time_secs: f32,
            their_time_secs: f32,
            moves_to_go: u16,
            pv: String,
        }

        if !self.moves.is_empty() {
            writeln!(
                w,
                "{}",
                Table::new(self.moves.iter().enumerate().map(|(ply, gm)| {
                    let b = self.board_for_ply(ply);
                    let mut row = Row {
                        fmvn: b.fullmove_number() as usize,
                        wb: b.color_us(),
                        ply,
                        .. Row::default()
                    };
                    if let Some(sr) = &gm.sr {
                        row.depth = sr.depth;
                        row.seldepth = sr.seldepth;
                        row.time_millis = sr.time_millis;
                        row.nodes_k = sr.nodes / 1000;
                        row.nps_k = sr.nps / 1000;
                        row.branching_factor = sr.bf;
                        row.hashfull = format!("{}%", sr.hashfull_per_mille / 10);
                        row.mv = sr.best_move().unwrap_or_default();
                        row.score_pov = sr.score();
                        row.pv = sr.pv().to_san(&b);
                    }
                    // if let TimeControl::Fischer(rt) = gm.tc {
                    //     row.our_time_secs = rt.our_time_and_inc().0.as_secs_f32();
                    //     row.their_time_secs = rt.their_time_and_inc().0.as_secs_f32();
                    //     row.moves_to_go = rt.moves_to_go;
                    // };
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
                debug_assert!(existing_mv == new_mv, 
                    "record_variation: (exising move #{i}) {existing_mv} != {new_mv} (from variation {var})");
            }
        }
        for mv in var.moves().skip(self.moves.len()) {
            self.moves.push(GameMove {
                mv: mv.to_inner(),
                ..GameMove::default()
            });
        }
    }

    pub fn len(&self) -> usize {
        self.moves.len()
    }


    pub fn set_outcome(&mut self, outcome: Outcome) {
        self.outcome = outcome;
        if outcome.is_game_over() {
            self.header_mut().set("Result", outcome.as_pgn());
            let ply = self.len().to_string();
            self.header_mut().set("PlyCount", ply);
        }
    }

    /// sets outcome and result too
    pub fn make_move(&mut self, mv: BareMove) {
        debug!("Game move {ply} {mv}", ply = self.len());
        self.moves.push(GameMove {
            mv,
            .. GameMove::default()
        });
        self.set_outcome(self.board_for_ply(self.moves.len()).outcome());
    }

    /// captures engine search results
    pub fn make_engine_move(&mut self, sr: SearchResults, elapsed: Duration) {
        self.make_move(sr.best_move().unwrap_or_default());
        let mut gm = self.last_move_mut();
        gm.sr = Some(sr);        
        gm.emt = elapsed;

    }

    pub fn last_move_mut(&mut self) -> &mut GameMove {
        self.moves.last_mut().unwrap()
    }

    pub fn outcome(&self) -> &Outcome{
        &self.outcome
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::SearchResults, infra::utils::Displayable};
    use itertools::Itertools;
    use test_log::test;

    #[test]
    fn test_game() {
        let _sr = SearchResults::default();
        let mut game = Game::with_time_control(TimeControl::Depth(5));
        assert_eq!(game.header().starting_pos().color_us(), Color::White);
        assert_eq!(game.header().starting_pos().fullmove_number(), 1);
        assert_eq!(game.board_for_ply(0).fullmove_number(), 1);
        assert_eq!(game.time_control_for_ply(0), TimeControl::Depth(5));

        game.make_move(BareMove::parse_uci("a2a3").unwrap());
        game.make_move(BareMove::parse_uci("h7h6").unwrap());
        println!("{game}");
        println!("{board}", board = game.board_for_ply(2));
        assert_eq!(game.board_for_ply(2).fullmove_number(), 2);

    }

    #[test]
    fn test_tc_for_ply() {
        let mut game = Game::with_time_control(TimeControl::FischerMulti{ moves: 2, secs: 10., inc: 0.});

        let sr = SearchResults::parse_uci("bestmove a2a4").unwrap();
        game.make_engine_move(sr, Duration::from_secs(1));
        let sr = SearchResults::parse_uci("bestmove a7a5").unwrap();
        game.make_engine_move(sr, Duration::from_secs(4));

        let sr = SearchResults::parse_uci("bestmove h2h4").unwrap();
        game.make_engine_move(sr, Duration::from_secs(2));
        let sr = SearchResults::parse_uci("bestmove h7h5").unwrap();
        game.make_engine_move(sr, Duration::from_secs(4));

        let tc = game.time_control_for_ply(0);
        println!("{}",tc);
        assert_eq!(tc.to_string(), "tc=(rt=RemainingTime { our_color: White, wtime: 10s, btime: 10s, winc: 0ns, binc: 0ns, moves_to_go: 2 })");
        println!("{}",game.time_control_for_ply(1));

        println!("{}",game.time_control_for_ply(2));
        println!("{}",game.time_control_for_ply(3));

        let tc = game.time_control_for_ply(4);
        println!("{}",tc);
        assert_eq!(tc.to_string(), "tc=(rt=RemainingTime { our_color: White, wtime: 17s, btime: 12s, winc: 0ns, binc: 0ns, moves_to_go: 2 })");
    }

    #[test]
    fn parse_move_and_comment() {
        let (mv, c, rest) = GameMove::match_move_and_comment("a4").unwrap();
        assert_eq!(mv, "a4");
        assert_eq!(c, None);
        assert_eq!(rest, "");

        let (mv, c, rest) = GameMove::match_move_and_comment("a5 {+0.37/14 4.4s}").unwrap();
        assert_eq!(mv, "a5");
        assert_eq!(c, Some("+0.37/14 4.4s"));
        assert_eq!(rest, "");

        let (mv, c, rest) = GameMove::match_move_and_comment("a5  { my comment  } ").unwrap();
        assert_eq!(mv, "a5");
        assert_eq!(c, Some(" my comment  "));
        assert_eq!(rest, "");

        let (mv, c, rest) = GameMove::match_move_and_comment("a5  { my comment  } blob").unwrap();
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
        for game in Game::parse_pgn_string(s) {
            // println!("{game:#?}");
            match game {
                Ok(game) => println!("{}", Displayable(|f| game.fmt_pgn(f))),
                Err(e) => println!("{e}"),
            }
        }
        let games = Game::parse_pgn_string(s).collect_vec();
        assert_eq!(games.len(), 2);
        assert_eq!(games.first().unwrap().as_ref().unwrap().len(), 11);
        assert_eq!(games.first().unwrap().as_ref().unwrap().var().first(), 
            Some(BareMove::parse_uci("e2e4").unwrap()));

    }

 
}
