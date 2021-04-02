use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::error::Error;

// get
// set
// defaultmin
// max
// parse

#[derive(Clone, Debug, PartialEq)]
pub enum Setting {
    Bool { default: bool, value: bool },
    Int { default: i64, value: i64, minmax: (i64, i64) },
    String { default: String, value: String },
    Combo { default: usize, value: usize, choices: Vec<String> },
}

impl Setting {

    // fn set_boo(&self, value: &str) {
    //     Setting::Bool { &mut value, default:_ } => self.set_bool(s.parse()),
    //     Setting::Int { value, default:_, minmax:_ } => self.value = i64::parse(s),
    //     Setting::String { value, default:_ } => self.set( String::parse(s) ),
    //     Setting::Combo { value, default:_, choices:_ } => self.set( String::parse(s) ),
    // }


    pub fn parse(&mut self, s: &str) -> Result<(), String> {
        *self = match *self {
            Setting::Bool { value:_, default } => Setting::Bool{ value: s.parse::<bool>().unwrap(), default },
            Setting::Int { value:_, default, minmax } => Setting::Int{ value: s.parse::<i64>().unwrap(), default, minmax },
            Setting::String { value:_, default } => Setting::String{ value: s.to_string(), default },
            Setting::Combo { value:_, default, choices } => {
                if let Some(pos) = choices.iter().position(|v| v == s) {
                    Setting::Combo{ value:pos, default, choices }
                } else {
                    panic!("Could not find {} in {:?}", s, choices)
                }
            }
        };
        Ok(())
    }
}


impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Setting::Bool { value, default: _ } => write!(f, "{}", value)?,
            Setting::Int { value, default: _, minmax: _ } => write!(f, "{}", value)?,
            Setting::String { value, default: _ } => write!(f, "{}", value)?,
            _ => {}
        }
        Ok(())
    }
}

pub trait Configurable {
    fn define() -> Config;
    fn configure(&mut self, config: &Config);
}



#[derive(Clone, Debug)]
pub struct Config {
    pub settings: HashMap<String, Setting>,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (name, setting) in self.settings.iter() {
            if f.alternate() {
                match setting {
                    Setting::Int { default, value, minmax } => writeln!(
                        f,
                        "{:<30} = {:<10} default = {:<10} min = {:<10} max = {:<10} ",
                        name, value, default, minmax.0, minmax.1
                    )?,
                    Setting::Bool { default, value } => {
                        write!(f, "{:<30} = {:<10} default = {:<10}", name, value, default)?
                    }
                    Setting::String { default, value } => {
                        write!(f, "{:<30} = {:<10} default = {:<10}", name, value, default)?
                    }
                    Setting::Combo { default, value, choices  } => {
                        write!(f, "{:<30} = {:<10} default = {:<10}", name, value, choices[*default])?
                    }                   
                }
            } else {
                writeln!(f, "{}={}", name, setting)?;
            }
        }
        Ok(())
    }
}

// const fn int(min: i64, max: i64, default: i64) -> Setting {
//     Setting::Int { min, max, default, value: default }
// }

impl Default for Config {
    fn default() -> Self {
        let mut c = Config { settings: HashMap::new() };
        const MAX: i64 = 100_000;
        c.define_int("eval.pawn.value", 0, MAX, 100);
        c.define_int("eval.knight.value", 0, MAX, 325);
        c.define_int("eval.bishop.value", 0, MAX, 350);
        c.define_int("eval.rook.value", 0, MAX, 500);
        c.define_int("eval.queen.value", 0, MAX, 900);

        crate::comms::uci::Uci::define(&mut c);
        c
    }
}

impl Config {
    pub fn system() -> &'static Config {
        static INSTANCE: OnceCell<Config> = OnceCell::new();
        INSTANCE.get_or_init(|| Config::default())
    }

    pub fn get(&mut self, name: &str) -> Option<&mut Setting> {
        self.settings.get_mut(name)
    }

    pub fn define_bool(&mut self, name: &str, default: bool) {
        self.settings.insert(name.to_string(), Setting::Bool { default, value: default });
    }

    pub fn define_string(&mut self, name: &str, default: &str) {
        self.settings.insert(
            name.to_string(),
            Setting::String { default: default.to_string(), value: default.to_string() },
        );
    }

    pub fn define_int(&mut self, name: &str, default: i64, min: i64, max: i64) {
        self.settings.insert(name.to_string(), Setting::Int { default, value: default, minmax: (min, max) });
    }

    pub fn bool(&self, name: &str) -> bool {
        if let Setting::Bool { value, default: _ } = self.settings[name] {
            return value;
        }
        panic!("Setting {} is wrong type, expected bool", self.settings[name]);
    }

    pub fn string(&self, name: &str) -> &String {
        if let Setting::String { value, default: _ } = &self.settings[name] {
            return value;
        }
        panic!("Setting {} is wrong type, expected string", self.settings[name]);
    }

    pub fn int(&mut self, name: &str) -> i64 {
        if let Setting::Int { value, default: _, minmax: _ } = self.settings[name] {
            return value;
        }
        panic!("Setting {} is wrong type, expected bool", self.settings[name]);
    }
}

// eval.configure(&mut self, c: &Config) {
//     self.position = c.evaluation_position.value;
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config::default();
        println!("config\n{}", config);
        println!("config#\n{:#}", config);
    }
}
