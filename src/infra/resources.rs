use include_dir::{include_dir, Dir};
// use std::path::Path;

pub const RESOURCE_DIR: Dir = include_dir!("resources");

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_resources() {
        info!("{:?}", RESOURCE_DIR);
        let file = RESOURCE_DIR.get_file("config.toml").unwrap();
        let body = file.contents_utf8().unwrap();
        assert!(body.contains("enabled"));
    }
}
