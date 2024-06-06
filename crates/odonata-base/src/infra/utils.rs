use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::BufRead;
use std::ops::{Bound, RangeBounds};
use std::path::Path;
use std::time::Duration;

use format_num::*;
use tabled::settings::Style;
use tabled::Table;

use crate::domain::node::Timing;
use crate::infra::metric::Metrics;
use crate::prelude::*;

// /// things that can be used to lookup in an array
// pub trait Indexer<'t, T> {
//     fn index(self) -> usize;

//     fn indexing(&self, array: &'t [T]) -> &'t T {
//         #[cfg(feature = "unchecked_indexing")]
//         unsafe {
//             array.get_unchecked(self.index())
//         }

//         #[cfg(not(feature = "unchecked_indexing"))]
//         &array[(self.index())]
//     }
// }

/// compares two things and reports differences or perhaps just the first
///  difference in a human readable form

pub fn type_suffix(type_name: &str) -> &str {
    if let Some(i) = type_name.rfind("::") {
        &type_name[i + 2..]
    } else {
        type_name
    }
}

pub trait Uci: Sized {
    /// format does not include the object name
    /// so position will be formatted without the leading "position" keyword
    fn fmt_uci(&self, f: &mut fmt::Formatter) -> fmt::Result;
    /// parse will ignore the object name if supplied
    /// eg "position fen K7/..." or
    ///    "info depth 5 ..."
    fn parse_uci(s: &str) -> anyhow::Result<Self>;
    fn to_uci(&self) -> String {
        Displayable(|fmt| self.fmt_uci(fmt)).to_string()
    }
}

pub struct UciString;

impl UciString {
    pub fn to_uci(s: &str) -> String {
        match s {
            "" => "<empty>".to_string(),
            s => s.to_string(),
        }
    }

    pub fn parse_uci(s: &str) -> String {
        match s {
            "\"\"" | "<empty>" => "".to_string(),
            _ => s.to_string(),
        }
    }
}

pub fn file_open(filename: impl AsRef<Path>) -> anyhow::Result<File> {
    let f = File::open(&filename).with_context(|| {
        format!(
            "failed to read file: {s} cwd: {cur}",
            s = filename.as_ref().display(),
            cur = std::env::current_dir().unwrap_or_default().display()
        )
    })?;
    Ok(f)
}

pub fn file_create(filename: impl AsRef<Path>) -> anyhow::Result<File> {
    let f = File::create(&filename).with_context(|| {
        format!(
            "failed to create file: {s} cwd: {cur}",
            s = filename.as_ref().display(),
            cur = std::env::current_dir().unwrap_or_default().display()
        )
    })?;
    Ok(f)
}

/// puts a ";" or "#" for example at the begiining of all lines
pub fn prefix_comment(s: &str, comment: char) -> String {
    format!("{comment} {}", s.replace('\n', &format!("\n{comment} ")))
}

// #[inline]
// pub fn win_probability_from_cp_and_k_fast(centipawns: f32, k: f32) -> f32 {
//     #[inline]
//     fn pow10(b: f32) -> f32 {
//         const LOG_OF_10: f32 = 2.302_585_125; // ln(10.0)
//         fast_math::exp(b * LOG_OF_10)
//     }
//     1.0 / (1.0 + pow10(-centipawns / (k * 100.0)))
// }

// branching factor formulae:
//
//   N: Total number of nodes processed accross all iterations
//   d: Depth
//   bf: Effective branching factor.
//   Nodes iter d   = 1 + bf^1 + (bf)^2 + ... + (bf)^d
//   Nodes last d-1 = 1 + bf + (bf)^2 + ... + (bf)^(d-1)
//   Nodes iter 1   = 1 + bf
//   Nodes iter 0   = 1
//   Total nodes    = (d+1) + (d) bf + (d-1) * fb^2 + .... + 1 * (bf)^d

