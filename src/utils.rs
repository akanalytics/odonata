use format_num::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt;
use std::ops::{Bound, RangeBounds};
use std::time::Duration;

//
// https://stackoverflow.com/questions/59413614/cycle-a-rust-iterator-a-given-number-of-times
//
pub trait Itermisc: std::iter::Iterator {
    fn cycle_n(self, n: u64) -> CycleN<Self>
    where
        Self: Clone,
    {
        CycleN::new(self, n)
    }
}

impl<T: ?Sized> Itermisc for T where T: std::iter::Iterator {}

pub struct CycleN<I> {
    orig: I,
    iter: I,
    tick: u64,
}
impl<I: Clone> CycleN<I> {
    pub fn new(iter: I, n: u64) -> CycleN<I> {
        CycleN {
            orig: iter.clone(),
            iter,
            tick: n,
        }
    }
}

impl<I> Iterator for CycleN<I>
where
    I: Clone + Iterator,
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        match self.iter.next() {
            None if self.tick > 0 => {
                self.tick -= 1;
                self.iter = self.orig.clone();
                self.iter.next()
            }
            y => y,
        }
    }
}

pub struct Formatting;

impl Formatting {
    pub fn decimal<F: Into<f64>>(decimal_places: u32, n: F) -> String {
        match decimal_places {
            0 => format_num!(".0f", n),
            1 => format_num!(".1f", n),
            2 => format_num!(".2f", n),
            3 => format_num!(".3f", n),
            4 => format_num!(".4f", n),
            _ => format_num!(".8f", n),
        }
    }

    pub fn f64(n: f64) -> String {
        format_num!(".4s", n)
    }

    pub fn u128(n: u128) -> String {
        format_num!(",d", n as f64)
    }

    pub fn duration(d: Duration) -> String {
        DurationNewType(d).to_string()
    }
}

pub struct DurationNewType(pub Duration);

fn pluralize(n: u64) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

// eg 2 days 15h 4m 3.003s
impl fmt::Display for DurationNewType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let days = self.0.as_secs() / (60 * 60 * 24);
        let hours = self.0.as_secs() / (60 * 60) % 24;
        let mins = self.0.as_secs() / 60;
        let secs = self.0.as_secs_f32() - (60 * mins) as f32;
        let mins = mins % 60;
        if days > 0 {
            write!(f, "{} day{} ", days, pluralize(days))?;
        }
        if hours > 0 {
            write!(f, "{}h ", hours)?;
        }
        if mins > 0 {
            write!(f, "{}m ", mins)?;
        }
        write!(f, "{:.3}s", secs)?;
        Ok(())
    }
}

//
// https://users.rust-lang.org/t/how-to-get-a-substring-of-a-string/1351/9
//
pub trait StringUtils {
    fn take_substring(&self, start: usize, len: usize) -> &str;
    fn take_slice(&self, range: impl RangeBounds<usize>) -> &str;
    fn take_char_at(&self, i: usize) -> Option<char>;
    fn split_off_first_word(&self) -> (&str, &str);
    fn trim_first_n_words(&self, n: u16) -> &str;
    fn split_vars_int(&self) -> HashMap<&str, i32>;
}

