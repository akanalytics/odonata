use crate::bitboard::bitboard::Bitboard;
use crate::bitboard::castling::CastlingRights;
use crate::bitboard::precalc::BitboardDefault;
use crate::bitboard::square::Square;
use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::bound::NodeType;
use crate::cache::lockless_hashmap::{Bucket, SharedTable};
use crate::eval::score::Score;
use crate::infra::component::Component;
use crate::mv::Move;
use crate::trace::stat::{ArrayStat, Stat};
use crate::types::{Hash, Piece, Ply};
use crate::variation::Variation;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::fmt;
use std::sync::Arc;

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct TtNode {
    pub score: Score,
    pub depth: Ply,
    pub nt: NodeType,
    pub bm: Move,
}

impl Score {
    pub fn pack_16bits(&self) -> u64 {
        let bytes = self.as_i16().to_le_bytes();
        u64::from_le_bytes([bytes[0], bytes[1], 0, 0, 0, 0, 0, 0])
    }

    pub fn unpack_16bits(bits: u64) -> Score {
        // if bits == 0 {
        //     return -Score::INFINITY;
        // }

        let bytes = bits.to_le_bytes();
        let int = i16::from_le_bytes([bytes[0], bytes[1]]);
        Score::from_cp(int as i32)
    }
}

impl Move {
    pub fn pack_20bits(&self) -> u64 {
        if self.is_null() {
            return 0;
        }
        let to = self.to();
        let mut from = self.from();
        if self.is_promo() {
            // the rank of to-sq pawn promo can be deduced from the to-sq,
            // so we use from-rank as a store the promo piece
            let file = from.file_index();
            let rank = self.promo_piece().index();
            from = Square::from_xy(file as u32, rank as u32);
        }
        #[allow(clippy::identity_op)]
        let mut bits = (from.index() as u64) << 0; // 0-63 bits 0-5
        bits |= (to.index() as u64) << 6; // 0-63 bits 6-11
        let capture = self.capture_piece();
        let mover = self.mover_piece();
        bits |= (capture.index() as u64 & 7) << 12; //bits 12-14
        bits |= (mover.index() as u64 & 7) << 15; // bits 15-17
        bits |= (self.is_pawn_double_push() as u64 & 1) << 18;
        bits |= (self.is_ep_capture() as u64 & 1) << 19;
        bits
    }

    pub fn unpack_20bits(bits: u64) -> Move {
        if bits == 0 {
            return Move::NULL_MOVE;
        }
        let capture = Piece::from_index((bits >> 12) as usize & 7);
        let mover = Piece::from_index((bits >> 15) as usize & 7);
        let is_pawn_double_push = (bits >> 18) & 1 == 1;
        let is_ep_capture = (bits >> 19) & 1 == 1;

        let mut from = Square::from_u32(bits as u32 & 63);
        let to = Square::from_u32((bits >> 6) as u32 & 63);
        let mut promo = Piece::None;
        if mover == Piece::Pawn && to.as_bb().intersects(Bitboard::RANK_8 | Bitboard::RANK_1) {
            // its a pawn promo, from encodes the promo-piece
            let file = from.file_index();
            promo = Piece::from_index(from.rank_index());
            let rank = if to.rank_index() == 7 { 6 } else { 1 }; // 7->6 and 0->1
            from = Square::from_xy(file as u32, rank);
        }

        if mover == Piece::King && BitboardDefault::default().chebyshev_distance(from, to) > 1 {
            Move::new_castle(from, to, CastlingRights::from_king_move(to))
        } else if capture == Piece::None {
            if mover != Piece::Pawn {
                Move::new_quiet(mover, from, to)
            } else if promo == Piece::None {
                if is_pawn_double_push {
                    let ep = BitboardDefault::default().strictly_between(from, to).square();
                    Move::new_double_push(from, to, ep)
                } else {
                    Move::new_quiet(Piece::Pawn, from, to)
                }
            } else {
                Move::new_promo(from, to, promo)
            }
        } else if mover != Piece::Pawn {
            Move::new_capture(mover, from, to, capture)
        } else if is_ep_capture {
            let capture_sq = Square::from_xy(to.file_index() as u32, from.rank_index() as u32);
            Move::new_ep_capture(from, to, capture_sq)
        } else if promo != Piece::None {
            Move::new_promo_capture(from, to, promo, capture)
        } else {
            Move::new_capture(mover, from, to, capture)
        }
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
        bits |= (node.nt as u64 & 3) << 16; // bits 16 and 17
        bits |= (node.score.pack_16bits()) << 18; // bits 18-33
        bits |= (node.bm.pack_20bits()) << 34; // bits 34+
        bits
    }

