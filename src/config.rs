use std::fmt;




#[derive(Clone, Debug, PartialEq)]
pub enum Setting {
    Int { name: &'static str, min: i64, max: i64, default: i64, value: i64 },
    // Float { name: String, min: f32, max: f32, default: f32, value: f32 },
    //Boolean { name: String, default: bool, value: bool },
}

impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Setting::Int { name, min, max, default, value } => {
                if f.alternate() {
                    write!(f, "{:30} = {:<10} min = {:<10} max = {:<10} default = {:<10}", name, value, min, max, default)?;
                } else {
                    write!(f, "{}={}", name, value)?
                }
            }
        }
        Ok(())
    }
}





#[derive(Clone, Debug)]
pub struct Config {
    settings: Vec<Setting>,
}



impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}


impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for s in self.settings.iter() {
            if f.alternate() {
                writeln!(f, "{:#}", s)?;
            } else {
                writeln!(f, "{}", s)?;
            }
        }
        Ok(())
    }
}

fn int( name: &'static str, min: i64, max: i64, default: i64 ) -> Setting {
    Setting::Int { name: name.into(), min, max, default, value: default }
} 



impl Config {
    pub fn new()-> Self {
        const MAX:i64 = 100_000;
        Config { settings: vec![
            int("eval.pawn.value", 0, MAX, 100),
            int("eval.knight.value", 0, MAX, 325),
            int("eval.bishop.value", 0, MAX, 350),
            int("eval.rook.value", 0, MAX, 500),
            int("eval.queen.value", 0, MAX, 900),
        ]}
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config::new();
        println!("{}", config);
        println!("\n");
        println!("{:#}", config);
    }
}