// gp summation formula:
//
//    Sn = a + ar + ar2 + ar3 +…+ arn-1
//
//    The formula to find the sum of n terms of GP is:
//
//    Sn = a[(rn – 1)/(r – 1)] if r ≠ 1 and r > 1
//
pub fn calculate_branching_factor_by_nodes_and_depth(nodes: u64, depth: Ply) -> anyhow::Result<f64> {
    let f = |bf: f64| {
        (0..=depth)
            .map(|d: i32| (depth + 1 - d) as f64 * bf.powi(d))
            .sum::<f64>()
            - nodes as f64
    };
    anyhow::ensure!(depth > 0 && nodes > 0, "Depth {depth} and nodes {nodes} must be > 0");
    let guess = (nodes as f64).powf(1.0 / depth as f64);
    trace!("initial guess = {guess} on nodes = {nodes} depth = {depth}");
    solver_bisection(&f, (0.0, guess), 40, 1e-6)
}

pub fn solver_bisection<FUNC>(f: &FUNC, mut interval: (f64, f64), max_iters: usize, epsilon: f64) -> anyhow::Result<f64>
where
    FUNC: Fn(f64) -> f64,
{
    let mut f_0 = f(interval.0);
    let mut error = 0.;

    // ensure we started with valid bracket
    anyhow::ensure!(
        f_0 * f(interval.1) <= 0.,
        "Window [{x},{y}] must bound root",
        x = interval.0,
        y = interval.1
    );

    for iter in 0..max_iters {
        let mid = (interval.0 + interval.1) / 2.;
        let f_mid = f(mid);

        if f_0 * f_mid < 0. {
            interval.1 = mid;
        } else {
            interval.0 = mid;
            f_0 = f_mid;
        }

        error = (interval.1 - interval.0).abs();
        trace!(
            "iter {iter} [{x},{y}] f(x) = {fx}",
            x = interval.0,
            y = interval.1,
            fx = f_mid
        );
        if error < epsilon {
            return Ok(interval.0);
        }
    }
    anyhow::bail!(
        "Unable to find solution after {max_iters}, best so far is {x} with error {error}",
        x = interval.0
    );
}

/// https://en.wikipedia.org/wiki/Golden-section_search
pub fn solver_minimum_golden_section<FUNC>(
    f: &FUNC,
    interval: (f64, f64),
    max_iters: usize,
    epsilon: f64,
) -> anyhow::Result<f64>
where
    FUNC: Fn(f64) -> f64,
{
    let gr: f64 = (f64::sqrt(5.0) + 1.0) / 2.0;

    // def gss(f, a, b, tol=1e-5):
    //     """Golden-section search
    //     to find the minimum of f on [a,b]
    //     f: a strictly unimodal function on [a,b]

    //     Example:
    //     >>> f = lambda x: (x-2)**2
    //     >>> x = gss(f, 1, 5)
    //     >>> print("%.15f" % x)
    //     2.000009644875678

    //     """
    let (mut a, mut b) = interval;
    for iter in 0..max_iters {
        if (b - a).abs() <= epsilon {
            return Ok((b + a) / 2.0);
        }
        let c = b - (b - a) / gr;
        let d = a + (b - a) / gr;
        if f(c) < f(d) {
            // f(c) > f(d) to find the maximum
            b = d;
        } else {
            a = c
        }
        trace!(target: "solver", "iter {iter:>3} {a:.5} {b:.5}");
    }

    anyhow::bail!("Unable to find solution after {max_iters}, best so far is ({a},{b})");
}

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

pub fn read_file(filename: impl AsRef<Path> + Clone) -> Result<Vec<String>> {
    debug!(
        "reading lines from file {file} with current dir {pwd}",
        file = filename.as_ref().display(),
        pwd = std::env::current_dir().unwrap_or_default().display()
    );
    let t = Metrics::timing_start();
    let file = file_open(filename)?;
    let lines = std::io::BufReader::new(file).lines();
    let vec = lines.collect::<Result<Vec<String>, _>>()?;
    Metrics::profile(t, Timing::TimingReadFile);
    Ok(vec)
}

