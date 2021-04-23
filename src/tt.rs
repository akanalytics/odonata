use crate::eval::score::Score;
use crate::movelist::Move;
use crate::types::{Hash, Ply};
use crate::stat::Stat;
use std::mem;


#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd )]
pub enum EntryType {
    Unused = 0,
    Alpha = 1, 
    Beta = 2,
    Exact = 3,   
}

impl Default for EntryType {
    fn default() -> Self { Self::Unused }
}


#[derive(Copy, Clone, Default, Debug, Eq, PartialEq )]
pub struct Entry {
    hash: Hash,
    score: Score,
    ply: Ply,
    entry_type: EntryType, 
    best_move: Move,    
}


impl Default for TranspositionTable {
    fn default() -> Self {
        Self {
            table: Vec::new(),
            hits: Stat::new("TT.HITS"),
            misses: Stat::new("TT.MISSES"),
            collisions: Stat::new("TT.COLLISIONS"),
            inserts: Stat::new("TT.INSERTS"),
        }
    }
}

#[derive(Clone)]
pub struct TranspositionTable {
    table: Vec<Entry>,

    hits: Stat<'static>,
    misses: Stat<'static>,
    collisions: Stat<'static>,
    inserts: Stat<'static>,
}

impl TranspositionTable {

    pub fn new_in_mb(mb: usize) -> Self {
        let entries = 1_000_000 * mb / mem::size_of::<Entry>();
        Self::new(entries)
    }

    pub fn new(size: usize) -> Self {
        TranspositionTable {
            table: vec![Entry::default(); size],
            .. Self::default()
        }
        // tt.table.resize(size, Entry::default());
    }

    pub fn size(&self) -> usize {
        self.table.capacity() as usize
    }

    pub fn index(&self, hash: Hash) -> usize {
        hash as usize % self.size()
    }

    pub fn insert(&mut self, new: Entry) {
        self.inserts.increment();
        let index = self.index(new.hash);
        let old = &mut self.table[index];
        if new.ply > old.ply || new.ply == old.ply && new.entry_type > old.entry_type {
            self.inserts.increment();
            *old = new;
            return;
        }
    }

    pub fn get(&self, hash: Hash) -> Option<&Entry> {
        let entry = &self.table[self.index(hash)];
        if entry.entry_type != EntryType::Unused {
            if entry.hash == hash {
                self.hits.increment();
                return Some(entry);
            } else {
                self.collisions.increment();
                return None;
            }
        }
        self.misses.increment();
        return None;
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
            ply: 2,
            entry_type: EntryType::Exact, 
            best_move: Move::new_null(),    
        };

        let entry456 = Entry {
            hash: 456,
            score: Score::Cp(200),
            ply: 3,
            entry_type: EntryType::Exact, 
            best_move: Move::new_null(),    
        };

        let entry456b = Entry {
            hash: 456,
            score: Score::Cp(201),
            ply: 4,
            entry_type: EntryType::Exact, 
            best_move: Move::new_null(),    
        };

        let mut tt = TranspositionTable::new_in_mb(10);
        assert_eq!( tt.size(), 178_571);
        assert!(tt.get(123).is_none());
        tt.insert(entry123);
        tt.insert(entry456);
        assert_eq!(tt.get(123), Some(&entry123));
        assert_eq!(tt.get(124), None);
        assert_eq!(tt.get(456), Some(&entry456));
        tt.insert(entry456b);
        assert_eq!(tt.get(456), Some(&entry456b));

        // insert fails due to ply, leaving 456b in place
        tt.insert(entry456);
        assert_eq!(tt.get(456), Some(&entry456b));
    }
}
