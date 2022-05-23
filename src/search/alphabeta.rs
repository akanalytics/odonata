use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::tt2::TtNode;
use crate::eval::score::Score;
use crate::mv::Move;
use crate::prelude::*;
use crate::pvtable::PvTable;
use crate::search::algo::Algo;
use crate::search::node::Node;
use crate::types::{Ply, MAX_PLY};

use super::node::Event;

pub struct AlphaBeta;

// terminology
// ply is moves made. so ply 3 means w-> b-> w-> after which we score position
//
impl Algo {
    // #[inline]
    // pub fn is_leaf(&self, _ply: Ply, depth: Ply) -> bool {
    //     depth <= 0
    // }

    pub fn run_alphabeta(&mut self, board: &mut Board, node: &mut Node) -> (Score, Event) {
        let depth = node.depth;
        self.max_depth = depth;
        self.stats.depth = depth;
        self.pv_table = PvTable::new(MAX_PLY as usize);
        debug_assert!(self.current_variation.len() == 0);

        let (score, category) = match self.alphabeta_recursive(
            board,
            node.ply,
            self.max_depth,
            node.alpha,
            node.beta,
            Move::NULL_MOVE,
        ) {
            Ok((score, category)) => (score, category),
            Err(category) => (-Score::INFINITY, category),
        };
        self.stats.set_score(score, category);
        debug_assert!(
            self.current_variation.len() == 0
                || category == Event::UserCancelled
                || category == Event::TimeUp
        );
        let (pv, _score) = if self.tt.use_tt_for_pv {
            self.tt.extract_pv_and_score(board)
        } else {
            (self.pv_table.extract_pv(), Some(Score::default()))
        };
        self.results.set_pv(category, &pv);
        self.results.score = score;
        if node.alpha == -Score::INFINITY
            && node.beta == Score::INFINITY
            && category != Event::TimeUp
            && category != Event::UserCancelled
        {
            debug_assert!(
                score.is_numeric_or_mate(),
                "Score was inf: node {:?} cat {} \n{}",
                node,
                category,
                self
            );
        }

        self.stats.record_iteration(self.max_depth, category, pv);
        (score, category)
    }

