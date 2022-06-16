use crate::piece::Hash;
use std::cell::Cell;
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};

// #[derive(Clone, Debug)]
// pub struct ArrayCache<T: Copy, const N: usize> {
//     data: [Cell<Option<T>>; N],
//     hash: [Cell<Hash>; N],
// }

// impl<T: Copy, const N: usize> Default for ArrayCache<T, N> {
//     fn default() -> Self {
//         // const INIT: Cell<Option<T>> = Cell::new(None);
//         Self {
//             data: [Self::INIT_DATA; N],
//             hash: [Self::INIT_HASH; N],
//         }
//     }
// }

// impl<T: Copy, const N: usize> ArrayCache<T, N> {
//     // work around for array initilization > 32
//     const INIT_DATA: Cell<Option<T>> = Cell::new(None);
//     const INIT_HASH: Cell<Hash> = Cell::new(0);

//     #[inline]
//     pub fn probe(&self, key: usize, hash: Hash) -> Option<T> {
//         if self.hash[key].get() == hash {
//             self.data[key].get()
//         } else {
//             None
//         }
//     }

//     #[inline]
//     pub fn store(&self, key: usize, hash: Hash, t: T) {
//         self.hash[key].set(hash);
//         self.data[key].set(Some(t));
//     }

//     pub fn hashfull_per_mille(&self) -> u32 {
//         let count = self
//             .data
//             .iter()
//             .take(200)
//             .filter(|&c| c.get().is_some())
//             .count();
//         count as u32 * 1000 / std::cmp::min(self.data.len() as u32, 200)
//     }

//     pub fn clear(&self) {
//         self.data.iter().for_each(|c| c.set(None));
//         self.hash.iter().for_each(|c| c.set(Hash::default()));
//     }
// }

#[derive(Clone, Debug)]
pub struct VecCache<T: Copy> {
    array: Vec<(Cell<Hash>, Cell<Option<T>>)>,
}

impl<T: Copy> Default for VecCache<T> {
    fn default() -> Self {
        // const INIT: Cell<Option<T>> = Cell::new(None);
        Self { array: vec![] }
    }
}

impl<T: Copy> VecCache<T> {
    // work around for array initilization > 32
    const INIT: (Cell<Hash>, Cell<Option<T>>) = (Cell::new(0), Cell::new(None));

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            array: vec![Self::INIT; capacity],
        }
    }

    pub fn capacity(&self) -> usize {
        self.array.len()
    }

    pub fn with_size_bytes(bytes: usize) -> Self {
        let capacity = bytes / std::mem::size_of_val(&Self::INIT);
        Self {
            array: vec![Self::INIT; capacity],
        }
    }

    #[inline]
    pub fn probe(&self, key: usize, hash: Hash) -> Option<T> {
        if self.array[key].0.get() == hash {
            self.array[key].1.get()
        } else {
            None
        }
    }

    #[inline]
    pub fn store(&self, key: usize, hash: Hash, t: T) {
        self.array[key].0.set(hash);
        self.array[key].1.set(Some(t));
    }

    pub fn hashfull_per_mille(&self) -> u32 {
        let count = self
            .array
            .iter()
            .take(200)
            .filter(|&c| c.1.get().is_some())
            .count();
        count as u32 * 1000 / std::cmp::min(self.array.len() as u32, 200)
    }

    pub fn clear(&self) {
        self.array.iter().for_each(|c| {
            c.1.set(None);
            c.0.set(Hash::default())
        });
    }
}

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
        if aligned {
            self.vec = aligned_vec(capacity + buckets)
        } else {
            self.vec = Vec::with_capacity(capacity + buckets);
            self.vec.resize_with(capacity + buckets, Bucket::default);
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

    pub fn utilization(&self) -> usize {
        self.vec
            .iter()
            .filter(|&b| {
                let kd = (b.key(), b.data());
                !Bucket::is_empty(kd.0 ^ kd.1, kd.1)
            })
            .count()
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
    }
}

// https://stackoverflow.com/questions/60180121/how-do-i-allocate-a-vecu8-that-is-aligned-to-the-size-of-the-cache-line
// #[repr(align(64))]
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
    use crate::eval::score::WhiteScore;

    use super::*;
    use std::mem::size_of;

    #[test]
    fn tt_size() {
        assert_eq!(size_of::<AlignToCacheLine>(), 64, "AlignToCacheLine");
        assert_eq!(VecCache::<WhiteScore>::with_size_bytes(300_000).capacity(),18750);
    }
}
