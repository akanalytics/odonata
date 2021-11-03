use crate::bound::NodeType;
use crate::infra::parsed_config::Component;
use crate::search::node::Node;
use crate::board::Board;
use crate::mv::Move;
use std::{fmt};
use serde::{Deserialize, Serialize};
use crate::search::algo::Algo;
use crate::types::{MoveType, Ply};
use super::endgame::EndGame;
use super::score::Score;




#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Recognizer {
    enabled: bool, 
    short_circuit: Ply,
}


impl Default for Recognizer {
    fn default() -> Self {
        Self {
            enabled: false,
            short_circuit: 100,
        }
    }
}

impl Component for Recognizer {
    fn new_game(&mut self) {
    }

    fn new_position(&mut self) {
    }
}

impl fmt::Display for Recognizer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:#?}", self)?;
        Ok(())
    }
}


impl Algo {
    #[inline]
    pub fn lookup(&mut self, b: &mut Board, n: &mut Node) -> (Option<Score>, Option<Move>) {
        if n.ply == 0 {
            return (None, None)
        }

        // let (score, mv) = self.wdl_detection(b, n);
        // if score.is_some() {
        //     return (score,mv);
        // }

        if self.tt.probe_leaf_nodes || self.is_leaf(n.ply, n.depth) {

            if let Some(entry) = self.tt.probe_by_board(b, n.ply, n.depth) {

                // FIXME! v33
                if entry.draft >= n.depth && !(self.repetition.avoid_tt_on_repeats && b.repetition_count().total > 0) {
                //if entry.draft >= draft  && (ply >= 1 || self.tt.allow_tt_at_root) && !(b.repetition_count().total > 0 && self.repetition.avoid_tt_on_repeats)

                    if n.ply == 0 && self.restrictions.is_none() {
                        return (Some(entry.score), Some(entry.bm))
                    }

                    // if entry.draft >= draft && !(b.repetition_count().total > 1 && self.repetition.avoid_tt_on_repeats)
                    match entry.node_type {
                        NodeType::Pv => {
                            // previously this position raised alpha, but didnt trigger a cut
                            // no point going through moves as we know what the max score is
                            if entry.score >= n.beta {
                                self.stats.inc_node_cut(n.ply, MoveType::Hash, -1);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                self.report_refutation(n.ply);
                                return (Some(entry.score), None);
                            }
                            if entry.score <= n.alpha {
                                self.stats.inc_node_all(n.ply);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                return (Some(entry.score), None);
                            }

                            if self.tt.allow_truncated_pv && entry.score > n.alpha {
                                self.record_truncated_move(n.ply, &entry.bm);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                return (Some(entry.score), None);
                            }
                            return (None, Some(entry.bm)); // else we just use the hash move for move ordering
                        }
                        NodeType::Cut => {
                            // previously this position raised alpha (sufficiently to cause a cut).
                            // not all child nodes were scored, so score is a lower bound
                            // FIXME: probably dont set alpha just the hinted mv and re-search the node
                            if entry.score >= n.beta {
                                self.stats.inc_node_cut(n.ply, MoveType::Hash, -1);
                                // self.record_truncated_move(ply, &entry.bm);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                self.report_refutation(n.ply);
                                return (Some(entry.score), None);
                            }
                            if self.tt.allow_truncated_pv && entry.score > n.alpha {
                                // nt = NodeType::Pv;
                                // n.alpha = entry.score;
                                self.record_truncated_move(n.ply, &entry.bm);
                                // score = entry.score;
                                // bm = entry.bm; // need to set bm as alpha raising mv might be skipped
                                //             // tt_mv = Some(entry.bm); // might help with move ordering
                                return (Some(entry.score), Some(entry.bm)); // else we just use the hash move for move ordering
                            }
                            return (None, Some(entry.bm)); // else we just use the hash move for move ordering
                        }
                        NodeType::All => {
                            // previously this position didnt raise alpha, the score is an upper bound
                            // if the score is still below alpha, this too is an ALL node
                            if entry.score <= n.alpha {
                                // self.record_truncated_move(ply, &entry.bm);
                                self.stats.inc_leaf_tt_nodes(n.ply);
                                return (Some(entry.score), None);
                            }
                        }
                        NodeType::Unused | NodeType::Terminal => unreachable!(),
                    }
                }
                // not enough draft - just use for move guidance
                return (None, Some(entry.bm))
            }
            // not found
        }
        // was leaf and isnt probed
        let (score, mv) = self.wdl_detection(b, n);
        if score.is_some() {
            return (score,mv);
        }

        return (None, None);
    }

