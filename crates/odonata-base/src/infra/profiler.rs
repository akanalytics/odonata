#[cfg(target_os = "linux")]
use perf_event::{events::Hardware, Builder, Counter, Group};

use super::utils::{DecimalFormatter, IntegerFormatter};
use std::{
    fmt,
    hint::black_box,
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct Flamegraph<'a> {
    #[cfg(target_os = "linux")]
    guard: Option<pprof::ProfilerGuard<'a>>,

    #[cfg(not(target_os = "linux"))]
    guard: Option<&str<'a>>,
    name:  Option<String>,
}

impl fmt::Debug for Flamegraph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfProfiler")
            .field("name", &self.name)
            .field("guard", &self.guard.is_some())
            .finish()
    }
}

/// enable with RUST_LOG=flamegraph=trace
impl<'a> Flamegraph<'a> {
    pub fn new(name: String) -> Flamegraph<'a> {
        let mut prof = Flamegraph {
            guard: None,
            name:  None,
        };
        if Self::is_requested() {
            prof.enable(name);
            eprintln!("Flamegraph enabled");
        } else {
            println!("Flamegraph disabled");
        }
        prof
    }

    pub fn is_requested() -> bool {
        std::env::var("RUST_FLAME").is_ok()
    }

    pub fn enable(&mut self, name: String) {
        if self.guard.is_none() {
            #[cfg(target_os = "linux")]
            {
                self.guard = Some(
                    pprof::ProfilerGuardBuilder::default()
                        .frequency(299)
                        // .frequency(10921)
                        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
                        .build()
                        .unwrap(),
                );
            }
            self.name = Some(name);
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn report(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// returns the filenames created
    #[cfg(target_os = "linux")]
    pub fn report(&mut self) -> anyhow::Result<()> {
        // the call stack is in reverse item [0][0] is the most nested call
        fn remove_recursion(stacks: &mut Vec<Vec<pprof::Symbol>>) {
            // println!("vvvvvv\n{}\n^^^^^^^", stacks.iter().flatten().format("\n"));
            for x in (0..stacks.len()).rev() {
                for y in (0..stacks[x].len()).rev() {
                    if y >= stacks[x].len() {
                        continue;
                    }
                    let item = stacks[x][y].name();
                    // remove all items before this one
                    for x1 in 0..stacks.len() {
                        for y1 in 0..stacks[x1].len() {
                            if y1 >= stacks[x1].len() {
                                continue;
                            }
                            if (x1 < x || x1 == x && y1 < y) && stacks[x1][y1].name() == item {
                                stacks[x1].remove(y1);
                            }
                        }
                    }
                }
            }
        }

        // for n in 0..stacks.iter().flatten().count() {
        //     let first = stacks.iter().flatten().nth(n).cloned();
        //     let Some(first) = first else {
        //         return;
        //     };
        //     // remove all matching from nth onwards
        //     let mut index = 0;
        //     for t in 0..stacks.len() {
        //         stacks[t].retain(|s| {
        //             let remove = s.name() == first.name() && index > n;
        //             index += 1;
        //             !remove
        //         });
        //     }
        // // remove all matching excluding 0th in stack 0
        // let mut index = 0;
        // stacks[0].retain(|s| {
        //     let remove = s.name() != first.name() && index != 0;
        //     index += 0;
        //     !remove
        // });
        // }
        // }
        //     for t in 0..stacks.len() {
        //         let mut index = 0;
        //         for _ in 0..stacks[t].len() {

        //             if stacks[t][0..index]
        //                 .iter()
        //                 .chain(stacks[t+1..].iter().flatten())
        //                 .any(|s| s.name() == stacks[t][index].name())
        //             {
        //                 // println!("removing {} at index {}", callstack[index].name(), index);
        //                 stacks[t].remove(index);
        //             } else {
        //                 // println!("not removing {} at index {}", callstack[index].name(), index);
        //                 index += 1;
        //             }
        //         }
        //     }
        // }

        fn frames_post_processor() -> impl Fn(&mut pprof::Frames) {
            move |frames| remove_recursion(&mut frames.frames)
        }
        // use itertools::Itertools;
        let Some(guard) = &self.guard else {
            return Ok(());
        };
        let Ok(report) = guard
            .report()
            .frames_post_processor(frames_post_processor())
            .build()
        else {
            // frames_post_processor(proc()).build() else {
            anyhow::bail!("Unable to build flamegraph report");
        };

        let Some(name) = &self.name else {
            panic!("name not set on flamegraph profiler");
        };
        use std::{fs::File, path::Path};

        let name1 = format!("flamegraph_{name}_1.svg")
            .replace(':', "_")
            .replace(' ', "_");
        let file = File::create(&name1)?;
        let mut options = pprof::flamegraph::Options::default();
        options.flame_chart = false;
        report.flamegraph_with_options(file, &mut options)?;

        let name2 = format!("flamegraph_{name}_2.svg")
            .replace(':', "_")
            .replace(' ', "_");
        let file = File::create(&name2)?;
        let mut options = pprof::flamegraph::Options::default();
        options.reverse_stack_order = true;
        report.flamegraph_with_options(file, &mut options)?;
        eprintln!(
            "Generated file:://{f}  flamegraphs {name1} and {name2} with wd={wd}",
            f = Path::new(&name1).canonicalize().unwrap().display(),
            wd = std::env::current_dir().unwrap().display()
        );
        Ok(())
    }
}

impl<'a> Drop for Flamegraph<'a> {
    fn drop(&mut self) {
        let files = self.report();
        if let Err(s) = files {
            error!("Unable to report flamegraph {s}");
            println!("Unable to report flamegraph {s}");
        }
        // for filename in &files {
        //     let path = PathBuf::from(filename).as_path().canonicalize().unwrap();

        //     let Ok(url) = Url::from_file_path(&path) else {
        //             eprintln!("Invalid flamegraph path {}", path.display());
        //         };
        //     eprintln!("Flamegraph: {url}")
        // }
    }
}

pub struct PerfProfiler {
    benchmark_iters: u64,
    start_time:      Instant,
    elapsed:         Duration,
    group:           Group,
    name:            String,
    iters:           u64,
    ins:             Counter,
    branches:        Counter,
    branch_misses:   Counter,
    cache_misses:    Counter,
    // cache_refs: Counter,
    cycles:          Counter,
}

// static PROFILERS: Lazy<Mutex<IndexMap<String, PerfProfiler>>> = Lazy::new(Mutex::default);

static FLAMEGRAPH: Mutex<Flamegraph> = Mutex::new(Flamegraph {
    guard: None,
    name:  None,
});

impl fmt::Display for PerfProfiler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

const NOOP_INSTRUCTION_OVERHEAD: u64 = 76;
const NOOP_BRANCH_OVERHEAD: u64 = 20;

impl PerfProfiler {
    #[inline]
    pub fn new(name: &str) -> Self {
        let name = name.to_string();
        let benchmark_iters = if let Ok(s) = std::env::var("RUST_BENCH") {
            s.parse()
                .unwrap_or_else(|_| panic!("RUST_BENCH not an integer: {s}"))
        } else {
            1
        };
        if Flamegraph::is_requested() {
            FLAMEGRAPH.lock().unwrap().enable(name.clone());
            eprintln!("Flamegraph enabled with {benchmark_iters} iterations");
        };
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
        Self {
            benchmark_iters,
            start_time: Instant::now(),
            elapsed: Duration::ZERO,
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

    pub fn bench<R>(&mut self, mut f: impl FnMut() -> R) -> R {
        // let mut last = 0;
        self.start();
        for _iter in 1..self.benchmark_iters {
            let _ret = black_box(f());
            self.iters += 1;
            // let ins = self.group.read().unwrap()[&self.ins];
            // eprintln!("{iter} = {}", ins - last);
            // last = ins;
        }
        let ret = black_box(f());
        self.stop();
        self.iters += 1;
        ret
    }

    #[inline]
    pub fn start(&mut self) {
        self.start_time = Instant::now(); // time before cycles etc
        self.group.enable().unwrap();
    }

    #[inline]
    pub fn stop(&mut self) {
        self.group.disable().unwrap();
        self.elapsed += self.start_time.elapsed(); // time after cycles etc
    }

    pub fn cycles(&mut self) -> u64 {
        self.group.read().unwrap()[&self.cycles]
    }

    pub fn instructions(&mut self) -> u64 {
        self.group.read().unwrap()[&self.ins] - NOOP_INSTRUCTION_OVERHEAD
    }

    pub fn set_iters(&mut self, iters: u64) {
        self.iters = iters
    }

    // pub fn assert_counts(&mut self, expected: u64) {
    //     let low = (expected as f64 * 0.95) as u64;
    //     let high = (expected as f64 * 1.05) as u64;
    //     let ins = self.instructions() / self.iters;
    //     assert!(
    //         ins >= low && ins <= high,
    //         "Profiler for {name} failed with {low} < {ins} < {high}",
    //         name = self.name
    //     );
    // }

    #[inline]
    pub fn write<W: std::io::Write>(&mut self, mut w: W) -> anyhow::Result<()> {
        let counts = self.group.read().unwrap();
        self.iters = std::cmp::max(1, self.iters);
        let iters = self.iters;
        writeln!(
            w,
            "PROFH: {:<30}\t{:>13}\t{:>13}\t{:>13}\t{:>13}\t{:>13}\t{:>13}\t{:>13}\t{:>13}", /* \t{:>13}\t{:>13}", */
            "name",
            "instructions",
            "iters",
            "cycles",
            "realtime_ms",
            "branches",
            "branch-misses",
            "cache-misses",
            "cache-refs",
            // "cycles-per-ins",
            // "cache-hit-%",
        )?;
        writeln!(w,
        "PROFD: {n:<30}\t{ins:>13}\t{it:>13}\t{cy:>13}\t{rt:>13}\t{br:>13}\t{bm:>13}\t{cm:>13}\t{cr:>13}\n", // \t{cpi:>13.2}\t{ch:>13.2}\n",
        n = self.name,
        ins = (((counts[&self.ins]) / iters).saturating_sub(NOOP_INSTRUCTION_OVERHEAD)).human(),
        it = iters.human(),
        // Formatting::u128((0u32).into()),
        cy = (counts[&self.cycles] / iters).human(),
        rt = (self.elapsed.as_micros() as f64 / 1000.0 ).dp(3),
        br = (counts[&self.branches] / iters).human(),  // ((counts[&self.branches] / iters).wrapping_sub(NOOP_BRANCH_OVERHEAD)).human(),
        bm = (counts[&self.branch_misses] / iters).human(),
        cm = (counts[&self.cache_misses] / iters).human(),
        cr = (0u32).human(),
        // (counts[&self.cache_refs] / iters).human()),
        // (counts[&self.cycles] as f64 / counts[&self.ins] as f64),
        // cpi = (0u32).human(),
        // ch = (0u32).human(),
        // 100.0 - (counts[&self.cache_misses] as f64 * 100.0 / counts[&self.cache_refs] as f64)
    )?;
        Ok(())
    }
}

impl Drop for PerfProfiler {
    fn drop(&mut self) {
        if std::env::var("RUST_FLAME").is_ok() {
            let mut flamer = FLAMEGRAPH.lock().unwrap();
            let _files = flamer.report(); // silent fail in drop
            flamer.guard = None;
        }

        if std::env::var("RUST_BENCH").is_ok() {
            let mut buf = Vec::new();
            self.write(&mut buf).unwrap();
            let s = String::from_utf8(buf).unwrap();
            println!("{s}");
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{test_log::test, trace::stat::Stat};
    use std::hint::black_box;

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
    fn bench_noop() {
        let mut prof1 = PerfProfiler::new("noop");
        prof1.bench(|| {})
    }

    #[test]
    fn bench_process() {
        use std::process::Command;
        let mut prof1 = PerfProfiler::new("out_of_process_ls_l");
        let mut prof2 = PerfProfiler::new("out_of_process_ls_r");

        prof1.bench(|| {
            let _output = Command::new("ls")
                .args(["-l", "-a"])
                .output()
                .expect("ls command failed to start");
        });
        prof2.bench(|| {
            let _output = Command::new("ls")
                .args(["-R", "-a"])
                .output()
                .expect("ls command failed to start");
        });
        println!();
    }

    #[test]
    fn bench_struct_access() {
        let mut prof1 = PerfProfiler::new("struct_access");

        for _iter in 0..100 {
            let mut s = Struct::default();
            prof1.bench(
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
        let mut prof2 = PerfProfiler::new("array_access");
        for _iter in 0..100 {
            let mut a = Array::default();
            prof2.bench(
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
                },
            )
        }
    }

    use std::cell::Cell;
    use thread_local::ThreadLocal;

    use super::PerfProfiler;

    thread_local! {
        static COUNTER1A: Cell<u64> = Cell::new(0);
    }

    thread_local! {
        static COUNTER1B: Cell<u64> = Cell::new(10);
    }

    static COUNTER2: Stat = Stat::new("Counter");

    // let COUNTER3: ThreadLocal<Cell<u64>> = ThreadLocal::new();

    #[test]
    fn bench_thread_local() {
        // thread_local macro
        let mut pr = PerfProfiler::new("thread_local!");
        for _iter in 0..10001 {
            pr.bench(|| COUNTER1A.with(|c| c.set(black_box(c.get() + 1))));
        }
        assert_eq!(COUNTER1A.with(|c| c.get()), 10001);

        let mut pr = PerfProfiler::new("thread_local.set (rust 1.73+)");
        for _iter in 0..10001 {
            pr.bench(|| COUNTER1B.set(black_box(COUNTER1B.get() + 1)));
        }
        assert_eq!(COUNTER1B.with(|c| c.get()), 10011);

        // Stat struct
        let mut pr = PerfProfiler::new("thread-aligned -atomic (Stat)");
        for _iter in 0..10002 {
            pr.bench(|| COUNTER2.increment())
        }

        // metrics are removed during features=fast profiling!
        // assert_eq!(COUNTER2.get(), 10002);

        let counter3 = ThreadLocal::new();

        let mut pr = PerfProfiler::new("thread_local crate");
        for _iter in 0..10003 {
            let _count = pr.bench(|| {
                let cell = counter3.get_or(|| Cell::new(0));
                let count = cell.get() + 1;
                cell.set(count);
                count
            });
        }
        assert_eq!(counter3.get_or(|| Cell::new(0)).get(), 10003);

        use crate::bits::Bitboard;
        let mut pr = PerfProfiler::new("bitboard popcounts");
        for _iter in 0..10004 {
            let _count = pr.bench(|| {
                let bb = Bitboard::RANK_1;
                let count1 = (bb & Bitboard::FILE_A).popcount();
                let count2 = (black_box(Bitboard::RANK_3) & Bitboard::FILE_A).popcount();
                count1 + count2
            });
        }
    }
}
