use std::collections::HashMap;
use std::fmt;
// use static_init::{dynamic};
use once_cell::sync::Lazy;
use crate::{info, logger::LogInit};
use std::env;
use crate::eval::weight::Weight;


pub trait Component {
    fn settings(&self, config: &mut Config);
    fn configure(&mut self, config: &Config);
    fn new_game(&mut self);
    fn new_position(&mut self);
}


// #[dynamic(lazy)]
static STATIC_INSTANCE: Lazy<Config> = Lazy::new( || Config::read_from_env());


#[derive(Clone, Debug)]
pub struct Config {
    settings: HashMap<String, String>,
    insertion_order: Vec<String>,
}

impl Config {
    pub fn new() -> Config {
        Self::default()
    }

    pub fn from_env() -> &'static Config {
        &STATIC_INSTANCE
    }

    fn read_from_env() -> Config {
        let mut config = Config::new();
        for arg in env::vars() {
            // format is odonata_key1_key2_key3 = value which we translate to key1.key2.key3=value
            if arg.0.to_lowercase().starts_with("odonata_") {
                if let Some(combo) = arg.0.split_once("_") {
                    let (_odonata, key) = combo;
                    let value = arg.1;
                    let key = key.replace("_", ".");
                    config.set(&key, &value);
                }
            }
        }
        if !config.is_empty() {
            warn!("Using configuration\n{}", config);
        } else {
            info!("No configuration overrides");
        }
        config
    }

    pub fn set_weight(&mut self, k: &str, w: &Weight) {
        let (k1, k2) = (k.to_string() + ".s", k.to_string() + ".e");
        let s = "type spin min -9999 max 9999 default ".to_string();
        if self.settings.insert(k1.to_owned(), s.clone() + w.s().to_string().as_str()).is_none() {
            self.insertion_order.push(k1);
        }
        if self.settings.insert(k2.to_owned(), s + w.e().to_string().as_str()).is_none() {
            self.insertion_order.push(k2);
        }
    }

    pub fn set(&mut self, k: &str, v: &str) -> Config {
        if self.settings.insert(k.to_string(), v.to_string()).is_none() {
            self.insertion_order.push(k.to_string());
        }
        self.clone()
    }

    pub fn bool(&self, name: &str) -> Option<bool> {
        if let Some(v) = self.settings.get(&name.to_string()) {
            if let Ok(res) = v.parse::<bool>() {
                info!("config {} = {}", name, res);
                return Some(res);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.settings.len() == 0
    }

    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (&String, &String)> + 'a> {
        Box::new(self.insertion_order.iter().map(move |k| (k, &self.settings[k])))
    }

    pub fn string(&self, name: &str) -> Option<String> {
        let s = self.settings.get(name);
        if let Some(res) = s {
            info!("config {} = {}", name, res);
        }
        s.cloned()
    }

    pub fn combo(&self, name: &str) -> Option<String> {
        self.settings.get(name).cloned()
    }

    pub fn weight(&self, name: &str, default: &Weight) -> Weight {
        let (mut s, mut e) = (default.s(), default.e());
        if let Some(v) = self.settings.get(&(name.to_string() + ".s")) {
            if let Ok(res) = v.parse::<i32>() {
                info!("config {}.s = {}", name, res);
            }
            s = v.parse::<i32>().unwrap_or(default.s());
        }
        if let Some(v) = self.settings.get(&(name.to_string() + ".e")) {
            if let Ok(res) = v.parse::<i32>() {
                info!("config {}.e = {}", name, res);
            }
            e = v.parse::<i32>().unwrap_or(default.e());
        }
        Weight::new(s, e)
    }

    pub fn int(&self, name: &str) -> Option<i64> {
        if let Some(v) = self.settings.get(name) {
            if let Ok(res) = v.parse::<i64>() {
               info!("config {} = {}", name, res);
                return Some(res);
            }
        }
        None
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (k, v) in self.iter() {
            writeln!(f, "{:<30} = {}", k, v)?
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            settings: HashMap::new(),
            insertion_order: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestStruct {
        integer: i64,
        string: String,
    }
    impl Component for TestStruct {
        fn settings(&self, c: &mut Config) {
            c.set("engine.wheels", "type spin default=4 min=2 max=6");
            c.set("engine.color", "default=blue var=blue var=yellow var=red");
            c.set("engine.fast", "type check default=false");
        }

        fn configure(&mut self, config: &Config) {
            if let Some(i) = config.int("engine.wheels") {
                self.integer = i;
            }
            if let Some(s) = config.string("engine.color") {
                self.string = s;
            }
        }

        fn new_game(&mut self) {}

        fn new_position(&mut self) {}
    }

    #[test]
    fn test_config() {
        let c1 = Config::default();
        println!("c1\n{}", c1);

        let mut cs2 = Config::new();
        let mut ts = TestStruct {
            integer: 0,
            string: "cat".to_string(),
        };
        ts.settings(&mut cs2);
        println!("cs2\n{}", cs2);

        // check the config iterators in insertion order
        let vec: Vec<(&String, &String)> = cs2.iter().collect();
        assert_eq!(vec[0].0, "engine.wheels");
        assert_eq!(vec[1].0, "engine.color");

        let mut c3 = Config::new();
        c3.set("engine.wheels", "6");
        c3.set("engine.color", "red");
        println!("c3\n{}", c3);
        ts.configure(&c3);
        assert_eq!(ts.integer, 6);
        assert_eq!(ts.string, "red");
    }
}
