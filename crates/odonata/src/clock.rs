use crate::infra::{
    component::{Component, State},
    utils::DurationFormatter,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    ops::Sub,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Default, Debug)]
#[repr(align(64))]
struct Aligned(AtomicU64);


// type AtomicPaddedU64 = CachePadded<AtomicCell<u64>>;

// impl Clone for Aligned {
//     fn clone(&self) -> Self {
//         Aligned(AtomicU64::new(self.0.load(Ordering::Relaxed)))
//     }
// }

#[derive(Clone, Copy, Debug, Default)]
pub struct Measure {
    pub time:         Duration,
    pub nodes:        u64,
    pub instructions: u64,
    pub cycles:       u64,
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

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    include_q_nodes: bool,

    #[serde(skip)]
    state: ClockState,
}


#[derive(Clone, Debug)]
struct ClockState{
    start_search: Measure,
    start_iter: Measure,
    timer: Instant,
    thread_index: u32,
    int_nodes: Arc<Vec<Aligned>>,
    q_nodes: Arc<Vec<Aligned>>,
}



impl Default for ClockState {
    fn default() -> Self {
        ClockState {
            start_search:    Measure::new(),
            start_iter:      Measure::new(),
            timer:           Instant::now(),
            // profiler: RefCell::new(Profiler::new("Clock".to_string())),
            thread_index:    0,
            int_nodes:       Arc::new({
                let mut v = Vec::with_capacity(32);
                v.extend(std::iter::repeat_with(|| Aligned(AtomicU64::default())).take(32));
                v
            }),
            q_nodes:         Arc::new({
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
            StartSearch => self.new_position(),
            EndSearch => {}
            StartDepthIteration(_) => self.new_iter(),
            Shutdown => {}
        }
    }

    fn new_game(&mut self) {
        self.new_position();
    }

    fn new_iter(&mut self) {
        self.state.start_iter = self.elapsed();
    }

    fn set_thread_index(&mut self, thread_index: u32) {
        self.state.thread_index = thread_index;
    }

    fn new_position(&mut self) {
        *self = Clock {
            include_q_nodes: self.include_q_nodes,
            state: ClockState::default()
        };
    }
}
// impl fmt::Debug for Clock {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_struct("Clock")
//             .field("start_search", &self.start_search)
//             .field("start_iter", &self.start_iter)
//             .field("timer", &self.timer)
//             .field("thread_index", &self.thread_index)
//             .field("nodes", &self.nodes)
//             .finish()
//     }
// }

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::infra::utils::IntegerFormatter;

        let s = self.elapsed_search().time.human();
        writeln!(f, "search time      : {s}")?;

        let s = self.elapsed_search().nodes.human();
        writeln!(f, "search int nodes : {s}")?;

        let s = self.elapsed_search().nodes.human();
        writeln!(f, "search nodes     : {s}")?;

        let s = self.cumul_knps_all_threads().human();
        writeln!(f, "search knps      : {s}")?;

        let s = self.elapsed_iter_this_thread().time.human();
        writeln!(f, "iter time        : {s}")?;

        let s = self.cumul_nodes_this_thread();
        writeln!(f, "cumul nodes      : {s}")?;

        let s = self.cumul_nodes_all_threads();
        writeln!(f, "cumul nodes all  : {s}")?;
        Ok(())
    }
}

impl Clock {
    pub fn restart_elapsed_search_clock(&mut self) {
        self.state.start_search = self.elapsed();
    }

    #[inline]
    /// cumulative accross all iterations
    /// iter1: ply1_nodes
    /// iter2: (ply1_nodes) + ply1_nodes + ply2_nodes
    /// iter3: (ply1_nodes) + (ply1_nodes + ply2_nodes) + (ply1_nodes + ply2_nodes + ply3_nodes)
    pub fn cumul_nodes_this_thread(&self) -> u64 {
        self.state.int_nodes[self.state.thread_index as usize]
            .0
            .load(Ordering::Relaxed)
            + if self.include_q_nodes {
                self.state.q_nodes[self.state.thread_index as usize]
                    .0
                    .load(Ordering::Relaxed)
            } else {
                0
            }
    }

    // #[inline]
    // pub fn cumul_leaf_nodes(&self) -> u64 {
    //     self.nodes[self.thread_index as usize].0.load(Ordering::Relaxed)
    // }

    #[inline]
    pub fn cumul_nodes_all_threads(&self) -> u64 {
        self.state.int_nodes
            .iter()
            .map(|e| e.0.load(Ordering::Relaxed))
            .sum::<u64>()
            + if self.include_q_nodes {
                self.state.q_nodes
                    .iter()
                    .map(|e| e.0.load(Ordering::Relaxed))
                    .sum::<u64>()
            } else {
                0
            }
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
    pub fn inc_int_nodes(&self) {
        self.state.int_nodes[self.state.thread_index as usize]
            .0
            .fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_q_nodes(&self) {
        self.state.q_nodes[self.state.thread_index as usize]
            .0
            .fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn q_nodes(&self) -> u64 {
        self.state.q_nodes[self.state.thread_index as usize]
            .0
            .load(Ordering::Relaxed)
    }

    #[inline]
    pub fn elapsed(&self) -> Measure {
        let m = Measure {
            time:         self.state.timer.elapsed(),
            nodes:        self.cumul_nodes_this_thread(),
            instructions: 0,
            cycles:       0,
        };
        // if true {
        //     let mut prof = self.profiler.borrow_mut();
        //     m.cycles = prof.cycles();
        //     m.instructions = prof.instructions();
        // }
        m
    }

    #[inline]
    // since start of search
    pub fn elapsed_search(&self) -> Measure {
        self.elapsed() - self.state.start_search
    }

    #[inline]
    // since start of play
    pub fn elapsed_iter_this_thread(&self) -> Measure {
        self.elapsed() - self.state.start_iter
    }
}
