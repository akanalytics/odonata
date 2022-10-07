use std::iter::{self, FromIterator};

use crate::eval::eval::Eval;
use crate::eval::score::Score;
use crate::infra::utils::{calculate_branching_factor_by_nodes_and_depth, Uci};
use crate::mv::{BareMove, Move};
use crate::other::outcome::Outcome;
use crate::piece::Ply;
use crate::search::timecontrol::TimeControl;
use crate::tags::Tag;
use crate::variation::Variation;
use crate::{board::Board, Algo, MoveList, Position};
use serde::{Deserialize, Serialize};
use std::fmt;
use tabled::builder::Builder;

use super::info::Info;

#[derive(Clone, Default, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchResults {
    #[serde(skip)]
    pub board: Board,
    pub depth: Ply,
    pub seldepth: Ply,
    pub time_millis: u64,
    pub nodes: u64,
    pub nodes_thread: u64,
    pub nps: u64,
    pub tbhits: u64,
    pub bf: f32,
    pub hashfull_per_mille: u32,
    pub outcome: Outcome,

    #[serde(skip)]
    pub multi_pv: Vec<(Variation, Score)>,

    #[serde(skip)]
    pub tc: TimeControl,
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        Ok(())
    }
}

pub struct SearchResultsWithExplanation<'a> {
    sr: &'a SearchResults,
    eval: &'a Eval,
}

impl fmt::Display for SearchResultsWithExplanation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.sr)?;
        let mut bu = Builder::new();
        bu.set_columns(["Score", "PV", "Explain"]);
        for pv in &self.sr.multi_pv {
            let b = pv.0.apply_to(&self.sr.board);
            bu.add_record([
                pv.1.to_string(),
                pv.0.to_string(),
                format!("{}\n{}", b, self.eval.w_eval_explain(&b)),
            ]);
        }
        writeln!(f, "{}", bu.build())?;
        Ok(())
    }
}

fn parse_bestmove_uci(s: &str) -> anyhow::Result<(BareMove, Option<BareMove>)> {
    let mut words = s.split_whitespace().fuse();
    let (bm, pm) = match (
        words.next(),
        words.next(),
        words.next(),
        words.next(),
        words.next(),
    ) {
        (Some("bestmove"), Some(bm), Some("ponder"), Some(pm), None) => (bm, Some(pm)),
        (Some("bestmove"), Some(bm), None, _, _) => (bm, None),
        (_, _, _, _, Some(_)) => anyhow::bail!("too many words in '{s}'"),
        _ => anyhow::bail!("expected: bestmove bm [ponder pm] but found '{s}'"),
    };
    let bm = BareMove::parse_uci(bm)?;
    let pm = match pm {
        Some(pm) => Some(BareMove::parse_uci(pm)?),
        None => None,
    };
    Ok((bm, pm))
}

impl Uci for SearchResults {
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(mv) = self.best_move() {
            write!(f, "bestmove {mv}", mv = mv.to_uci())?;
        } else {
            write!(f, "bestmove 0000")?;
        }

        if self.pv().len() > 1 {
            write!(f, "ponder {}", self.pv()[1].to_uci())?;
        }
        Ok(())
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut infos = vec![];
        let mut iter = s.lines().peekable();
        // let (bestmove, ponder) = (None, None);
        while let Some(line) = iter.next() {
            if iter.peek().is_none() {
                let (bm, pm) = parse_bestmove_uci(line)?;
                return Ok(SearchResults::from_infos(bm, pm, infos));
            }
            // let mut t = some_iter.tuples();
            // for (prev, next) in t.by_ref() {
            //     println!("{}--{}", prev, next);
            // }
            // for leftover in t.into_buffer() {
            //     println!("{}", leftover);
            // }
            // // parse as last line
            // let mut words = line.split_whitespace();
            // if words.next() == Some("bestmove") {
            //     bestmove = words.next();
            //     if bestmove.is_none() {
            //         anyhow::bail!("word #2 (bestmove) missing from '{line}'");
            //     }
            //     ponder = if words.next() == Some("ponder") {
            //         let ponder = words.next();
            //         if let Some(ponder) = ponder {
            //             BareMove::parse_uci(ponder)?
            //         } else {
            //             anyhow::bail!("word #4 (ponder move) missing from '{line}'");
            //         }
            //     } else {
            //         None
            //     };
            // } else {
            //     anyhow::bail!("expected 'bestmove' but found '{line}'");
            // }
            else {
                infos.push(Info::parse_uci(line)?);
            }
        }
        unreachable!()
    }
}

