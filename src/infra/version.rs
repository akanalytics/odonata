use super::utils::ToStringOr;

pub struct Version {}

impl Version {
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    pub const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
    pub const NAME: &'static str = env!("CARGO_PKG_NAME");
    pub const HOMEPAGE: &'static str = env!("CARGO_PKG_HOMEPAGE");
    pub const GIT_COMMIT_MSG: &'static str = env!("GIT_COMMIT_MSG");
    pub const SMALL_IMAGE: &'static str = r##"
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
        use std::io::Write;
        writeln!(std::io::stdout(), "{}", Version::compiler_splash()).unwrap();
    }

    #[test]
    fn test_version() {
        assert_eq!(Version::VERSION.is_empty(), false);
        assert_eq!(Version::AUTHORS.is_empty(), false);
        assert_eq!(Version::NAME, "odonata");
        assert_eq!(Version::HOMEPAGE.is_empty(), false);
        println!("authors      : {}", Version::AUTHORS);
        println!("version      : {}", Version::VERSION);
        println!("name         : {}", Version::NAME);
        println!("homepage     : {}", Version::HOMEPAGE);
        println!("{}", Version::compiler_splash());
    }
}
