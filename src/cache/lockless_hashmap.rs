use crate::stat::{Stat};
use crate::types::{Hash};
use crate::{debug, trace, logger::LogInit};
use std::sync::atomic::{AtomicU64, Ordering};


#[derive(Default)]
pub struct SharedTable {
    vec: Vec<AtomicU64>,

    capacity: usize,
    mask: usize,
    // buckets: usize,
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

// 
// Design copied straight from https://binarydebt.wordpress.com/2013/09/29/lockless-transposition-tables/
// with adjustments for bit-sizing of items
//
impl SharedTable {
    pub fn new_with_capacity(capacity: usize) -> SharedTable {
        let capacity = capacity.next_power_of_two();
        let mut st = SharedTable {
            capacity,
            mask: (capacity - 1) << 1,
            vec: Vec::new(),
            ..SharedTable::default()
        };
        st.vec.resize_with(2 * capacity, || AtomicU64::new(0));
        debug!(
            "New transposition table with capacity {} mask {:x} len {:x}",
            st.capacity,
            st.mask,
            st.vec.len()
        );
        st
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn index(&self, h: Hash) -> usize {
        h as usize & self.mask
    }

    pub fn probe_by_index(&self, i: usize) -> (Hash, u64) {
        let xor_hash = self.vec[i].load(Ordering::Relaxed);
        let data = self.vec[i + 1].load(Ordering::Relaxed);
        // trace!("load {:x} {:x} from position {}", xor_hash, data, i);
        let hash = xor_hash ^ data;
        (hash, data)
    }

    pub fn probe(&self, h: Hash) -> (Hash, u64) {
        let xor_hash = self.vec[self.index(h)].load(Ordering::Relaxed);
        let data = self.vec[self.index(h) + 1].load(Ordering::Relaxed);
        // trace!("load {:x} {:x} from position {}", xor_hash, data, self.index(h));
        let hash = xor_hash ^ data;
        (hash, data)
    }

    pub fn store(&self, h: Hash, data: u64) {
        let xor_hash = h ^ data;
        trace!("store {:x} {:x} in position {}", xor_hash, data, self.index(h));
        self.vec[self.index(h)].store(xor_hash, Ordering::Relaxed);
        self.vec[self.index(h) + 1].store(data, Ordering::Relaxed);
    }

    pub fn delete(&self, h: Hash) {
        self.vec[self.index(h)].store(0, Ordering::Relaxed);
        self.vec[self.index(h) + 1].store(0, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        self.vec.iter().for_each(|e| e.store(0, Ordering::Relaxed))
    }
}