impl SearchResults {
    pub fn from_infos(_bm: BareMove, _pm: Option<BareMove>, infos: Vec<Info>) -> Self {
        if let Some(info) = infos.last() {
            SearchResults {
                depth: info.depth.unwrap_or_default(),
                seldepth: info.seldepth.unwrap_or_default(),
                time_millis: info.time_millis.unwrap_or_default(),
                nodes: info.nodes.unwrap_or_default(),
                nodes_thread: info.nodes_thread.unwrap_or_default(),
                nps: info.nps.unwrap_or_default(),
                tbhits: info.tbhits.unwrap_or_default(),
                bf: 0.,
                hashfull_per_mille: info.hashfull_per_mille.unwrap_or_default(),
                outcome: Outcome::Unterminated,
                multi_pv: vec![(Variation::new(), info.score.unwrap_or_default())],
                tc: TimeControl::default(),
                board: Board::default(),
            }
        } else {
            SearchResults::default()
        }
    }

    pub fn new(algo: &Algo, depth: Ply, multi_pv: Vec<(Variation, Score)>) -> Self {
        let nodes_thread_last_iter = algo.clock.elapsed_iter_this_thread().1;
        let bf = calculate_branching_factor_by_nodes_and_depth(nodes_thread_last_iter, depth)
            .unwrap_or_default() as f32;
        SearchResults {
            board: algo.board.clone(),
            tc: *algo.mte.time_control(),
            outcome: Outcome::Unterminated,
            tbhits: 0,
            nodes: algo.clock.cumul_nodes_all_threads(),
            nodes_thread: algo.clock.cumul_nodes_this_thread(),
            nps: algo.clock.cumul_knps_all_threads() * 1000,
            depth,
            seldepth: algo.pv_table.selective_depth(),
            time_millis: algo.clock.elapsed_search().0.as_millis() as u64,
            hashfull_per_mille: algo.tt.hashfull_per_mille(),
            bf,
            multi_pv,
        }
    }

    pub fn explain<'a>(&'a self, eval: &'a Eval) -> SearchResultsWithExplanation<'a> {
        SearchResultsWithExplanation { sr: self, eval }
    }

    /// outcome could be abandoned or win/draw reason
    pub fn best_move(&self) -> Result<Move, Outcome> {
        if self.pv().len() > 0 {
            Ok(self.pv()[0])
        } else {
            Err(self.outcome)
        }
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    pub fn pv(&self) -> &Variation {
        if self.multi_pv.len() > 0 {
            &self.multi_pv[0].0
        } else {
            Variation::empty()
        }
    }

    pub fn score(&self) -> Score {
        if self.multi_pv.len() > 0 {
            self.multi_pv[0].1
        } else {
            Score::default()
        }
    }

    pub fn multi_pv(&self) -> &[(Variation, Score)] {
        &self.multi_pv
    }

    pub fn to_position(&self) -> Position {
        let mut pos = Position::from_board(self.board.clone());
        pos.set(Tag::Pv(self.pv().clone()));
        if self.pv().len() > 0 {
            pos.set(Tag::SuppliedMove(self.pv()[0]));
            pos.set(Tag::BestMoves(MoveList::from_iter(iter::once(
                self.pv()[0],
            ))));
        }
        pos.set(Tag::CentipawnEvaluation(self.score().as_i16() as i32));
        pos.set(Tag::AnalysisCountDepth(self.depth));
        pos.set(Tag::AnalysisCountSelDepth(self.seldepth));
        pos.set(Tag::AnalysisCountNodes(self.nodes as u128));
        pos.set(Tag::BranchingFactorPercent((100.0 * self.bf) as u32));
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::*,
        search::{engine::Engine, timecontrol::TimeControl},
    };
    use test_log::test;

    #[test]
    fn test_uci_searchresults() {
        let (bm, pm) = parse_bestmove_uci("bestmove a1a2 ponder b2b3").unwrap();
        assert_eq!(bm.to_uci(), "a1a2");
        assert_eq!(pm.unwrap().to_uci(), "b2b3");

        let (bm, pm) = parse_bestmove_uci("bestmove a1a2").unwrap();
        assert_eq!(bm.to_uci(), "a1a2");
        assert_eq!(pm, None);

        let (bm, pm) = parse_bestmove_uci("bestmove a1a2  ").unwrap();
        assert_eq!(bm.to_uci(), "a1a2");
        assert_eq!(pm, None);

        let (bm, pm) = parse_bestmove_uci("bestmove   a1a2  ").unwrap();
        assert_eq!(bm.to_uci(), "a1a2");
        assert_eq!(pm, None);

        assert_eq!(parse_bestmove_uci("bestmove a1a2 ponder").is_err(), true);
        assert_eq!(parse_bestmove_uci("bestmove").is_err(), true);
        assert_eq!(parse_bestmove_uci("xyz").is_err(), true);
        assert_eq!(
            parse_bestmove_uci("bestmove a1a2 ponder b2b3 extraneous").is_err(),
            true
        );
    }

    #[test]
    fn test_explain_results() {
        let pos = Catalog::test_position();
        let mut engine = Engine::new();
        engine.set_position(pos);
        engine.algo.set_timing_method(TimeControl::Depth(8));
        // engine.algo.set_callback(Uci::uci_info);
        engine.search();

        println!("{}", engine.algo.results.explain(&engine.algo.eval));
    }
}