    // score >= beta fail high
    // score <= alpha fail low
    // for [alpha-1, a] a score of either bound indicates fail low or high
    pub fn alphabeta_recursive(
        &mut self,
        b: &mut Board,
        ply: Ply,
        depth: Ply,
        alpha: Score,
        beta: Score,
        last_move: Move,
    ) -> Result<(Score, Event), Event> {
        profile_method!(alphabeta_recursive);
        // hprof::enter("alphabeta");

        self.clear_move(ply);
        self.report_progress();

        let mut n = Node {
            ply,
            depth,
            alpha,
            beta,
        };

        let (cancelled, mut cat) = self.time_up_or_cancelled(ply, false);
        if cancelled {
            self.counts.inc(&n, cat);
            return Err(cat);
        }

        debug_assert!(
            n.alpha < n.beta || self.minmax,
            "alpha={}, beta={}, minmax={}",
            n.alpha,
            n.beta,
            self.minmax
        );

        self.clock.inc_nodes();
        self.counts.inc(&n, Event::Clock);
        // self.results.set_seldepth(&n);

        if n.is_zw() {
            self.counts.inc(&n, Event::NodeTypeZw);
            self.stats.inc_zw_nodes(ply);
        }
        let _g;
        // let _qsearch : firestorm::internal::SpanGuard;
        if n.is_qs() {
            self.counts.inc(&n, Event::NodeTypeQuiesce);
            profile_section!(qsearch);
            // hprof::enter("qsearch");
            _g = qsearch;
        } else {
            self.counts.inc(&n, Event::NodeInterior);
        }

        let mut score = -Score::INFINITY;
        let mut category = Event::Unknown;
        let mut bm = Move::NULL_MOVE;
        let mut nt = NodeType::UpperAll;

        // we dont draw at root, as otherwise it wont play a move if insufficient-material draw [v32]
        if ply > 0 && b.draw_outcome().is_some() {
            // if board.draw_outcome().is_some() {
            self.stats.inc_leaf_nodes(&n);
            self.counts.inc(&n, Event::NodeLeafDraw);
            return Ok((b.eval_draw(&mut self.eval, &n), Event::NodeLeafDraw)); // will return a draw score
        }

        let mut tt_mv = Move::NULL_MOVE;
        match self.lookup(b, &mut n) {
            (Some(ab), None) => {
                debug_assert!(ab.is_numeric_or_mate(), "lookup returned {}", ab);
                return Ok((ab, Event::HashHit));
            } // alpha, beta or a terminal node
            (None, Some(bm)) => tt_mv = bm,
            (Some(s), Some(mv)) => {
                tt_mv = mv;
                score = s;
                bm = mv;
                category = Event::HashHit;
            }
            _ => {}
        }
        self.stats.inc_interior_nodes(&n);

        // static eval
        let mut eval = b.eval_some(&self.eval);

        if self.tt.use_tt_for_eval {
            if let Some(entry) = self.tt.probe_by_board(b, n.ply, n.depth) {
                // if entry.depth >= n.depth {
                if entry.nt == NodeType::ExactPv {
                    eval = entry.score;
                } else if entry.nt == NodeType::LowerCut && entry.score > eval {
                    eval = entry.score;
                } else if entry.nt == NodeType::UpperAll && entry.score < eval {
                    eval = entry.score;
                }
                // }
            }
        }
        if let Some(s) = self.standing_pat(b, &mut n, eval) {
            return Ok((s, Event::PruneStandingPat));
        }
        if let Some(alphabeta) = self.razor(last_move, b, eval, &n)? {
            return Ok((alphabeta, Event::PruneRazor));
        }
        if let Some(s) = self.nmp(b, &n, eval)? {
            return Ok((s, Event::PruneNullMovePrune));
        }

        let mut sorted_moves = self
            .move_orderer
            .create_sorted_moves(n, b, tt_mv, last_move);
        let mut count = 0;
        let mut quiets = 0;
        while let Some((move_type, mv)) = sorted_moves.next_move(b, self) {
            if self.restrictions.skip_move(ply, &mv) {
                continue;
            }
            self.counts.inc(&n, Event::Moves);
            self.counts.inc_move(&n, move_type);
            count += 1;
            self.stats.inc_move(ply);
            let mut child_board = b.make_move(&mv);
            let ext = self.extend(b, &child_board, mv, move_type, count, &n, last_move);
            let is_quiet = self.is_quiet(b, mv, count, move_type, &child_board, &n, ext);
            if is_quiet {
                quiets += 1;
            }

            if score > -Score::INFINITY {
                if let Some(est_score) =
                    self.can_prune_move(mv, count, move_type, b, &child_board, eval, &n, ext)
                {
                    self.stats.inc_fp_move(ply);
                    if score == -Score::INFINITY {
                        score = est_score;
                    }
                    if self.can_prune_remaining_moves(b, move_type, &n) {
                        break;
                    }
                    continue;
                }
            }
            let lmr = if !self.minmax {
                self.lmr(
                    b,
                    mv,
                    count,
                    quiets,
                    move_type,
                    &child_board,
                    &n,
                    nt,
                    ext,
                    tt_mv,
                )
            } else {
                0
            };

            if self.can_lmp(is_quiet, quiets, &n) {
                continue;
            }

            self.repetition.push_move(&mv, &child_board);
            self.current_variation.push(mv);
            // self.explainer.start(&n, &self.current_variation);
            child_board.set_repetition_count(self.repetition.count(&child_board));

            let pvs = self.pvs_permitted(nt, b, &n, count);
            let (mut child_score, mut cat) = if pvs {
                debug_assert!(n.alpha.is_numeric());
                self.stats.inc_pvs_move(ply);
                // using [alpha, alpha + 1]
                self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - lmr - 1,
                    -n.alpha - Score::from_cp(1),
                    -n.alpha,
                    mv,
                )?
            } else {
                self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - lmr - 1,
                    -n.beta,
                    -n.alpha,
                    mv,
                )?
            };
            child_score = -child_score;

            // window was [alpha, alpha + 1]
            // child_score >= beta => fail high as we are fail-soft
            // child_score <= alpha => fail low
            // alpha < child_score < score < beta and search upper bound was beta/!pvs then skip as too low
            // alpha < child_score < score < beta and search upper bound was alpha+1/pvs => failed high so research
            // if (lmr > 0 && self.lmr.re_search || pvs) && child_score > score && child_score < n.beta {
            if (lmr > 0 && self.lmr.re_search || pvs)
                && child_score < n.beta
                && child_score > n.alpha
                && (child_score >= score || pvs)
            {
                // research with full window without reduction in depth
                self.stats.inc_pvs_research(ply);
                let res = self.alphabeta_recursive(
                    &mut child_board,
                    ply + 1,
                    depth + ext - 1,
                    -n.beta,
                    -n.alpha,
                    mv,
                )?;
                if lmr > 0 && self.lmr.re_search {
                    self.counts.inc(&n, Event::LmrReSearch);
                }
                if pvs {
                    self.counts.inc(&n, Event::PvsReSearch);
                }
                child_score = -res.0;
                cat = res.1;
            }
            b.undo_move(&mv);
            self.current_variation.pop();
            self.repetition.pop();
            self.explain_move(mv, child_score, cat, &n);

            // println!("move {} score {} alpha {} beta {}", mv, score, alpha, beta);
            debug_assert!(
                !(child_score == -Score::INFINITY && count == 1),
                "board: {}\nmove: {}\ncat: {}\nlmr: {}\npvs: {}",
                b,
                mv,
                cat,
                lmr,
                pvs
            );