    pub fn unpack(bits: u64) -> (TtNode, u8) {
        let draft = (bits >> 8) & 255;
        let node_type = NodeType::unpack_2bits((bits >> 16) & 3);
        let score = Score::unpack_16bits((bits >> 18) & ((2 << 16) - 1));
        let bm = Move::unpack_20bits(bits >> 34);
        (
            TtNode {
                depth: draft as i32,
                nt: node_type,
                bm,
                score,
            },
            (bits & 255) as u8,
        )
    }
}

impl fmt::Display for TtNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{:>6} {:>10} {:>3} {:>2}",
                self.bm.uci(),
                self.score.to_string(),
                self.depth,
                self.nt
            )
        } else {
            write!(f, "{} scoring {} draft {} type {}", self.bm.uci(), self.score, self.depth, self.nt)
        }
    }
}

// #[derive(Default, Debug)]
// struct Element {
//     hash: Hash,
//     entry: TtNode,
//     age: AtomicI16,
// }

// impl Clone for Element {
//     #[inline]
//     fn clone(&self) -> Self {
//         Element {
//             entry: self.entry,
//             hash: self.hash,
//             age: AtomicI16::new(self.age.load(Ordering::Relaxed)),
//         }
//     }
// }

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Replacement {
    Always,
    Age,
    AgeTypeDepth,
    AgeDepthType,
    AgeBlend,
}

// FIXME Mates as score
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TranspositionTable2 {
    #[serde(skip)]
    table: Arc<SharedTable>,

    pub enabled: bool,
    aging: bool,
    pub use_tt_for_pv: bool,
    pub use_tt_for_eval: bool,
    pub allow_truncated_pv: bool,
    pub mb: i64,
    hmvc_horizon: i32,
    min_ply: Ply,
    min_depth: Ply,
    buckets: usize,
    aligned: bool,
    rewrite_pv: bool,
    freshen_on_fetch: bool,
    replacement: Replacement,
    preserve_bm: bool,

    #[rustfmt::skip] #[serde(skip)] pub current_age: u8,
    #[rustfmt::skip] #[serde(skip)] pub hits: Stat,
    #[rustfmt::skip] #[serde(skip)] pub misses: Stat,
    #[rustfmt::skip] #[serde(skip)] pub collisions: Stat,
    #[rustfmt::skip] #[serde(skip)] pub bad_hash: Stat,
    #[rustfmt::skip] #[serde(skip)] pub exclusions: Stat,
    #[rustfmt::skip] #[serde(skip)] pub inserts: Stat,
    #[rustfmt::skip] #[serde(skip)] pub updates: Stat,
    #[rustfmt::skip] #[serde(skip)] pub pv_overwrites: Stat,
    // #[rustfmt::skip] #[serde(skip)] pub deletes: Stat,
    // #[rustfmt::skip] #[serde(skip)] pub fail_priority: Stat,
    // #[rustfmt::skip] #[serde(skip)] pub fail_ownership: Stat,
}

impl Default for TranspositionTable2 {
    fn default() -> Self {
        Self {
            table: Arc::new(SharedTable::default()),
            enabled: true,
            use_tt_for_pv: false,
            allow_truncated_pv: false,
            use_tt_for_eval: true,
            mb: 8,
            aging: true,
            buckets: 2,
            aligned: false,
            current_age: 10, // to allow us to look back
            hmvc_horizon: 85,
            min_ply: 1, // search restrictions on ply=0
            min_depth: 1,
            rewrite_pv: true,
            freshen_on_fetch: true,
            replacement: Replacement::AgeTypeDepth,
            preserve_bm: false,

            hits: Stat::new("hits"),
            misses: Stat::new("misses"),
            collisions: Stat::new("collisions"),
            exclusions: Stat::new("exclusions"),
            bad_hash: Stat::new("bad_hash"),
            inserts: Stat::new("inserts"),
            updates: Stat::new("updates"),
            pv_overwrites: Stat::new("pv overwrites"),
            // deletes: Stat::new("deletes"),
            // fail_priority: Stat::new("ins fail priority"),
            // fail_ownership: Stat::new("ins fail owner"),
        }
    }
}