    #[inline]
    pub fn wdl_detection(&mut self, b: &mut Board, n: &mut Node) -> (Option<Score>, Option<Move>) {

        if !self.recognizer.enabled  || n.depth == 0 || n.ply == 0 {
            return (None, None)
        }
        let endgame = EndGame::from_board(b);

        if endgame.is_immediately_declared_draw() {
            let draw = b.eval_draw(&mut self.eval, &n); // will return a draw score
            return (Some(draw), None)
        }

        if b.draw_outcome().is_some()  {
            self.stats.inc_leaf_nodes(n.ply);
            let draw = b.eval_draw(&mut self.eval, &n); // will return a draw score
            return (Some(draw), None)
        }

        // its a helpmate or draw like KNkn, so cease search only after a few moves since last capture
        if endgame.is_draw() {
            if n.depth > self.recognizer.short_circuit {
                n.depth = self.recognizer.short_circuit;
            }
        }
            // if b.fifty_halfmove_clock() >= 0  {
            //     let draw = b.eval_draw(&mut self.eval, &n); // will return a draw score
            //     return (Some(draw), None)
            // } else {
            //     // we let a couple of plies "play out" - enough that the game continues and we don't lose a piece
            //     return (None, None)
            // }

        if let Some(_color) = endgame.try_winner() {
            if n.depth > self.recognizer.short_circuit {
                n.depth = self.recognizer.short_circuit;
            }
            // if b.fifty_halfmove_clock() >= 0  {
            //     let sc = b.eval(&mut self.eval, &n); // will return a best-move-motivating score
            //     return (Some(sc), None)
            // } else {
            //     // we let a couple of plies "play out" - so that we discover if this is a draw (unlikely)
            //     return (None, None)
            // }    
        }
        return (None, None)
    }

}

#[cfg(test)]
mod tests {
    use crate::{Position, search::{engine::Engine, timecontrol::TimeControl}};
    use test_env_log::test;

    #[test]
    fn test_recognizer() {
        // let pos = Position::parse_epd("8/NN6/8/8/8/2K2nk1/4P3/8 w - - 0 1; id 'RECOG.01'; am e2f3; bm Nd6;c0 'white shouldnt take knight as recapture of pawn makes it KNN v k'").unwrap();
        // let pos = Position::parse_epd("k7/1p6/3N4/8/8/8/6N1/K6B w - - 5 1; id 'RECOG.02'; bm Nxb7; c0 'white should take pawn to leave KBN v k'").unwrap();
        let pos = Position::parse_epd("k7/8/K1p5/8/3N4/8/6N1/7B w - - 5 1; id 'RECOG.03'; am Nxc6; bm Kb6; c0 'white shouldnt take pawn c4 as triggers stalemate'").unwrap();
        // let pos = Position::parse_epd("k6K/8/2pN4/8/3N4/8/8/8 w - - 5 1;  id 'RECOG.04'; bm Nxc6; c0 'white should force stalemate'").unwrap();
        let mut engine = Engine::new();
        engine.set_position(pos.clone());
        engine.algo.set_timing_method(TimeControl::Depth(3));
        engine.algo.recognizer.enabled = true;
        engine.search();
        eprintln!("{}", engine.algo.results_as_position());
        eprintln!("{}", pos);
        assert_eq!(engine.algo.results_as_position().bm().unwrap(), pos.bm().unwrap());
    }
}

        // debug!("{}", sea

// 8/NN6/8/8/8/2K2nk1/4P3/8 w - - 0 1 - white shouldn't take knight
// k7/8/K1p5/8/3N4/8/6N1/7B w - - 5 1 - white shouldn't take pawn c4 as triggers stalemate