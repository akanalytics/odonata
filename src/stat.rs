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

    pub fn display(f: &mut fmt::Formatter, stats: &[&PlyStat]) -> fmt::Result {
        let max_len = stats.iter().map(|ps| ps.len() ).max().unwrap();
        Self::fmt_header(f, stats)?;
        Self::fmt_underline(f, stats)?;
        for p in 0..max_len {
            Self::fmt_data(f, stats, p as u32)?;
        }
        Ok(())


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

    pub fn len(&self) -> usize {
        if let Some(d) = (0..MAX_PLY).rposition(|ply| self.value(ply as u32) != 0) {
            return 1 + d; // 1 off the end for all "size" types
        }
        0
    }


    pub fn display(f: &mut fmt::Formatter, stats: &[&PlyStat]) -> fmt::Result {
        let max_len = stats.iter().map(|ps| ps.len() ).max().unwrap();
        Self::fmt_header(f, stats)?;
        Self::fmt_underline(f, stats)?;
        for p in 0..max_len {
            Self::fmt_data(f, stats, p as u32)?;
        }
        Ok(())
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

    fn fmt_header(f: &mut fmt::Formatter, stats: &[&PlyStat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value:>11}", value = s.name(),)?;
        }
        Ok(())
    }

    fn fmt_underline(f: &mut fmt::Formatter, stats: &[&PlyStat]) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value:>11}", value = "----------",)?;
        }
        Ok(())
    }

    fn fmt_data(f: &mut fmt::Formatter, stats: &[&PlyStat], ply: u32) -> fmt::Result {
        for s in stats.iter() {
            write!(f, "{value:>11}", value = s.value(ply),)?;
        }
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat() {
        let ps = PlyStat::new("count-per-ply");
        let s = Stat::new("count");
        ps.add(4, 8);
        ps.add(5, 10);
        ps.add(2, 4);
        s.add(42);
        println!("{}", s.display());
        println!("{}", display(&[&ps]));
    }
}
