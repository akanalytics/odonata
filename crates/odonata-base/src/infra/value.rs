use std::fmt::{self, Debug};
use std::mem;
use std::ops::{Add, AddAssign, Div, Index, IndexMut, Neg, Sub};

use indexmap::IndexMap;
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::utils::IntegerFormatter;
use crate::infra::utils::DecimalFormatter;
use crate::piece::Dual;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
struct CountingMetrics {
    nodes:  u64,
    misses: i64,
    hits:   i64,
}

impl Value {
    pub fn to_pgn(&self) -> String {
        self.to_string()
    }

    pub fn parse_pgn(s: &str) -> Value {
        Value::parse(s)
    }
}

pub trait Aggregate {
    fn aggregate(&mut self, other: Self);
}

impl Aggregate for Value {
    fn aggregate(&mut self, other: Self) {
        *self += other;
    }
}

impl<K, V> Aggregate for IndexMap<K, V>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: Aggregate + Clone,
{
    fn aggregate(&mut self, other: Self) {
        for (k, v) in self.iter_mut() {
            v.aggregate(other[k].clone());
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    #[default]
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    // Average(f64, u32),
    // Percent(f64, f64),
    // Elapsed(Duration, u32),
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<u32> for Value {
    fn from(i: u32) -> Self {
        Self::Int(i as i64)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Self::Int(i as i64)
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Self {
        Self::Float(x)
    }
}

impl From<u64> for Value {
    fn from(x: u64) -> Self {
        Self::Int(x as i64)
    }
}

impl From<f32> for Value {
    fn from(x: f32) -> Self {
        Self::Float(x as f64)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl std::iter::Sum for Value {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Value::Int(0), |mut acc, x| {
            acc += x;
            acc
        })
    }
}

impl Default for &Value {
    fn default() -> Self {
        &Value::Null
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Null => "-".to_string(),
            Self::Int(i) => i.human(),
            Self::Float(f) => f.to_string(),
            Self::Text(s) => s.to_string(),
            // Self::Average(total, count) => (total / *count as f64).to_string(),
            // Self::Percent(x, y) => format!("{}%", (*x * 100. / *y).dp(3)),
            // Self::Elapsed { .. } => "".to_string(),
        };
        write!(f, "{s}")?;
        Ok(())
    }
}

impl Add for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Null, r) => r,
            (s, Self::Null) => s,
            (s @ Self::Text(..), _) => s,
            (_, r @ Self::Text(..)) => r,
            (Self::Int(i), Self::Int(j)) => Value::Int(i + j),
            (Self::Int(i), Self::Float(j)) => Value::Float(i as f64 + j),
            (Self::Float(i), Self::Int(j)) => Value::Float(i + j as f64),
            (Self::Float(x), Self::Float(y)) => Value::Float(x + y),
            // (Self::Average(t1, c1), Self::Average(t2, c2)) => Value::Average(t1 + t2, c1 + c2),
            // (Self::Percent(x1, y1), Self::Percent(x2, y2)) => Value::Percent(x1 + y1, x2 + y2),
            // (s, rhs) => panic!("Cannot add {s:?} with {rhs:?}"),
        }
    }
}
impl Neg for Value {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Self::Null => self,
            Self::Int(i) => Self::Int(-i),
            Self::Float(f) => Self::Float(-f),
            Self::Text(s) => Self::Text(s),
        }
    }
}

impl Sub for Value {
    type Output = Value;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Self) -> Self::Output {
        self + rhs.neg()
    }
}

impl Div for Value {
    type Output = Value;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Null, r) => r,
            (s, Self::Null) => s,
            (s @ Self::Text(..), _) => s,
            (_, r @ Self::Text(..)) => r,
            (Self::Int(i), Self::Int(j)) => Value::Float(i as f64 / j as f64),
            (Self::Int(i), Self::Float(j)) => Value::Float(i as f64 / j),
            (Self::Float(i), Self::Int(j)) => Value::Float(i / j as f64),
            (Self::Float(x), Self::Float(y)) => Value::Float(x / y),
        }
    }
}

impl Div for &Value {
    type Output = Value;
    fn div(self, rhs: Self) -> Self::Output {
        self.clone() / rhs.clone()
    }
}

impl Sub for &Value {
    type Output = Value;

