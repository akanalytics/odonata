use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::score::Score;
use crate::log_debug;
use crate::movelist::{Move, MoveList};
use crate::stat::{ArrayStat, Stat};
use crate::types::{Hash, Ply};
use std::cmp;
use std::fmt;
use std::mem;
use std::sync::atomic::{AtomicI16, Ordering};
use std::sync::Arc;

// pub static HITS: Stat = Stat::new("HITS");
// pub static MISSES: Stat = Stat::new("MISSES");
// pub static COLLISIONS: Stat = Stat::new("COLLISIONS");
// pub static INSERTS: Stat = Stat::new("INSERTS");
// pub static DELETES: Stat = Stat::new("DELETES");
// pub static FAIL_PRIORITY: Stat = Stat::new("INS FAIL PRIORITY");
// pub static FAIL_OWNERSHIP: Stat = Stat::new("INS FAIL OWNER");

// pub static TT_COUNTS: ArrayStat = ArrayStat(&[
//     &HITS,
//     &MISSES,
//     &COLLISIONS,
//     &INSERTS,
//     &FAIL_PRIORITY,
//     &FAIL_OWNERSHIP,
//     &DELETES,
// ]);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum NodeType {
    Unused = 0,
    All = 1, // All node, score = upperbound ()
    Cut = 2, // Cut node, score = lowerbound (we've not looked at all possible scores)
    Pv = 3,  // PV node. score is exact
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Unused
    }
}

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Entry {
    pub score: Score,
    pub draft: Ply, // draft is plies to q/leaf
    pub node_type: NodeType,
    pub bm: Move,
}

#[derive(Default, Debug)]
struct StoredEntry {
    hash: Hash,
    entry: Entry,
    age: AtomicI16,
}

impl Clone for StoredEntry {
    fn clone(&self) -> Self {
        Self {
            entry: self.entry,
            hash: self.hash,
            age: AtomicI16::new(self.age.load(Ordering::Relaxed)),
        }
    }
}

// FIXME Mates as score
#[derive(Clone)]
pub struct TranspositionTable {
    table: Arc<Vec<StoredEntry>>,

    pub aging: bool,
    pub current_age: i16,
    pub enabled: bool,
    pub use_tt_for_pv: bool,
    pub mb: i64,
    pub hmvc_horizon: i32,
    pub hits: Stat,
    pub misses: Stat,
    pub collisions: Stat,
    pub exclusions: Stat,
    pub inserts: Stat,
    pub pv_overwrites: Stat,
    pub deletes: Stat,
    pub fail_priority: Stat,
    pub fail_ownership: Stat,
}

impl fmt::Debug for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TranspositionTable")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("enabled", &self.enabled)
            .field("use_tt_for_pv", &self.use_tt_for_pv)
            .field("mb", &self.mb)
            .field("hmvc_horizon", &self.hmvc_horizon)
            .field("aging", &self.aging)
            .field("current_age", &self.current_age)
            .field("table", &self.table.len()) // dont show large table!
            .finish()
    }
}

impl fmt::Display for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "use tt for pv    : {}", self.use_tt_for_pv)?;
        writeln!(f, "capacity         : {}", self.table.capacity())?;
        writeln!(f, "size in mb       : {}", self.mb)?;
        writeln!(f, "entry size bytes : {}", mem::size_of::<Entry>())?;
        writeln!(f, "aging            : {}", self.aging)?;
        writeln!(f, "current age      : {}", self.current_age)?;
        writeln!(f, "hmvc horizon     : {}", self.hmvc_horizon)?;
        writeln!(f, "table            : {}", self.table.len())?;
        writeln!(f, "entry: pv        : {}", self.count_of(NodeType::Pv))?;
        writeln!(f, "entry: cut       : {}", self.count_of(NodeType::Cut))?;
        writeln!(f, "entry: all       : {}", self.count_of(NodeType::All))?;
        writeln!(f, "entry: unused    : {}", self.count_of(NodeType::Unused))?;
        let tot = self.hits.get() + self.misses.get() + self.collisions.get() + self.exclusions.get();
        let tot = cmp::max(1, tot);
        writeln!(f, "% hits           : {}", 100 * self.hits.get() / tot)?;
        writeln!(f, "% misses         : {}", 100 * self.misses.get() / tot)?;
        writeln!(f, "% collisions     : {}", 100 * self.collisions.get() / tot)?;
        writeln!(f, "% exclusions     : {}", 100 * self.exclusions.get() / tot)?;
        for i in 0..10 {
            writeln!(
                f,
                "ages (cur-{})     : {}",
                i,
                self.count_of_age(self.current_age - i)
            )?;
        }
        writeln!(
            f,
            "tt stats\n{}",
            ArrayStat(&[
                &self.hits,
                &self.misses,
                &self.collisions,
                &self.exclusions,
                &self.inserts,
                &self.pv_overwrites,
                &self.fail_priority,
                &self.fail_ownership,
                &self.deletes,
            ])
        )?;
        Ok(())
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new_with_mb(8)
    }
}

