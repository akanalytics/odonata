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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(Version::VERSION.is_empty(), false);
        assert_eq!(Version::AUTHORS.is_empty(), false);
        assert_eq!(Version::NAME, "odonata");
        assert_eq!(Version::HOMEPAGE.is_empty(), true);
        println!("{}", Version::AUTHORS);
        println!("{}", Version::IMAGE);
        println!("{}", Version::VERSION);
        println!("{}", Version::NAME);
        println!("{}", Version::HOMEPAGE);
    }
}
