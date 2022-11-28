#[cfg(any(test, feature = "profiler"))]
use std::fmt;

#[cfg(any(test, feature = "profiler"))]
use perf_event::{events::Hardware, Builder, Counter, Group};

#[cfg(any(test, feature = "profiler"))]
use super::{black_box, utils::IntegerFormatter};

#[cfg(any(test, feature = "profiler"))]
pub struct ProfProfiler<'a> {
    guard: pprof::ProfilerGuard<'a>,
    name: String,
}

#[cfg(any(test, feature = "profiler"))]
impl<'a> ProfProfiler<'a> {
    pub fn new(name: String) -> ProfProfiler<'a> {
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1_000_000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .unwrap();
        ProfProfiler { guard, name }
    }
}

#[cfg(any(test, feature = "profiler"))]
impl<'a> ProfProfiler<'a> {
    pub fn report(&mut self) -> anyhow::Result<()> {
        fn proc() -> impl Fn(&mut pprof::Frames) {
            move |frames| {
                // let vec = &frames.frames;
                frames.frames = frames
                    .frames
                    .clone()
                    .into_iter()
                    .unique_by(|vec| vec[0].name())
                    .collect();
            }
        }
        use itertools::Itertools;
        if let Ok(report) = self.guard.report().frames_post_processor(proc()).build() {
            use std::fs::File;
            let file = File::create(format!("{name}_flamegraph_1.svg", name = self.name))?;
            let mut options = pprof::flamegraph::Options::default();
            options.flame_chart = false;
            report.flamegraph_with_options(file, &mut options)?;

            let file = File::create(format!("{name}_flamegraph_2.svg", name = self.name))?;
            let mut options = pprof::flamegraph::Options::default();
            options.reverse_stack_order = true;
            report.flamegraph_with_options(file, &mut options)?;
        };
        Ok(())
    }
}

#[cfg(any(test, feature = "profiler"))]
pub struct PerfProfiler {
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

#[cfg(any(test, feature = "profiler"))]
impl fmt::Display for PerfProfiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(any(test, feature = "profiler"))]
impl PerfProfiler {
    #[inline]
    pub fn new(name: String) -> PerfProfiler {
        let mut group = Group::new().unwrap();
        // REF_CPU_CYCLES not supported on ZEN3
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
        PerfProfiler {
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

    pub fn benchmark<R>(&mut self, f: impl FnOnce() -> R) -> R {
        self.start();
        let ret = black_box(f());
        self.stop();
        ret
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

    pub fn cycles(&mut self) -> u64 {
        self.group.read().unwrap()[&self.cycles]
    }

    pub fn instructions(&mut self) -> u64 {
        self.group.read().unwrap()[&self.ins]
    }

    pub fn set_iters(&mut self, iters: u64) {
        self.iters = iters as u64
    }

    pub fn assert_counts(&mut self, expected: u64) {
        let low = (expected as f64 * 0.95) as u64;
        let high = (expected as f64 * 1.05) as u64;
        let ins = self.instructions() / self.iters;
        assert!(
            ins >= low && ins <= high,
            "Profiler for {name} failed with {low} < {ins} < {high}",
            name = self.name
        );
    }

    #[inline]
    pub fn write<W: std::io::Write>(&mut self, mut w: W) -> anyhow::Result<()> {
        let counts = self.group.read().unwrap();
        self.iters = std::cmp::max(1, self.iters);
        writeln!(
            w,
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
        )?;
        writeln!(w,
        "PROFD: {:<25}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15}\t{:>15.2}\t{:>15.2}\n",
        self.name,
        self.iters,
        // Formatting::u128((0u32).into()),
        (counts[&self.cycles] / self.iters).human(),
        (counts[&self.ins] / self.iters).human(),
        (counts[&self.branches] / self.iters).human(),
        (counts[&self.branch_misses] / self.iters).human(),
        (counts[&self.cache_misses] / self.iters).human(),
        (0u32).human(),
        // (counts[&self.cache_refs] / self.iters).human()),
        // (counts[&self.cycles] as f64 / counts[&self.ins] as f64),
        (0u32).human(),
        (0u32).human(),
        // 100.0 - (counts[&self.cache_misses] as f64 * 100.0 / counts[&self.cache_refs] as f64)
    )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    impl Drop for PerfProfiler {
        fn drop(&mut self) {
            // if log::log_enabled!(log::Level::Trace) {
            let mut buf = Vec::new();
            self.write(&mut buf).unwrap();
            let s = String::from_utf8(buf).unwrap();
            println!("{s}");
            // let _ = self.write(std::io::stdout());
            // }
        }
    }

    use crate::infra::black_box;
    use crate::test_log::test;
    use crate::trace::stat::Stat;

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
    fn bench_process() {
        use std::process::Command;
        let mut prof1 = PerfProfiler::new("out_of_process_ls_l".into());
        let mut prof2 = PerfProfiler::new("out_of_process_ls_r".into());

        prof1.benchmark(|| {
            let _output = Command::new("ls")
                .args(["-l", "-a"])
                .output()
                .expect("ls command failed to start");
        });
        prof2.benchmark(|| {
            let _output = Command::new("ls")
                .args(["-R", "-a"])
                .output()
                .expect("ls command failed to start");
        });
        println!();
    }

    #[test]
    fn bench_struct_access() {
        let mut prof1 = PerfProfiler::new("struct_access".into());

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
    fn bench_array_access() {
        let mut prof2 = PerfProfiler::new("array_access".into());
        for _iter in 0..100 {
            let mut a = Array::default();
            prof2.benchmark(
                #[inline]
                || {
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

    use super::PerfProfiler;

    thread_local! {
        static COUNTER1: Cell<u64> = Cell::new(0);
    }

    static COUNTER2: Stat = Stat::new("Counter");

    // let COUNTER3: ThreadLocal<Cell<u64>> = ThreadLocal::new();

    #[test]
    fn bench_thread_local() {
        // thread_local macro
        let mut pr = PerfProfiler::new("thread_local!".into());
        for _iter in 0..10001 {
            pr.benchmark(|| COUNTER1.with(|c| c.set(c.get() + 1)));
        }
        assert_eq!(COUNTER1.with(|c| c.get()), 10001);

        // Stat struct
        let mut pr = PerfProfiler::new("thread-Stat".into());
        for _iter in 0..10002 {
            pr.benchmark(|| COUNTER2.increment())
        }
        assert_eq!(COUNTER2.get(), 10002);

        #[allow(non_snake_case)]
        let COUNTER3 = ThreadLocal::new();

        let mut pr = PerfProfiler::new("ThreadLocal".into());
        for _iter in 0..10003 {
            let _count = pr.benchmark(|| {
                let cell = COUNTER3.get_or(|| Cell::new(0));
                let count = cell.get() + 1;
                cell.set(count);
                count
            });
        }
        assert_eq!(COUNTER3.get_or(|| Cell::new(0)).get(), 10003);

        use crate::bits::Bitboard;
        let mut pr = PerfProfiler::new("bitboard".into());
        for _iter in 0..10004 {
            let _count = pr.benchmark(|| {
                let bb = Bitboard::RANK_1;
                let count1 = (bb & Bitboard::FILE_A).popcount();
                let count2 = (black_box(Bitboard::RANK_3) & Bitboard::FILE_A).popcount();
                count1 + count2
            });
        }
    }
}
