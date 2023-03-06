use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::infra::utils::DecimalFormatter;
use crate::piece::Hash;
use std::cell::Cell;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fmt, mem};


#[derive(Default, Clone, PartialEq, Eq, Serialize)]
pub(super) struct AlignedVec<T> {
    #[serde(skip)]
    vec: Vec<T>,

    min_capacity: usize,
    capacity: usize,
    mask: usize,
    bucket_size: usize,
    overlapping_buckets: bool,
    size_of: usize,
    align_of: usize,
    max_index_offset: usize,
    index_offset: usize,
    aligned: bool,
    cacheline_size: usize,
    n_resizes: i32,
}

impl<T: Debug> Debug for AlignedVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())
    }
}

impl<T: Default> fmt::Display for AlignedVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "type:           {}", std::any::type_name::<T>())?;
        writeln!(f, "min_capacity:   {}", self.min_capacity)?;
        writeln!(f, "capacity:       {}", self.capacity)?;
        writeln!(f, "mask:           {:<64b}", self.mask)?;
        writeln!(f, "bucket_size:    {}", self.bucket_size)?;
        writeln!(f, "bucket_count:   {}", self.bucket_count())?;
        writeln!(f, "overlapping:    {}", self.overlapping_buckets)?;
        writeln!(f, "max_index_off:  {}", self.max_index_offset)?;
        writeln!(f, "index_offset:   {}", self.index_offset)?;
        writeln!(f, "aligned:        {}", self.aligned)?;
        writeln!(f, "cacheline_size: {}", self.cacheline_size)?;
        writeln!(f, "n_resizes:      {}", self.n_resizes)?;
        writeln!(
            f,
            "bkt size bytes: {}",
            mem::size_of::<T>() * self.bucket_size
        )?;
        writeln!(
            f,
            "&vec[0]:        {:>64b}",
            (&self.vec[0]) as *const _ as usize
        )?;
        (0..10.clamp(0, self.bucket_count())).for_each(|i| {
            writeln!(
                f,
                "&bucket[{i}]:     {:>64b}",
                &(self.bucket(i as u64)[0]) as *const _ as usize
            )
            .unwrap()
        });
        writeln!(f, "cacheline:      {:>64b}", self.cacheline_size)?;
        Ok(())
    }
}

impl<T: Default> AlignedVec<T> {
    pub fn new(
        min_capacity: usize,
        bucket_size: usize,
        aligned: bool,
        overlapping_buckets: bool,
        cacheline_size: usize,
    ) -> Self {
        let mut me = Self::default();
        me.resize(
            min_capacity,
            bucket_size,
            aligned,
            overlapping_buckets,
            cacheline_size,
        );
        me
    }

    #[inline]
    pub fn index(&self, hash: u64) -> usize {
        (hash as usize & self.mask) + self.index_offset
    }

    // how large to make the vector/array to hold capcity + padding
    fn len(&self) -> usize {
        self.capacity + self.max_index_offset + self.bucket_size
    }

    // capacity < len, as some of length is used for alignment
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub const fn convert_mb_to_capacity(mb: i64) -> usize {
        (mb as usize * 1_000_000 / mem::size_of::<T>()).next_power_of_two()
    }

    // pub const fn convert_capacity_to_mb(cap: usize) -> usize {
    //     (cap * Self::BUCKET_SIZE) as usize / 1_000_000
    // }

    fn size_in_mb(&self) -> usize {
        self.len() * self.size_of / 1_000_000
    }

    #[inline]
    pub fn bucket(&self, h: Hash) -> &[T] {
        &self.vec[self.index(h)..(self.index(h) + self.bucket_size)]
    }

    pub fn bucket_count(&self) -> usize {
        self.capacity() / self.bucket_size
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter().skip(self.index_offset).take(self.capacity)
    }

    pub fn resize(
        &mut self,
        min_capacity: usize,
        bucket_size: usize,
        aligned: bool,
        overlapping_buckets: bool,
        cacheline_size: usize,
    ) {
        self.n_resizes += 1;
        self.min_capacity = min_capacity;
        self.aligned = aligned & !overlapping_buckets;
        self.overlapping_buckets = overlapping_buckets;
        self.bucket_size = bucket_size;
        self.cacheline_size = cacheline_size;

        self.size_of = mem::size_of::<T>();
        self.capacity = min_capacity.next_power_of_two();
        self.mask = if overlapping_buckets {
            // we dont bother aligning either. an index into the middle of the bucket will be returned
            self.capacity - 1
        } else {
            // index will be to start of bucket
            (self.capacity - 1) ^ (bucket_size - 1)
        };
        self.max_index_offset = bucket_size * 16;
        self.vec.resize_with(self.len(), || T::default());

        let ptr: *const T = &self.vec[0];
        self.index_offset = if self.aligned {
            ptr.align_offset(self.cacheline_size)
        } else {
            0
        };
        // warn!("resize vector {self:?}");
        if self.index_offset > self.max_index_offset {
            info!(
                "***Index offset {offset} and aligned {a}",
                offset = self.index_offset,
                a = self.aligned
            );
            // self.aligned = false;
            panic!("Could not align {}", self.index_offset);
        }
        debug_assert!(self.index_offset <= self.max_index_offset);
        debug_assert!(self.capacity >= self.min_capacity);
        debug!("New aligned vector {self}");
    }
}