impl Configurable for TranspositionTable {
    fn settings(&self, c: &mut Config) {
        c.set("tt.aging", "type check default true");
        c.set("tt.use_tt_for_pv", "type check default true");
        c.set("Hash", "type spin default 33 min 0 max 4000");
        c.set("tt.hmvc_horizon", "type spin default 35 min 0 max 100");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("tt.configure with {}", c);
        self.aging = c.bool("tt.aging").unwrap_or(self.aging);
        self.use_tt_for_pv = c.bool("tt.use_tt_for_pv").unwrap_or(self.use_tt_for_pv);
        self.mb = c.int("Hash").unwrap_or(self.mb);
        // table is resized on next clear / generation
        self.hmvc_horizon = c.int("tt.hmvc_horizon").unwrap_or(self.hmvc_horizon as i64) as i32;
    }
}

impl TranspositionTable {
    pub const fn convert_mb_to_capacity(mb: i64) -> usize {
        mb as usize * 1_000_000 / mem::size_of::<Entry>()
    }

    pub const fn convert_capacity_to_mb(cap: usize) -> i64 {
        (cap * mem::size_of::<Entry>()) as i64 / 1_000_000
    }

    pub fn new_with_mb(mb: usize) -> Self {
        Self {
            table: Arc::new(vec![StoredEntry::default(); 0]),
            enabled: true,
            use_tt_for_pv: true,
            mb: mb as i64,
            aging: true,
            current_age: 10, // to allow us to look back
            hmvc_horizon: 42,
            hits: Stat::new("hits"),
            misses: Stat::new("misses"),
            collisions: Stat::new("collisions"),
            exclusions: Stat::new("exclusions"),
            inserts: Stat::new("inserts"),
            pv_overwrites: Stat::new("pv overwrites"),
            deletes: Stat::new("deletes"),
            fail_priority: Stat::new("ins fail priority"),
            fail_ownership: Stat::new("ins fail owner"),
        }
    }

    pub fn destroy(&mut self) {
        self.table = Arc::new(vec![StoredEntry::default(); 0]);
        // Arc::make_mut(&mut self.table);
    }

    pub fn next_generation(&mut self) {
        if self.requires_resize() {
            self.clear_and_resize();
        } else {
            if self.aging {
                self.current_age += 1;
            }
        }
    }

    pub fn requires_resize(&self) -> bool {
        let capacity = Self::convert_mb_to_capacity(self.mb);
        self.table.capacity() != capacity
    }

    pub fn clear_and_resize(&mut self) {
        if self.requires_resize() {
            let capacity = Self::convert_mb_to_capacity(self.mb);
            self.table = Arc::new(vec![StoredEntry::default(); capacity]);
            self.current_age = 10;
            return;
        }
        let table = Arc::get_mut(&mut self.table);
        if let Some(table) = table {
            table.iter_mut().for_each(|e| *e = StoredEntry::default());
            self.current_age = 10;
        } else {
            panic!("Unable to clear cache");
        }
    }

    pub fn count_of(&self, t: NodeType) -> usize {
        self.table.iter().filter(|e| e.entry.node_type == t).count()
    }

    pub fn count_of_age(&self, age: i16) -> usize {
        self.table
            .iter()
            .filter(|e| e.age.load(Ordering::Relaxed) == age)
            .count()
    }

