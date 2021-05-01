use crate::board::Board;
use crate::config::{Config, Configurable};
use crate::eval::score::Score;
use crate::log_debug;
use crate::movelist::Move;
use crate::stat::{ArrayStat, Stat};
use crate::types::{Hash, Ply};
use std::fmt;
use std::mem;
use std::sync::Arc;

pub static HITS: Stat = Stat::new("HITS");
pub static MISSES: Stat = Stat::new("MISSES");
pub static COLLISIONS: Stat = Stat::new("COLLISIONS");
pub static INSERTS: Stat = Stat::new("INSERTS");
pub static DELETES: Stat = Stat::new("DELETES");
pub static FAIL_PRIORITY: Stat = Stat::new("INS FAIL PRIORITY");
pub static FAIL_OWNERSHIP: Stat = Stat::new("INS FAIL OWNER");

pub static TT_COUNTS: ArrayStat = ArrayStat(&[
    &HITS,
    &MISSES,
    &COLLISIONS,
    &INSERTS,
    &FAIL_PRIORITY,
    &FAIL_OWNERSHIP,
    &DELETES,
]);

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
    pub hash: Hash,
    pub score: Score,
    pub depth: Ply, // depth is depth to q/leaf
    pub node_type: NodeType,
    pub bm: Move,
}

// FIXME Mates as score
#[derive(Clone)]
pub struct TranspositionTable {
    table: Arc<Vec<Entry>>,

    pub enabled: bool,
    pub capacity: usize,
    pub hmvc_horizon: i32,
}

impl fmt::Debug for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TranspositionTable")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("enabled", &self.enabled)
            .field("capacity", &self.capacity)
            .field("hmvc_horizon", &self.hmvc_horizon)
            .field("table", &self.table.len()) // dont show large table!
            .finish()
    }
}

impl fmt::Display for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.capacity() > 0)?;
        writeln!(f, "capacity         : {}", self.capacity())?;
        writeln!(f, "size in mb       : {}", Self::convert_capacity_to_mb(self.capacity()))?;
        writeln!(f, "entry size bytes : {}", mem::size_of::<Entry>())?;
        writeln!(f, "hmvc horizon     : {}", self.hmvc_horizon)?;
        writeln!(f, "table            : {}", self.table.len())?;
        writeln!(f, "entry: pv        : {}", self.count_of(NodeType::Pv))?;
        writeln!(f, "entry: cut       : {}", self.count_of(NodeType::Cut))?;
        writeln!(f, "entry: all       : {}", self.count_of(NodeType::All))?;
        writeln!(f, "entry: unused    : {}", self.count_of(NodeType::Unused))?;
        writeln!(f, "tt stats\n{}", TT_COUNTS)?;
        Ok(())
    }
}



impl Default for TranspositionTable {
    fn default() -> Self {
        Self {
            table: Arc::new(vec![Entry::default(); Self::convert_mb_to_capacity(33)]),
            enabled: true,
            capacity: Self::convert_mb_to_capacity(33),
            hmvc_horizon: 35,
        }
    }
}

impl Configurable for TranspositionTable {
    fn settings(&self, c: &mut Config) {
        c.set("Hash", "type spin default 33 min 0 max 4000");
        c.set("tt.hmvc_horizon", "type spin default 35 min 0 max 100");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("tt.configure with {}", c);
        let capacity = Self::convert_mb_to_capacity(c.int("Hash").unwrap_or(self.capacity as i64));
        if capacity != self.capacity() {
            self.capacity = capacity;
            self.table = Arc::new(vec![Entry::default(); capacity]);
        }
        self.hmvc_horizon = c.int("tt.hmvc_horizon").unwrap_or(self.hmvc_horizon as i64) as i32;
    }
}



impl TranspositionTable {
    pub const fn convert_mb_to_capacity( mb: i64) -> usize {
        mb as usize * 1_000_000  / mem::size_of::<Entry>() 
    }

    pub const fn convert_capacity_to_mb( cap: usize) -> i64 {
        (cap * mem::size_of::<Entry>()) as i64 / 1_000_000 
    }

    pub fn with_capacity(capacity: usize) -> Self {
        TranspositionTable {
            table: Arc::new(vec![Entry::default(); capacity]),
            capacity,
            ..Self::default()
        }
        // tt.table.resize(size, Entry::default());
    }

    pub fn destroy(&mut self) {
        Arc::make_mut(&mut self.table);
    }

