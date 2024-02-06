use crate::infra::utils::read_file;

use super::resources::read_resource_file;
use anyhow::Context;
use config;
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Toml,
    Yaml,
}

impl Format {
    fn from_filename(filename: &str) -> anyhow::Result<Format> {
        if filename.ends_with(".toml") || filename.ends_with(".TOML") {
            Ok(Format::Toml)
        } else if filename.ends_with(".yaml")
            || filename.ends_with(".YAML")
            || filename.ends_with(".yml")
            || filename.ends_with(".YML")
        {
            Ok(Format::Yaml)
        } else {
            anyhow::bail!(
                "could not determine file format from file extension for filename '{filename}'"
            );
        }
    }
}

#[derive(Debug, Clone)]
enum Source {
    Text(&'static str, Format),
    OwnedText(String, Format),
    File(PathBuf),
    ResourceFile(PathBuf),
    EnvVarsProps { prefix: String },
    Props(HashMap<String, String>),
    Substitutions(HashMap<String, String>),
}

#[derive(Default, Debug, Clone)]
pub struct Config {
    src: Vec<Source>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn file(mut self, path: impl AsRef<Path>) -> Self {
        let pathbuf = path.as_ref().to_owned();
        self.src.push(Source::File(pathbuf));
        self
    }

    pub fn resource(mut self, path: impl AsRef<Path>) -> Self {
        let pathbuf = path.as_ref().to_owned();
        self.src.push(Source::ResourceFile(pathbuf));
        self
    }

    pub fn static_text(mut self, content: &'static str, format: Format) -> Self {
        self.src.push(Source::Text(content, format));
        self
    }

    pub fn owned_text(mut self, content: String, format: Format) -> Self {
        self.src.push(Source::OwnedText(content, format));
        self
    }

    /// if a property has the key of the filename or resource filename,
    /// then that file/resource is amended to an alternative File (not resource)
    /// eg
    ///   props["config.toml"] = "/path/to/alternative/config.toml"
    ///   props["config2.toml"] = "/path/to/alternative/config2.toml"
    /// .
    pub fn allow_override_files(self) -> Self {
        let mut res = Self::default();
        for input in self.src.into_iter() {
            if let Source::Props(mut props) = input {
                for output in res.src.iter_mut() {
                    if let Source::File(path) | Source::ResourceFile(path) = output {
                        if let Some(file) = props.remove(path.to_string_lossy().as_ref()) {
                            *output = Source::File(file.into());
                        }
                    }
                }
                res = res.props(props);
            } else {
                res.src.push(input);
            }
        }
        res
    }

    pub fn substitutions(mut self, subs: HashMap<String, String>) -> Self {
        self.src.push(Source::Substitutions(subs));
        self
    }

    /// Call with env! for a compilation unit env var (env! expanded at call site crate)
    pub fn substitute(self, from: &str, to: &str) -> Self {
        let subs: HashMap<String, String> = [(from.to_string(), to.to_string())].into();
        self.substitutions(subs)
    }

    pub fn props(mut self, props: HashMap<String, String>) -> Self {
        self.src.push(Source::Props(props));
        self
    }

    pub fn property(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let props = HashMap::from([(key.into(), value.into())]);
        self.props(props)
    }

    pub fn env_var_props(mut self, prefix: &str) -> Self {
        let prefix = prefix.to_string();
        self.src.push(Source::EnvVarsProps { prefix });
        self
    }

    pub fn deserialize<'de, T>(&self) -> anyhow::Result<T>
    where
        T: Default,
        T: Deserialize<'de>,
    {
        // let key = type_suffix(std::any::type_name::<T>()).to_lowercase();
        self.deserialize_node("")
    }

    pub fn deserialize_node<'de, T>(&self, key: &str) -> anyhow::Result<T>
    where
        T: Default + Deserialize<'de>,
    {
        let config = self.get_config()?;
        let res = if key.is_empty() {
            config.try_deserialize()
        } else {
            config.get(key)
        };

        match res {
            Ok(t) => Ok(t),
            Err(e) => {
                println!(
                    "Failed with ConfigError {e:?} during deserialize {c:#?}\n",
                    c = self.get_config()?
                );
                // let mut s = "".to_string();
                // Self::write_error(&e, &mut s).unwrap();

                // error!(target:"config", "error {s} during deserialize");
                let type_name = type_suffix(std::any::type_name::<T>());
                Err(anyhow::anyhow!(e))
                    .context(format!("during configuration for {type_name}"))
            }
        }
    }

    fn conv(fmt: &Format) -> config::FileFormat {
        match fmt {
            Format::Toml => config::FileFormat::Toml,
            Format::Yaml => config::FileFormat::Yaml,
        }
    }

    fn replace_params(&self, s: &str, subs: &HashMap<String, String>) -> String {
        let mut s = s.to_string();
        for (from, to) in subs {
            s = s.replace(from, to);
        }
        s
    }