    #[inline]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    #[inline]
    pub fn index(&self, hash: Hash) -> usize {
        hash as usize % self.capacity()
    }

    pub fn store(&mut self, hash: Hash, entry: Entry) {
        if !self.enabled || self.capacity() == 0 {
            return;
        }
        let new = StoredEntry {
            entry,
            hash,
            age: AtomicI16::new(self.current_age),
        };
        let index = self.index(new.hash);
        let table = Arc::get_mut(&mut self.table);
        if let Some(table) = table {
            let old = &mut table[index];
            let old_age = old.age.load(Ordering::Relaxed);
            // if self.current_age > old_age
            // || self.current_age == old_age
            //     && (new.entry.draft > old.entry.draft
            //         || new.entry.draft == old.entry.draft && new.entry.node_type >= old.entry.node_type)
            if self.current_age > old_age
                || self.current_age == old_age
                    && (new.entry.node_type > old.entry.node_type
                        || new.entry.node_type == old.entry.node_type && new.entry.draft >= old.entry.draft)
            {
                if new.hash != old.hash && self.current_age == old_age && old.entry.node_type == NodeType::Pv
                {
                    self.pv_overwrites.increment();
                }
                debug_assert!(new.entry.score > Score::MinusInf);
                debug_assert!(
                    new.entry.node_type != NodeType::Pv || !new.entry.bm.is_null(),
                    "bm is null at {:?} mv {:?}",
                    new.entry.node_type,
                    new.entry.bm
                );
                self.inserts.increment();
                *old = new;
                return;
            } else {
                self.fail_priority.increment();
            }
        } else {
            self.fail_ownership.increment();
            panic!("fail ownership tt store");
        }
    }

    pub fn delete(&mut self, key: Hash) {
        if !self.enabled || self.capacity() == 0 {
            return;
        }
        let index = self.index(key);
        let table = Arc::get_mut(&mut self.table);
        if let Some(table) = table {
            let old = &mut table[index];
            self.deletes.increment();
            *old = StoredEntry::default();
            return;
        } else {
            self.fail_ownership.increment();
            panic!();
        }
    }

    pub fn probe_by_board(&self, board: &Board) -> Option<&Entry> {
        if !self.enabled || self.capacity() == 0 {
            return None;
        }
        if board.fifty_halfmove_clock() > self.hmvc_horizon {
            self.exclusions.increment();
            None
        } else {
            let entry = self.probe_by_hash(board.hash());
            debug_assert!(
                entry.is_none() || entry.unwrap().bm.is_null() || board.is_legal_move(&entry.unwrap().bm)
            );
            entry
        }
    }

    fn probe_by_hash(&self, hash: Hash) -> Option<&Entry> {
        if !self.enabled || self.capacity() == 0 {
            return None;
        }
        let stored = &self.table[self.index(hash)];
        if stored.entry.node_type != NodeType::Unused {
            if stored.hash == hash {
                self.hits.increment();
                stored.age.store(self.current_age, Ordering::Relaxed); // freshen the age on access
                return Some(&stored.entry);
            } else {
                self.collisions.increment();
                return None;
            }
        }
        self.misses.increment();
        None
    }