    pub fn clear(&mut self) {
        self.table = Arc::new(vec![Entry::default(); self.capacity()]);
        // tt.table.resize(size, Entry::default());
    }

    pub fn count_of(&self,  t: NodeType ) -> usize {
        self.table.iter().filter( |e| e.node_type == t).count()
    }


    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn index(&self, hash: Hash) -> usize {
        hash as usize % self.capacity()
    }

    pub fn insert(&mut self, new: Entry) {
        if !self.enabled {
            return;
        }
        let index = self.index(new.hash);
        let table = Arc::get_mut(&mut self.table);
        if let Some(table) = table {
            let old = &mut table[index];
            if new.depth > old.depth || new.depth == old.depth && new.node_type > old.node_type {
                assert!(new.score > Score::MinusInf);
                INSERTS.increment();
                *old = new;
                return;
            } else {
                FAIL_PRIORITY.increment();
            }
        } else {
            FAIL_OWNERSHIP.increment();
        }
    }

    pub fn delete(&mut self, key: Hash) {
        if !self.enabled {
            return;
        }
        let index = self.index(key);
        let table = Arc::get_mut(&mut self.table);
        if let Some(table) = table {
            let old = &mut table[index];
            DELETES.increment();
            *old = Entry::default();
            return;
        } else {
            FAIL_OWNERSHIP.increment();
        }
    }

    pub fn probe_by_board(&self, board: &Board) -> Option<&Entry> {
        if !self.enabled {
            return None;
        }
        if board.fifty_halfmove_clock() > self.hmvc_horizon {
            None
        } else {
            self.probe_by_hash(board.hash())
        }
    }
    pub fn probe_by_hash(&self, hash: Hash) -> Option<&Entry> {
        if !self.enabled {
            return None;
        }
        let entry = &self.table[self.index(hash)];
        if entry.node_type != NodeType::Unused {
            if entry.hash == hash {
                HITS.increment();
                return Some(entry);
            } else {
                COLLISIONS.increment();
                return None;
            }
        }
        MISSES.increment();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry123() -> Entry {
        Entry {
            hash: 123,
            score: Score::Cp(300),
            depth: 2,
            node_type: NodeType::Pv,
            bm: Move::new_null(),
        }
    }

    fn entry456() -> Entry {
        Entry {
            hash: 456,
            score: Score::Cp(200),
            depth: 3,
            node_type: NodeType::Pv,
            bm: Move::new_null(),
        }
    }

    fn entry456b() -> Entry {
        Entry {
            hash: 456,
            score: Score::Cp(201),
            depth: 4,
            node_type: NodeType::Pv,
            bm: Move::new_null(),
        }
    }

    #[test]
    fn test_tt() {
        let mut tt1 = TranspositionTable::with_capacity(TranspositionTable::convert_mb_to_capacity(10));
        manipulate(&mut tt1);

        {
            let mut tt2 = tt1.clone();
            println!("Cloned tt1 -> tt2 ...{}", Arc::strong_count(&tt1.table));
            tt2.insert(entry123());
            tt2.insert(entry456());
            println!("{:?}", tt2);
            println!("{}", tt2);
        }
        println!("Dropped tt2 ...{}", Arc::strong_count(&tt1.table));
        manipulate(&mut tt1);

        let mut tt3 = tt1.clone();
        tt1.destroy();
        println!("Clone tt1 -> tt3 and destroy tt1 ...{}", Arc::strong_count(&tt3.table));
        manipulate(&mut tt3);
    }

    fn manipulate(tt: &mut TranspositionTable) {
        let entry123 = entry123();
        let entry456 = entry456();
        let entry456b = entry456b();

        assert_eq!(tt.capacity(), 178_571);
        tt.delete(entry123.hash);
        tt.delete(entry456.hash);
        tt.delete(entry456b.hash);
        assert!(tt.probe_by_hash(123).is_none());
        tt.insert(entry123);
        tt.insert(entry456);
        assert_eq!(tt.probe_by_hash(123), Some(&entry123));
        assert_eq!(tt.probe_by_hash(124), None);
        assert_eq!(tt.probe_by_hash(456), Some(&entry456));
        tt.insert(entry456b);
        assert_eq!(tt.probe_by_hash(456), Some(&entry456b));

        // insert fails due to ply, leaving 456b in place
        tt.insert(entry456);
        assert_eq!(tt.probe_by_hash(456), Some(&entry456b));
        println!("{:?}", tt);
        println!("{}", tt);
        tt.clear();
        assert!(tt.probe_by_hash(123).is_none());
    }
}