    fn sub(self, rhs: Self) -> Self::Output {
        self.clone() - rhs.clone()
    }
}

impl Add for &Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        self.clone() + rhs.clone()
    }
}

impl AddAssign for Value {
    fn add_assign(&mut self, rhs: Self) {
        let s = mem::take(self);
        let _ = mem::replace(self, s + rhs);
    }
}

impl Value {
    fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_i32(&self) -> i32 {
        match self {
            Self::Null => 0,
            Self::Int(i) => *i as i32,
            Self::Float(f) => *f as i32,
            Self::Text(..) => 0,
            // Self::Elapsed { .. } => 0,
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            Self::Null => 0,
            Self::Int(i) => *i,
            Self::Float(f) => *f as i64,
            Self::Text(..) => 0,
            // Self::Elapsed { .. } => 0,
        }
    }
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Null => 0.,
            Self::Int(i) => *i as f64,
            Self::Float(f) => *f,
            Self::Text(..) => 0.,
            // Self::Elapsed { .. } => 0,
        }
    }

    // pub fn percent(&self) -> Value {
    //     match self {
    //         Value::Null => Value::Null,
    //         Value::Int(i) => format!("{i}%").into(),
    //         Value::Float(f) => format!("{perc}%", perc = (f * 100.).dp(2)).into(),
    //         _ => panic!("cannot turn {self:?} into a percentage"),
    //     }
    // }

    fn parse(s: &str) -> Value {
        if s == "-" {
            trace!(target:"pgn", "parsed value from '{s}' as Null");
            return Value::Null;
        }
        if let Ok(v) = s.parse() {
            trace!(target:"pgn", "parsed value from '{s}' as Int({v})");
            return Value::Int(v);
        }
        // if it has commas and reformats as the same string, parse as int
        if let Ok(v) = s.replace(',', "").parse::<i64>() {
            if v.human() == s {
                trace!(target:"pgn", "parsed value from '{s}' as Int({v})");
                return Value::Int(v);
            }
        }
        if let Ok(v) = s.parse() {
            trace!(target:"pgn", "parsed value from '{s}' as Float({v})");
            return Value::Float(v);
        }
        // if it has commas and reformats as the same string, parse as int
        if let Ok(v) = s.replace(',', "").parse::<f64>() {
            if v.dp(2) == s {
                trace!(target:"pgn", "parsed value from '{s}' as Int({v})");
                return Value::Float(v);
            }
        }

        trace!(target:"pgn", "parsed value from '{s}' as Text()");
        Value::Text(s.to_string())
    }
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Stats {
    map: IndexMap<String, Value>,
}

impl From<IndexMap<String, Value>> for Stats {
    fn from(map: IndexMap<String, Value>) -> Self {
        Self { map }
    }
}

impl Index<&str> for Stats {
    type Output = Value;

    fn index(&self, i: &str) -> &Self::Output {
        self.map.get(i).unwrap_or_default()
    }
}

impl IndexMut<&str> for Stats {
    fn index_mut(&mut self, i: &str) -> &mut Self::Output {
        self.map.entry(i.to_string()).or_default()
    }
}

impl Aggregate for Stats {
    fn aggregate(&mut self, other2: Self) {
        for (k2, v2) in other2.map.into_iter() {
            match self.map.get_mut(&k2) {
                Some(v1) => v1.aggregate(v2),
                None => {
                    self.map.insert(k2, v2.clone());
                }
            }
        }
    }
}

impl Sub for Stats {
    type Output = Stats;

    fn sub(mut self, rhs: Self) -> Self::Output {
        for (k2, v2) in rhs.map {
            match self.map.get_mut(&k2) {
                Some(v1) => *v1 = v1.clone() - v2,
                None => {
                    self.map.insert(k2, v2.neg());
                }
            }
        }
        self
    }
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.map.iter().map(|(k, _v)| k.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut Value)> {
        self.map.iter_mut().map(|(k, v)| (k.as_str(), v))
    }

    pub fn sort_keys(&mut self) {
        self.map.sort_keys();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn filter(mut self, re: &Regex) -> Stats {
        self.map.retain(|k, _v| re.is_match(k));
        self
    }

    pub fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // derived.calc_derived_fields();
        writeln!(f, "stats:")?;
        for (k, v) in &self.map {
            writeln!(f, " {k} = \t{v}")?;
        }
        Ok(())
    }