impl<T: Default + PartialEq + Debug> AlignedVec<T> {
    pub fn utilization(&self) -> usize {
        self.iter().filter(|&t| *t != T::default()).count()
    }

    pub fn hashfull_per_mille(&self) -> u32 {
        let count = self
            .iter()
            .take(200)
            .filter(|&t| *t != T::default())
            .count();
        (count * 1000 / self.capacity().min(200).max(1)) as u32
    }
}

#[cfg(test)]
mod tests1 {
    use super::*;
    use test_log::test;

    #[test]
    #[ignore]
    fn test_aligned_vector() {
        type Item1 = (u64, u64, u64);
        let av = AlignedVec::<Item1>::new(10, 2, true, false, 64);
        println!("{av}");

        type I2 = u8;
        let av = AlignedVec::<I2>::new(5, 7, true, false, 64);
        println!("{av}");

        type I3 = (u64, u64);
        let av = AlignedVec::<I3>::new(40, 4, true, false, 64);
        println!("{av}");

        let av = AlignedVec::<I3>::new(40, 4, true, false, 128);
        println!("{av}");

        let av = AlignedVec::<I3>::new(40, 8, true, false, 64);
        println!("{av}");

        let av = AlignedVec::<I3>::new(40, 8, true, false, 128);
        println!("{av}");

        type I4 = (u64, u64);
        let av = AlignedVec::<I4>::new(30, 3, false, false, 64);
        println!("{av}");

        type I5 = [u8; 17];
        let av = AlignedVec::<I5>::new(30, 3, true, false, 64);
        println!("{av}");
    }
}

#[derive(Clone, Debug, Default)]
pub struct UnsharedTable<T: Copy> {
    array: Vec<(Cell<Hash>, Cell<T>)>,

    pub hits: Cell<u64>,
    pub misses: Cell<u64>,
    pub collisions: Cell<u64>,
}

impl<T: Copy + Default> fmt::Display for UnsharedTable<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "hitrate:{}% hits:{} misses:{} collisions:{} ",
            self.cache_hits_percent().dp(2),
            self.hits.get(),
            self.misses.get(),
            self.collisions.get()
        )
    }
}

// impl<T: Copy> Default for UnsharedTable<T> {
//     fn default() -> Self {
//         // const INIT: Cell<Option<T>> = Cell::new(None);
//         Self {
//             array: vec![],
//             hits: Cell::new(0),
//             misses: Cell::new(0),
//             collisions: Cell::new(0),
//         }
//     }
// }

impl<T: Copy + Default> UnsharedTable<T> {
    // work around for array initilization > 32
    // work around for array initilization > 32
    // const INIT: (Cell<Hash>, Cell<T>) = ;

    pub fn with_size(capacity: usize) -> Self {
        Self {
            // array: vec![Self::INIT; capacity],
            array: (0..capacity)
                .map(|_| (Cell::new(0), Cell::new(T::default())))
                .collect_vec(),
            ..Self::default()
        }
    }

    pub fn capacity(&self) -> usize {
        self.array.len()
    }

    pub fn with_size_bytes(bytes: usize) -> Self {
        let capacity = bytes / std::mem::size_of::<(Cell<Hash>, Cell<T>)>();
        Self::with_size(capacity)
    }

    #[inline]
    pub fn probe(&self, hash: Hash) -> Option<T> {
        let key = hash as usize % self.capacity();

        if self.array[key].0.get() == hash && hash != 0 {
            self.hits.set(self.hits.get() + 1);
            Some(self.array[key].1.get())
        } else {
            self.misses.set(self.misses.get() + 1);
            None
        }
    }

    #[inline]
    pub fn store(&self, hash: Hash, t: T) {
        let key = hash as usize % self.capacity();
        self.array[key].0.set(hash);
        self.array[key].1.set(t);
    }

    pub fn cache_hits_percent(&self) -> f32 {
        let hits = self.hits.get() as f32;
        let misses = self.misses.get() as f32;
        100. * hits / (hits + misses).max(1.)
    }

    pub fn hashfull_per_mille(&self) -> u32 {
        let count = self
            .array
            .iter()
            .take(200)
            .filter(|&c| c.0.get() != 0)
            .count();
        count as u32 * 1000 / std::cmp::min(self.array.len() as u32, 200)
    }

    pub fn clear(&self) {
        self.array.iter().for_each(|c| {
            c.0.set(Hash::default());
            // c.1.set(T::default());
        });
    }
}

#[derive(Default, Debug)]
pub struct HashEntry {
    key: AtomicU64,
    data: AtomicU64,
}

impl PartialEq for HashEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key() && self.data() == other.data()
    }
}

