use crate::types::MAX_PLY;
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};



#[derive(Default, Debug)]
pub struct Stat<'a> {
    name: &'a str,
    counter: AtomicI64,
}

impl<'a> Clone for Stat<'a> {
    fn clone(&self) -> Self {
        Stat {
            name: self.name,
            counter: AtomicI64::new(self.counter.load(Ordering::Relaxed)),
        }
    }
}


#[derive(Debug, Default)]
pub struct PlyStat {
    name: String,
    values: Box<[AtomicI64]>,
}

impl Clone for PlyStat {
    fn clone(&self) -> Self {
        let ps = PlyStat::new(self.name());
        for ply in 0..self.len() {
            ps.set(ply as u32, self.get(ply as u32));
        }
        ps
    }
}


pub struct ArrayStat<'a> (pub &'a [&'a Stat<'a>]);

impl fmt::Display for ArrayStat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{:<20} {:>11}", "name", "value")?;
        writeln!(f, "{:<20} {:>11}", "--------------------", "-----------")?;
        for s in self.0.iter() {
            writeln!(f, "{:<20} {:>11}", s.name(), s.get())?;
        }
        Ok(())
    }
}

impl<'a> Stat<'a> {


    
    pub const fn new(name: &str) -> Stat {
        Stat { name: name, counter: AtomicI64::new(0) }
    }

    #[inline]
    pub fn add(&self, add: i64) {
        self.counter.fetch_add(add, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment(&self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn set(&self, value: i64) {
        self.counter.store(value, Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> i64 {
        self.counter.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct ArrayPlyStat<'a> (pub &'a [&'a PlyStat]);

impl fmt::Display for ArrayPlyStat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        PlyStat::display(f, self.0)
    }
}


impl PlyStat {
    pub fn new(name: &str) -> PlyStat {
        let mut vec = Vec::with_capacity(MAX_PLY);
        (0..MAX_PLY).into_iter().for_each(|_| vec.push(AtomicI64::new(0)));
        Self { 
            name: name.to_string(), 
            values: vec.into_boxed_slice(),
        }
    }

    pub fn len(&self) -> usize {
        if let Some(d) = (0..MAX_PLY).rposition(|ply| self.get(ply as u32) != 0) {
            return 1 + d; // 1 off the end for all "size" types
        }
        0
    }


    fn display(f: &mut fmt::Formatter, stats: &[&PlyStat]) -> fmt::Result {
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

    pub fn get(&self, ply: u32) -> i64 {
        self.values[ply as usize].load(Ordering::Relaxed)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn fmt_header(f: &mut fmt::Formatter, stats: &[&PlyStat]) -> fmt::Result {
        write!(f, "{:>3} ", "ply")?; 
        for s in stats.iter() {
            write!(f, "{:>14}", s.name())?;
        }
        writeln!(f)?;
        Ok(())
    }

    fn fmt_underline(f: &mut fmt::Formatter, stats: &[&PlyStat]) -> fmt::Result {
        write!(f, "{:>3} ", "---")?; 
        for _s in stats.iter() {
            write!(f, "{:>14}", "-------------")?;
        }
        writeln!(f)?;
        Ok(())
    }

    fn fmt_data(f: &mut fmt::Formatter, stats: &[&PlyStat], ply: u32) -> fmt::Result {
        write!(f, "{:>3} ", ply)?; 
        for s in stats.iter() {
            write!(f, "{:>14}", s.get(ply),)?;
        }
        writeln!(f)?;
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat() {
        let s1 = Stat::new("count");
        let s2 = Stat::new("another");
        let s3 = Stat::new("yet another");
        s1.add(42);
        s2.add(3);
        s3.add(1415);
        println!("{}", ArrayStat(&[&s1, &s2, &s3]));

        let ps = PlyStat::new("count-per-ply");
        ps.add(4, 8);
        ps.add(5, 10);
        ps.add(2, 4);
        println!("{}", ArrayPlyStat(&[&ps]));
        println!("{}", ArrayPlyStat(&[&ps.clone()]));
    }
}
