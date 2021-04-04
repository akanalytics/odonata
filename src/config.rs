use std::collections::HashMap;
use std::fmt;

pub trait Configurable {
    fn define(&self, config: &mut Config);
    fn configure(&mut self, config: &Config);
}

#[derive(Clone, Debug)]
pub struct Config {
    settings: HashMap<String, String>,
    insertion_order: Vec<String>,
}


impl Config {
    pub fn new() -> Config {
        Self::default()
    }

    pub fn set(&mut self, k: &str, v: &str) -> Config {
        if self.settings.insert(k.to_string(), v.to_string()).is_none() {
            self.insertion_order.push(k.to_string());
        }
        self.clone()
    }

    pub fn bool(&self, name: &str) -> Option<bool> {
        if let Some(v) = self.settings.get(name) {
            return v.parse::<bool>().ok();
        }
        None
    }

    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item=(&String,&String)>  + 'a> {
        Box::new(self.insertion_order.iter().map( move |k| (k, &self.settings[k])))
    }

    pub fn string(&self, name: &str) -> Option<String> {
        self.settings.get(name).cloned()
    }

    pub fn int(&self, name: &str) -> Option<i64> {
        if let Some(v) = self.settings.get(name) {
            return v.parse::<i64>().ok();
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
        Config { settings: HashMap::new(), insertion_order: Vec::new() }
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
    impl Configurable for TestStruct {
        fn define(&self, c: &mut Config) {
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
    }

    #[test]
    fn test_config() {
        let c1 = Config::default();
        println!("c1\n{}", c1);

        let mut cs2 = Config::new();
        let mut ts = TestStruct { integer: 0, string: "cat".to_string() };
        ts.define(&mut cs2);
        println!("cs2\n{}", cs2);

        // check the config iterators in insertion order
        let vec: Vec<(&String,&String)> = cs2.iter().collect();
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
