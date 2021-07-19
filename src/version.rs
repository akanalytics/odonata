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
        
image by jgs"##;
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
