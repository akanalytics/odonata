use include_dir::{include_dir, Dir};
use std::path::{Path, PathBuf};

pub static RESOURCE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/resources");

pub fn read_resource_file(path: impl AsRef<Path>) -> &'static str {
    RESOURCE_DIR
        .get_file(&path)
        .unwrap_or_else(|| panic!("unable to load resource {p}", p = path.as_ref().display()))
        .contents_utf8()
        .unwrap()
}

pub fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

pub fn interpolate_path(glob_pattern: impl AsRef<Path>) -> String {
    let glob_pattern = glob_pattern
        .as_ref()
        .as_os_str()
        .to_string_lossy()
        .replace("${CARGO_MANIFEST_DIR}", env!("CARGO_MANIFEST_DIR"));
    glob_pattern.replace("${ROOT}", &workspace_dir().as_os_str().to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::fs;

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
        assert!(read_resource_file("config.toml").contains("enabled"));
    }

    #[test]
    fn test_resource_dir() {
        let files = RESOURCE_DIR.files().map(|f| f.path().display()).join(";");
        assert!(files.contains("bk.epd"));
    }

    #[test]
    fn test_workspace_dir() {
        let mut path = workspace_dir();
        path.push("crates");
        path.push("odonata-base");
        path.push("resources");
        let paths = fs::read_dir(&path).unwrap_or_else(|_| panic!("{}", path.display()));

        assert!(
            paths
                .into_iter()
                .any(|dir| dir.unwrap().file_name() == "bk.epd"),
            "{}",
            path.display()
        );
    }
}
