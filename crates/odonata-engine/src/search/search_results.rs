use super::trail::ChessTree;
use crate::eval::Eval;
use anyhow::Context;
use itertools::Itertools;
use odonata_base::{
    boards::Position,
    domain::info::Info,
    infra::utils::calculate_branching_factor_by_nodes_and_depth,
    movelist::ScoredMoveList,
    other::outcome::Outcome,
    prelude::*,
    variation::{MultiVariation, ScoredVariation},
    Epd,
};
use std::{fmt, io::Write, time::Duration};
use tabled::builder::Builder;

#[derive(Clone, Default, Debug)]
pub struct SearchResults {
    pub supplied_move:      Move,
    pub depth:              Ply,
    pub seldepth:           Ply,
    pub time_millis:        u64,
    pub nodes:              u64,
    pub nodes_thread:       u64,
    pub nps:                u64,
    pub tbhits:             u64,
    pub bf:                 f64,
    pub hashfull_per_mille: u32,
    pub outcome:            Outcome,
    pub emt:                Duration,
    pub tc:                 Option<TimeControl>,
    pub pos:                Option<Epd>,
    pub multi_pv:           MultiVariation,
    pub infos:              Vec<Info>,
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bm={bm} sc={sc} depth={d} seldepth={sd} ms={ms} nodes={nodes} pv={pv} mpv={mpv}",
            d = self.depth,
            sd = self.seldepth,
            ms = self.time_millis,
            nodes = self.nodes,
            bm = self.supplied_move().unwrap_or_default(),
            sc = self.score().unwrap_or_default(),
            pv = self.pv(),
            mpv = self.multi_pv,
        )?;
        if f.alternate() {
            writeln!(f, "{mpv:#}", mpv = self.multi_pv)?;
            writeln!(f, "n_infos: {}", self.infos.len())?;
            for info in self.infos.iter().rev().take(6) {
                writeln!(f, "{info}", info = info.to_uci())?;
            }
        }
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
        (Some("bestmove"), Some(bm), None, ..) => (bm, None),
        (_, _, _, _, Some(_)) => anyhow::bail!("too many words in '{s}'"),
        _ => anyhow::bail!("expected: bestmove bm [ponder pm] but found '{s}'"),
    };
    let bm = Move::parse_uci(bm, b).with_context(|| format!("parsing best move from '{s}'"))?;
    let pm = match pm {
        Some(pm) => Some(
            Move::parse_uci(pm, &b.make_move(bm))
                .with_context(|| format!("parsing ponder move from '{s}'"))?,
        ),
        None => None,
    };
    Ok((bm, pm))
}

