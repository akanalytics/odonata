use crate::infra::resources::RESOURCE_DIR;
use figment::providers::Env;
use figment::providers::{Format, Toml};
use figment::{Error, Figment, Metadata, Profile, Provider};
use serde::{Deserialize, Serialize};
use static_init::dynamic;

// The library's required configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    test_string: String,
}

// The default configuration.
impl Default for Config {
    fn default() -> Self {
        Config {
            test_string: String::from("hello world"),
        }
    }
}

use figment::value::{Dict, Map};

// Make `Config` a provider itself for composability.
impl Provider for Config {
    fn metadata(&self) -> Metadata {
        Metadata::named("Library Config")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        figment::providers::Serialized::defaults(Config::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        // Optionally, a profile that's selected by default.
        None
    }
}




impl Config {
    // Allow the configuration to be extracted from any `Provider`.
    fn from<T: Provider>(provider: T) -> Result<Config, Error> {
        Figment::from(provider).extract()
    }

    // Provide a default provider, a `Figment`.
    fn figment() -> Figment {
        // In reality, whatever the library desires.
        Figment::from(Config::default()).merge(Env::prefixed("APP_"))
    }
}

mod tests {
    use super::*;
    use test_env_log::test;

    // #[test]
    // fn parse_test() {
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