pub trait ToStringOr {
    fn to_string_or(&self, s: &str) -> String;
}

impl<T: ToString> ToStringOr for Option<T> {
    fn to_string_or(&self, s: &str) -> String {
        match self {
            Some(t) => t.to_string(),
            None => s.to_string(),
        }
    }
}

pub trait DecimalFormatter {
    fn dp(&self, decimal_places: i32) -> String;
    fn sig_fig(&self, sig_fig: i32) -> String;
}

pub trait IntegerFormatter {
    fn human(&self) -> String;
}

pub trait DurationFormatter {
    fn hhmmss(&self) -> String;
    fn hhmmss_ms(&self) -> String; // 01:34:45.123
    fn human(&self) -> String;
}

// impl<N> IntegerFormatter for N
// where
//     N: Into<i128> + Copy,
// {
//     fn human(&self) -> String {
//         format_num!(",d", Into::<i128>::into(*self) as f64)
//     }
// }

impl IntegerFormatter for usize {
    fn human(&self) -> String {
        format_num!(",d", *self as f64)
    }
}

impl IntegerFormatter for u64 {
    fn human(&self) -> String {
        format_num!(",d", *self as f64)
    }
}
impl IntegerFormatter for i64 {
    fn human(&self) -> String {
        format_num!(",d", *self as f64)
    }
}
impl IntegerFormatter for u32 {
    fn human(&self) -> String {
        format_num!(",d", *self as f64)
    }
}

impl IntegerFormatter for i32 {
    fn human(&self) -> String {
        format_num!(",d", *self as f64)
    }
}

impl DurationFormatter for Duration {
    fn hhmmss(&self) -> String {
        let hours = self.as_secs() / (60 * 60) % 24;
        let mins = self.as_secs() / 60;
        let secs = self.as_secs_f32() - (60 * mins) as f32;
        let mins = mins % 60;
        format!("{hours:02}:{mins:02}:{secs:02.0}")
    }

    fn hhmmss_ms(&self) -> String {
        let millis = self.subsec_millis();
        format!("{hms}.{millis:03.0}", hms = self.hhmmss())
    }

    fn human(&self) -> String {
        DurationNewType(*self).to_string()
    }
}

impl<N> DecimalFormatter for N
where
    N: Into<f64> + Copy,
{
    fn dp(&self, decimal_places: i32) -> String {
        match decimal_places {
            0 => format_num!(",.0f", *self),
            1 => format_num!(",.1f", *self),
            2 => format_num!(",.2f", *self),
            3 => format_num!(",.3f", *self),
            4 => format_num!(",.4f", *self),
            _ => format_num!(",.8f", *self),
        }
    }

    /// 12345567.0 -> "12.35M"
    /// 123456.0 -> "123.5k"
    fn sig_fig(&self, sig_fig: i32) -> String {
        match sig_fig {
            0 => format_num!(".0s", *self),
            1 => format_num!(".1s", *self),
            2 => format_num!(".2s", *self),
            3 => format_num!(".3s", *self),
            4 => format_num!(".4s", *self),
            _ => format_num!(".8s", *self),
        }
    }
}

/// https://docs.rs/format_num/0.1.0/format_num/
pub struct Formatting;

impl Formatting {
    pub fn decimal<F: Into<f64>>(decimal_places: usize, n: F) -> String {
        match decimal_places {
            0 => format_num!(".0f", n),
            1 => format_num!(".1f", n),
            2 => format_num!(".2f", n),
            3 => format_num!(".3f", n),
            4 => format_num!(".4f", n),
            _ => format_num!(".8f", n),
        }
    }

    pub fn hhmmss(d: Duration) -> String {
        let hours = d.as_secs() / (60 * 60) % 24;
        let mins = d.as_secs() / 60;
        let secs = d.as_secs_f32() - (60 * mins) as f32;
        let mins = mins % 60;
        format!("{hours:02}:{mins:02}:{secs:02.0}")
    }

