use crate::eval::eval::Eval;
use crate::eval::score::Score;
use crate::infra::utils::{
    calculate_branching_factor_by_nodes_and_depth, Differ, Displayable, Uci,
};
use crate::movelist::ScoredMoveList;
use crate::mv::{BareMove, Move};
use crate::other::outcome::Outcome;
use crate::piece::Ply;
use crate::search::timecontrol::TimeControl;
use crate::tags::Tag;
use crate::variation::Variation;
use crate::{board::Board, Algo, MoveList, Position};
use anyhow::Context;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::{self, FromIterator};
use std::time::Duration;
use tabled::builder::Builder;

use super::info::{BareMoveVariation, Info};

#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchResults {
    #[serde(skip)]
    pub bm: BareMove,
    pub depth: Ply,
    pub seldepth: Ply,
    pub time_millis: u64,
    pub nodes: u64,
    pub nodes_thread: u64,
    pub nps: u64,
    pub tbhits: u64,
    pub bf: f64,
    pub hashfull_per_mille: u32,
    pub outcome: Outcome,

    pub emt: Duration,
    pub tc: Option<TimeControl>,
    pub pos: Option<Position>,

    #[serde(skip)]
    pub multi_pv: Vec<(BareMoveVariation, Score)>,

    #[serde(skip)]
    pub infos: Vec<Info>,
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bm={bm} sc={sc} depth={d} seldepth={sd} ms={ms} nodes={nodes} pv={pv}",
            d = self.depth,
            sd = self.seldepth,
            ms = self.time_millis,
            nodes = self.nodes,
            bm = self.best_move().unwrap_or_default(),
            sc = self.score().unwrap_or_default(),
            pv = self.pv(),
        )?;
        if f.alternate() {
            for (bmv, sc) in &self.multi_pv {
                writeln!(f, "[{sc}] pv:{}", bmv.to_uci())?;
            }
            writeln!(f, "n_infos: {}", self.infos.len())?;
            for info in self.infos.iter().rev().take(6) {
                writeln!(f, "{info}", info = info.to_uci())?;
            }
        }
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
            let b = self.board.make_moves(&pv.0);
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

fn parse_bestmove_uci(s: &str, b: &Board) -> anyhow::Result<(Move, Option<Move>)> {
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
    let bm = Move::parse_uci(bm, b).with_context(|| format!("parsing best move from '{s}'"))?;
    let pm = match pm {
        Some(pm) => Some(
            Move::parse_uci(pm, &b.make_move(&bm))
                .with_context(|| format!("parsing ponder move from '{s}'"))?,
        ),
        None => None,
    };
    Ok((bm, pm))
}

impl SearchResults {
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

    pub fn to_uci(&self) -> String {
        Displayable(|fmt| self.fmt_uci(fmt)).to_string()
    }

    pub fn parse_uci(s: &str, b: &Board) -> anyhow::Result<Self> {
        let mut infos = vec![];
        let mut iter = s.lines().peekable();
        while let Some(line) = iter.next() {
            if iter.peek().is_none() {
                let (bm, pm) = parse_bestmove_uci(line, b)?;
                let sr = SearchResults::from_infos(bm.to_inner(), pm.map(|m| m.to_inner()), infos);
                assert!(
                    // look for move occuing twice in multi-pv
                    !sr.multi_pv
                        .iter()
                        .tuple_windows()
                        .any(|(prev, next)| prev.0.first() == next.0.first()),
                    "{sr} has duplicate move in multi_pv\n{s}"
                );
                return Ok(sr);
            } else {
                // @todo
                let info = Info::parse_uci(line)?;
                // ignore "info depth 21 currmove g7g5 currmovenumber 18"
                // info nodes 100000 nps 1020000 hashfull 50 time 97
                // if info.depth.is_some() && info.pv.is_some() {
                infos.push(info);
                // }
            }
        }
        unreachable!()
    }
}

// impl Uci for SearchResults {
//     fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         if let Ok(mv) = self.best_move() {
//             write!(f, "bestmove {mv}", mv = mv.to_uci())?;
//             if let Some(ponder) = self.pv().second() {
//                 write!(f, "ponder {}", ponder.to_uci())?;
//             }
//         } else {
//             write!(f, "bestmove 0000")?;
//         }

//         Ok(())
//     }