            if child_score > score {
                score = child_score;
                category = cat;
            }
            if child_score > n.alpha {
                n.alpha = child_score;
                bm = mv;
                nt = NodeType::ExactPv;
                debug_assert!(b.is_pseudo_legal_move(&bm), "bm {} on board {}", bm, b);
                self.history.raised_alpha(&n, b, &mv);
                self.record_move(ply, &mv);
            } else {
                self.history.duff(&n, b, &mv);
            }

            if n.alpha >= n.beta && !self.minmax {
                nt = NodeType::LowerCut;
                self.stats.inc_node_cut(ply, move_type, (count - 1) as i32);
                self.killers.store(ply, &mv);
                self.history.beta_cutoff(&n, b, &mv);
                self.report_refutation(n.ply);
                break;
            }
        }

        if count == 0 {
            self.stats.inc_leaf_nodes(&n);
            if n.is_qs() {
                self.counts.inc(&n, Event::NodeLeafQuietEval);
                return Ok((
                    b.eval_with_outcome(&self.eval, &n),
                    Event::NodeLeafQuietEval,
                ));
            } else {
                self.counts.inc(&n, Event::NodeLeafStalemate);
                // FIXME VER:0.4.14
                // (board.eval_draw(&mut self.eval, &n),
                return Ok((
                    b.eval_with_outcome(&self.eval, &n),
                    Event::NodeLeafStalemate,
                ));
            }
        }
        if nt == NodeType::UpperAll {
            self.stats.inc_node_all(ply);
            cat = Event::NodeInteriorAll;
            self.counts.inc(&n, cat);
            // nothing
        } else if nt == NodeType::LowerCut {
            debug_assert!(!bm.is_null());
            cat = Event::NodeInteriorCut;
            self.counts.inc(&n, cat);
        } else if nt == NodeType::ExactPv {
            self.stats.inc_node_pv(ply);
            cat = Event::NodeInteriorPv;
            self.counts.inc(&n, cat);
            // self.record_new_pv(ply, &bm, false);
            debug_assert!(!bm.is_null())
        } else {
            panic!("Node type {:?} ", nt);
        }
        // aspiration search fails dont get stored
        if score > -Score::INFINITY && score < Score::INFINITY {
            let entry = TtNode {
                score,
                depth,
                nt,
                bm,
            };
            self.tt.store(b.hash(), entry);
        }
        debug_assert!(
            n.alpha != -Score::INFINITY || n.beta != Score::INFINITY || score.is_numeric_or_mate(),
            "alpha/beta full width with score: {} node {}\ncat: {} count: {} quiets: {}\nbm: {}\nnt: {}\nboard: {:#}",
            score,
            n,
            category,
            count,
            quiets,
            bm,
            nt,
            b,
        );
        self.explain_node(bm, nt, score, eval, &n, cat, &self.pv_table.extract_pv_for(ply));
        Ok((score, category))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::search::timecontrol::*;
    use anyhow::Result;

    #[test]
    fn test_2_mates() -> Result<()> {
        let positions = Catalog::mate_in_2();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            search.set_timing_method(TimeControl::Depth(8)); // .set_callback(Uci::uci_info);
                                                             // search.tt.enabled = false;
            search.set_position(pos.clone()).search();
            // println!("{}", search);
            assert_eq!(
                search.pv().to_string(),
                pos.pv()?.to_string(),
                "#{} {}",
                i,
                pos
            );
            assert_eq!(search.score().is_mate(), true);
            println!("Mate in {}", search.score().mate_in().unwrap());
            assert_eq!(search.score().mate_in(), Some(2), "#{} {}", i, pos);
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_mate_in_3() -> Result<()> {
        let positions = Catalog::mate_in_3();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            search.set_timing_method(TimeControl::Depth(5));
            let expected_pv = pos.pv()?;
            search.set_position(pos.clone()).search();
            println!("{}", search);

            assert_eq!(search.score().mate_in(), Some(3), "#{} {}", i, pos);
            assert_eq!(search.pv_table.extract_pv(), expected_pv, "#{} {}", i, pos);
        }
        Ok(())
    }

    #[test]
    #[ignore]
    fn test_mate_in_4() -> Result<()> {
        let positions = Catalog::mate_in_4();
        for (i, pos) in positions.iter().enumerate() {
            let mut search = Algo::new();
            search.set_timing_method(TimeControl::Depth(7));
            search.set_position(pos.clone()).search();
            // println!("{}", search);
            if pos.get("pv").is_ok() {
                let expected_pv = pos.pv()?;
                assert_eq!(
                    search.pv_table.extract_pv().to_string(),
                    expected_pv.to_string(),
                    "#{} {}",
                    i,
                    pos
                );
            }
            println!("Mate in {}", search.score().mate_in().unwrap());
            assert_eq!(search.score().mate_in(), Some(4), "#{} {}", i, pos);
        }
        Ok(())
    }
}
