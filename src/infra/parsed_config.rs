use std::collections::HashMap;
use std::{fmt, fs};
use static_init::{dynamic};
use std::env;
use crate::eval::weight::Weight;
use crate::infra::resources::RESOURCE_DIR;
use std::path::PathBuf;



pub trait Component {
    fn settings(&self, config: &mut ParsedConfig);
    fn configure(&mut self, config: &ParsedConfig);
    fn new_game(&mut self);
    fn new_position(&mut self);
}





#[dynamic(lazy)]
static mut STATIC_INSTANCE: ParsedConfig = { 
    let c = ParsedConfig::parse(&RESOURCE_DIR.get_file("old-format.toml").unwrap().contents_utf8().unwrap(), "<internal>");
    if c.is_err() {
        warn!("Unable to open config.toml");
        return ParsedConfig::default()
    }
    c.unwrap()
};


#[derive(Clone, Debug)]
pub struct ParsedConfig {
    settings: HashMap<String, String>,
    insertion_order: Vec<String>,
}

impl ParsedConfig {

    pub fn new() -> ParsedConfig {
        Self::default()
    }

    pub fn global() -> ParsedConfig {
        let config = STATIC_INSTANCE.read();
        if !config.is_empty() {
            debug!("Using configuration\n{}", &*config);
        } else {
            debug!("No configuration file or overrides");
        }
        ParsedConfig::clone(&config)
    }

    pub fn set_global(config: ParsedConfig) {
        *STATIC_INSTANCE.write() = config;
    }

    pub fn read_from_file(filename: &str) -> Result<ParsedConfig, String> {
        let path = PathBuf::from(filename);
        let s = fs::read_to_string(path);
        let s = s.map_err(|_| 
                format!("Error reading config toml file {} in working dir {:?}", 
                    filename, 
                    env::current_dir().unwrap(), 
                ))?;
        //.or_else()?;
        ParsedConfig::parse(&s, filename)
        // let file = File::open(filename).map_err(|err| format!("Error opening config toml file {:?} in working dir {:?} {}", path, env::current_dir().unwrap(), err.to_string()))?;
        // let lines = io::BufReader::new(file).raedlines();
        // let results: Result<Vec<_>, _> = lines.collect();  // omg!
        // let successes = results.map_err(|e| e.to_string())?;
        // Self::parse_from_lines(&successes, filename)
    }


    pub fn parse(s: &str, filename: &str) -> Result<ParsedConfig, String> {

        let mut config = ParsedConfig::new();
        let mut count = 0;
        for (n, line) in s.lines().enumerate() {
            if n > 0 && n % 1000 == 0 {
                debug!("Read {} lines from {:?}", n, filename);
            }
            let s = line;
            let s = s.trim();
            if s.is_empty() || s.starts_with("#") {
                continue;
            }

            count += 1;

            if let Some(combo) = s.split_once("=") {
                let (key, value) = combo;
                config.set(&key, &value);
            } else {
                return Err(format!("Failed parsing line {} in file {}: '{}'", n, filename, s))
            }
        }
        debug!("Read {} items from {:?}", count, filename);
        Ok(config)
    }

    // fn read_from_env() -> ParsedConfig {
    //     let mut config = ParsedConfig::new();
    //     for arg in env::vars() {
    //         // format is odonata_key1_key2_key3 = value which we translate to key1.key2.key3=value
    //         if arg.0.to_lowercase().starts_with("odonata_") {
    //             if let Some(combo) = arg.0.split_once("_") {
    //                 let (_odonata, key) = combo;
    //                 let value = arg.1;
    //                 let key = key.replace("_", ".");
    //                 config.set(&key, &value);
    //             }
    //         }
    //     }
    //     config
    // }

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

    pub fn set(&mut self, k: &str, v: &str) -> ParsedConfig {
        let v = v.trim().trim_matches('"');
        let k = k.trim();
        debug!("config set [{}] = {}", k, v);
        if self.settings.insert(k.to_string(), v.to_string()).is_none() {
            self.insertion_order.push(k.to_string());
        }
        self.clone()
    }

    pub fn bool(&self, name: &str) -> Option<bool> {
        if let Some(v) = self.settings.get(&name.to_string()) {
            if let Ok(res) = v.parse::<bool>() {
                debug!("config fetch {} = [bool] {}", name, res);
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
            debug!("config fetch {} = [string] {}", name, res);
        }
        s.cloned()
    }

    pub fn combo(&self, name: &str) -> Option<String> {
        debug!("config fetch {} = [combo] {}", name, self.settings.get(name).unwrap_or(&String::new()));
        self.settings.get(name).cloned()
    }

    pub fn weight(&self, name: &str, default: &Weight) -> Weight {
        debug!("config search stem {} = [weight]", name);
        let (mut s, mut e) = (default.s(), default.e());
        if let Some(v) = self.settings.get(&(name.to_string() + ".s")) {
            debug!("config found {} = [weight]", name);
            if let Ok(res) = v.parse::<f32>() {
                debug!("config fetch {}.s = [weight f32] {}", name, res);
                s = res as f32;
            }
            if let Ok(res) = v.parse::<i32>() {
                debug!("config fetch {}.s = [weight i32] {}", name, res);
                s = res as f32;
            }
        }
        if let Some(v) = self.settings.get(&(name.to_string() + ".e")) {
            if let Ok(res) = v.parse::<f32>() {
                debug!("config fetch {}.e = [weight f32] {}", name, res);
                e = res as f32;
            }
            if let Ok(res) = v.parse::<i32>() {
                debug!("config fetch {}.e = [weight i32] {}", name, res);
                e = res as f32;
            }
        }
        Weight::from_f32(s, e)
    }

    pub fn int(&self, name: &str) -> Option<i64> {
        if let Some(v) = self.settings.get(name) {
            if let Ok(res) = v.parse::<i64>() {
                debug!("config fetch {} = [int] {}", name, res);
                return Some(res);
            }
        }
        None
    }
}

impl fmt::Display for ParsedConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (k, v) in self.iter() {
            writeln!(f, "{:<30} = {}", k, v)?
        }
        Ok(())
    }
}

impl Default for ParsedConfig {
    fn default() -> Self {
        ParsedConfig {
            settings: HashMap::new(),
            insertion_order: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::engine::*;
    use test_env_log::test;    

    #[derive(Clone, Debug)]
    struct TestStruct {
        integer: i64,
        string: String,
    }
    impl Component for TestStruct {
        fn settings(&self, c: &mut ParsedConfig) {
            c.set("engine.wheels", "type spin default=4 min=2 max=6");
            c.set("engine.color", "default=blue var=blue var=yellow var=red");
            c.set("engine.fast", "type check default=false");
        }

        fn configure(&mut self, config: &ParsedConfig) {
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
        let c1 = ParsedConfig::default();
        debug!("c1\n{}", c1);

        let mut cs2 = ParsedConfig::new();
        let mut ts = TestStruct {
            integer: 0,
            string: "cat".to_string(),
        };
        ts.settings(&mut cs2);
        debug!("cs2\n{}", cs2);

        // check the config iterators in insertion order
        let vec: Vec<(&String, &String)> = cs2.iter().collect();
        assert_eq!(vec[0].0, "engine.wheels");
        assert_eq!(vec[1].0, "engine.color");

        let mut c3 = ParsedConfig::new();
        c3.set("engine.wheels", "6");
        c3.set("engine.color", "red");
        debug!("c3\n{}", c3);
        ts.configure(&c3);
        assert_eq!(ts.integer, 6);
        assert_eq!(ts.string, "red");
    }
}
