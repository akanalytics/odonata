use crate::domain::info::BareMoveVariation;
use crate::{eval::score::Score, mv::BareMove};
use crate::domain::SearchResults;
use crate::piece::Ply;
use crate::position::Position;
use crate::search::timecontrol::TimeControl;
use crate::variation::Variation;
use anyhow::Result;
use std::fmt;
use std::io::Write;
use tabled::{Style, Table, Tabled};

// #[derive(Clone, Default)]
// pub struct GameMove {
//     pub tc: TimeControl,
//     pub sr: SearchResults,
// }

#[derive(Clone, Default)]
pub struct Game {
    pub game_id: u32,
    starting_pos: Position,
    moves: Variation,
    // board: Board,
    // _tags: Tags,
    search_results: Vec<SearchResults>,
    // event: String,
    // site: String,
    // date: String,
    // round: String,
    // name_w: String,
    // name_b: String,
    // outcome: Outcome,
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

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "#,best move,depth,seldepth,")?;
        for (i, mv) in self.search_results.iter().enumerate() {
            writeln!(f, "{i}")?;
            writeln!(f, "{}", mv.best_move().unwrap_or_default())?;
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
    pub fn new() -> Self {
        Self::default()
    }

    // pub fn last_search_results(&self) -> SearchResults {
    //     self.moves.last().unwrap_or_default()
    // }

    pub fn clear_moves(&mut self) {
        self.search_results.clear();
        self.moves.clear();
    }

    pub fn set_starting_pos(&mut self, pos: Position) -> &mut Self {
        // self.board = pos.supplied_variation().apply_to(pos.board());
        self.starting_pos = pos;
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

        if !self.search_results.is_empty() {
            writeln!(
                w,
                "{}",
                Table::new(self.search_results.iter().enumerate().map(|(i, s)| {
                    let mut row = Row {
                        id: i,
                        depth: s.depth,
                        seldepth: s.seldepth,
                        time_millis: s.time_millis,
                        nodes_k: s.nodes / 1000,
                        nps_k: s.nps / 1000,
                        branching_factor: s.bf,
                        hashfull: format!("{}%", s.hashfull_per_mille / 10),
                        mv: s.best_move().unwrap_or_default(),
                        score_pov: s.score(),
                        pv: s.pv(),
                        our_time_secs: 0.0,
                        their_time_secs: 0.0,
                        moves_to_go: 0,
                    };
                    if let TimeControl::Fischer(rt) = s.tc {
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

    // pub fn export<W: Write>(&self, mut w: W) -> Result<()> {
    //     if !self.moves.is_empty() {
    //         for (i, sr) in self.moves.iter().enumerate() {
    //             write!(w, "{},", i)?;
    //             write!(w, "{},", sr.depth)?;
    //             write!(w, "{},", sr.seldepth)?;
    //             write!(w, "{},", sr.time_millis)?;
    //             write!(w, "{},", sr.nodes)?;
    //             write!(w, "{},", sr.nps)?;
    //             write!(w, "{},", sr.branching_factor)?;
    //             write!(w, "{},", sr.hashfull_per_mille)?;
    //             write!(w, "{},", sr.best_move().unwrap_or_default())?;
    //             write!(w, "{},", sr.multi_pv().first().unwrap().1)?;
    //             write!(w, "\"{}\",", sr.multi_pv().first().unwrap().0.uci())?;
    //             match sr.tc {
    //                 TimeControl::Fischer(ref tc) => {
    //                     write!(w, "{}", tc.moves_to_go)?;
    //                 }
    //                 _ => {
    //                     write!(w, "{}", "")?;
    //                 }
    //             };
    //             writeln!(w)?;
    //         }
    //         w.flush()?;
    //     }
    //     Ok(())
    // }

    // if !self.moves.is_empty() {
    //     writeln!(w, "{}", "depth,seldepth,nodes,time_millis")?;
    //     for (i, sr) in self.moves.iter().enumerate() {
    //         write!(w, "{},", sr.depth)?;
    //         write!(w, "{},", sr.seldepth)?;
    //         write!(w, "{},", sr.time_millis)?;
    //     }
    // }
    // Ok(())

    pub fn capture_missing_moves(&mut self, var: &Variation) {
        for (i, mv) in var.iter().enumerate() {
            if i < self.moves.len() {
                // we should have already captured this move
                let existing_mv = self.moves[i];
                debug_assert!(existing_mv == *mv, "record_variation: (exising move #{i}) {existing_mv} != {mv} (from variation {var})");
            } else {
                self.moves.push(*mv);
            }
        }
    }

    pub fn record_search(&mut self, sr: SearchResults) {
        self.search_results.push(sr);
    }
}

#[cfg(test)]
mod tests {

    use crate::domain::SearchResults;

    #[test]
    fn test_game() {
        let _sr = SearchResults::default();
        // println!("{}", Table::new(vec![sr]).to_string())
    }
}
