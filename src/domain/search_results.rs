use std::iter::{self, FromIterator};

use crate::eval::eval::Eval;
use crate::eval::score::Score;
use crate::infra::utils::{calculate_branching_factor_by_nodes_and_depth, Uci};
use crate::mv::BareMove;
use crate::other::outcome::Outcome;
use crate::piece::Ply;
use crate::search::timecontrol::TimeControl;
use crate::tags::Tag;
use crate::variation::Variation;
use crate::{board::Board, Algo, MoveList, Position};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use tabled::builder::Builder;

use super::info::{BareMoveVariation, Info};

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
    pub multi_pv: Vec<(BareMoveVariation, Score)>,

    #[serde(skip)]
    pub tc: TimeControl,

    #[serde(skip)]
    pub infos: Vec<Info>,
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        writeln!(f, "n_infos: {}", self.infos.len())?;
        Ok(())
    }
}

pub struct SearchResultsWithExplanation<'a> {
    sr: &'a SearchResults,
    eval: &'a Eval,
    board: &'a Board,
}

impl fmt::Display for SearchResultsWithExplanation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.sr)?;
        let mut bu = Builder::new();
        bu.set_columns(["Score", "PV", "Explain"]);
        for pv in &self.sr.multi_pv {
            let b = self.sr.board.make_moves(&pv.0);
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
            if let Some(ponder) = self.pv().second() {
                write!(f, "ponder {}", ponder.to_uci())?;
            }
        } else {
            write!(f, "bestmove 0000")?;
        }

        Ok(())
    }

    fn parse_uci(s: &str) -> anyhow::Result<Self> {
        let mut infos = vec![];
        let mut iter = s.lines().peekable();
        while let Some(line) = iter.next() {
            if iter.peek().is_none() {
                let (bm, pm) = parse_bestmove_uci(line)?;
                return Ok(SearchResults::from_infos(bm, pm, infos));
            } else {
                infos.push(Info::parse_uci(line)?);
            }
        }
        unreachable!()
    }
}

impl SearchResults {
    pub fn from_infos(_bm: BareMove, _pm: Option<BareMove>, infos: Vec<Info>) -> Self {
        fn calculate_nodes_for_iid(n: Ply, infos: &[Info]) -> anyhow::Result<u64> {
            let info_n = infos
                .iter()
                .find_or_last(|inf| inf.depth == Some(n))
                .ok_or(anyhow::anyhow!("no info for depth {n}"))?
                .nodes_thread
                .ok_or(anyhow::anyhow!(
                    "found info for depth {n} but no node counts"
                ))?;
            Ok(info_n)
        }

        if let Some(info) = infos.last() {
            let bf = if let Some(depth) = info.depth {
                if let Ok(nodes) = calculate_nodes_for_iid(depth, &infos) {
                    calculate_branching_factor_by_nodes_and_depth(nodes, depth)
                        .ok()
                        .map(|bf| bf as f32)
                } else {
                    None
                }
            } else {
                None
            };
            SearchResults {
                depth: info.depth.unwrap_or_default(),
                seldepth: info.seldepth.unwrap_or_default(),
                time_millis: info.time_millis.unwrap_or_default(),
                nodes: info.nodes.unwrap_or_default(),
                nodes_thread: info.nodes_thread.unwrap_or_default(),
                nps: info.nps.unwrap_or_default(),
                tbhits: info.tbhits.unwrap_or_default(),
                bf: bf.unwrap_or_default(),
                hashfull_per_mille: info.hashfull_per_mille.unwrap_or_default(),
                outcome: Outcome::Unterminated,
                multi_pv: vec![(info.pv.clone().unwrap_or_default(), info.score.unwrap_or_default())],
                tc: TimeControl::default(),
                board: Board::default(),
                infos,
            }
        } else {
            SearchResults::default()
        }
    }

    pub fn new(algo: &Algo, depth: Ply, multi_pv: Vec<(BareMoveVariation, Score)>) -> Self {
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
            infos: vec![],
        }
    }

    pub fn explain<'a>(
        &'a self,
        eval: &'a Eval,
        board: &'a Board,
    ) -> SearchResultsWithExplanation<'a> {
        SearchResultsWithExplanation {
            sr: self,
            eval,
            board,
        }
    }

    /// outcome could be abandoned or win/draw reason
    pub fn best_move(&self) -> Result<BareMove, Outcome> {
        if let Some(mv) = self.pv().first() {
            Ok(mv)
        } else {
            Err(self.outcome)
        }
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    pub fn pv(&self) -> BareMoveVariation {
        if self.multi_pv.len() > 0 {
            self.multi_pv[0].0.clone()
        } else {
            BareMoveVariation::new()
        }
    }

    pub fn score(&self) -> Score {
        if self.multi_pv.len() > 0 {
            self.multi_pv[0].1
        } else {
            Score::default()
        }
    }

    pub fn multi_pv(&self) -> Vec<(BareMoveVariation, Score)> {
        self.multi_pv
            .iter()
            .map(|(var, sc)| (var.clone(), *sc))
            .collect_vec()
    }

    pub fn to_position(&self, b: Board) -> Position {
        let mut pos = Position::from_board(b);
        let var = Variation::from_inner(&self.pv(), pos.board());
        pos.set(Tag::Pv(var));
        if let Some(ref mv) = self.pv().first() {
            let mv = pos.board().augment_move(*mv);
            pos.set(Tag::SuppliedMove(mv));
            pos.set(Tag::BestMoves(MoveList::from_iter(iter::once(mv))));
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

        println!(
            "{}",
            engine
                .algo
                .results
                .explain(&engine.algo.eval, &engine.algo.board)
        );
    }
}
