use crate::infra::component::{Component, State};
use crate::infra::utils::{DurationFormatter};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Sub;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default, Debug)]
#[repr(align(64))]
struct Aligned(AtomicU64);

// impl Clone for Aligned {
//     fn clone(&self) -> Self {
//         Aligned(AtomicU64::new(self.0.load(Ordering::Relaxed)))
//     }
// }

#[derive(Clone, Copy, Debug, Default)]
pub struct Measure {
    pub time: Duration,
    pub nodes: u64,
    pub instructions: u64,
    pub cycles: u64,
}

impl Measure {
    fn new() -> Self {
        Self::default()
    }
}

impl Sub for Measure {
    type Output = Measure;

    fn sub(mut self, rhs: Measure) -> Self::Output {
        self.time = self.time - rhs.time;
        self.nodes = self.nodes - rhs.nodes;
        self.instructions = self.instructions - rhs.instructions;
        self.cycles = self.cycles - rhs.cycles;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clock {
    #[serde(skip)]
    start_search: Measure,

    #[serde(skip)]
    start_iter: Measure,

    #[serde(skip)]
    timer: Instant,

    #[serde(skip)]
    thread_index: u32,

    #[serde(skip)]
    nodes: Arc<Vec<Aligned>>,
    // #[serde(skip)]
    // leaf_nodes: Aligned,
}

impl Default for Clock {
    fn default() -> Self {
        Clock {
            start_search: Measure::new(),
            start_iter: Measure::new(),
            timer: Instant::now(),
            thread_index: 0,
            nodes: Arc::new({
                let mut v = Vec::with_capacity(32);
                v.extend(std::iter::repeat_with(|| Aligned(AtomicU64::default())).take(32));
                v
            }),
        }
    }
}

impl Component for Clock {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => self.new_game(),
            SetPosition => self.new_position(),
            StartSearch => *self = Self::default(),
            EndSearch => {}
            StartDepthIteration(_) => self.new_iter(),
            Shutdown => {}
        }
    }
    fn new_game(&mut self) {
        *self = Self::default()
    }

    fn new_iter(&mut self) {
        self.start_iter = self.elapsed();
    }

    fn set_thread_index(&mut self, thread_index: u32) {
        self.thread_index = thread_index;
    }

    fn new_position(&mut self) {
        *self = Self::default()
    }
}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::infra::utils::IntegerFormatter;

        let s = self.elapsed_search().time.human();
        writeln!(f, "search time      : {s}")?;

        let s = self.elapsed_search().nodes.human();
        writeln!(f, "search nodes     : {s}")?;

        let s = self.cumul_knps_all_threads().human();
        writeln!(f, "search knps      : {s}")?;

        let s = self.elapsed_iter_this_thread().time.human();
        writeln!(f, "iter time        : {s}")?;

        let s = self.elapsed_iter_this_thread().nodes.human();
        writeln!(f, "iter nodes       : {s}")?;

        let s = self.cumul_nodes_this_thread();
        writeln!(f, "cumul nodes      : {s}")?;

        let s = self.cumul_nodes_all_threads();
        writeln!(f, "cumul nodes all  : {s}")?;
        Ok(())
    }
}

impl Clock {
    pub fn restart_elapsed_search_clock(&mut self) {
        self.start_search = self.elapsed();
    }

    #[inline]
    /// cumulative accross all iterations
    /// iter1: ply1_nodes
    /// iter2: (ply1_nodes) + ply1_nodes + ply2_nodes
    /// iter3: (ply1_nodes) + (ply1_nodes + ply2_nodes) + (ply1_nodes + ply2_nodes + ply3_nodes)
    ///
    pub fn cumul_nodes_this_thread(&self) -> u64 {
        self.nodes[self.thread_index as usize]
            .0
            .load(Ordering::Relaxed)
    }

    // #[inline]
    // pub fn cumul_leaf_nodes(&self) -> u64 {
    //     self.nodes[self.thread_index as usize].0.load(Ordering::Relaxed)
    // }

    #[inline]
    pub fn cumul_nodes_all_threads(&self) -> u64 {
        self.nodes.iter().map(|e| e.0.load(Ordering::Relaxed)).sum()
    }

    pub fn cumul_knps_all_threads(&self) -> u64 {
        self.cumul_nodes_all_threads() / (1 + self.elapsed_search().time.as_millis() as u64)
    }

    pub fn cumul_knps_this_thread(&self) -> u64 {
        self.cumul_nodes_this_thread() / (1 + self.elapsed_search().time.as_millis() as u64)
    }

    // pub fn branching_factor(&self) -> f32 {
    //     let interior = std::cmp::max(1, self.cumul_nodes() - self.cumul_leaf_nodes());
    //     self.cumul_leaf_nodes() as f32 / interior as f32
    // }

    #[inline]
    pub fn inc_nodes(&self) {
        self.nodes[self.thread_index as usize]
            .0
            .fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn elapsed(&self) -> Measure {
        Measure {
            time: self.timer.elapsed(),
            nodes: self.cumul_nodes_this_thread(),
            instructions: 0,
            cycles: 0,
        }
    }

    #[inline]
    // since start of search
    pub fn elapsed_search(&self) -> Measure {
        self.elapsed() - self.start_search
    }

    #[inline]
    // since start of play
    pub fn elapsed_iter_this_thread(&self) -> Measure {
        self.elapsed() - self.start_iter
    }
}
