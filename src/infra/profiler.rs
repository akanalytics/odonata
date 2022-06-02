use crate::utils::Formatting;
use perf_event::{events::Hardware, Builder, Counter, Group};

use super::black_box;

pub struct Profiler {
    group: Group,
    name: String,
    iters: u64,
    ins: Counter,
    branches: Counter,
    branch_misses: Counter,
    cache_misses: Counter,
    // cache_refs: Counter,
    cycles: Counter,
}

impl Profiler {
    #[inline]
    pub fn new(name: String) -> Profiler {
        let mut group = Group::new().unwrap();
        let cycles = Builder::new()
            .group(&mut group)
            .kind(Hardware::CPU_CYCLES)
            .build()
            .unwrap();
        let ins = Builder::new()
            .group(&mut group)
            .kind(Hardware::INSTRUCTIONS)
            .build()
            .unwrap();
        let branches = Builder::new()
            .group(&mut group)
            .kind(Hardware::BRANCH_INSTRUCTIONS)
            .build()
            .unwrap();
        let branch_misses = Builder::new()
            .group(&mut group)
            .kind(Hardware::BRANCH_MISSES)
            .build()
            .unwrap();
        let cache_misses = Builder::new()
            .group(&mut group)
            .kind(Hardware::CACHE_MISSES)
            .build()
            .unwrap();
        // let cache_refs = Builder::new()
        // .group(&mut group)
        // .kind(Hardware::CACHE_REFERENCES)
        // .build()
        // .unwrap();
        Profiler {
            name,
            group,
            ins,
            cycles,
            branches,
            branch_misses,
            cache_misses,
            // cache_refs,
            iters: 0,
        }
    }

    pub fn benchmark<R>(&mut self, f: impl FnOnce() -> R) {
        self.start();
        black_box(f());
        self.stop();
    }

    #[inline]
    pub fn start(&mut self) {
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
            "PROFH: {:<25}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}",
            "name",
            "iters",
            "cycles",
            "instructions",
            "branches",
            "branch-misses",
            "cache-misses",
            "cache-refs",
            "cycles-per-ins",
            "cache-hit-%",
        );
        println!(
            "PROFD: {:<25}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15.2}\t{:>15.2}\n",
            self.name,
            self.iters,
            // Formatting::u128((0u32).into()),
            Formatting::u128((counts[&self.cycles] / self.iters).into()),
            Formatting::u128((counts[&self.ins] / self.iters).into()),
            Formatting::u128((counts[&self.branches] / self.iters).into()),
            Formatting::u128((counts[&self.branch_misses] / self.iters).into()),
            Formatting::u128((counts[&self.cache_misses] / self.iters).into()),
            Formatting::u128((0u32).into()),
            // Formatting::u128((counts[&self.cache_refs] / self.iters).into()),
            // (counts[&self.cycles] as f64 / counts[&self.ins] as f64),
            Formatting::u128((0u32).into()),
            Formatting::u128((0u32).into()),
            // 100.0 - (counts[&self.cache_misses] as f64 * 100.0 / counts[&self.cache_refs] as f64)
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

    use super::*;
    use crate::test_log::test;
    use crate::{infra::black_box, trace::stat::Stat};

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
            prof1.benchmark(
                #[inline]
                || {
                    s.a0 = black_box(0);
                    s.a1 = black_box(1);
                    s.a2 = black_box(2);
                    s.a3 = black_box(3);
                    s.a4 = black_box(4);
                    s.a5 = black_box(5);
                    s.a6 = black_box(6);
                    s.a7 = black_box(7);
                },
            )
        }
    }

    #[test]
    fn bench_simple_array() {
        let mut prof2 = Profiler::new("array_access".into());
        for _iter in 0..100 {
            let mut a = Array::default();
            prof2.benchmark(|| {
                a.a[0] = black_box(0);
                a.a[1] = black_box(1);
                a.a[2] = black_box(2);
                a.a[3] = black_box(3);
                a.a[4] = black_box(4);
                a.a[5] = black_box(5);
                a.a[6] = black_box(6);
                a.a[7] = black_box(7);
            })
        }
    }

    use std::cell::Cell;
    use thread_local::ThreadLocal;

    thread_local! {
        static COUNTER1: Cell<u64> = Cell::new(0);
    }

    static COUNTER2: Stat = Stat::new("Counter");

    // let COUNTER3: ThreadLocal<Cell<u64>> = ThreadLocal::new();


    #[test]
    fn bench_thread_local() {
        // thread_local macro
        let mut pr = Profiler::new("thread_local!".into());
        for _iter in 0..10001 {
            pr.benchmark(|| COUNTER1.with(|c| c.set(c.get() + 1)));
        }
        assert_eq!(COUNTER1.with(|c| c.get()), 10001);

        // Stat struct
        let mut pr = Profiler::new("thread-Stat".into());
        for _iter in 0..10002 {
            pr.benchmark(|| COUNTER2.increment())
        }
        assert_eq!(COUNTER2.get(), 10002);

        #[allow(non_snake_case)]
        let COUNTER3 = ThreadLocal::new();

        let mut pr = Profiler::new("ThreadLocal".into());
        for _iter in 0..10003 {
            pr.benchmark(|| {
                let cell = COUNTER3.get_or(|| Cell::new(0));
                let count = cell.get() + 1;
                cell.set(count);
                count
            })
        }
        assert_eq!(COUNTER3.get_or(|| Cell::new(0)).get(), 10003);
    }
}