impl StringUtils for str {
    fn take_substring(&self, start: usize, len: usize) -> &str {
        let mut char_pos = 0;
        let mut byte_start = 0;
        let mut it = self.chars();
        loop {
            if char_pos == start {
                break;
            }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_start += c.len_utf8();
            } else {
                break;
            }
        }
        char_pos = 0;
        let mut byte_end = byte_start;
        loop {
            if char_pos == len {
                break;
            }
            if let Some(c) = it.next() {
                char_pos += 1;
                byte_end += c.len_utf8();
            } else {
                break;
            }
        }
        &self[byte_start..byte_end]
    }
    fn take_slice(&self, range: impl RangeBounds<usize>) -> &str {
        let start = match range.start_bound() {
            Bound::Included(bound) | Bound::Excluded(bound) => *bound,
            Bound::Unbounded => 0,
        };
        let len = match range.end_bound() {
            Bound::Included(bound) => *bound + 1,
            Bound::Excluded(bound) => *bound,
            Bound::Unbounded => self.len(),
        } - start;
        self.take_substring(start, len)
    }

    fn take_char_at(&self, i: usize) -> Option<char> {
        self.chars().nth(i)
    }

    fn split_off_first_word(&self) -> (&str, &str) {
        let mut iter = self.trim_start().splitn(2, ' ');
        if let Some(word1) = iter.next() {
            if let Some(word2) = iter.next() {
                return (word1, word2.trim_start());
            }
            return (word1, "");
        }
        ("", "")
    }

    // x=5;var1=-3;var2=24
    fn split_vars_int(&self) -> HashMap<&str, i32> {
        let mut map = HashMap::new();
        let words = self.split_terminator(';').collect_vec();
        for w in words {
            if let Some((var, value)) = w.split_terminator('=').collect_tuple() {
                if let Ok(value) = value.parse::<i32>() {
                    map.insert(var, value);
                }
            }
        }
        map
    }

    fn trim_first_n_words(&self, n: u16) -> &str {
        let mut string = self;
        for _ in 0..n {
            let (_first, rest) = string.split_off_first_word();
            string = rest;
        }
        string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_off_first_word() {
        let (w1, w2) = "Mary had a little lambda".split_off_first_word();
        assert_eq!(w1, "Mary");
        assert_eq!(w2, "had a little lambda");

        let (w1, w2) = "  Mary had a little lambda".split_off_first_word();
        assert_eq!(w1, "Mary");
        assert_eq!(w2, "had a little lambda");

        let (w1, w2) = "Mary   had a little lambda".split_off_first_word();
        assert_eq!(w1, "Mary");
        assert_eq!(w2, "had a little lambda");

        let (w1, w2) = "".split_off_first_word();
        assert_eq!(w1, "");
        assert_eq!(w2, "");

        let (w1, w2) = "  ".split_off_first_word();
        assert_eq!(w1, "");
        assert_eq!(w2, "");
    }

    #[test]
    fn test_split_vars_int() {
        let map = "a=1;b=4;xyz=-8".split_vars_int();
        // let words = "a=1;b=4;xyz=-8".split_terminator(';').collect_vec();
        assert_eq!(map["a"], 1);
        assert_eq!(map["b"], 4);
        assert_eq!(map["xyz"], -8);
    }

    #[test]
    fn test_trim_first_n_words() {
        let rest = "Mary had a little lambda".trim_first_n_words(0);
        assert_eq!(rest, "Mary had a little lambda");
        let rest = "Mary had a little lambda".trim_first_n_words(1);
        assert_eq!(rest, "had a little lambda");
        let rest = "Mary had a little lambda".trim_first_n_words(5);
        assert_eq!(rest, "");
        let rest = "Mary had a little lambda".trim_first_n_words(50);
        assert_eq!(rest, "");
        let rest = "Mary had a little lambda   ".trim_first_n_words(5);
        assert_eq!(rest, "");
    }

    #[test]
    fn string_utils() {
        let s = "abcdèfghij";
        assert_eq!(s.take_char_at(50), None);
        // All three statements should print:
        // "abcdè, abcdèfghij, dèfgh, dèfghij, g."
        let expected = "abcdè, abcdèfghij, dèfgh, dèfghij, g.";
        assert_eq!(
            expected,
            format!(
                "{}, {}, {}, {}, {}.",
                s.take_substring(0, 5),
                s.take_substring(0, 50),
                s.take_substring(3, 5),
                s.take_substring(3, 50),
                s.take_char_at(6).unwrap()
            )
        );
        assert_eq!(
            expected,
            format!(
                "{}, {}, {}, {}, {}.",
                s.take_slice(..5),
                s.take_slice(..50),
                s.take_slice(3..8),
                s.take_slice(3..),
                s.take_char_at(6).unwrap()
            )
        );
        assert_eq!(
            expected,
            format!(
                "{}, {}, {}, {}, {}.",
                s.take_slice(..=4),
                s.take_slice(..=49),
                s.take_slice(3..=7),
                s.take_slice(3..),
                s.take_char_at(6).unwrap()
            )
        );
    }

    #[test]
    fn test_formatter() {
        assert_eq!(Formatting::f64(12345567.0).as_str(), "12.35M");
        assert_eq!(Formatting::f64(0.0).as_str(), "0.000");
        assert_eq!(Formatting::f64(1234567890123.0).as_str(), "1.235T");
        assert_eq!(Formatting::f64(123456.0).as_str(), "123.5k");
        assert_eq!(Formatting::f64(0.0000123).as_str(), "12.30µ");
        assert_eq!(Formatting::f64(0.0124).as_str(), "12.40m");
        assert_eq!(Formatting::decimal(2, 0.0124 as f32).as_str(), "0.01");
        assert_eq!(Formatting::decimal(0, 4.0124).as_str(), "4");
        assert_eq!(Formatting::decimal(4, 4.012).as_str(), "4.0120");
    }
}
