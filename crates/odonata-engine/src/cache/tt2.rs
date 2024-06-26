use std::fmt;
use std::sync::Arc;

use odonata_base::domain::node::{Counter, Timing};
use odonata_base::domain::BoundType;
use odonata_base::infra::component::{Component, State};
use odonata_base::infra::lockless_hashmap::HashEntry;
use odonata_base::infra::metric::Metrics;
use odonata_base::mv::BareMove;
use odonata_base::prelude::*;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use tracing::{debug, info};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TtNode {
    pub score: TtScore,
    pub depth: Ply,
    pub bt:    BoundType,
    pub bm:    BareMove,
    pub eval:  Score,
}

/// TtScore has mate scores relative to current ply, NOT to root board
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TtScore(Score);

impl TtScore {
    #[inline]
    pub fn new(s: Score, ply: Ply) -> TtScore {
        TtScore(match s {
            s if s >= Score::we_win_in(0) => Score::we_win_in(s.ply_win() - ply),
            s if s <= Score::we_lose_in(0) => Score::we_lose_in(s.ply_loss() - ply),
            _ => s,
        })
    }

    #[inline]
    pub fn as_score(&self, ply: Ply) -> Score {
        match self.0 {
            s if s >= Score::we_win_in(0) => Score::we_win_in(s.ply_win() + ply),
            s if s <= Score::we_lose_in(0) => Score::we_lose_in(s.ply_loss() + ply),
            _ => self.0,
        }
    }
    pub fn pack_16bits(&self) -> u64 {
        let bytes = self.0.as_i16().to_le_bytes();
        u64::from_le_bytes([bytes[0], bytes[1], 0, 0, 0, 0, 0, 0])
    }

    pub fn unpack_16bits(bits: u64) -> TtScore {
        // if bits == 0 {
        //     return -Score::INFINITY;
        // }

        let bytes = bits.to_le_bytes();
        let int = i16::from_le_bytes([bytes[0], bytes[1]]);
        TtScore(Score::from_cp(int as i32))
    }
}

// pub fn unpack_12bits_part1(bits: U64, b: &Board) -> (Square, Square, Piece) {
//     let from = Square::from_index(bits & 63);
//     let mut to = Square::from_index((bits >> 6) & 63);
//     let mover = b.piece_at(from.as_bb());
//     if from.as_bb().intersects(
//         b.pawns() & (
//             BitBoard::RANK_7 & b.white()
//             |
//             Bitboard::RANK_2 & b.black())) {
//         // its a pawn promo
//         let file = to.file_index();
//         let promo = Piece::from_index(to.rank());
//         let rank = if from.rank_index() == 6 { 7 } else { 0 };  // 6->7 and 1->0
//         to = Square::from_xy(file, rank);
//     }
//     return (from, to, promo)
// }

impl TtNode {
    // const HIGH26: u64 = (1 << 26 ) -1 >> (64-26);

    // pub fn pack(hash: Hash, node: &TTNode) -> u64 {
    //     // age in bits 0-7
    //     let bits = (node.draft as u8) << 8;         // bits 8-15
    //     bits |= (node.node_type as u16 & 3) << 16;   // bits 16 and 17
    //     bits |= (node.bm.pack_12bits()) << 18;  // bits 18-29
    //     bits |= (node.score.pack_16bits())  << 30;   // bits 30-45
    //     bits;
    // }

    // pub fn unpack(mut bits: u64, b: &Board, hash: Hash) -> u64 {
    //     let draft = (bits >> 8) & 255;
    //     let node_type = (bits >> 16) & 3;
    //     let bm = Move::unpack_12bits((bits >> 18));
    //     let score = Score::unpack_10bits((bits >> 30) & ((2<<11) -1))
    //     TtNode { draft, node_type, bm, score }
    // }

    pub fn pack(node: &TtNode, age: u8) -> u64 {
        let mut bits = age as u64; // age in bits 0-7
        bits |= ((node.depth & 255) as u64) << 8; // bits 8-15
        bits |= (node.bt as u64 & 3) << 16; // bits 16 and 17
        bits |= (node.score.pack_16bits()) << 18; // bits 18-33
        bits |= (node.bm.pack_14bits()) << 34; // bits 34-47
        bits |= (TtScore::new(node.eval, 0).pack_16bits()) << 48; // bits 48-63
        bits
    }

