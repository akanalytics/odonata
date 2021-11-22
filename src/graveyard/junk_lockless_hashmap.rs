use crate::stat::Stat;
use crate::types::Hash;
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Bucket {
    key: AtomicU64,
    data: AtomicU64,
}

impl Bucket {
    #[inline]
    pub fn key(&self) -> Hash {
        self.key.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn data(&self) -> u64 {
        self.data.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn is_empty(h: Hash, data: u64) -> bool {
        h == 0 && data == 0
    }

    #[inline]
    pub fn has_hash(h: Hash, (k,d): (Hash, u64)) -> bool {
        k ^ d == h
    }

    #[inline]
    pub fn write(&self, h: Hash, data: u64) {
        let key = h ^ data;
        // trace!("store {:x} {:x} in position {}", xor_hash, data, self.index(h));
        self.key.store(key, Ordering::Relaxed);
        self.data.store(data, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_empty(&self) {
        self.key.store(0, Ordering::Relaxed);
        self.data.store(0, Ordering::Relaxed);
    }
}

#[derive(Default)]
#[repr(align(64))]
pub struct Entry {
    buckets: [Bucket; 4],
}

#[derive(Default)]
pub struct SharedTable {
    vec: Vec<Entry>,

    capacity: usize,
    mask: usize,
    // buckets: usize,
    pub utilization: Stat,

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
// Design copied straight from
// https://binarydebt.wordpress.com/2013/09/29/lockless-transposition-tables/
// with adjustments for bit-sizing of items
//
impl SharedTable {
    pub fn new_with_capacity(capacity: usize) -> SharedTable {
        let capacity = capacity.next_power_of_two();
        let mut st = SharedTable {
            capacity,
            mask: (capacity - 1),
            vec: Vec::new(),
            ..SharedTable::default()
        };
        st.vec.resize_with(capacity, || Entry::default());
        debug!(
            "New transposition table with capacity {} mask {:x} len {:x}",
            st.capacity,
            st.mask,
            st.vec.len()
        );
        st
    }

    pub const ENTRY_SIZE: usize = mem::size_of::<Entry>();

    pub const fn convert_mb_to_capacity(mb: i64) -> usize {
        (mb as usize * 1_000_000 / Self::ENTRY_SIZE).next_power_of_two()
    }

    pub const fn convert_capacity_to_mb(cap: usize) -> usize {
        (cap * Self::ENTRY_SIZE) as usize / 1_000_000
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn utilization(&self) -> usize {
        self.utilization.get() as usize
    }

    #[inline]
    pub fn index(&self, h: Hash) -> usize {
        h as usize & self.mask
    }

    // #[inline]
    // pub fn probe_by_index(&self, i: usize) -> (Hash, u64) {
    //     for bucket in &self.vec[i].buckets {
    //         let xor_hash = bucket.hash.load(Ordering::Relaxed);
    //         let data = bucket.data.load(Ordering::Relaxed);
    //         let hash = xor_hash ^ data;
    //         (hash, data)
    //     }
    // trace!("load {:x} {:x} from position {}", xor_hash, data, i);

    #[inline]
    pub fn probe(&self, h: Hash) -> Option<(u64,&Bucket)> {
        for bucket in &self.vec[self.index(h)].buckets {
            let key = bucket.key();
            let data = bucket.data();
            if Bucket::is_empty(key, data) {
                continue;
            }
            let hash = key ^ data;
            if hash == h {
                return Some((data,bucket));
            }
        }
        None
    }

    #[inline]
    pub fn buckets(&self, h: Hash) -> &[Bucket] {
        &self.vec[self.index(h)].buckets
    }

    // #[inline]
    // pub fn store(&self, h: Hash, data: u64) {
    //     let xor_hash = h ^ data;
    //     trace!("store {:x} {:x} in position {}", xor_hash, data, self.index(h));
    //     self.vec[self.index(h)].store(xor_hash, Ordering::Relaxed);
    //     self.vec[self.index(h) + 1].store(data, Ordering::Relaxed);
    // }

    // pub fn delete(&self, h: Hash) {
    //     self.vec[self.index(h)].store(0, Ordering::Relaxed);
    //     self.vec[self.index(h) + 1].store(0, Ordering::Relaxed);
    // }

    pub fn clear(&self) {
        for h in 0..self.mask as u64 {
            self.vec[self.index(h)].buckets.iter().for_each(|b| b.set_empty())
        }
        self.utilization.clear();
    }
}
