use crate::types::MAX_PLY;
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Default, Debug)]
struct Stat {
    name: String,
    counter: AtomicI64,
}

#[derive(Debug, Default)]
struct PlyStat {
    name: String,
    values: Box<[AtomicI64]>,
}

impl Stat {
    pub fn new(name: &str) -> Stat {
        Stat { name: name.to_string(), ..Stat::default() }
    }

    pub fn add(&self, add: i64) {
        self.counter.fetch_add(add, Ordering::Relaxed);
    }

    pub fn set(&self, value: i64) {
        self.counter.store(value, Ordering::Relaxed);
    }

    pub fn value(&self) -> i64 {
        self.counter.load(Ordering::Relaxed)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn fmt_header(f: &mut fmt::Formatter, stats: &[&Stat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value}", value = s.name(),)?;
        }
        Ok(())
    }

    fn fmt_underline(f: &mut fmt::Formatter, stats: &[&Stat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value}", value = "----------",)?;
        }
        Ok(())
    }

    fn fmt_data(&self, f: &mut fmt::Formatter, stats: &[&Stat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value}", value = s.value(),)?;
        }
        Ok(())
    }
}

impl PlyStat {
    pub fn new(name: &str) -> PlyStat {

        let mut vec = Vec::with_capacity(MAX_PLY);
        [0..MAX_PLY].iter().for_each(|_| vec.push(AtomicI64::new(0)));
        Self { 
            name: name.to_string(), 
            values: vec.into_boxed_slice(),
        }
    }

    pub fn add(&self, ply: u32, add: i64) {
        self.values[ply as usize].fetch_add(add, Ordering::Relaxed);
    }

    pub fn set(&self, ply: u32, value: i64) {
        self.values[ply as usize].store(value, Ordering::Relaxed);
    }

    pub fn value(&self, ply: u32) -> i64 {
        self.values[ply as usize].load(Ordering::Relaxed)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn fmt_header(f: &mut fmt::Formatter, stats: &[&Stat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value}", value = s.name(),)?;
        }
        Ok(())
    }

    fn fmt_underline(f: &mut fmt::Formatter, stats: &[&Stat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value}", value = "----------",)?;
        }
        Ok(())
    }

    fn fmt_data(&self, f: &mut fmt::Formatter, stats: &[&Stat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value}", value = s.value(),)?;
        }
        Ok(())
    }
}