    pub fn unpack(bits: u64) -> (TtNode, u8) {
        // age
        let draft = (bits >> 8) & 255;
        let node_type = BoundType::unpack_2bits((bits >> 16) & 3);
        let score = TtScore::unpack_16bits((bits >> 18) & ((2 << 16) - 1));
        let bm = BareMove::unpack_14bits(bits >> 34);
        let eval = TtScore::unpack_16bits((bits >> 48) & ((2 << 16) - 1)).as_score(0);
        (
            TtNode {
                depth: draft as i32,
                bt: node_type,
                bm,
                score,
                eval,
            },
            (bits & 255) as u8,
        )
    }

    pub fn validate_move(&self, bd: &Board) -> Move {
        match self.bm.validate(bd) {
            Ok(mv) => mv,
            Err(_e) => {
                Metrics::incr(Counter::TtIllegalMove);
                Move::new_null()
            }
        }
    }
}

impl fmt::Display for TtNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{:>6} {:>10} {:>3} {:>2}",
                self.bm,
                self.score.0.to_string(),
                self.depth,
                self.bt
            )
        } else {
            write!(
                f,
                "{} scoring {} draft {} type {}",
                self.bm, self.score.0, self.depth, self.bt
            )
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, EnumString)]
enum Replacement {
    Always,
    Age,
    AgeTypeDepth,
    AgeDepthType,
    AgeBlend,
}

use odonata_base::infra::lockless_hashmap::AlignedVec;
type Table = AlignedVec<HashEntry>;
// use super::lockless_hashmap::SharedTable;
// type TABLE=SharedTable;

// FIXME Mates as score
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct TranspositionTable2 {
    #[serde(skip)]
    table: Arc<Table>,

    pub enabled:            bool,
    pub shared:             bool,
    aging:                  bool,
    persistent:             bool,
    improve_non_pv_bounds:  bool, // https://www.talkchess.com/forum3/viewtopic.php?t=59856
    pub use_tt_for_pv:      bool,
    pub eval_from_tt:       EvalFromTt,
    pub tt_for_eval_depth:  i32,
    pub allow_truncated_pv: bool,
    pub mb:                 i64,
    hmvc_horizon:           i32,
    min_ply:                Ply,
    min_depth:              Ply,
    buckets:                usize,
    aligned:                bool,
    overlapping_buckets:    bool,
    cacheline_size:         usize,
    rewrite_pv:             bool,
    rewrite_pv_score:       bool,
    freshen_on_fetch:       bool,
    replacement:            Replacement,
    preserve_bm:            bool,

    #[serde(skip)]
    pub current_age: u8,
}

impl Default for TranspositionTable2 {
    fn default() -> Self {
        Self {
            table:                 Arc::new(Table::default()),
            enabled:               true,
            shared:                true,
            use_tt_for_pv:         false,
            improve_non_pv_bounds: false,
            allow_truncated_pv:    false,
            eval_from_tt:          EvalFromTt::UseTtScore,
            tt_for_eval_depth:     0,
            mb:                    8,
            aging:                 true,
            persistent:            true,
            buckets:               2,
            aligned:               false,
            cacheline_size:        64,
            overlapping_buckets:   false,
            current_age:           10, // to allow us to look back
            hmvc_horizon:          85,
            min_ply:               1, // search restrictions on ply=0
            min_depth:             1,
            rewrite_pv:            true,
            rewrite_pv_score:      false,
            freshen_on_fetch:      true,
            replacement:           Replacement::AgeTypeDepth,
            preserve_bm:           false,
            // deletes: Stat::new("deletes"),
            // fail_priority: Stat::new("ins fail priority"),
            // fail_ownership: Stat::new("ins fail owner"),
        }
    }
}

