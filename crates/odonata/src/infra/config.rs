use std::{
    collections::HashMap,
    io::{stdout, Write},
};

use config;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum Format {
    Toml,
    Yaml,
}

#[derive(Default, Debug, Clone)]
pub struct Config {
    filenames: Vec<String>,
    strings:   Vec<(String, Format)>,
    props:     HashMap<String, String>,
}

fn type_suffix(type_name: &str) -> &str {
    if let Some(i) = type_name.rfind("::") {
        &type_name[i + 2..]
    } else {
        type_name
    }
}


/// ODONATA__TUNER__SEARCH_DEPTH=98 
/// -Dtuner.search_depth=99
impl Config {
    pub fn include_file(mut self, filename: &str) -> Self {
        self.filenames.push(filename.to_string());
        self
    }

    pub fn include_text(mut self, content: &str, format: Format) -> Self {
        self.strings.push((content.to_string(), format));
        self
    }

    pub fn include_settings(mut self, props: HashMap<String, String>) -> Self {
        self.props = props;
        self
    }

    fn get_config(&self) -> config::Config {
        let mut builder = config::Config::builder();
        for (s, format) in &self.strings {
            let fmt = match format {
                Format::Toml => config::FileFormat::Toml,
                Format::Yaml => config::FileFormat::Yaml,
            };
            builder = builder.add_source(config::File::from_str(s, fmt));
        }
        for f in &self.filenames {
            builder = builder.add_source(config::File::with_name(f));
        }
        // for (k, v) in &self.props {
        //     // builder = builder.set_override(k, v.as_ref()).unwrap();
        // }
        builder = builder.add_source(
            config::Environment::default()
                .try_parsing(true)  // force bools and numbers to be parsed
                .source(Some(self.props.clone())),
        );

        builder = builder.add_source(
            config::Environment::default()
                .try_parsing(true)  // force bools and numbers to be parsed
                .separator("__")
                .prefix("ODONATA")
                .prefix_separator("__")
        );

        builder.build().expect("building config")
    }

    pub fn deserialize<'de, T: Default + Deserialize<'de>>(&self) -> anyhow::Result<T> {
        // let key = type_suffix(std::any::type_name::<T>()).to_lowercase();
        self.deserialize_node("")
    }

    pub fn deserialize_node<'de, T: Default + Deserialize<'de>>(
        &self,
        key: &str,
    ) -> anyhow::Result<T> {
        let config = self.get_config();
        let res = if key.is_empty() {
            config.try_deserialize()
        } else {
            config.get(key)
        };

        match res {
            Ok(t) => Ok(t),
            Err(e) => {
                println!("Failed!\n");
                Self::write_error(&e, stdout()).unwrap();
                stdout().flush().unwrap();
                // println!("{config:#?}");
                Err(e.into())
            }
        }
    }

    fn write_error<W: Write>(e: &config::ConfigError, mut f: W) -> std::io::Result<()> {
        match e {
            config::ConfigError::Frozen => write!(f, "configuration is frozen"),

            config::ConfigError::PathParse(ref kind) => write!(f, "PathParse {:?}", kind),

            config::ConfigError::Message(ref s) => write!(f, "Message {}", s),

            config::ConfigError::Foreign(ref cause) => write!(f, "Foreign {:?}", cause),

            config::ConfigError::NotFound(ref key) => {
                write!(f, "NotFound {key:?}")
            }

            config::ConfigError::Type {
                ref origin,
                ref unexpected,
                expected,
                ref key,
            } => {
                write!(f, "Type {unexpected:?}, expected: {expected:?}")?;

                if let Some(ref key) = *key {
                    write!(f, " for key `{}`", key)?;
                }

                if let Some(ref origin) = *origin {
                    write!(f, " in {}", origin)?;
                }

                Ok(())
            }

            config::ConfigError::FileParse { ref cause, ref uri } => {
                write!(f, "FileParse cause {cause:?}")?;

                if let Some(ref uri) = *uri {
                    write!(f, " in {uri}")?;
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::Config;
    use crate::{
        eval::Feature,
        infra::{config::Format, resources::RESOURCE_DIR},
        search::engine::ThreadedSearch,
    };
    use serde::Deserialize;
    use test_log::test;

    #[test]
    fn test_config_basics() {
        #[derive(Deserialize, Debug, Default)]
        #[serde(deny_unknown_fields)]
        struct Engine {
            power: u32,
            kind:  String,
        }

        #[derive(Deserialize, Debug, Default)]
        #[serde(deny_unknown_fields)]
        struct Car {
            model:  String,
            seats:  u32,
            engine: Engine,
        }

        let mut cfg = Config::default();
        cfg = cfg.include_text(
            r###"
            car:
                model: Ford
                seats: 6
                engine: 
                    power: 1000
                    kind: Diesel

            ev:
                model: Tesla
                seats: 4
                engine: 
                    power: 700
                    kind: Electric

            f1:
                model: Toyota
                seats: 5
                engine: 
                    power: 2000
                    kind: Petrol
            "###,
            Format::Yaml,
        );

        cfg = cfg.include_text(
            r###"
            f1.engine:
                power: 500
                kind: Hybrid
            "###,
            Format::Yaml,
        );

        let cfg = cfg.include_settings(HashMap::from([
            ("suv.seats".to_string(), "7".to_string()),
            ("cat3".to_string(), "cat2".to_string()),
        ]));

        let config = cfg.get_config();
        println!("get config = {config:#?}");

        let car: Car = cfg.deserialize_node("car").unwrap();
        println!("load car = {car:#?}");

        let ev: Car = cfg.deserialize_node("ev").unwrap();
        println!("load ev = {ev:#?}");

        let f1_eng: Engine = cfg.deserialize_node("f1.engine").unwrap();
        println!("load f1_eng = {f1_eng:#?}");

        let f1: Car = cfg.deserialize_node("f1").unwrap();
        println!("load f1 = {f1:#?}");
    }

    #[test]
    fn test_config_engine() {
        let mut cfg = Config::default();

        let toml = RESOURCE_DIR
            .get_file("config.toml")
            .unwrap()
            .contents_utf8()
            .unwrap();
        cfg = cfg.include_text(toml, Format::Toml);
        let eng: ThreadedSearch = cfg.deserialize_node("").unwrap();
        println!(
            "eng.queen = {:#?}",
            eng.algo.eval.weights_raw.wts[Feature::MaterialQueen.index()]
        );
    }
    //     info!("default {:?}", Config::default());

    //     let toml = RESOURCE_DIR
    //         .get_file("figment.toml")
    //         .unwrap()
    //         .contents_utf8()
    //         .unwrap();

    //     let toml = Toml::string(toml);
    //     let config: Config = Figment::new().merge(toml).extract().unwrap();

    //     info!("file {:?}", config);
    // }
}