impl fmt::Debug for TranspositionTable2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TranspositionTable")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("enabled", &self.enabled)
            .field("use.tt.for.pv", &self.use_tt_for_pv)
            .field("allow.truncated.pv", &self.allow_truncated_pv)
            .field("mb", &self.mb)
            .field("buckets", &self.buckets)
            .field("aligned", &self.aligned)
            .field("hmvc.horizon", &self.hmvc_horizon)
            .field("aging", &self.aging)
            .field("current.age", &self.current_age)
            .finish()
    }
}

impl fmt::Display for TranspositionTable2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())?;
        writeln!(f, "table capacity   : {}", self.table.capacity())?;
        // writeln!(f, "entry: pv        : {}", self.count_of(NodeType::ExactPv))?;
        // writeln!(f, "entry: cut       : {}", self.count_of(NodeType::LowerCut))?;
        // writeln!(f, "entry: all       : {}", self.count_of(NodeType::UpperAll))?;
        // writeln!(f, "entry: unused    : {}", self.count_of(NodeType::Unused))?;
        let tot = self.hits.get() + self.misses.get() + self.collisions.get() + self.exclusions.get();
        let tot = cmp::max(1, tot);
        writeln!(f, "% hits           : {}", 100 * self.hits.get() / tot)?;
        writeln!(f, "% misses         : {}", 100 * self.misses.get() / tot)?;
        writeln!(f, "% collisions     : {}", 100 * self.collisions.get() / tot)?;
        writeln!(f, "% exclusions     : {}", 100 * self.exclusions.get() / tot)?;
        // for i in 0..10 {
        //     writeln!(
        //         f,
        //         "ages (cur-{})     : {}",
        //         i,
        //         self.count_of_age(self.current_age - i)
        //     )?;
        // }
        writeln!(
            f,
            "tt stats\n{}",
            ArrayStat(&[
                &self.hits,
                &self.misses,
                &self.collisions,
                &self.exclusions,
                &self.bad_hash,
                &self.inserts,
                &self.updates,
                &self.pv_overwrites,
                // &self.fail_priority,
                // &self.fail_ownership,
                // &self.deletes,
            ])
        )?;
        Ok(())
    }
}

impl Component for TranspositionTable2 {
    fn new_game(&mut self) {
        self.resize_if_required();
        self.current_age = 10;
        self.table.clear()
    }

    fn new_position(&mut self) {
        self.resize_if_required();
        self.next_generation();
    }
}

impl TranspositionTable2 {
    fn resize_if_required(&mut self) {
        if self.requires_resize() {
            let capacity = SharedTable::convert_mb_to_capacity(self.mb);
            info!("tt resized so capacity is now {} with {} buckets", capacity, self.buckets);
            let mut table = SharedTable::default();
            table.resize(capacity, self.buckets, self.aligned);
            self.table = Arc::new(table);
            self.current_age = 10;
        }
    }

    pub fn rewrite_pv(&self, b: &Board) {
        if self.rewrite_pv {
            let _pv = self.extract_pv_and_score(b);
        }
    }

    pub fn fmt_nodes(&self, f: &mut fmt::Formatter, b: &Board) -> fmt::Result {
        let nodes = self.extract_nodes(b);
        for n in nodes {
            writeln!(f, "{:#}", n)?
        }
        Ok(())
    }

    pub fn next_generation(&mut self) {
        // if self.requires_resize() {
        //     info!("Resizing tt");
        //     self.new_game();
        // } else {
        if self.aging {
            self.current_age = self.current_age.wrapping_add(1);
            debug!("aging tt to age {}", self.current_age);
        }
        // }
    }