impl Configurable for TranspositionTable2 {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.enabled.set(p.get("enabled"))?;
        self.shared.set(p.get("shared"))?;
        self.use_tt_for_pv.set(p.get("use_tt_for_pv"))?;
        self.improve_non_pv_bounds.set(p.get("improve_non_pv_bounds"))?;
        self.allow_truncated_pv.set(p.get("allow_truncated_pv"))?;
        self.eval_from_tt.set(p.get("eval_from_tt"))?;
        self.tt_for_eval_depth.set(p.get("tt_for_eval_depth"))?;
        self.mb.set(p.get("mb"))?;
        self.aging.set(p.get("aging"))?;
        self.persistent.set(p.get("persistent"))?;
        self.buckets.set(p.get("buckets"))?;
        self.aligned.set(p.get("aligned"))?;
        self.cacheline_size.set(p.get("cacheline_size"))?;
        self.overlapping_buckets.set(p.get("overlapping_buckets"))?;
        self.current_age.set(p.get("current_age"))?;
        self.hmvc_horizon.set(p.get("hmvc_horizon"))?;
        self.min_ply.set(p.get("min_ply"))?;
        self.min_depth.set(p.get("min_depth"))?;
        self.rewrite_pv.set(p.get("rewrite_pv"))?;
        self.rewrite_pv_score.set(p.get("rewrite_pv_score"))?;
        self.freshen_on_fetch.set(p.get("freshen_on_fetch"))?;
        self.replacement.set(p.get("replacement"))?;
        self.preserve_bm.set(p.get("preserve_bm"))?;
        Ok(p.is_modified())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, EnumString)]
pub enum EvalFromTt {
    Never,
    UseTtEval,
    UseTtScore,
}

impl fmt::Display for TranspositionTable2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(&self).unwrap())?;
        writeln!(
            f,
            "table:\n{}",
            toml::to_string_pretty(self.table.as_ref())
                .unwrap()
                .lines()
                .map(|s| "    ".to_string() + s)
                .join("\n")
        )?;
        Ok(())
    }
}

impl Component for TranspositionTable2 {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => {
                self.resize_if_required();
                self.current_age = 10;
                self.clear()
            }
            SetPosition => self.next_generation(),
            StartSearch => self.resize_if_required(),
            EndSearch => {}
            StartDepthIteration(_) => {}
            Shutdown => {}
        }
    }

    fn new_game(&mut self) {
        unreachable!();
        // self.set_state(State::NewGame)
    }

    fn new_position(&mut self) {
        unreachable!();
        // self.set_state(State::SetPosition)
    }
}

impl TranspositionTable2 {
    fn resize_if_required(&mut self) {
        if self.requires_resize() {
            let capacity = Table::convert_mb_to_capacity(self.mb);
            debug!(
                "tt resized so capacity is now {} with {} buckets",
                capacity, self.buckets
            );

            debug!("In resize table with aligned = {} cap = {}", self.aligned, capacity);
            let mut table = Table::default();
            table.resize(
                capacity,
                self.buckets,
                self.aligned,
                self.overlapping_buckets,
                self.cacheline_size,
            );
            self.table = Arc::new(table);
            self.current_age = 10;
        }
    }

    fn clear(&mut self) {
        self.table.iter().for_each(|he| he.set_empty());
    }

    pub fn rewrite_pv(&mut self, base: &Board, pv: &Variation) {
        if self.rewrite_pv {
            for i in 0..pv.len() {
                let b = base.make_moves(&pv.take(i));
                if let Some(mut entry) = self.probe_by_hash(b.hash()) {
                    entry.bm = pv[i].to_inner();
                    self.store(b.hash(), entry);
                }
            }
        }
    }

    pub fn fmt_extract_pv_and_score(&self, f: &mut fmt::Formatter, b: &Board) -> fmt::Result {
        let (var, _) = self.extract_pv_and_score(b);
        for mv in var.moves() {
            writeln!(f, "{:#}", mv)?
        }
        Ok(())
    }

