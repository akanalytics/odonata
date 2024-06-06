use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::Context as _;
use include_dir::{include_dir, Dir};

use crate::infra::version::Version;

pub static RESOURCE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/resources");

pub fn read_resource_binary_file(path: impl AsRef<Path>) -> &'static [u8] {
    RESOURCE_DIR
        .get_file(&path)
        .unwrap_or_else(|| panic!("unable to load resource {p}", p = path.as_ref().display()))
        .contents()
}

pub fn read_resource_or_file_text(path: impl AsRef<Path>) -> anyhow::Result<Cow<'static, str>> {
    let cow = match RESOURCE_DIR.get_file(&path) {
        Some(file) => {
            let s = file
                .contents_utf8()
                .context(format!("{} not a text file", path.as_ref().display()))?;
            Cow::Borrowed(s)
        }
        None => {
            let s = std::fs::read_to_string(workspace_dir().join(path))?;
            Cow::Owned(s)
        }
    };
    Ok(cow)
}

pub fn read_resource_file(path: impl AsRef<Path>) -> &'static str {
    RESOURCE_DIR
        .get_file(&path)
        .unwrap_or_else(|| panic!("unable to load resource {p}", p = path.as_ref().display()))
        .contents_utf8()
        .unwrap()
}


pub fn relative_path(path: impl AsRef<Path>) -> PathBuf {
    workspace_dir().join(path)
}


/// if run from cargo
pub fn workspace_dir() -> PathBuf {
    let Ok(cargo) = std::env::var("CARGO") else {
        trace!("CARGO env var not set");
        return PathBuf::new();
    };

    // for env in std::env::vars() {
    //     trace!("===> {} = {}", env.0, env.1);
    // }

    // for arg in std::env::args() {
    //     trace!("----> {arg}");
    // }

    // let current_exe = std::env::current_exe().unwrap_or_default();
    // if !current_exe
    //     .file_name()
    //     .unwrap_or_default()
    //     .to_string_lossy()
    //     .contains("CARGO")
    if Version::compiled_profile_name() != "debug" && Version::compiled_profile_name() != "release" {
        trace!("not debug/release profile - refusing to run cargo");
        return PathBuf::new();
    };
    static CARGO_PATH: OnceLock<PathBuf> = OnceLock::new();

    CARGO_PATH
        .get_or_init(|| {
            let output = std::process::Command::new(cargo)
                .arg("locate-project")
                .arg("--workspace")
                .arg("--message-format=plain")
                .output()
                .unwrap()
                .stdout;
            PathBuf::from(std::str::from_utf8(&output).unwrap().trim())
        })
        .parent()
        .unwrap()
        .to_path_buf()
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

    use include_dir::File;
    use itertools::Itertools;
    use test_log::test;

    use super::*;

    #[test]
    fn test_resources() {
        info!("{:?}", RESOURCE_DIR);
        let file: &'static File = RESOURCE_DIR.get_file("iq.epd").unwrap();
        let body: &'static str = file.contents_utf8().unwrap();
        assert!(body.contains("IQ test suite"));
        assert!(read_resource_file("iq.epd").contains("IQ test suite"));
    }

    #[test]
    fn test_resource_dir() {
        let files = RESOURCE_DIR.files().map(|f| f.path().display()).join(";");
        assert!(files.contains("bk.epd"));
    }

    #[test]
    fn test_workspace_dir() {
        let mut path = workspace_dir();
        println!("workspace dir: {path:?}");
        path.push("crates");
        path.push("odonata-base");
        path.push("resources");
        let paths = fs::read_dir(&path).unwrap_or_else(|_| panic!("{}", path.display()));

        assert!(
            paths.into_iter().any(|dir| dir.unwrap().file_name() == "bk.epd"),
            "{}",
            path.display()
        );
    }
}