//     // null best moves of "0000" permitted, but result in an empty multipv
//     // and hence a bestmove() of Err
//     fn parse_uci(s: &str) -> anyhow::Result<Self> {
//         let mut infos = vec![];
//         let mut iter = s.lines().peekable();
//         while let Some(line) = iter.next() {
//             if iter.peek().is_none() {
//                 let (bm, pm) = parse_bestmove_uci(line)?;
//                 let sr = SearchResults::from_infos(bm, pm, infos);
//                 assert!(
//                     // look for move occuing twice in multi-pv
//                     !sr.multi_pv
//                         .iter()
//                         .tuple_windows()
//                         .any(|(prev, next)| prev.0.first() == next.0.first()),
//                     "{sr} has duplicate move in multi_pv\n{s}"
//                 );
//                 return Ok(sr);
//             } else {
//                 // @todo
//                 let info = Info::parse_uci(line)?;
//                 // ignore "info depth 21 currmove g7g5 currmovenumber 18"
//                 // info nodes 100000 nps 1020000 hashfull 50 time 97
//                 // if info.depth.is_some() && info.pv.is_some() {
//                 infos.push(info);
//                 // }
//             }
//         }
//         unreachable!()
//     }
// }

impl Differ<SearchResults> for SearchResults {
    fn diff(sr1: &Self, sr2: &Self) -> Option<String> {
        let mut sr1 = sr1.clone();
        sr1.emt = sr2.emt; // emt will be different
        if &sr1 != sr2 {
            return Some(String::from("differences!"));
        }
        None
    }
}

impl SearchResults {
    fn extract_multi_pv(infos: &Vec<Info>) -> Vec<(BareMoveVariation, Score)> {
        // step #1, find max multipv index
        let max_index = infos.iter().map(|i| i.multi_pv.unwrap_or(1)).max();

        // step #2, find max depth with this multipv index
        let max_depth = infos
            .iter()
            .filter(|i| i.multi_pv.unwrap_or(1) == max_index.unwrap_or(1))
            .map(|i| i.depth.unwrap_or_default())
            .max();

        let mut multi_pv = infos
            .iter()
            .rev() // look backwards
            .filter(|i| i.depth == max_depth)
            .map(|i| {
                (
                    i.pv.clone().unwrap_or_default(),
                    i.score.unwrap_or_default(),
                )
            })
            // we see duplicate moves with diferent hashfulls (and scores)
            .unique_by(|(pv, _sc)| pv.first()) // so we remove duplicate moves
            .collect_vec();
        multi_pv.sort_by_key(|(_pv, sc)| sc.negate());
        multi_pv
    }

    pub fn from_infos(bm: BareMove, pm: Option<BareMove>, infos: Vec<Info>) -> Self {
        // gets nodecount for nodes at last depth reported
        // fn calculate_nodes_for_iid(n: Ply, infos: &[Info]) -> anyhow::Result<u64> {
        //     let info_n = infos
        //         .iter()
        //         .filter(|i| i.pv.is_some())
        //         .find_or_last(|inf| inf.depth == Some(n))
        //         .ok_or(anyhow::anyhow!("no info for depth {n}"))?
        //         .nodes_thread
        //         .ok_or(anyhow::anyhow!(
        //             "found info for depth {n} but no node counts"
        //         ))?;
        //     Ok(info_n)
        // }

        if let Some(_info) = infos.last() {
            let depth = infos
                .iter()
                .rev()
                .filter(|i| i.pv.is_some())
                .find_map(|i| i.depth);
            let seldepth = infos.iter().rev().find_map(|i| i.seldepth);
            let ms = infos.iter().rev().find_map(|i| i.time_millis);
            let nodes = infos.iter().rev().find_map(|i| i.nodes);
            let nodes_thread = infos.iter().rev().find_map(|i| i.nodes_thread);
            let nps = infos.iter().rev().find_map(|i| i.nps);
            let tbhits = infos.iter().rev().find_map(|i| i.tbhits);
            let hashfull = infos.iter().rev().find_map(|i| i.hashfull_per_mille);

            // not quite right depth is last pv depth, nodes is total including
            // overcalculated nodes
            let bf = if let Some(depth) = depth {
                // if let Ok(nodes_thread) = calculate_nodes_for_iid(depth, &infos) {
                if let Some(nodes_thread) = nodes_thread {
                    calculate_branching_factor_by_nodes_and_depth(nodes_thread, depth)
                } else {
                    info!("info did not contain nodes needed for bf");
                    Err(anyhow::anyhow!("info did not contain nodes needed for bf"))
                }
            } else {
                info!("info did not contain depth needed for bf");
                Err(anyhow::anyhow!("info did not contain depth needed for bf"))
            };

            // null best move => no multipv set and score of zero
            let multi_pv = if !bm.is_null() {
                Self::extract_multi_pv(&infos)
            } else {
                vec![(BareMoveVariation::new(), Score::zero())]
            };
            SearchResults {
                bm,
                depth: depth.unwrap_or_default(),
                seldepth: seldepth.unwrap_or_default(),
                time_millis: ms.unwrap_or_default(),
                // time_millis: elapsed_move_time.as_millis() as u64,
                nodes: nodes.unwrap_or_default(),
                nodes_thread: nodes_thread.unwrap_or_default(),
                nps: nps.unwrap_or_default(),
                tbhits: tbhits.unwrap_or_default(),
                bf: bf.unwrap_or_default(),
                hashfull_per_mille: hashfull.unwrap_or_default(),
                outcome: Outcome::Unterminated,
                multi_pv,
                infos,
                emt: Duration::ZERO,
                pos: None,
                tc: None,
            }
        } else {
            let mut sr = SearchResults::default();
            let mut var = BareMoveVariation::new();
            sr.bm = bm;
            if !bm.is_null() {
                var.push(bm);
            }
            if let Some(pm) = pm {
                var.push(pm);
            }
            sr.multi_pv = vec![(var, Score::zero())];
            sr
        }
    }