    pub fn next_generation(&mut self) {
        // if self.requires_resize() {
        //     info!("Resizing tt");
        //     self.new_game();
        // } else {
        if !self.persistent {
            self.clear()
        }
        if self.aging {
            self.current_age = self.current_age.wrapping_add(1);
            debug!("aging tt to age {}", self.current_age);
        }
        // }
    }

    pub fn requires_resize(&self) -> bool {
        let capacity = Table::convert_mb_to_capacity(self.mb);
        debug!(
            "tt current capacity {} and {} mb implies capacity of {}",
            self.table.capacity(),
            self.mb,
            capacity
        );
        self.table.capacity() != capacity
    }

    // pub fn count_of(&self, t: NodeType) -> usize {
    //     let mut count = 0;
    //     for i in 0..self.table.capacity() {
    //         let (h, d) = self.table.probe_by_index(i);
    //         if h == 0 && d == 0 {
    //             continue;
    //         }
    //         if self.table.index(h) == i {
    //             let tt_node = TtNode::unpack(d).0;
    //             if tt_node.node_type == t {
    //                 count += 1;
    //             }
    //         }
    //     }
    //     count
    // }

    // pub fn count_of_age(&self, age: u8) -> usize {
    //     (0..self.table.capacity())
    //         .into_iter()
    //         .filter(|&i| self.table.probe_by_index(i).1 & 255 == age as u64)
    //         .count()
    // }

    #[inline]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    #[inline]
    fn age_of(b: &HashEntry) -> u8 {
        (b.data() & 255) as u8
    }

    pub fn hashfull_per_mille(&self) -> u32 {
        self.table.hashfull_per_mille()
    }

    //     let count = self
    //         .table
    //         .iter()
    //         .take(200)
    //         .filter(|&b| Self::age_of(b) == self.current_age)
    //         .count();
    //     count as u32 * 1000 / 200
    // }

    #[inline]
    pub fn store(&mut self, h: Hash, mut new_node: TtNode) {
        // FIXME maybe store QS results
        if !self.enabled && new_node.bt != BoundType::ExactPv || self.capacity() == 0 || new_node.depth < 0 {
            return;
        }
        let t = Metrics::timing_start();
        debug_assert!(new_node.bt != BoundType::Unused, "Cannot store unused nodes in tt");
        debug_assert!(
            new_node.score.0.is_finite(),
            "Cannot store score {} in tt\n{}",
            new_node.score.0,
            new_node
        );

        // probe by hash not board so any "conditions" are bypassed
        let mut bucket_to_overwrite = None;
        let buckets = self.table.bucket(h);
        // // FIXME! race condition here we dont worry about
        // if false && hash != h {
        //     self.table.utilization.increment();
        //     let new_data = TtNode::pack(&new_node, self.current_age);
        //     let unpacked = TtNode::unpack(new_data).0;
        //     debug_assert!(unpacked == new_node, "{:?} {:?}", unpacked, new_node);
        //     self.table.store(h, new_data);
        //     return;
        // }

        enum MatchType {
            Empty,
            SameHash,
            DifferentHash,
        }
        let mut match_type = MatchType::Empty;
        // try and find a matching hash first
        let new_data = TtNode::pack(&new_node, self.current_age);
        for bucket in buckets.iter() {
            let key = bucket.key();
            let data = bucket.data();
            if HashEntry::has_hash(h, (key, data)) {
                bucket_to_overwrite = Some(bucket);
                match_type = MatchType::SameHash;
                break;
            }
        }
        // find an empty one
        if bucket_to_overwrite.is_none() {
            for bucket in buckets.iter() {
                let key = bucket.key();
                let data = bucket.data();
                if HashEntry::is_empty(key, data) {
                    bucket_to_overwrite = Some(bucket);
                    match_type = MatchType::Empty;
                    break;
                }
            }
        }
        // find oldest
        if bucket_to_overwrite.is_none() {
            let mut oldest = 1000;
            for bucket in buckets.iter() {
                let data = bucket.data();
                let old_age = data & 255;
                // let (_old_node, old_age) = TtNode::unpack(data);
                if (old_age as i32) < oldest {
                    oldest = old_age as i32;
                    match_type = MatchType::DifferentHash;
                    bucket_to_overwrite = Some(bucket);
                }
            }
        }

        let data = bucket_to_overwrite.unwrap().data();
        let (old_node, old_age) = TtNode::unpack(data);

        let replace = match (self.replacement, &match_type) {
            (_, MatchType::Empty) => true,
            (Replacement::Always, _) => true,
            (Replacement::Age, _) => self.current_age > old_age,
            (Replacement::AgeTypeDepth, _) => {
                self.current_age > old_age
                    || self.current_age == old_age
                        && (new_node.bt > old_node.bt || new_node.bt == old_node.bt && new_node.depth >= old_node.depth)
            }
            (Replacement::AgeDepthType, _) => {
                self.current_age > old_age
                    || self.current_age == old_age
                &&
                // even when the draft is the same we overwrite, as more nodes may have been used in calculating due to a fuller tt..
                (new_node.depth >= old_node.depth
                || new_node.depth == old_node.depth && new_node.bt > old_node.bt)
            }
            (Replacement::AgeBlend, _) => {
                self.current_age > old_age
                    || self.current_age == old_age
                &&
                // overwrite with deeper as long as not overwriting an exact with a non-exact
                new_node.depth >= old_node.depth
                &&
                (new_node.bt == BoundType::ExactPv || old_node.bt != BoundType::ExactPv)
            }
        };
        if replace {
            // new.hash != old.hash &&
            if self.current_age == old_age && old_node.bt == BoundType::ExactPv {
                Metrics::incr(Counter::TtPvOverwrite);
            }
            debug_assert!(new_node.score.0 > -Score::INFINITY && new_node.score.0 < Score::INFINITY);
            debug_assert!(
                new_node.bt != BoundType::ExactPv || !new_node.bm.is_null(),
                "bm is null at {:?} mv {:?}",
                new_node.bt,
                new_node.bm
            );
            if let MatchType::SameHash = match_type {
                if self.preserve_bm && new_node.bm.is_null() {
                    new_node.bm = old_node.bm;
                }
            }
            bucket_to_overwrite.unwrap().write(h, new_data);
        } else {
            // self.fail_priority.increment();
        }
        Metrics::profile(t, Timing::TimingTtStore);
    }