    // non recursive
    pub fn extract_pv(&self, b: &Board) -> MoveList {
        let mut board = b.clone();
        let mut moves = MoveList::new();
        // board = board.make_move(&first);
        // moves.push(*first);
        let mut mv;
        while moves.len() < 50 {
            let entry = self.probe_by_board(&board);
            if let Some(entry) = entry {
                if entry.node_type == NodeType::Pv {
                    mv = &entry.bm;
                    if !mv.is_null() && board.is_valid_move(&mv) && board.is_legal_move(&mv) {
                        board = board.make_move(&mv);
                        moves.push(*mv);
                        continue;
                    } else {
                        debug_assert!(
                            false,
                            "Invalid move {} in tt for board position {} moves {} from starting {}",
                            mv,
                            board.to_fen(),
                            moves,
                            b.to_fen()
                        );
                        error!("Invalid move {} in tt for board position {}", mv, board.to_fen());
                        return moves;
                    }
                }
            }
            // println!("Unable to find hash {} after move {}", board.hash(), mv) ;
            break;
        }
        moves
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::*;
    use crate::globals::constants::*;
    use crate::search::algo::*;
    use crate::search::timecontrol::*;
    use crate::types::*;
    use crate::comms::uci::*;

    fn entry123() -> Entry {
        Entry {
            score: Score::Cp(300),
            draft: 2,
            node_type: NodeType::Pv,
            bm: Move::new_quiet(Piece::Pawn, b7.square(), b6.square()),
        }
    }

    fn entry456() -> Entry {
        Entry {
            score: Score::Cp(200),
            draft: 3,
            node_type: NodeType::Pv,
            bm: Move::new_quiet(Piece::Pawn, a2.square(), a3.square()),
        }
    }

    fn entry456b() -> Entry {
        Entry {
            score: Score::Cp(201),
            draft: 4,
            node_type: NodeType::Pv,
            bm: Move {
                to: a1.square(),
                from: a2.square(),
                ..Move::default()
            },
        }
    }

    #[test]
    fn test_tt() {
        let mut tt1 = TranspositionTable::new_with_mb(10);
        tt1.clear_and_resize();
        let board = Catalog::starting_position();
        // let first = Move::new_quiet(Piece::Pawn, e2.square(), e4.square());
        let moves = tt1.extract_pv(&board);
        assert_eq!(moves.uci(), "e2e4");
        manipulate(&mut tt1);

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
        manipulate(&mut tt1);

        let mut tt3 = tt1.clone();
        tt1.destroy();
        println!(
            "Clone tt1 -> tt3 and destroy tt1 ...{}",
            Arc::strong_count(&tt3.table)
        );
        manipulate(&mut tt3);
    }

    fn manipulate(tt: &mut TranspositionTable) {
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
        assert_eq!(tt.probe_by_hash(123), Some(&entry123));
        assert_eq!(tt.probe_by_hash(124), None);
        assert_eq!(tt.probe_by_hash(456), Some(&entry456));
        tt.store(456, entry456b);
        assert_eq!(tt.probe_by_hash(456), Some(&entry456b));

        // store fails due to ply, leaving 456b in place
        tt.store(456, entry456);
        assert_eq!(tt.probe_by_hash(456), Some(&entry456b));

        // store succeeds due to age
        tt.next_generation();
        tt.store(456, entry456);
        assert_eq!(tt.probe_by_hash(456), Some(&entry456));

        println!("{:?}", tt);
        println!("{}", tt);
        tt.clear_and_resize();
        assert!(tt.probe_by_hash(123).is_none());
    }

    #[test]
    #[ignore]
    fn tt_end_games() -> Result<(), String> {
        // //let mut tt1 = TranspositionTable::with_capacity(TranspositionTable::convert_mb_to_capacity(10));
        let mut algo = Algo::new();
        algo.tt.mb = 512;
        // use simple evaluation as we look at great depth
        algo.eval.position = false;
        algo.eval.mobility = false;
        algo.set_callback(Uci::uci_info);
        algo.set_timing_method(TimeControl::Depth(33));
        algo.new_game();
        for pos in Catalog::end_games().iter() {
            algo.new_game();
            algo.search(pos.board());
            assert_eq!(algo.bm().uci(), pos.bm()?.uci(), "{}\n{}", pos, algo);
        }
        Ok(())
    }

    #[test]
    fn tt_test_pv_extraction() -> Result<(), String> {
        let mut algo = Algo::new();
        let d = 2;
        algo.set_timing_method(TimeControl::Depth(d));
        for pos in Catalog::bratko_kopec().iter() {
            algo.new_game();
            algo.search(pos.board());
//            let pv = algo.tt.extract_pv(&algo.bm(), pos.board());
            let pv = algo.tt.extract_pv(pos.board());
            assert_eq!(algo.pv().len(), d as usize, "{} {}\n{}", algo.pv(), pv, algo);
            assert_eq!(pv.len(), d as usize, "{} {}\n{}", algo.pv(), pv, algo);
            // assert_eq!(algo.bm().uci(), pos.bm()?.uci());
            println!(">>>>>> {}", pv);
        }
        Ok(())
    }
}