    fn get_config(&self) -> anyhow::Result<config::Config> {
        let mut b = config::Config::builder();
        let mut all_props = HashMap::new();
        let mut substitutions = HashMap::new();
        let mut env_vars_prefix = None;
        for source in &self.src {
            match source {
                Source::Substitutions(map) => substitutions.extend(map.clone()),

                Source::Text(c, fmt) => {
                    b = b.add_source(config::File::from_str(c, Self::conv(fmt)));
                }
                Source::OwnedText(c, fmt) => {
                    b = b.add_source(config::File::from_str(c.as_str(), Self::conv(fmt)));
                }

                Source::File(pathbuf) => {
                    // let path = match &self.base_path {
                    //     Some(base) if pathbuf.is_relative() => base.join(pathbuf),
                    //     _ => pathbuf.to_owned(),
                    // };
                    let pathname = pathbuf
                        .as_path()
                        .to_str()
                        .ok_or(anyhow::anyhow!("filename must be unicode"))?;
                    let pathname = self.replace_params(pathname, &substitutions);
                    // let path = pathname.to_owned();
                    let lines = read_file(&pathname)?;
                    let s = lines.join("\n");
                    let s = self.replace_params(&s, &substitutions);
                    info!(target:"config", "add file {pathname}");
                    let fmt = Format::from_filename(&pathname)?;
                    b = b.add_source(config::File::from_str(&s, Self::conv(&fmt)));
                }
                Source::ResourceFile(pathbuf) => {
                    info!(target:"config", "add resource {s}", s = pathbuf.display());
                    let s = read_resource_file(pathbuf).to_string();
                    let s = self.replace_params(&s, &substitutions);
                    let pathname = pathbuf
                        .as_path()
                        .to_str()
                        .ok_or(anyhow::anyhow!("filename must be unicode"))?;
                    let fmt = Format::from_filename(pathname)?;
                    b = b.add_source(config::File::from_str(&s, Self::conv(&fmt)));
                }

                Source::Props(map) => all_props.extend(map.clone()),
                Source::EnvVarsProps { prefix } => env_vars_prefix = Some(prefix.to_string()),
            }
        }
        info!(target:"config", "add props with keys [{keys}]", keys = all_props.keys().join(","));

        // props with a "," in the value need to have key set as "with list"
        let mut cfg = config::Environment::default().try_parsing(true); // force bools and numbers to be parsed
        for key in all_props.keys().filter(|k| k.contains(',')) {
            cfg = cfg.list_separator(","); // arrays
            cfg = cfg.with_list_parse_key(key);
        }
        cfg = cfg.source(Some(all_props));
        b = b.add_source(cfg);

        if let Some(env_vars_prefix) = env_vars_prefix {
            b = b.add_source(
                config::Environment::default()
                    .try_parsing(true) // force bools and numbers to be parsed
                    .list_separator(",") // arrays
                    .separator("__")
                    .prefix(&env_vars_prefix)
                    .prefix_separator("__"),
            );
        }
        Ok(b.build()?)
    }

    fn write_error<W: fmt::Write>(e: &config::ConfigError, mut f: W) -> fmt::Result {
        use config::ConfigError as CE;
        match e {
            CE::Frozen => write!(f, "configuration is frozen"),
            CE::PathParse(kind) => write!(f, "PathParse {:?}", kind),
            CE::Message(s) => write!(f, "Message {}", s),
            CE::Foreign(cause) => write!(f, "Foreign {:?}", cause),
            CE::NotFound(key) => write!(f, "NotFound {key:?}"),

            CE::Type {
                origin,
                unexpected,
                expected,
                key,
            } => {
                write!(f, "Type {unexpected:?}, expected: {expected:?}")?;
                if let Some(key) = key {
                    write!(f, " for key `{}`", key)?;
                }
                if let Some(origin) = origin {
                    write!(f, " in {}", origin)?;
                }
                Ok(())
            }

            CE::FileParse { cause, uri } => {
                write!(f, "FileParse cause {cause:?}")?;
                if let Some(uri) = uri {
                    write!(f, " in {uri}")?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::infra::config::Format;
    use serde::Deserialize;
    use std::collections::HashMap;
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
        cfg = cfg.static_text(
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

            suv:
                model: Jaguar
                seats: 9
                engine: 
                    power: 4000
                    kind: Nuclear
                "###,
            Format::Yaml,
        );

        let cfg2 = cfg.static_text(
            r###"
            f1.engine:
                power: 500
                kind: Hybrid
            "###,
            Format::Yaml,
        );

        let cfg_p = cfg2.clone().props(HashMap::from([
            ("suv.seats".to_string(), "7".to_string()),
            ("cat3".to_string(), "cat2".to_string()),
        ]));

        let config = cfg_p.get_config();
        println!("get config = {config:#?}");

        let car: Car = cfg_p.deserialize_node("car").unwrap();
        println!("load car = {car:#?}");
        assert_eq!(car.engine.power, 1000);

        let ev: Car = cfg_p.deserialize_node("ev").unwrap();
        println!("load ev = {ev:#?}");
        assert_eq!(ev.engine.power, 700);

        let f1_eng: Engine = cfg_p.deserialize_node("f1.engine").unwrap();
        println!("load f1_eng = {f1_eng:#?}");
        assert_eq!(f1_eng.power, 500); // from cfg2

        let f1: Car = cfg_p.deserialize_node("f1").unwrap();
        println!("load f1 = {f1:#?}");
        assert_eq!(f1.engine.power, 500); // from cfg2

        let suv: Car = cfg2.deserialize_node("suv").unwrap();
        println!("load suv1 = {suv:#?}");
        assert_eq!(suv.seats, 9); // before props override

        let suv: Car = cfg_p.deserialize_node("suv").unwrap();
        println!("load suv2 = {suv:#?}");
        assert_eq!(suv.seats, 7); // from props
    }

    // #[test]
    // fn test_config_engine() {
    //     let mut cfg = Config::default();

    //     let toml = RESOURCE_DIR
    //         .get_file("config.toml")
    //         .unwrap()
    //         .contents_utf8()
    //         .unwrap();
    //     cfg = cfg.include_text(toml, Format::Toml);
    //     let eng: ThreadedSearch = cfg.deserialize_node("").unwrap();
    //     println!(
    //         "eng.queen = {:#?}",
    //         eng.algo.eval.weights_raw.wts[Feature::MaterialQueen.index()]
    //     );
    // }
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
