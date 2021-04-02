use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::error::Error;




pub trait Configurable {
    fn define(&self, config: &mut Config);
    fn configure(&mut self, config: &Config);
}



impl Config {
    pub fn new() -> Config {
        Self::default()
    }


    pub fn set(&mut self, k: &str, v: &str) -> Config {
        self.settings.insert(k.to_string(), v.to_string());
        self.clone()
    }


    pub fn bool(&self, name: &str) -> Option<bool> {
        if let Some(v) = self.settings.get(name) {
            return v.parse::<bool>().ok();
        }
        None
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


//     pub fn system() -> &'static Config {
//         static INSTANCE: OnceCell<Config> = OnceCell::new();
//         INSTANCE.get_or_init(|| Config::default())
//     }

//     pub fn get(&mut self, name: &str) -> Option<&mut Setting> {
//         self.settings.get_mut(name)
//     }

//     pub fn define_bool(&mut self, name: &str, default: bool) {
//         self.settings.insert(name.to_string(), Setting::Bool { default, value: default });
//     }

//     pub fn define_string(&mut self, name: &str, default: &str) {
//         self.settings.insert(
//             name.to_string(),
//             Setting::String { default: default.to_string(), value: default.to_string() },
//         );
//     }

//     pub fn define_int(&mut self, name: &str, default: i64, min: i64, max: i64) {
//         self.settings.insert(name.to_string(), Setting::Int { default, value: default, minmax: (min, max) });
//     }

// }


#[derive(Clone, Debug)]
pub struct Config {
    pub settings: HashMap<String, String>,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (k,v) in self.settings.iter() {
            writeln!(f,"{:<30} = {}", k,v )?
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config { settings: HashMap::new() }
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestStruct { integer: i64, string: String }
    
    impl Configurable for TestStruct {
        
        fn define(&self, c: &mut Config) {
            c.set("engine.wheels", "default=4 min=2 max=6");
            c.set("engine.color", "default=blue var=blue var=yellow var=red" );
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
        let mut ts = TestStruct { integer:0, string: "cat".to_string() };
        ts.define(&mut cs2);
        println!("cs2\n{}", cs2);

        let mut c3 = Config::new();
        c3.set("engine.wheels", "6");
        c3.set("engine.color", "red");
        println!("c3\n{}", c3);
        
        ts.configure(&c3);
        assert_eq!(ts.integer, 6);
        assert_eq!(ts.string, "red");

    }
}