impl HashEntry {
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

#[derive(Default, Serialize, Deserialize)]
pub struct SharedTable {
    #[serde(skip)]
    vec: Vec<HashEntry>,

    capacity: usize,
    mask: usize,
    bucket_size: usize,
}

impl Debug for SharedTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", toml::to_string_pretty(self).unwrap())
    }
}

//
// Design taken from
// https://binarydebt.wordpress.com/2013/09/29/lockless-transposition-tables/
// with adjustments for bit-sizing of items and multi-buckets
//
impl SharedTable {
    pub fn resize(
        &mut self,
        capacity: usize,
        buckets: usize,
        aligned: bool,
        _overlapping: bool,
        _cacheline: usize,
    ) {
        self.capacity = capacity.next_power_of_two();
        self.bucket_size = buckets;
        self.mask = capacity - 1;
        if aligned {
            self.vec = aligned_vec(capacity + buckets)
        } else {
            self.vec = Vec::with_capacity(capacity + buckets);
            self.vec.resize_with(capacity + buckets, HashEntry::default);
        };
        debug!(
            "New transposition table with capacity {} mask {:x} len {:x}",
            self.capacity,
            self.mask,
            self.vec.len()
        );
    }

    pub const HASH_ENTRY_SIZE: usize = mem::size_of::<HashEntry>();

    pub const fn convert_mb_to_capacity(mb: i64) -> usize {
        (mb as usize * 1_000_000 / Self::HASH_ENTRY_SIZE).next_power_of_two()
    }

    // pub const fn convert_capacity_to_mb(cap: usize) -> usize {
    //     (cap * Self::BUCKET_SIZE) as usize / 1_000_000
    // }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn utilization(&self) -> usize {
        self.vec
            .iter()
            .filter(|&b| {
                let kd = (b.key(), b.data());
                !HashEntry::is_empty(kd.0 ^ kd.1, kd.1)
            })
            .count()
    }

    pub fn hashfull_per_mille(&self) -> u32 {
        let count = self
            .iter()
            .take(200)
            .filter(|&t| *t != HashEntry::default())
            .count();
        (count * 1000 / self.capacity().min(200).max(1)) as u32
    }

    #[inline]
    pub fn index(&self, h: Hash) -> usize {
        h as usize & self.mask
    }

    #[inline]
    pub fn probe(&self, h: Hash) -> Option<(u64, &HashEntry)> {
        for bucket in &self.vec[self.index(h)..self.index(h) + self.bucket_size] {
            let key = bucket.key();
            let data = bucket.data();
            if HashEntry::is_empty(key, data) {
                continue;
            }
            let hash = key ^ data;
            if hash == h {
                return Some((data, bucket));
            }
        }
        None
    }

    pub fn iter(&self) -> std::slice::Iter<'_, HashEntry> {
        self.vec.iter()
    }

    #[inline]
    pub fn bucket(&self, h: Hash) -> &[HashEntry] {
        &self.vec[self.index(h)..(self.index(h) + self.bucket_size)]
    }

    pub fn clear(&self) {
        self.vec.iter().for_each(|b| b.set_empty());
    }
}

// https://stackoverflow.com/questions/60180121/how-do-i-allocate-a-vecu8-that-is-aligned-to-the-size-of-the-cache-line
// #[repr(align(64))]
pub struct AlignToCacheLine([HashEntry; 4]);

fn aligned_vec(capacity: usize) -> Vec<HashEntry> {
    // Lazy math to ensure we always have enough.
    let n_units = capacity / 4 + 4;
    let mut aligned: Vec<AlignToCacheLine> = Vec::with_capacity(n_units);
    let ptr = aligned.as_mut_ptr();
    mem::forget(aligned);
    unsafe { Vec::from_raw_parts(ptr as *mut HashEntry, capacity, capacity) }
}

#[cfg(test)]
mod tests {
    use crate::{eval::score::WhiteScore, infra::profiler::PerfProfiler};

    use super::*;
    use std::mem::size_of;

    #[test]
    fn tt_size() {
        assert_eq!(size_of::<AlignToCacheLine>(), 64, "AlignToCacheLine");
        assert_eq!(
            UnsharedTable::<WhiteScore>::with_size_bytes(300_000).capacity(),
            18750
        );
    }

    #[test]
    fn bench_unshared_table() {
        let mut prof_p = PerfProfiler::new("unshared_table_probe".into());
        let mut prof_s = PerfProfiler::new("unshared_table_store".into());

        let cache = UnsharedTable::with_size_bytes(100_000);

        #[derive(Copy, Default, Clone)]
        struct Blob {
            i: usize,
        }

        for iter in 0..1000 {
            let blob = Blob { i: 2 * iter };
            prof_s.benchmark(|| {
                cache.store(iter as Hash, blob);
            })
        }

        let mut total = 0;
        for iter in 0..1000 {
            prof_p.benchmark(|| {
                let blob = cache.probe(iter as Hash);
                if let Some(blob) = blob {
                    total += blob.i;
                }
            })
        }
        assert_eq!(total, 999_000);
    }
}
