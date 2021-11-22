use crate::trace::stat::Stat;
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
    pub fn has_hash(h: Hash, (k, d): (Hash, u64)) -> bool {
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
pub struct SharedTable {
    vec: Vec<Bucket>,

    capacity: usize,
    mask: usize,
    buckets: usize,
    pub utilization: Stat,
}

//
// Design taken from
// https://binarydebt.wordpress.com/2013/09/29/lockless-transposition-tables/
// with adjustments for bit-sizing of items and multi-buckets
//
impl SharedTable {
    pub fn resize(&mut self, capacity: usize, buckets: usize, aligned: bool) {
        self.capacity = capacity.next_power_of_two();
        self.buckets = buckets;
        self.mask = capacity - 1;
        self.vec = if aligned {
            aligned_vec(capacity + buckets)
        } else {
            Vec::with_capacity(capacity + buckets)
        };
        debug!(
            "New transposition table with capacity {} mask {:x} len {:x}",
            self.capacity,
            self.mask,
            self.vec.len()
        );
    }

    pub const BUCKET_SIZE: usize = mem::size_of::<Bucket>();

    pub const fn convert_mb_to_capacity(mb: i64) -> usize {
        (mb as usize * 1_000_000 / Self::BUCKET_SIZE).next_power_of_two()
    }

    pub const fn convert_capacity_to_mb(cap: usize) -> usize {
        (cap * Self::BUCKET_SIZE) as usize / 1_000_000
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }


    #[inline]
    pub fn index(&self, h: Hash) -> usize {
        h as usize & self.mask
    }

    #[inline]
    pub fn probe(&self, h: Hash) -> Option<(u64, &Bucket)> {
        for bucket in &self.vec[self.index(h)..self.index(h) + self.buckets] {
            let key = bucket.key();
            let data = bucket.data();
            if Bucket::is_empty(key, data) {
                continue;
            }
            let hash = key ^ data;
            if hash == h {
                return Some((data, bucket));
            }
        }
        None
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Bucket> {
        self.vec.iter()
    }

    #[inline]
    pub fn buckets(&self, h: Hash) -> &[Bucket] {
        &self.vec[self.index(h)..self.index(h) + self.buckets]
    }

    pub fn clear(&self) {
        self.vec.iter().for_each(|b| b.set_empty());
        self.utilization.clear();
    }
}

// https://stackoverflow.com/questions/60180121/how-do-i-allocate-a-vecu8-that-is-aligned-to-the-size-of-the-cache-line
#[repr(align(64))]
pub struct AlignToCacheLine([Bucket; 4]);

fn aligned_vec(capacity: usize) -> Vec<Bucket> {
    // Lazy math to ensure we always have enough.
    let n_units = capacity / 4 + 4;
    let mut aligned: Vec<AlignToCacheLine> = Vec::with_capacity(n_units);
    let ptr = aligned.as_mut_ptr();
    mem::forget(aligned);
    unsafe { Vec::from_raw_parts(ptr as *mut Bucket, capacity, capacity) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn tt_size() {
        assert_eq!(size_of::<AlignToCacheLine>(), 64, "AlignToCacheLine");
    }
}