    pub fn requires_resize(&self) -> bool {
        let capacity = SharedTable::convert_mb_to_capacity(self.mb);
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
    fn age_of(b: &Bucket) -> u8 {
        (b.data() & 255) as u8
    }

    pub fn hashfull_per_mille(&self) -> u32 {
        let count = self.table.iter().take(200).filter(|&b| Self::age_of(b) == self.current_age).count();
        count as u32 * 1000 / 200
    }

    #[inline]
    pub fn store(&mut self, h: Hash, mut new_node: TtNode) {
        // FIXME maybe store QS results
        if !self.enabled && new_node.nt != NodeType::ExactPv || self.capacity() == 0 || new_node.depth < 0 {
            return;
        }
        debug_assert!(new_node.nt != NodeType::Unused, "Cannot store unused nodes in tt");
        debug_assert!(
            new_node.score.is_numeric_or_mate(),
            "Cannot store score {} in tt\n{}",
            new_node.score,
            new_node
        );

        // probe by hash not board so any "conditions" are bypassed
        let mut bucket_to_overwrite = None;
        let buckets = self.table.buckets(h);
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
            if Bucket::has_hash(h, (key, data)) {
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
                if Bucket::is_empty(key, data) {
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
                        && (new_node.nt > old_node.nt || new_node.nt == old_node.nt && new_node.depth >= old_node.depth)
            }
            (Replacement::AgeDepthType, _) => {
                self.current_age > old_age
                    || self.current_age == old_age
                &&
                // even when the draft is the same we overwrite, as more nodes may have been used in calculating due to a fuller tt..
                (new_node.depth >= old_node.depth
                || new_node.depth == old_node.depth && new_node.nt > old_node.nt)
            }
            (Replacement::AgeBlend, _) => {
                self.current_age > old_age
                    || self.current_age == old_age
                &&
                // overwrite with deeper as long as not overwriting an exact with a non-exact
                new_node.depth >= old_node.depth
                &&
                (new_node.nt == NodeType::ExactPv || old_node.nt != NodeType::ExactPv)
            }
        };
        if replace {
            // new.hash != old.hash &&
            if self.current_age == old_age && old_node.nt == NodeType::ExactPv {
                self.pv_overwrites.increment();
            }
            debug_assert!(new_node.score > -Score::INFINITY && new_node.score < Score::INFINITY);
            debug_assert!(
                new_node.nt != NodeType::ExactPv || !new_node.bm.is_null(),
                "bm is null at {:?} mv {:?}",
                new_node.nt,
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
        let tt_node = self.probe_by_hash(board.hash());
        if let Some(tt_node) = tt_node {
            if !tt_node.bm.is_null() && !board.is_pseudo_legal_move(&tt_node.bm) && !board.is_legal_move(&tt_node.bm) {
                self.bad_hash.increment();
                return None;
            }
            debug_assert!(
                tt_node.score.is_numeric_or_mate(),
                "tt_node {}\nboard {:#}\nply: {}\ndepth: {}",
                tt_node,
                board,
                ply,
                depth
            );
            assert!(
                tt_node.bm.is_null() || (board.is_pseudo_legal_move(&tt_node.bm) && board.is_legal_move(&tt_node.bm)),
                "{} {} {:?}",
                board.to_fen(),
                tt_node.bm.uci(),
                tt_node.bm
            );
        }
        tt_node
    }

    fn probe_by_hash(&self, h: Hash) -> Option<TtNode> {
        // debug!("Probe by hash");
        // if !self.enabled || self.capacity() == 0 {
        //     return None;
        // }
        if let Some((data, bucket)) = self.table.probe(h) {
            self.hits.increment();
            let new_data = (data & !255) | (self.current_age as u64 & 255);
            if self.freshen_on_fetch {
                bucket.write(h, new_data);
            }
            Some(TtNode::unpack(data).0)
        } else {
            self.misses.increment();
            self.collisions.increment();
            None
        }
    }

    pub fn extract_pv_and_score(&self, b: &Board) -> (Variation, Option<Score>) {
        let mut var = Variation::new();
        let nodes = self.extract_nodes(b);
        nodes.iter().for_each(|n| var.push(n.bm));
        let score = nodes.first().map(|n| n.score);
        (var, score)
    }

    // non recursive
    fn extract_nodes(&self, b: &Board) -> Vec<TtNode> {
        let mut board = b.clone();
        let mut nodes = Vec::new();
        // board = board.make_move(&first);
        // moves.push(*first);
        let mut mv;
        while nodes.len() < 50 {
            // probe by hash to avoid all the board filters (ply etc)
            let entry = self.probe_by_hash(board.hash());
            if let Some(entry) = entry {
                if entry.nt == NodeType::ExactPv {
                    mv = &entry.bm;
                    if !mv.is_null() && board.is_pseudo_legal_move(mv) && board.is_legal_move(mv) {
                        board = board.make_move(mv);
                        nodes.push(entry);
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
                            entry.nt,
                            board.to_fen()
                        );
                        println!(
                            "Invalid move {} for nt {} in tt for board position {}",
                            mv,
                            entry.nt,
                            board.to_fen()
                        );
                        return nodes;
                    }
                }
                if nodes.is_empty() {
                    println!("root node is {:?}", entry.nt);
                }
            }
            // println!("Unable to find hash {} after move {}", board.hash(), mv) ;
            break;
        }
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::comms::uci::*;
    use crate::globals::constants::*;
    use crate::search::algo::*;
    use crate::search::engine::Engine;
    use crate::search::timecontrol::*;
    use crate::types::*;
    use test_log::test;

    fn entry123() -> TtNode {
        TtNode {
            score: Score::from_cp(300),
            depth: 2,
            nt: NodeType::ExactPv,
            bm: Move::new_quiet(Piece::Pawn, b7.square(), b6.square()),
        }
    }

    fn entry456() -> TtNode {
        TtNode {
            score: Score::from_cp(200),
            depth: 3,
            nt: NodeType::ExactPv,
            bm: Move::new_quiet(Piece::Pawn, a2.square(), a3.square()),
        }
    }

    fn entry456b() -> TtNode {
        TtNode {
            score: Score::from_cp(201),
            depth: 4,
            nt: NodeType::ExactPv,
            bm: Move::new(
                a1.square(),
                a2.square(),
                Square::null(),
                Piece::Rook,
                Piece::None,
                Piece::None,
                CastlingRights::NONE,
            ),
        }
    }

    #[test]
    fn test_tt2() {
        let mut tt1 = TranspositionTable2::default();
        tt1.new_game();
        info!("diplay\n{}", tt1);
        info!("After new game");
        let board = Catalog::starting_board();
        let moves = tt1.extract_pv_and_score(&board).0;
        info!("After extract");
        assert_eq!(moves.uci(), "");
        assert_eq!(tt1.hashfull_per_mille(), 0);
        manipulate(&mut tt1);
        assert_eq!(tt1.table.utilization(), 2);
        tt1.new_game();
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
        tt1.new_game();
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

        assert!(tt.capacity() > 200000);
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
        assert_eq!(tt.hashfull_per_mille(), 0);
    }

    #[test]
    #[ignore]
    fn tt2_end_games() -> Result<(), String> {
        // //let mut tt1 = TranspositionTable::with_capacity(TranspositionTable::convert_mb_to_capacity(10));
        let mut algo = Algo::new();
        algo.tt.mb = 128;
        // use simple evaluation as we look at great depth
        algo.set_callback(Uci::uci_info);
        algo.set_timing_method(TimeControl::Depth(33));
        algo.new_game();
        for pos in Catalog::famous().iter() {
            algo.new_game();
            algo.set_position(pos.clone()).search();
            assert_eq!(algo.results.bm().uci(), pos.bm()?.uci(), "{}\n{}", pos, algo);
        }
        Ok(())
    }

    #[test]
    fn tt2_test_pv_extraction() -> Result<(), String> {
        info!("log");
        let mut algo = Algo::new();
        let d = 2;
        for pos in Catalog::bratko_kopec() {
            eprintln!("{}", pos);
            algo.new_game();
            eprintln!("new game");
            algo.set_position(pos.clone());
            algo.set_timing_method(TimeControl::Depth(d));
            algo.search();
            eprintln!("search done");
            //            let pv = algo.tt.extract_pv(&algo.bm(), pos.board());
            let pv = algo.tt.extract_pv_and_score(pos.board()).0;

            // No reason acd = pv length as pv line may be reduced due to lmr etc.
            assert!(pv.len() >= (d as usize) - 1, "algo.pv={} pv={}\n{}", algo.pv(), pv, algo);
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
            println!(">>>>>> {}", pv);
        }
        Ok(())
    }

    #[test]
    fn tt2_test_new_game() {
        let mut eng = Engine::new();
        eng.new_game();
        eng.set_position(Catalog::starting_position().clone());
        eng.algo.set_timing_method(TimeControl::Depth(6));
        eprintln!("Before 1\n{}", eng.algo);
        eng.search();
        eprintln!("After 1\n{}", eng.algo);
        let mut eng = Engine::new();
        eng.new_game();
        eng.set_position(Catalog::starting_position().clone());
        eng.algo.set_timing_method(TimeControl::Depth(6));
        eprintln!("Before 2\n{}", eng.algo);
        eng.search();
        eprintln!("After 2{}", eng.algo);
    }
}