    /// because stats come from different engine processes, and need to be aggregated
    /// and the engine code producing them can be different code versions
    /// they are treated as weakly typed, with naming doing the work
    /// "hit" + "miss" -> %age (of hits over (hits+misses))
    /// "total" + "count" -> average
    pub fn calc_derived_fields(&mut self) {
        let s = self;

        let keys = s.keys().map(ToString::to_string).collect_vec();
        for key in &keys {
            if key.ends_with("Hit") {
                let stem = key.trim_end_matches("Hit").to_string();
                let hits = &s[key];
                let misses = &s[&(stem + "Miss")];
                if !misses.is_null() {
                    let derived = (key.to_string() + " %").trim_start_matches('.').to_string();
                    s[&derived] = hits / &(hits + misses);
                }
            }
            if key.ends_with("Success") {
                let stem = key.trim_end_matches("Success").to_string();
                let hits = &s[key];
                let misses = &s[&(stem + "Fail")];
                if !misses.is_null() {
                    let derived = (key.to_string() + " %").trim_start_matches('.').to_string();
                    s[&derived] = hits / &(hits + misses);
                }
            }

            if key.ends_with("Sum") {
                let stem = key.trim_end_matches("Sum").to_string();
                let total = &s[key];
                let count = &s[&(stem.clone() + "Count")];
                if !count.is_null() {
                    let derived = (stem.to_string() + "Av").trim_start_matches('.').to_string();
                    s[&derived] = total / count;
                }
            }
        }
    }

    #[must_use]
    pub fn format_fields(mut self) -> Self {
        for (k, v) in self.iter_mut() {
            if k.ends_with('%') {
                *v = Value::Text((100. * v.as_f64()).dp(2) + "%");
            }
            match v {
                Value::Int(i) => *v = Value::Text(i.human()),
                Value::Float(f) => *v = Value::Text(f.dp(2)),
                _ => {}
            };
        }
        self
    }

    pub fn to_pgn<'a>(iter: impl IntoIterator<Item = (&'a str, &'a Stats)>) -> String {
        use std::fmt::Write;
        let (titles, statses): (Vec<&str>, Vec<&Stats>) = iter.into_iter().unzip();
        let mut s = String::new();
        let keys = statses.iter().flat_map(|stats| stats.map.keys()).unique().collect_vec();
        let width_key = keys.iter().cloned().map(String::len).max().unwrap_or_default();
        let widths = statses
            .iter()
            .enumerate()
            .map(|(i, stats)| {
                // width of stats[i] as a column
                stats
                    .map
                    .values()
                    .map(|r| r.to_string().len())
                    .max()
                    .unwrap_or_default()
                    .max(titles[i].len())
            })
            .collect_vec();

        write!(s, "{blank:<width_key$}", blank = "").unwrap();

        for (i, &title) in titles.iter().enumerate() {
            write!(s, " | {title:>width$}", width = widths[i]).unwrap();
        }
        writeln!(s).unwrap();
        for k in keys {
            write!(s, "{k:<width_key$}").unwrap();
            for (i, &stats) in statses.iter().enumerate() {
                let v = stats[k].to_pgn();
                write!(s, " | {v:>width$}", width = widths[i]).unwrap();
            }
            writeln!(s).unwrap();
        }
        writeln!(s).unwrap(); // black line signifies end of metrics for parsing
        s
    }

    pub fn parse_pgn(s: &str) -> anyhow::Result<Dual<Stats>> {
        let mut white = Stats::default();
        let mut black = Stats::default();

        for (i, line) in s.lines().enumerate() {
            if line.trim().is_empty() {
                break;
            }
            let line = line.trim_matches(' ');
            let vec = line.split('|').map(|s| s.trim_matches(' ')).collect_vec();
            if i == 0 {
                if vec != ["", "White", "Black"] {
                    anyhow::bail!("Expected '|White|Black' but got '{line}' instead");
                } else {
                    continue;
                }
            }
            match vec.as_slice() {
                [key, w, b] => {
                    white[key] = Value::parse(w);
                    black[key] = Value::parse(b);
                }
                _ => {
                    anyhow::bail!("Expected key | w-value | b-value but got '{line}' parsing Stats")
                }
            }
        }
        Ok(Dual::new(white, black))
    }

    pub fn transpose(stats_map: IndexMap<String, Stats>) -> IndexMap<String, Stats> {
        let mut new: IndexMap<String, Stats> = IndexMap::new();
        for (r, stats) in stats_map {
            for (c, v) in stats.iter() {
                new.entry(c.to_string()).or_default()[&r] = v.clone();
            }
        }
        new
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }
}

