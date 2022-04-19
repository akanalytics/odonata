use crate::utils::Formatting;
use perf_event::{events::Hardware, Builder, Counter, Group};

pub struct Profiler {
    group: Group,
    name: String,
    iters: u64,
    ins: Counter,
    branch_misses: Counter,
    cycles: Counter,
}

impl Profiler {
    #[inline]
    pub fn new(name: String) -> Profiler {
        let mut group = Group::new().unwrap();
        let cycles = Builder::new().group(&mut group).kind(Hardware::CPU_CYCLES).build().unwrap();
        let ins = Builder::new().group(&mut group).kind(Hardware::INSTRUCTIONS).build().unwrap();
        let branch_misses = Builder::new().group(&mut group).kind(Hardware::BRANCH_MISSES).build().unwrap();
        Profiler {
            name,
            group,
            ins,
            cycles,
            branch_misses,
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
        println!(
            "PROF: {:<25}\t{:<15}\t{:<15}\t{:<15}\t{:<15}\t{:<15}\nPROF: {:<25}\t{:<15}\t{:<15}\t{:<15}\t{:<15}\t{:.2}\n",
            "name",
            "iters",
            "cycles",
            "instructions",
            "branch misses",
            "cycles/ins",
            self.name,
            self.iters,
            Formatting::u128((counts[&self.cycles] / self.iters).into()),
            Formatting::u128((counts[&self.ins] / self.iters).into()),
            Formatting::u128((counts[&self.branch_misses] / self.iters).into()),
            (counts[&self.cycles] as f64 / counts[&self.ins] as f64)
        );
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        self.print()
    }
}