    pub fn new(
        algo: &Algo,
        depth: Ply,
        multi_pv: Vec<(BareMoveVariation, Score)>,
        seldepth: Option<Ply>,
    ) -> Self {
        let nodes_thread_cumul = algo.clock.cumul_nodes_this_thread();
        let bf = calculate_branching_factor_by_nodes_and_depth(nodes_thread_cumul, depth)
            .unwrap_or_default();
        SearchResults {
            bm: multi_pv
                .get(0)
                .map(|var| var.0.first().unwrap_or_default())
                .unwrap_or_default(),
            outcome: Outcome::Unterminated,
            tbhits: 0,
            nodes: algo.clock.cumul_nodes_all_threads(),
            nodes_thread: algo.clock.cumul_nodes_this_thread(),
            nps: algo.clock.cumul_knps_all_threads() * 1000,
            depth,
            seldepth: seldepth.unwrap_or_default(),
            time_millis: algo.clock.elapsed_search().time.as_millis() as u64,
            hashfull_per_mille: algo.tt.hashfull_per_mille(),
            bf,
            multi_pv,
            infos: vec![],
            emt: algo.clock.elapsed_search().time,
            pos: Some(algo.position.clone()),
            tc: Some(algo.mte.time_control().clone()),
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
        if !self.bm.is_null() {
            Ok(self.bm)
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

    pub fn score(&self) -> Option<Score> {
        if self.multi_pv.len() > 0 {
            Some(self.multi_pv[0].1)
        } else {
            None
        }
    }

    pub fn multi_pv(&self) -> Vec<(BareMoveVariation, Score)> {
        self.multi_pv
            .iter()
            .map(|(var, sc)| (var.clone(), *sc))
            .collect_vec()
    }

    pub fn scored_move_list(&self) -> ScoredMoveList {
        let mut list = ScoredMoveList::new();
        self.multi_pv
            .iter()
            .filter_map(|(var, sc)| {
                if let Some(mv) = var.first() {
                    Some((mv, *sc))
                } else {
                    None
                }
            })
            .for_each(|ms| list.push(ms));

        list
    }

    pub const TAGS: [&str; 8] = [
        Tag::SM,
        Tag::BM,
        Tag::CE,
        Tag::ACMS,
        Tag::ACD,
        Tag::ACSD,
        Tag::ACN,
        Tag::BF,
    ];

    pub fn to_position(&self, mut pos: Position, tags: &[&str]) -> Position {
        let var = Variation::from_inner(&self.pv(), pos.board());
        if tags.contains(&Tag::PV) {
            pos.set(Tag::Pv(var));
        }
        if let Some(ref mv) = self.pv().first() {
            let mv = pos.board().augment_move(*mv);
            if tags.contains(&Tag::SM) {
                pos.set(Tag::SuppliedMove(mv));
            }
            if tags.contains(&Tag::BM) {
                pos.set(Tag::BestMoves(MoveList::from_iter(iter::once(mv))));
            }
        }
        if tags.contains(&Tag::CE) {
            pos.set(Tag::CentipawnEvaluation(
                self.score().unwrap_or_default().as_i16() as i32,
            ));
        }
        if tags.contains(&Tag::ACD) {
            pos.set(Tag::AnalysisCountDepth(self.depth));
        }
        if tags.contains(&Tag::ACS) {
            pos.set(Tag::AnalysisCountSeconds((self.time_millis / 1000) as u32));
        }
        if tags.contains(&Tag::ACMS) {
            pos.set(Tag::AnalysisCountMilliSeconds(self.time_millis));
        }
        if tags.contains(&Tag::ACSD) {
            pos.set(Tag::AnalysisCountSelDepth(self.seldepth));
        }
        if tags.contains(&Tag::ACN) {
            pos.set(Tag::AnalysisCountNodes(self.nodes as u128));
        }
        if tags.contains(&Tag::BF) {
            pos.set(Tag::BranchingFactor(self.bf));
        }
        if tags.contains(&Tag::BSM) {
            pos.set(Tag::BestScoredMoves(self.scored_move_list()));
        }
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::testing::*;
    use crate::{
        catalog::*,
        search::{engine::ThreadedSearch, timecontrol::TimeControl},
    };
    use test_log::test;

    #[test]
    fn test_uci_searchresults() {
        let b = Board::starting_pos();
        let (bm, pm) = parse_bestmove_uci("bestmove a2a3 ponder b7b6", &b).unwrap();
        assert_eq!(bm.to_uci(), "a2a3");
        assert_eq!(pm.unwrap().to_uci(), "b7b6");

        let (bm, pm) = parse_bestmove_uci("bestmove a2a3", &b).unwrap();
        assert_eq!(bm.to_uci(), "a2a3");
        assert_eq!(pm, None);

        let (bm, pm) = parse_bestmove_uci("bestmove a2a3  ", &b).unwrap();
        assert_eq!(bm.to_uci(), "a2a3");
        assert_eq!(pm, None);

        let (bm, pm) = parse_bestmove_uci("bestmove 0000", &b).unwrap();
        assert_eq!(bm.is_null(), true);
        assert_eq!(bm.to_uci(), "0000");
        assert_eq!(pm, None);

        let (bm, pm) = parse_bestmove_uci("bestmove   a2a3  ", &b).unwrap();
        assert_eq!(bm.to_uci(), "a2a3");
        assert_eq!(pm, None);

        assert_eq!(
            parse_bestmove_uci("bestmove a7a6 ponder", &b).is_err(),
            true
        );
        assert_eq!(
            parse_bestmove_uci("bestmove a2a3 ponder", &b).is_err(),
            true
        );
        assert_eq!(parse_bestmove_uci("bestmove", &b).is_err(), true);
        assert_eq!(parse_bestmove_uci("xyz", &b).is_err(), true);
        assert_eq!(
            parse_bestmove_uci("bestmove a2a3 ponder b7b6 extraneous", &b).is_err(),
            true
        );
    }

    #[test]
    fn test_search_results_parse_uci() -> anyhow::Result<()> {
        let s = r#"info depth 10 seldepth 10 nodes 61329 nps 1039000 score mate 2 hashfull 40 time 58 pv h2h4 e7e5 b2b3
info depth 11 seldepth 12 nodes 82712 nps 973000 score mate 2 hashfull 45 time 84 pv e2e4 e7e5 a2a3
info nodes 100000 nps 1020000 hashfull 50 time 97
bestmove g2g4 ponder e7e5
"#;
        let b = Board::starting_pos();
        let sr = SearchResults::parse_uci(s, &b).unwrap();
        // assert_eq!(
        //     "g2-g4".parse::<Move>()?,
        //     Move::parse_uci("g2g4", b).unwrap()
        // );
        // assert_eq!("g3g6".parse::<BareMove>()?, "g3g6".try_into()?);
        // assert_eq!(BareMove::parse_uci("g3g6")?, "g3g6".mv());
        // assert_eq!(sr.best_move(), Ok("g3g6".try_into()?));
        assert_eq!(sr.nodes, 100000);
        assert_eq!(sr.best_move().unwrap(), "g2g4".mv());
        assert_eq!(sr.pv(), "e2e4 e7e5 a2a3".var());
        assert_eq!(sr.multi_pv(), vec![("e2e4 e7e5 a2a3".var(), "+M2".cp())]);
        assert_eq!(sr.depth, 11);
        assert_eq!(sr.bf > 2.5, true);
        assert_eq!(sr.bf < 3.0, true);
        info!("{}", "a3a4".mv());

        let s = r#"info depth 10 seldepth 10 nodes 61329 nps 1039000 score mate 2 hashfull 40 time 58 pv h2h4 e7e5 b2b3
info depth 11 seldepth 12 nodes 82712 nps 973000 score mate 2 hashfull 45 time 84 pv e2e4 e7e5 a2a3
info nodes 100000 nps 1020000 hashfull 50 time 97
bestmove 0000
"#;
        let sr = SearchResults::parse_uci(s, &b).unwrap();
        assert_eq!(sr.nodes, 100000);
        assert_eq!(sr.best_move().is_err(), true);
        assert_eq!(sr.pv(), "".var());

        Ok(())
    }

    #[test]
    fn test_explain_results() {
        let pos = Catalog::test_position();
        let mut engine = ThreadedSearch::new();
        engine.set_position(pos);
        engine.algo.set_timing_method(TimeControl::Depth(8));
        // engine.algo.set_callback(Uci::uci_info);
        engine.search_sync();

        println!(
            "{}",
            engine
                .algo
                .results
                .explain(&engine.algo.eval, &engine.algo.board)
        );
    }
}
