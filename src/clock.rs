use std::fmt;
use std::time::{Duration, Instant};
use crate::utils::Formatting;
use serde::{Deserialize, Serialize};
use crate::infra::component::Component;




#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Clock {

    #[serde(skip)]
    start_search: (Instant, u64),

    #[serde(skip)]
    start_iter: (Instant, u64),

    #[serde(skip)]
    nodes: u32,
}


impl Default for Clock {
    fn default() -> Self {
        let now = Instant::now();
        Clock {
            start_search: (now, 0),
            start_iter: (now, 0),
            nodes: 0,
        }
    }
}


impl Component for Clock {

    fn new_game(&mut self) {
        self.start_search();
    }

    fn new_position(&mut self) {
        self.start_search();
    }
}



impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "search time      : {}", Formatting::format_duration(self.elapsed_search().0))?;
        writeln!(f, "search nodes     : {}", Formatting::format_u128(self.elapsed_search().1 as u128))?;
        writeln!(f, "iter time        : {}", Formatting::format_duration(self.elapsed_iter().0))?;
        writeln!(f, "iter nodes       : {}", Formatting::format_u128(self.elapsed_iter().1 as u128))?;
        writeln!(f, "nodes            : {}", Formatting::format_u128(self.nodes as u128))
    }
}





impl Clock {
    #[inline]
    pub fn start_search(&mut self)  {
        *self = Self::default()
    }

    #[inline]
    pub fn incr_node_count(&mut self) {
        self.nodes += 1;
    }

    #[inline]
    pub fn start_iter(&mut self) {
        self.start_iter = (Instant::now(), self.nodes as u64);
    }

    #[inline]
    pub fn elapsed_search(&self) -> (Duration, u64) {
        (self.start_search.0.elapsed(), self.nodes as u64 - self.start_search.1)
    }

    #[inline]
    pub fn elapsed_iter(&self) -> (Duration, u64) {
        (self.start_iter.0.elapsed(), self.nodes as u64 - self.start_iter.1)
    }


}
