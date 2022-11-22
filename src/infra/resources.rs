use include_dir::{include_dir, Dir};
// use std::path::Path;

pub static RESOURCE_DIR: Dir<'static> = include_dir!("resources");

#[cfg(test)]
mod tests {
    use super::*;
    use include_dir::File;
    use itertools::Itertools;
    use test_log::test;

    #[test]
    fn test_resources() {
        info!("{:?}", RESOURCE_DIR);
        let file: &'static File = RESOURCE_DIR.get_file("config.toml").unwrap();
        let body: &'static str = file.contents_utf8().unwrap();
        assert!(body.contains("enabled"));
    }

    #[test]
    fn test_resource_dir() {
        let files = RESOURCE_DIR.files().map(|f| f.path().display()).join(";");
        assert!(files.contains("bk.epd"));
    }
}