    pub fn delete(&mut self, _h: Hash) {
        if !self.enabled || self.capacity() == 0 {}
        // self.deletes.increment();
        // self.table.delete(h);
    }

    pub fn probe_by_board(&self, board: &Board, ply: Ply, depth: Ply) -> Option<TtNode> {
        // never probe at root as we may retrict moves (or be using multi-pv there)
        if !self.enabled || self.capacity() == 0 || ply < self.min_ply || depth < self.min_depth {
            return None;
        }
        let t = Metrics::timing_start();
        let tt_node = self.probe_by_hash(board.hash());
        Metrics::profile(t, Timing::TimingTtProbe);
        tt_node
    }

    #[inline]
    fn probe_raw(&self, h: Hash) -> Option<(u64, &HashEntry)> {
        for entry in self.table.bucket(h) {
            let key = entry.key();
            let data = entry.data();
            if HashEntry::is_empty(key, data) {
                continue;
            }
            let hash = key ^ data;
            if hash == h {
                return Some((data, entry));
            }
        }
        None
    }

    pub fn probe_by_hash(&self, h: Hash) -> Option<TtNode> {
        // debug!("Probe by hash");
        // if !self.enabled || self.capacity() == 0 {
        //     return None;
        // }
        if let Some((data, bucket)) = self.probe_raw(h) {
            let new_data = (data & !255) | (self.current_age as u64 & 255);
            if self.freshen_on_fetch {
                bucket.write(h, new_data);
            }
            Some(TtNode::unpack(data).0)
        } else {
            Metrics::incr(Counter::TtCollision);
            None
        }
    }

