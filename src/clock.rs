use crate::infra::component::{Component, State};
use crate::utils::Formatting;
use serde::{Deserialize, Serialize};
use std::fmt;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clock {
    #[serde(skip)]
    start_search: (Instant, u64),

    #[serde(skip)]
    start_iter: (Instant, u64),

    #[serde(skip)]
    thread_index: u32,

    #[serde(skip)]
    nodes: Arc<Vec<Aligned>>,
    // #[serde(skip)]
    // leaf_nodes: Aligned,
}

impl Default for Clock {
    fn default() -> Self {
        let now = Instant::now();
        Clock {
            start_search: (now, 0),
            start_iter: (now, 0),
            thread_index: 0,
            // leaf_nodes: Aligned(Default::default()),
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
        }
    }
    fn new_game(&mut self) {
        *self = Self::default()
    }

    fn new_iter(&mut self) {
        self.start_iter = (Instant::now(), self.cumul_nodes_this_thread());
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
        writeln!(
            f,
            "search time      : {}",
            Formatting::duration(self.elapsed_search().0)
        )?;
        writeln!(
            f,
            "search nodes     : {}",
            Formatting::u128(self.elapsed_search().1 as u128)
        )?;
        writeln!(
            f,
            "search knps      : {}",
            Formatting::u128(self.cumul_knps_all_threads() as u128)
        )?;
        writeln!(
            f,
            "iter time        : {}",
            Formatting::duration(self.elapsed_iter().0)
        )?;
        writeln!(
            f,
            "iter nodes       : {}",
            Formatting::u128(self.elapsed_iter().1 as u128)
        )?;
        writeln!(
            f,
            "cumul nodes      : {}",
            Formatting::u128(self.cumul_nodes_this_thread() as u128)
        )?;
        writeln!(
            f,
            "cumul nodes all  : {}",
            Formatting::u128(self.cumul_nodes_all_threads() as u128)
        )?;
        // writeln!(f, "leaf nodes       : {}", Formatting::u128(self.cumul_leaf_nodes() as u128))?;
        Ok(())
    }
}

impl Clock {
    pub fn restart_elapsed_search_clock(&mut self) {
        self.start_search.0 = Instant::now();
        self.start_search.1 = 0;
    }


    #[inline]
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
        self.cumul_nodes_all_threads() / (1 + self.elapsed_search().0.as_millis() as u64)
    }

    pub fn cumul_knps_this_thread(&self) -> u64 {
        self.cumul_nodes_this_thread() / (1 + self.elapsed_search().0.as_millis() as u64)
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

    // #[inline]
    // pub fn inc_leaf_nodes(&self) {
    //     self.leaf_nodes.0.fetch_add(1, Ordering::Relaxed);
    // }

    #[inline]
    pub fn elapsed_search(&self) -> (Duration, u64) {
        (
            self.start_search.0.elapsed(),
            self.cumul_nodes_this_thread() - self.start_search.1,
        )
    }

    #[inline]
    pub fn elapsed_iter(&self) -> (Duration, u64) {
        (
            self.start_iter.0.elapsed(),
            self.cumul_nodes_this_thread() - self.start_iter.1,
        )
    }
}
