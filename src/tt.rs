use crate::config::{Config, Configurable};
use crate::eval::score::Score;
use crate::log_debug;
use crate::movelist::Move;
use crate::board::Board;
use crate::stat::{Stat, ArrayStat};
use crate::types::{Hash, Ply};
use std::mem;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum NodeType {
    Unused = 0,
    All = 1,   // All node, score = upperbound () 
    Cut = 2,   // Cut node, score = lowerbound (we've not looked at all possible scores)
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
    pub depth: Ply,  // depth is depth to q/leaf
    pub node_type: NodeType,
    pub bm: Move,
}


// FIXME Mates as score
#[derive(Clone)]
pub struct TranspositionTable {
    table: Vec<Entry>,

    pub enabled: bool,
    pub capacity: usize,
    pub hmvc_horizon: i32,


    hits: Stat<'static>,
    misses: Stat<'static>,
    collisions: Stat<'static>,
    inserts: Stat<'static>,
}


impl fmt::Debug for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TranspositionTable")
            // .field("pv_table", &self.pv_table.extract_pv().)
            .field("enabled", &self.enabled)
            .field("capacity", &self.capacity)
            .field("hmvc_horizon", &self.hmvc_horizon)
            .field("hits", &self.hits)
            .field("misses", &self.misses)
            .field("collisions", &self.collisions)
            .field("inserts", &self.inserts)
            .field("table", &self.table.len())  // dont show large table!
            .finish()
    }
}

impl fmt::Display for TranspositionTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "enabled          : {}", self.enabled)?;
        writeln!(f, "capacity         : {}", self.capacity())?;
        writeln!(f, "size in mb       : {}", self.size_in_mb())?;
        writeln!(f, "entry size bytes : {}", mem::size_of::<Entry>())?;
        writeln!(f, "hmvc horizon     : {}", self.hmvc_horizon)?;
        writeln!(f, "table            : {}", self.table.len())?;
        writeln!(f, "tt stats\n{}", ArrayStat(&[&self.hits, &self.misses, &self.collisions, &self.inserts]))?;
        Ok(())
    }
}


impl Default for TranspositionTable {
    fn default() -> Self {
        Self {
            table: vec![Entry::default(); 600_000],
            enabled: true,
            capacity: 600_000,
            hmvc_horizon: 35,
            hits: Stat::new("TT.HITS"),
            misses: Stat::new("TT.MISSES"),
            collisions: Stat::new("TT.COLLISIONS"),
            inserts: Stat::new("TT.INSERTS"),
        }
    }
}

impl Configurable for TranspositionTable {
    fn settings(&self, c: &mut Config) {
        c.set("tt.enabled", "type check default true");
        c.set("tt.capacity", "type spin default 200000 min 0 max 1000000");
        c.set("tt.hmvc_horizon", "type spin default 35 min 0 max 100");
    }
    fn configure(&mut self, c: &Config) {
        log_debug!("tt.configure with {}", c);
        self.enabled = c.bool("tt.enabled").unwrap_or(self.enabled);
        self.capacity = c.int("tt.capacity").unwrap_or(self.capacity as i64) as usize;
        self.hmvc_horizon = c.int("tt.hmvc_horizon").unwrap_or(self.hmvc_horizon as i64) as i32;
    }
}

impl TranspositionTable {
    pub fn new_in_mb(mb: usize) -> Self {
        let entries = 1_000_000 * mb / mem::size_of::<Entry>();
        Self::new(entries)
    }

    pub fn new(capacity: usize) -> Self {
        TranspositionTable { table: vec![Entry::default(); capacity], capacity, ..Self::default() }
        // tt.table.resize(size, Entry::default());
    }

    pub fn clear(&mut self) {
        self.table = vec![Entry::default(); self.capacity()];
        // tt.table.resize(size, Entry::default());
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn size_in_mb(&self) -> usize {
        self.capacity * mem::size_of::<Entry>() / 1_000_000
    }

    pub fn index(&self, hash: Hash) -> usize {
        hash as usize % self.capacity()
    }

    pub fn insert(&mut self, new: Entry) {
        if !self.enabled {
            return;
        }
        self.inserts.increment();
        let index = self.index(new.hash);
        let old = &mut self.table[index];
        if new.depth > old.depth || new.depth == old.depth && new.node_type > old.node_type {
            self.inserts.increment();
            *old = new;
            return;
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
            return None
        }
        let entry = &self.table[self.index(hash)];
        if  entry.node_type != NodeType::Unused {
            if entry.hash == hash {
                self.hits.increment();
                return Some(entry);
            } else {
                self.collisions.increment();
                return None;
            }
        }
        self.misses.increment();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tt() {
        let entry123 = Entry {
            hash: 123,
            score: Score::Cp(300),
            depth: 2,
            node_type: NodeType::Pv,
            bm: Move::new_null(),
        };

        let entry456 = Entry {
            hash: 456,
            score: Score::Cp(200),
            depth: 3,
            node_type: NodeType::Pv,
            bm: Move::new_null(),
        };

        let entry456b = Entry {
            hash: 456,
            score: Score::Cp(201),
            depth: 4,
            node_type: NodeType::Pv,
            bm: Move::new_null(),
        };

        let mut tt = TranspositionTable::new_in_mb(10);
        assert_eq!(tt.capacity(), 178_571);
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