impl SearchResults {
    pub fn write_explanation(
        &self,
        mut f: impl Write,
        eval: &Eval,
        mut pos: Position,
    ) -> Result<()> {
        writeln!(f, "{}", self)?;
        let mut bu = Builder::new();
        bu.set_columns(["Score", "PV", "Explain"]);
        for pv in self.multi_pv.iter() {
            pos.push_moves(pv.var.clone());
            bu.add_record([
                pv.score.to_string(),
                pv.var.to_string(),
                format!("{pos}\n{}", eval.static_eval_explain(&pos)),
            ]);
        }
        writeln!(f, "{}", bu.build())?;
        Ok(())
    }

    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(mv) = self.supplied_move() {
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
                let (sm, ponder_mv) = parse_bestmove_uci(line, b)?;
                let sr = SearchResults::from_infos(sm, ponder_mv, infos, b);
                assert!(
                    // look for move occuing twice in multi-pv
                    !sr.multi_pv
                        .iter()
                        .tuple_windows()
                        .any(|(prev, next)| prev.var.first() == next.var.first()),
                    "{sr} has duplicate move in multi_pv\n{s}"
                );
                return Ok(sr);
            }
            // TODO!
            if !line.starts_with("root node is ") && !line.starts_with("Invalid move 0000 for ") {
                let info = Info::parse_uci(line, b)?;
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

impl SearchResults {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_san(&self, b: &Board) -> String {
        format!(
            "bm={bm} sc={sc} depth={d} seldepth={sd} ms={ms} nodes={nodes} pv={pv} mpv={mpv}",
            d = self.depth,
            sd = self.seldepth,
            ms = self.time_millis,
            nodes = self.nodes,
            bm = self.supplied_move().unwrap_or_default().to_san(b),
            sc = self.score().unwrap_or_default(),
            pv = self.pv().to_san(b),
            mpv = self.multi_pv.to_san(b),
        )
    }

    pub fn extract_multi_pv(infos: &[Info], max_depth: Option<Ply>) -> MultiVariation {
        // step #1, find max multipv index - where multi_pv exists
        let max_index = infos.iter().map(|i| i.multi_pv.unwrap_or(1)).max();

        // step #2, find max depth with this multipv index
        let max_depth = infos
            .iter()
            .filter(|i| i.multi_pv.unwrap_or(1) == max_index.unwrap_or(1))
            .map(|i| i.depth.unwrap_or_default())
            .filter(|&dep| dep <= max_depth.unwrap_or(Ply::MAX)) // look only up to max depth
            .max();

        let mut multi_pv = infos
            .iter()
            .rev() // look backwards
            .filter(|i| i.depth == max_depth)
            .filter(|i| i.pv.is_some())
            .map(|i| ScoredVariation {
                var:   i.pv.clone().unwrap_or_default(),
                score: i.score.unwrap_or_default(),
            })
            // we see duplicate moves with different hashfulls (and scores)
            .unique_by(|sv| sv.var.first()) // so we remove duplicate moves
            .collect_vec();
        multi_pv.sort_by_key(|sv| sv.score.negate());
        multi_pv.into_iter().collect()
    }

    pub fn from_infos(
        supplied_move: Move,
        ponder_mv: Option<Move>,
        infos: Vec<Info>,
        b: &Board,
    ) -> Self {
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

            let multi_pv = Self::extract_multi_pv(&infos, None);
            assert!(
                depth == Some(0) || multi_pv.iter().filter(|vs| vs.var.is_empty()).count() == 0,
                "Found empty pv parsing\n{}",
                infos.iter().join("\n")
            );
            // null best move => no multipv set and score of zero
            // let multi_pv = if !bm.is_null() {
            //     Self::extract_multi_pv(&infos)
            // } else {
            //     vec![(Variation::new(), Score::zero())]
            // };
            SearchResults {
                supplied_move,
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
                pos: Some(Epd::from_board(b.clone())),
                tc: None,
            }
        } else {
            let mut sr = SearchResults {
                pos: Some(Epd::from_board(b.clone())),
                ..SearchResults::default()
            };
            let mut var = Variation::new();
            sr.supplied_move = supplied_move;
            if !supplied_move.is_null() {
                var.push(supplied_move);
            }
            if let Some(pm) = ponder_mv {
                var.push(pm);
            }
            sr.multi_pv = vec![ScoredVariation {
                var,
                score: Score::zero(),
            }]
            .into_iter()
            .collect();
            sr
        }
    }

    /// outcome could be abandoned or win/draw reason
    pub fn supplied_move(&self) -> Result<Move, Outcome> {
        if self.supplied_move.is_null() {
            Err(self.outcome)
        } else {
            Ok(self.supplied_move)
        }
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    pub fn pv(&self) -> Variation {
        self.multi_pv.first().map(|sv| sv.var).unwrap_or_default()
    }

    pub fn tree(&self) -> Option<&ChessTree> {
        None
        // self.tree.as_ref()
    }

    pub fn score(&self) -> Option<Score> {
        self.multi_pv.first().map(|sv| sv.score)
    }

    // filter out empty variations
    pub fn multi_variation(&self) -> MultiVariation {
        self.multi_pv.clone()
    }

    pub fn scored_move_list(&self) -> ScoredMoveList {
        self.multi_pv.clone().into()
    }

    pub fn to_epd(&self) -> Epd {
        let mut epd = Epd::from_board(self.pos.as_ref().unwrap().board());
        let b = &epd.board();
        epd.set_tag("sv", &self.pv().to_san(b));
        // if fields.contains(&Tags::SV) {
        //     tags.sv = Some(self.pv());
        // }
        if self.multi_variation().len() > 1 {
            // BUG
            // for (i, mpv) in self.multi_variation().iter().take(10).enumerate() {
            //     epd.set_tag(&format!("pv{i}"), &mpv.var.to_san(b));
            // }
            epd.set_tag("Mpv", &self.multi_variation().to_san(b));
        }
        epd.set_tag("sm", &self.pv().first().unwrap_or_default().to_san(b));
        epd.set_tag("ce", &self.score().unwrap_or_default().as_i16().to_string());
        epd.set_tag("acd", &self.depth.to_string());
        // if fields.contains(&Tags::AD) {
        //     tags.annotator_depth = Some(self.depth);
        // }
        epd.set_tag("acsd", &self.seldepth.to_string());
        epd.set_tag("acs", &(self.time_millis / 1000).to_string());
        epd.set_tag("Acms", &self.time_millis.to_string());
        epd.set_tag("acn", &self.nodes.to_string());
        epd.set_tag("Bf", &((self.bf * 1000.0) as i32).to_string());
        // if fields.contains(&Tags::ESM) {
        //     tags.eng_scored_moves = Some(self.scored_move_list());
        // }
        // if fields.contains(&Tags::ASM) {
        //     tags.ann_scored_moves = Some(self.scored_move_list());
        // }
        // tags
        epd
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::{
        prelude::{testing::Testing, *},
        variation::MultiVariation,
    };
    use test_log::test;

    use crate::search::search_results::{parse_bestmove_uci, SearchResults};

    #[test]
    fn test_uci_bestmove() {
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
    fn test_search_results() -> anyhow::Result<()> {
        let s = r#"info depth 10 seldepth 10 nodes 61329 nps 1039000 score mate 2 hashfull 40 time 58 pv h2h4 e7e5 b2b3
info depth 11 seldepth 12 nodes 82712 nps 973000 score mate 2 hashfull 45 time 84 pv e2e4 e7e5 a2a3
info nodes 100000 nps 1020000 hashfull 50 time 97
bestmove g2g4 ponder e7e5
"#;
        let b = &Board::starting_pos();
        let sr = SearchResults::parse_uci(s, b).unwrap();
        // assert_eq!(
        //     "g2-g4".parse::<Move>()?,
        //     Move::parse_uci("g2g4", b).unwrap()
        // );
        // assert_eq!("g3g6".parse::<BareMove>()?, "g3g6".try_into()?);
        // assert_eq!(BareMove::parse_uci("g3g6")?, "g3g6".mv());
        // assert_eq!(sr.best_move(), Ok("g3g6".try_into()?));
        assert_eq!(sr.nodes, 100_000);
        // "g2g4"[b]
        assert_eq!(
            sr.supplied_move().unwrap(),
            Move::parse_uci("g2g4", b).unwrap()
        );
        assert_eq!(sr.pv(), "e2e4 e7e5 a2a3".var(b));
        assert_eq!(
            sr.multi_variation(),
            MultiVariation::from_scored_variation("e2e4 e7e5 a2a3".var(b), "+M2".cp())
        );
        assert_eq!(sr.depth, 11);
        assert_eq!(sr.bf > 2.5, true);
        assert_eq!(sr.bf < 3.0, true);
        info!("{}", "a3a4".mv(b));

        let s = r#"info depth 10 seldepth 10 nodes 61329 nps 1039000 score mate 2 hashfull 40 time 58 pv h2h4 e7e5 b2b3
info depth 11 seldepth 12 nodes 82712 nps 973000 score mate 2 hashfull 45 time 84 pv e2e4 e7e5 a2a3
info nodes 100000 nps 1020000 hashfull 50 time 97
bestmove 0000
"#;
        let sr = SearchResults::parse_uci(s, b).unwrap();
        assert_eq!(sr.nodes, 100_000);
        assert_eq!(sr.supplied_move().is_err(), true);
        assert_eq!(sr.pv(), "e2e4 e7e5 a2a3".var(b));

        let s = r#"info depth 0 seldepth 0 multipv 1 score cp -717 nodes 2 nps 2000 hashfull 0 time 0 pv
bestmove 0000
"#;
        let sr = SearchResults::parse_uci(s, b).unwrap();
        dbg!(&sr);
        assert_eq!(sr.score(), Some(Score::from_cp(-717)));
        assert_eq!(sr.nodes, 2);
        assert_eq!(sr.supplied_move().is_err(), true);
        assert_eq!(sr.pv(), Variation::new());

        Ok(())
    }
}