    pub fn extract_pv_and_score(&self, b: &Board) -> (Variation, Option<Score>) {
        let mut board = b.clone();
        let mut var = Variation::new();
        let mut score = None;
        // board = board.make_move(&first);
        // moves.push(*first);
        while var.len() < 50 {
            // probe by hash to avoid all the board filters (ply etc)
            let entry = self.probe_by_hash(board.hash());
            if let Some(entry) = entry {
                if entry.bt == BoundType::ExactPv {
                    let mv = entry.validate_move(&board);
                    if !mv.is_null() {
                        board = board.make_move(mv);
                        var.push(mv);
                        if score.is_none() {
                            score = Some(entry.score.0); // score at root is same as WrtRoot
                        }
                        continue;
                    } else {
                        debug_assert!(
                            false,
                            "Invalid move {} in tt for board position {} from starting {}",
                            mv,
                            board.to_fen(),
                            b.to_fen()
                        );
                        error!(
                            "Invalid move {} for nt {} in tt for board position {}",
                            mv,
                            entry.bt,
                            board.to_fen()
                        );
                        break;
                    }
                }
                if var.is_empty() {
                    info!("root node is {:?}", entry.bt);
                }
            }
            // println!("Unable to find hash {} after move {}", board.hash(), mv) ;
            break;
        }
        (var, score)
    }
}

#[cfg(test)]
mod tests {
    use odonata_base::catalog::*;
    use odonata_base::other::tags::EpdOps as _;
    use test_log::test;

    use super::*;
    use crate::comms::uci_server::*;
    use crate::search::engine::ThreadedSearch;

    fn entry123() -> TtNode {
        TtNode {
            score: TtScore(300.cp()),
            depth: 2,
            bt:    BoundType::ExactPv,
            bm:    Move::new_quiet(Piece::Pawn, Square::B7, Square::B6, &Board::starting_pos()).to_inner(),
            eval:  123.cp(),
        }
    }

    fn entry456() -> TtNode {
        TtNode {
            score: TtScore(200.cp()),
            depth: 3,
            bt:    BoundType::ExactPv,
            bm:    Move::new_quiet(Piece::Pawn, Square::A2, Square::A3, &Board::starting_pos()).to_inner(),
            eval:  456.cp(),
        }
    }

    fn entry456b() -> TtNode {
        TtNode {
            score: TtScore(201.cp()),
            depth: 4,
            bt:    BoundType::ExactPv,
            bm:    Move::new_quiet(Piece::Rook, Square::A1, Square::A2, &Board::starting_pos()).to_inner(),
            eval:  Score::zero(),
        }
    }

    #[test]
    fn test_tt_score() {
        assert_eq!(TtScore::new(Score::we_lose_in(5), 3).as_score(3), Score::we_lose_in(5));
        assert_eq!(TtScore::new(Score::we_win_in(5), 3).as_score(3), Score::we_win_in(5));
    }

    #[test]
    fn test_tt() {
        let mut tt1 = TranspositionTable2::default();
        tt1.set_state(State::NewGame);
        info!("display\n{tt1}\ndebug\n{tt1:?}");
        info!("After new game");
        let board = Catalog::starting_board();
        let moves = tt1.extract_pv_and_score(&board).0;
        info!("After extract");
        assert_eq!(moves.to_uci(), "");
        assert_eq!(tt1.hashfull_per_mille(), 0);
        manipulate(&mut tt1);
        assert_eq!(tt1.table.utilization(), 2);
        tt1.set_state(State::NewGame);
        assert!(tt1.probe_by_hash(123).is_none());

        // triggers failed ownership panic
        if false {
            let mut tt2 = tt1.clone();
            println!("Cloned tt1 -> tt2 ...{}", Arc::strong_count(&tt1.table));
            tt2.store(123, entry123());
            tt2.store(456, entry456());
            println!("{:?}", tt2);
            println!("{}", tt2);
        }
        println!("Dropped tt2 ...{}", Arc::strong_count(&tt1.table));
        assert_eq!(tt1.table.utilization(), 0);
        manipulate(&mut tt1);
        assert_eq!(tt1.table.utilization(), 2);
        tt1.set_state(State::NewGame);
        assert!(tt1.probe_by_hash(123).is_none());

        let mut tt3 = tt1.clone();
        // tt1.destroy();
        // println!(
        //     "Clone tt1 -> tt3 and destroy tt1 ...{}",
        //     Arc::strong_count(&tt3.table)
        // );
        manipulate(&mut tt3);
    }

