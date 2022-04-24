use crate::utils::Formatting;
use perf_event::{events::Hardware, Builder, Counter, Group};

pub use iai::black_box;

pub struct Profiler {
    group: Group,
    name: String,
    iters: u64,
    ins: Counter,
    branch_misses: Counter,
    cache_misses: Counter,
    cache_refs: Counter,
    cycles: Counter,
}

impl Profiler {
    #[inline]
    pub fn new(name: String) -> Profiler {
        let mut group = Group::new().unwrap();
        let cycles = Builder::new().group(&mut group).kind(Hardware::CPU_CYCLES).build().unwrap();
        let ins = Builder::new().group(&mut group).kind(Hardware::INSTRUCTIONS).build().unwrap();
        let branch_misses = Builder::new().group(&mut group).kind(Hardware::BRANCH_MISSES).build().unwrap();
        let cache_misses = Builder::new().group(&mut group).kind(Hardware::CACHE_MISSES).build().unwrap();
        let cache_refs = Builder::new().group(&mut group).kind(Hardware::CACHE_REFERENCES).build().unwrap();
        Profiler {
            name,
            group,
            ins,
            cycles,
            branch_misses,
            cache_misses,
            cache_refs,
            iters: 0,
        }
    }

    #[inline]
    pub fn start(&mut self) {
        // let misses = Builder::new().group(&mut group).kind(Hardware::).build()?;    }
        self.group.enable().unwrap();
    }

    #[inline]
    pub fn stop(&mut self) {
        self.group.disable().unwrap();
        self.iters += 1
    }

    pub fn set_iters(&mut self, iters: u64) {
        self.iters = iters as u64
    }

    #[inline]
    pub fn print(&mut self) {
        let counts = self.group.read().unwrap();
        self.iters = std::cmp::max(1, self.iters);
        println!(
            "PROF: {:<25}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}",
            "name", "iters", "cycles", "instructions", "branch-misses", "cache-misses", "cache-refs", "cycles-per-ins", "cache-hit-%",
        );
        println!(
            "PROF: {:<25}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15.2}\t{:>15.2}\n",
            self.name,
            self.iters,
            Formatting::u128((counts[&self.cycles] / self.iters).into()),
            Formatting::u128((counts[&self.ins] / self.iters).into()),
            Formatting::u128((counts[&self.branch_misses] / self.iters).into()),
            Formatting::u128((counts[&self.cache_misses] / self.iters).into()),
            Formatting::u128((counts[&self.cache_refs] / self.iters).into()),
            (counts[&self.cycles] as f64 / counts[&self.ins] as f64),
            100.0 - (counts[&self.cache_misses] as f64 * 100.0 / counts[&self.cache_refs] as f64)
        );
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        self.print()
    }
}

#[cfg(test)]
mod tests {

    use iai::black_box;

    use super::*;
    use crate::test_log::test;

    #[derive(Default)]
    struct Struct {
        a0: i32,
        a1: i32,
        a2: i32,
        a3: i32,
        a4: i32,
        a5: i32,
        a6: i32,
        a7: i32,
    }

    #[derive(Default)]
    struct Array {
        a: [i32; 8],
    }

    #[test]
    fn bench_simple_struct() {
        let mut prof1 = Profiler::new("struct_access".into());

        for _iter in 0..100 {
            let mut s = Struct::default();
            prof1.start();
            s.a0 = black_box(0);
            s.a1 = black_box(1);
            s.a2 = black_box(2);
            s.a3 = black_box(3);
            s.a4 = black_box(4);
            s.a5 = black_box(5);
            s.a6 = black_box(6);
            s.a7 = black_box(7);
            prof1.stop();
        }
    }

    #[test]
    fn bench_simple_array() {
        let mut prof2 = Profiler::new("array_access".into());
        for _iter in 0..100 {
            let mut a = Array::default();
            prof2.start();
            a.a[0] = black_box(0);
            a.a[1] = black_box(1);
            a.a[2] = black_box(2);
            a.a[3] = black_box(3);
            a.a[4] = black_box(4);
            a.a[5] = black_box(5);
            a.a[6] = black_box(6);
            a.a[7] = black_box(7);
            prof2.stop();
        }
    }
}