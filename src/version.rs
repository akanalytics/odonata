pub struct Version {}

impl Version {
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    pub const AUTHORS: &'static str = env!("CARGO_PKG_AUTHORS");
    pub const NAME: &'static str = env!("CARGO_PKG_NAME");
    pub const HOMEPAGE: &'static str = env!("CARGO_PKG_HOMEPAGE");
    pub const IMAGE: &'static str = r##"
                          .-.
                         ()I()
                    "==.__:-:__.=="
                   "==.__/~|~\__.=="
                   "==._(  Y  )_.=="
        .-'~~""~=--...,__\/|\/__,...--=~""~~'-.
       (               ..=\=/=..               )
        `'-.        ,.-"`;/=\ ;"-.,_        .-'`
            `~"-=-~` .-~` |=| `~-. `~-=-"~`
                 .-~`    /|=|\    `~-.
              .~`       / |=| \       `~.
          .-~`        .'  |=|  `.        `~-.
        (`     _,.-="`    |=|    `"=-.,_     `)
         `~"~"`           |=|           `"~"~`
                          |=|
                          |=|
                          |=|
                          /=\
                          \=/
                           ^
        
"##;

    pub const SMALL_IMAGE: &'static str = r##"
 ()
%=====
 ()
"##;
    
    pub const QUOTE: &'static str = "May you touch dragonflies and stars...";

    pub fn small_splash() -> String {
        let mut s = String::new();
        s += &format!("{} {}\n", Version::NAME, Version::VERSION);
        s += &format!("{}\n", Version::SMALL_IMAGE);
        s += &format!("{}\n", Version::QUOTE);
        s += &format!("\n");
        s += &format!("email        : {}\n", Version::AUTHORS);
        s += &format!("homepage     : {}\n", Version::HOMEPAGE);
        s += &format!("compiled for : {} / {} / optimization level {}\n", built_info::TARGET, built_info::PROFILE, built_info::OPT_LEVEL);
        s += &format!("compiled at  : {}\n", built_info::BUILT_TIME_UTC);
        s += &format!("compiler     : {}\n", built_info::RUSTC_VERSION);
        s += &format!("features     : {}\n", built_info::FEATURES_STR);
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
    fn test_version() {
        assert_eq!(Version::VERSION.is_empty(), false);
        assert_eq!(Version::AUTHORS.is_empty(), false);
        assert_eq!(Version::NAME, "odonata");
        assert_eq!(Version::HOMEPAGE.is_empty(), false);
        println!("authors      : {}", Version::AUTHORS);
        println!("image        : {}", Version::IMAGE);
        println!("version      : {}", Version::VERSION);
        println!("name         : {}", Version::NAME);
        println!("homepage     : {}", Version::HOMEPAGE);
        println!("target       : {}", built_info::TARGET);
        println!("profile      : {}", built_info::PROFILE);
        println!("optimization : {}", built_info::OPT_LEVEL);
        println!("rustc        : {}", built_info::RUSTC_VERSION);
        println!("features     : {}", built_info::FEATURES_STR);
        println!("compiled at  : {}", built_info::BUILT_TIME_UTC);        
    }
}