    // significant digits
    pub fn f64(n: f64) -> String {
        let s = format_num!(".3s", n);
        if s.ends_with('k') {
            s.trim_matches('k').to_string() + " k"
        } else if s.ends_with('M') {
            s.trim_matches('M').to_string() + " M"
        } else {
            s
        }
    }

    pub fn u64(n: u64) -> String {
        format_num!(",d", n as f64)
    }

    pub fn duration(d: Duration) -> String {
        DurationNewType(d).to_string()
    }

    /// HashMap key is row (from top) and column. (0,0) is top left. (n,0) is bottom left
    pub fn to_table(cells: HashMap<(usize, usize), String>, empty_cell: &str) -> Table {
        let max_row = cells.keys().map(|(r, _c)| *r).max().unwrap_or_default();
        let max_col = cells.keys().map(|(_r, c)| *c).max().unwrap_or_default();
        let mut grid = vec![vec![empty_cell.to_string(); max_col + 1]; max_row + 1];
        for ((r, c), v) in cells.into_iter() {
            grid[r][c] = v;
        }
        let mut t = tabled::Table::from_iter(grid);
        t.with(Style::modern());
        // .with(Height::increase(5))
        // .with(Disable::row(Rows::first()));
        t
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
        if days > 0 || hours > 0 {
            write!(f, "{}h ", hours)?;
        }
        if days > 0 || hours > 0 || mins > 0 {
            write!(f, "{:>2}m ", mins)?;
        }
        write!(f, "{:>6.3}s", secs)?;
        Ok(())
    }
}

// keyword iterator
pub struct KeywordIter {
    words:       Vec<String>,
    greedy_word: Option<String>,
    text:        String,
}

impl KeywordIter {
    /// splits on words, with (matching_word, between) being returned
    /// optional greedy word consumes the rest of the string
    pub fn new(words: &[&str], greedy_word: Option<&str>, text: &str) -> Self {
        Self {
            // ensure words sorted in reverse length order: in cases one word
            // a substring of another, we need to match longest word first
            words:       words
                .iter()
                .chain(greedy_word.iter())
                .map(|s| s.to_string())
                .sorted_by_key(String::len)
                .rev()
                .collect_vec(),
            greedy_word: greedy_word.map(str::to_string),
            text:        text.to_owned(),
        }
    }
}

impl Iterator for KeywordIter {
    type Item = (String, String);

