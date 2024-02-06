use odonata_base::infra::metric::Metrics;
use odonata_base::infra::utils::ToStringOr;


pub struct Version {}

impl Version {
    pub const VERSION_NUMBER: &'static str = env!("CARGO_PKG_VERSION");
    pub const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
    const PACKAGE_NAME: &'static str = env!("CARGO_PKG_NAME");
    pub const HOMEPAGE: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const GIT_COMMIT_MSG: &'static str = env!("GIT_COMMIT_MSG");
    const SMALL_IMAGE: &'static str = r##"
 ()
%=====
 ()
"##;

    pub const QUOTE: &'static str = "May you touch dragonflies and stars...";

    #[allow(clippy::useless_format)]
    pub fn small_splash() -> String {
        let mut s = String::new();
        s += &format!("{}\n", Version::SMALL_IMAGE);
        s += &format!("{}\n", Version::QUOTE);
        s += &format!("\n");
        s += &format!("email        : {}\n", Version::AUTHORS);
        s += &format!("homepage     : {}\n", Version::HOMEPAGE);
        s + &Self::compiler_splash()
    }

    pub fn prog_name() -> &'static str {
        "Odonata"
    }


    pub fn name_and_version() -> String {
        let mut lc_name = Self::prog_name().to_string();
        let camel_case_name = lc_name.remove(0).to_uppercase().to_string() + &lc_name;
        let debug_asserts = match cfg!(debug_assertions) {
            true => "D",
            false => "",
        };
        let metrics_enabled = match Metrics::metrics_enabled() {
            true => "M",
            false => "",
        };

        format!(
            "{camel_case_name} {ver}{d}{m}",
            ver = Self::VERSION_NUMBER,
            d = debug_asserts,
            m = metrics_enabled
        )
    }

    thread_local! { static METRICS_ENABLED: std::cell::Cell<bool>  = cfg!(any(feature = "metrics", debug_assertions)).into(); }


    pub fn compiled_profile_name() -> &'static str {
        // https://stackoverflow.com/questions/73595435/
        // The profile name is always the 3rd last part of the path (with 1 based indexing).
        // e.g. /code/target/cli/build/my-build-info-9f91ba6f99d7a061/out
        std::env!("OUT_DIR")
            .split(std::path::MAIN_SEPARATOR) // compile time path separator
            .nth_back(3)
            .unwrap_or("unknown")
    }

    pub fn compiler_splash() -> String {
        let avx2 = false;
        let bmi2 = false;
        let popcnt = false;
        let lzcnt = false;

        // rustc --print  cfg -Ctarget-cpu=x86-64-v3
        //
        // [build]
        // rustflags = ["-C","target-cpu=x86-64-v3"]
        // avx2         : true
        // bmi2         : true
        // popcnt       : true
        // lzcnt        : true

        // rustflags = ["-C","target-cpu=generic"]
        // avx2         : false
        // bmi2         : false
        // popcnt       : false
        // lzcnt        : false

        #[cfg(target_feature = "avx2")]
        let avx2 = !avx2;

        #[cfg(target_feature = "bmi2")]
        let bmi2 = !bmi2;

        #[cfg(target_feature = "popcnt")]
        let popcnt = !popcnt;

        #[cfg(target_feature = "lzcnt")]
        let lzcnt = !lzcnt;
        let mut s = String::new();
        s += &format!(
            "compiled for : {} / {} / optimization level {}\n",
            built_info::TARGET,
            built_info::PROFILE,
            built_info::OPT_LEVEL
        );
        s += &format!("compiled at  : {}\n", built_info::BUILT_TIME_UTC);
        s += &format!(
            "git version  : {}\n",
            built_info::GIT_VERSION.to_string_or("")
        );
        s += &format!(
            "git branch   : {}\n",
            built_info::GIT_HEAD_REF.to_string_or("")
        );
        s += &format!(
            "git commit   : {}\n",
            built_info::GIT_COMMIT_HASH_SHORT.to_string_or("")
        );
        s += &format!("git message  : {}\n", Self::GIT_COMMIT_MSG);
        s += &format!(
            "uncommitted  : {}\n",
            built_info::GIT_DIRTY.to_string_or("")
        );
        s += &format!("compiler     : {}\n", built_info::RUSTC_VERSION);
        s += &format!("features     : {}\n", built_info::FEATURES_STR);
        s += &format!(
            "debug asserts: {}\n",
            if cfg!(debug_assertions) {
                "enabled"
            } else {
                "disabled"
            }
        );
        s += &format!(
            "metrics      : {}\n",
            if Metrics::metrics_enabled() {
                "enabled"
            } else {
                "disabled"
            }
        );
        s += &format!("cargo profile: {}\n", Version::compiled_profile_name());
        s += &format!("avx2         : {}\n", avx2);
        s += &format!("bmi2         : {}\n", bmi2);
        s += &format!("popcnt       : {}\n", popcnt);
        s += &format!("lzcnt        : {}\n", lzcnt);

        s
    }
}

// see https://docs.rs/built/0.5.1/built/
pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_aaa_version() {
        println!("{}", Metrics::metrics_enabled());
    }

    #[test]
    fn test_version() {
        assert_eq!(Version::VERSION_NUMBER.is_empty(), false);
        // assert_eq!(Version::AUTHORS.is_empty(), false);
        assert_eq!(Version::prog_name(), "Odonata");
        // assert_eq!(Version::HOMEPAGE.is_empty(), false);
        println!("authors      : {}", Version::AUTHORS);
        println!("version      : {}", Version::VERSION_NUMBER);
        println!("name         : {}", Version::prog_name());
        println!("homepage     : {}", Version::HOMEPAGE);
        println!("{}", Version::compiler_splash());
    }
}