trait Metrics {
    fn into_stats(self) -> Stats;

    fn derive(_stats: &mut Stats) {}

    fn to_stats_default(self) -> Stats
    where
        Self: Serialize + Sized,
    {
        let jv = serde_json::to_value(self).unwrap();
        let map: IndexMap<String, Value> = serde_json::from_value(jv).unwrap();
        map.into()
    }
}

#[cfg(test)]
mod tests {
    use indexmap::indexmap;
    use test_log::test;

    use super::*;

    #[test]
    fn test_value_metrics() {
        // let a: std::rc::Rc<dyn Stats>;

        #[derive(Debug, Serialize, Deserialize)]
        struct MyStruct {
            one: Value,
            two: Value,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct MyStruct2 {
            a: i64,
            b: String,
            c: f64,
            d: f64,
            // e: [i32; 64],
        }

        impl Metrics for MyStruct2 {
            fn into_stats(self) -> Stats {
                Self::to_stats_default(self)
            }
        }

        struct MyStruct3(IndexMap<String, Value>);

        impl Metrics for MyStruct3 {
            fn into_stats(self) -> Stats {
                let map: IndexMap<String, Value> =
                    self.0.into_iter().map(|(k, v)| (k.to_ascii_uppercase(), v)).collect();
                Stats { map }
            }

            fn derive(stats: &mut Stats) {
                stats["total.of.3A.and.3B"] = stats["3A"].clone() + stats["3B"].clone();
            }
        }

        let my_struct = MyStruct {
            one: Value::Int(1),
            two: Value::Text("two".to_string()),
        };
        let my_struct2 = MyStruct2 {
            a: 1,
            b: "two".to_string(),
            c: 3.5,
            d: 6.0,
            // e: [1; 64],
        };

        let _my_struct2b = MyStruct2 {
            a: 5,
            b: "three".to_string(),
            c: 4.5,
            d: 5.0,
            // e: [2; 64],
            // e: (4, 4, 5),
        };

        let map = indexmap::indexmap! { "one" => Value::Int(1), "two" => Value::Text("cat".to_string()) };
        let string_map = serde_json::to_string(&map).unwrap();
        let string_struct = serde_json::to_string(&my_struct).unwrap();

        println!("string_map = {string_map}\nstring_struct = {string_struct}");
        let json_value_map = serde_json::to_value(&map).unwrap();
        println!("json_value_map = {json_value_map:?}");

        let json_value_struct = serde_json::to_value(&my_struct).unwrap();
        println!("json_value_struct = {json_value_struct:?}");

        let json_value_struct2 = serde_json::to_value(&my_struct2).unwrap();
        println!("json_value_struct2 = {json_value_struct2:?}");

        let map_from: IndexMap<String, Value> = serde_json::from_value(json_value_struct).unwrap();
        println!("map_from = {map_from:?}");

        let map_from2: IndexMap<String, Value> = serde_json::from_value(json_value_struct2).unwrap();
        println!("map_from2 = {map_from2:?}");

        let my_struct4 = my_struct2;
        // my_struct4.aggregate(&my_struct2b);
        println!("agg = {my_struct4:?}");
        println!("agg = \n{}", my_struct4.into_stats());

        let my_struct3a = MyStruct3(indexmap! { "3a".to_string() => 13.into(), "3b".to_string() => 14.into()});

        let my_struct3b = MyStruct3(indexmap! { "3a".to_string() => 23.into(), "3b".to_string() => 24.into()});

        let mut map3a = my_struct3a.into_stats();
        let map3b = my_struct3b.into_stats();
        map3a.aggregate(map3b);
        println!("agg3 (Display) = \n{}", map3a);
        println!("agg3 (Pgn) = \n{}", Stats::to_pgn([("map3a", &map3a)]));
    }

    #[test]
    fn test_value_core() {
        assert_eq!(Value::Int(1) + Value::Int(2), Value::Int(3));
        assert_eq!(Value::Int(2000).to_string(), "2,000");
        assert_eq!(Value::Int(3000), 3000_i64.into());
        assert_eq!(Value::Text("cat".to_string()), "cat".into());
        assert_eq!(Value::Float(2.3).to_string(), "2.3");
        assert_eq!(Value::Float(2.3), Value::parse("2.3"));
        assert_eq!(Value::Int(2), Value::parse("2"));
        assert_eq!(Value::Int(2000), Value::parse("2,000"));
        assert_eq!(Value::Float(2000.12), Value::parse("2,000.12"));
        assert_ne!(Value::Float(2000.12), Value::parse(",2,000.12"));
        assert_eq!(serde_json::to_string(&Value::Int(2)).unwrap(), "2");
        assert_eq!(serde_json::to_string(&Value::Null).unwrap(), "null");
        assert_eq!(Value::parse_pgn("3"), Value::Int(3));
        assert_eq!(Value::parse_pgn("3.3"), Value::Float(3.3));
    }
    #[test]
    fn test_value_aggregate() {
        let mut i1 = Value::Int(1);
        let i2 = Value::Int(2);

        let mut f1 = Value::Float(1.0);
        let f2 = Value::Float(2.0);

        let mut n1 = Value::Null;
        let n2 = Value::Null;

        let mut t1 = Value::Text("cat".to_string());
        let t2 = Value::Text("dog".to_string());

        i1.aggregate(i2.clone());
        assert_eq!(i1, Value::Int(3));

        i1.aggregate(n2.clone());
        assert_eq!(i1, Value::Int(3));

        i1.aggregate(f2.clone());
        assert_eq!(i1, Value::Float(5.0));

        i1.aggregate(t2.clone());
        assert_eq!(i1, Value::Text("dog".to_string()));

        f1.aggregate(f2);
        assert_eq!(f1, Value::Float(3.0));

        f1.aggregate(i2.clone());
        assert_eq!(f1, Value::Float(5.0));

        n1.aggregate(i2);
        assert_eq!(n1, Value::Int(2));

        n1.aggregate(n2.clone());
        assert_eq!(n1, Value::Int(2));

        t1.aggregate(n2);
        assert_eq!(t1, Value::Text("cat".to_string()));

        n1.aggregate(t2);
        assert_eq!(n1, Value::Text("dog".to_string()));

        let mut s1 = Stats::new();
        s1["r1"] = 101.into();
        s1["r2"] = 102.into();

        let mut s2 = Stats::new();
        s2["r1"] = 103.into();
        s2["r3"] = 104.into();
        s1.aggregate(s2);
        info!("s1 = {s1}");
        assert_eq!(s1["r1"], 204.into());
        assert_eq!(s1["r2"], 102.into());
        assert_eq!(s1["r3"], 104.into());
        assert_eq!(s1.len(), 3);
    }

    #[test]
    fn test_stats_core() {
        let mut stats = Stats::new();
        stats["name"] = "Bob".into();
        stats["age"] = 23.into();
        stats["weight"] = (78.5).into();
        let json = serde_json::to_string(&stats).unwrap();
        assert_eq!(json, r#"{"name":"Bob","age":23,"weight":78.5}"#);
        assert_eq!(stats.len(), 3);
        assert_eq!(stats.keys().collect_vec(), ["name", "age", "weight"]);
        assert_eq!(stats.iter().contains(&("age", &Value::Int(23))), true);
    }

    #[test]
    fn test_value_parse() {
        assert_eq!(Stats::parse_pgn("|White | Black\n\n").unwrap(), Dual::default());
        assert_eq!(
            Stats::parse_pgn("|n\n").unwrap_err().to_string(),
            "Expected '|White|Black' but got '|n' instead"
        );
        let text = "|White|Black\nkey1|10|2.2\nkey2|3|-\n\n";
        let dual = Stats::parse_pgn(text).unwrap();
        let Dual { white, black } = dual.clone();
        assert_eq!(white.len(), 2);
        assert_eq!(white["key1"], Value::Int(10));
        assert_eq!(white["key2"], Value::Int(3));
        assert_eq!(black["key1"], (2.2).into());
        assert_eq!(black["key2"], Value::Null);
        assert_eq!(
            Stats::to_pgn([("White", &dual.white), ("Black", &dual.black)]).replace(' ', ""),
            text
        ); // remove padding
    }
}