    fn next(&mut self) -> Option<(String, String)> {
        if self.text.trim().is_empty() {
            return None;
        }
        let text = self.text.clone();
        let mut match_text = text.as_str();
        let mut skip_text = text.as_str();
        let mut match_word = "";

        // now look for any of the other words
        for word in &self.words {
            if let Some((first, rest)) = text.trim_start().split_once(word) {
                if first.len() < skip_text.len() {
                    // found an earlier match
                    skip_text = first;
                    match_word = word;
                    match_text = rest;
                    // shorten the match_text if its curtailed by another keyword
                    if self.greedy_word.as_deref() != Some(match_word) {
                        for word in &self.words {
                            if let Some((first2, _rest2)) = match_text.split_once(word) {
                                if first2.len() < match_text.len() {
                                    match_text = first2;
                                }
                            }
                        }
                    }
                }
            }
        }
        if !match_word.is_empty() {
            // strip off <skip> + <word> + <match>
            self.text = text
                .trim_start()
                .strip_prefix(skip_text)
                .unwrap()
                .strip_prefix(match_word)
                .unwrap()
                .strip_prefix(match_text)
                .unwrap()
                .to_string();
            return Some((match_word.to_string(), match_text.to_string()));
        }
        None
    }
}

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
    use test_log::test;

    use super::*;

    #[test]
    fn test_to_table() {
        let mut cells = HashMap::new();
        cells.insert((0, 0), "0,0".to_string());
        cells.insert((0, 1), "0,1".to_string());
        cells.insert((0, 3), "0,3".to_string());
        cells.insert((2, 3), "2,3".to_string());
        println!("{:^5}", Formatting::to_table(cells, "*"));
    }

    #[test]
    fn test_keyword_iter() {
        let text = "Mary had a little lambda";
        let words = ["Mary", "James", "Bob"];
        let greedy_word = Some("TakeRest");
        println!("\n{text}");
        let mut i = KeywordIter::new(&words, greedy_word, text);
        assert_eq!(i.next(), Some(("Mary".into(), " had a little lambda".into())));
        assert_eq!(i.next(), None);

        let text = "Mary had a little lambda TakeRest Mary had a little dog";
        let words = ["Mary", "James", "Bob"];
        let greedy_word = Some("TakeRest");
        println!("\n{text}");
        for (k, v) in KeywordIter::new(&words, greedy_word, text) {
            println!("k = '{k}' v = '{v}'");
        }

        let text = "Mary likes Bob who likes James but TakeRest Mary had a little dog";
        let words = ["Mary", "James", "Bob"];
        let greedy_word = Some("TakeRest");
        println!("\n{text}");
        for (k, v) in KeywordIter::new(&words, greedy_word, text) {
            println!("k = '{k}' v = '{v}'");
        }
        let text = "Marie likes Bill who likes Jim but TakeNothing Marie had a little dog";
        let words = ["Mary", "James", "Bob"];
        let greedy_word = Some("TakeRest");
        println!("\n{text}");
        for (k, v) in KeywordIter::new(&words, greedy_word, text) {
            println!("k = '{k}' v = '{v}'");
        }
        let text = "Marie likes James who likes Jim but TakeNothing Marie had a little dog";
        let words = ["Mary", "James", "Bob"];
        let greedy_word = Some("TakeRest");
        println!("\n{text}");
        for (k, v) in KeywordIter::new(&words, greedy_word, text) {
            println!("k = '{k}' v = '{v}'");
        }
    }

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
        assert_eq!(Formatting::hhmmss(Duration::from_millis(12345678)).as_str(), "03:25:46");
        assert_eq!(Duration::from_millis(12345678).hhmmss_ms().as_str(), "03:25:46.678");
        assert_eq!(Duration::from_millis(11345678).human(), "3h  9m  5.678s");
        assert_eq!(Formatting::f64(12345567.0).as_str(), "12.3 M");
        assert_eq!(Formatting::f64(0.0).as_str(), "0.00");
        assert_eq!(Formatting::f64(1234567890123.0).as_str(), "1.23T");
        assert_eq!(Formatting::f64(123456.0).as_str(), "123 k");
        assert_eq!(Formatting::f64(0.0000123).as_str(), "12.3µ");
        assert_eq!(Formatting::f64(0.0124).as_str(), "12.4m");
        assert_eq!(Formatting::decimal(2, 0.0124).as_str(), "0.01");
        assert_eq!(Formatting::decimal(0, 4.0124).as_str(), "4");
        assert_eq!(Formatting::decimal(4, 4.012).as_str(), "4.0120");
    }

    #[test]
    fn test_calculate_branching_factor() {
        let bf = calculate_branching_factor_by_nodes_and_depth(14, 3).unwrap();
        assert!((1.339 - bf).abs() < 0.001, "{bf}");

        let bf = calculate_branching_factor_by_nodes_and_depth(30, 4).unwrap();
        assert!((1.51086 - bf).abs() < 0.001, "{bf}");

        let bf = calculate_branching_factor_by_nodes_and_depth(500_000_000, 30).unwrap();
        assert!((1.8512 - bf).abs() < 0.001, "{bf}");
    }

    #[test]
    fn test_solver_minimum() {
        let f = |x: f64| (x - 2.0) * (x - 2.0);
        let x0 = solver_minimum_golden_section(&f, (0.5, 3.0), 100, 1e-5).unwrap();
        assert_eq!((x0 - 2.0).abs() < 1e-5, true);
    }
}