    fn manipulate(tt: &mut TranspositionTable2) {
        let entry123 = entry123();
        let entry456 = entry456();
        let entry456b = entry456b();

        assert_eq!(tt.capacity(), 524288);
        tt.delete(123);
        tt.delete(456);
        tt.delete(456);
        assert!(tt.probe_by_hash(123).is_none());
        tt.store(123, entry123);
        tt.store(456, entry456);
        assert_eq!(tt.probe_by_hash(123), Some(entry123));
        assert_eq!(tt.probe_by_hash(124), None);
        assert_eq!(tt.probe_by_hash(456), Some(entry456));
        tt.store(456, entry456b);
        assert_eq!(tt.probe_by_hash(456), Some(entry456b));

        // store fails due to ply, leaving 456b in place
        tt.store(456, entry456);
        assert_eq!(tt.probe_by_hash(456), Some(entry456b));

        // store succeeds due to age
        tt.next_generation();
        tt.store(456, entry456);
        assert_eq!(tt.probe_by_hash(456), Some(entry456));

        println!("{:?}", tt);
        println!("{}", tt);
        println!(
            "utilization {}, capacity {}",
            tt.table.utilization(),
            tt.table.capacity()
        );

        // 1 entry in 200 is 5%%
        assert_eq!(tt.hashfull_per_mille(), 5);
    }

    #[test]
    #[ignore]
    fn tt2_end_games() -> anyhow::Result<()> {
        // //let mut tt1 = TranspositionTable::with_capacity(TranspositionTable::convert_mb_to_capacity(10));
        let mut eng = ThreadedSearch::new();
        eng.search.tt.mb = 128;
        // use simple evaluation as we look at great depth
        eng.set_callback(UciServer::uci_info);
        for epd in Catalog::famous().iter() {
            eng.new_game();
            let res = eng.search(epd.clone(), TimeControl::Depth(33)).unwrap();
            assert_eq!(
                res.supplied_move().unwrap().to_uci(),
                epd.moves("bm").unwrap().to_uci(),
                "{epd}\n{eng}"
            );
        }
        Ok(())
    }

    #[test]
    fn tt2_test_pv_extraction() {
        info!("log");
        let mut eng = ThreadedSearch::new();
        let d = 2;
        for pos in Catalog::bratko_kopec() {
            println!("{}", pos);
            eng.new_game();
            let res = eng.search(pos.clone(), TimeControl::Depth(d)).unwrap();
            let tt_pv = eng.search.tt.extract_pv_and_score(&pos.board()).0;
            let res_pv = res.pv();

            // No reason acd = pv length as pv line may be reduced due to lmr etc.
            assert!(tt_pv.len() >= (d as usize) - 1, "eng.pv=<{res_pv}> ttpv=<{tt_pv}>",);
            // certainly pv can be longer as it has qsearch
            // assert!(
            //     pv.len() <= d as usize,
            //     "{}\n algo.pv={} pv={}",
            //     algo,
            //     algo.pv(),
            //     pv,
            // );
            // assert!(algo.pv().len() >= d as usize, "{} {}\n{}", algo.pv(), pv, algo);
            // assert_eq!(algo.bm().uci(), pos.bm()?.uci());
            println!(">>>>>> {}", tt_pv);
        }
    }

    #[test]
    fn tt2_test_new_game() {
        let mut eng = ThreadedSearch::new();
        // eng.new_game();
        eng.new_game();
        println!("Before 1\n{:#?}", eng.search.eval);
        eng.search(Catalog::starting_position(), TimeControl::Depth(6)).unwrap();
        println!("After 1\n{}", eng.search);
        let mut eng = ThreadedSearch::new();
        eng.new_game();
        println!("Before 2\n{}", eng.search);
        eng.search(Catalog::starting_position(), TimeControl::Depth(6)).unwrap();
        println!("After 2{}", eng.search);
    }
}
