fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");
    use std::process::Command;
    // taken from https://stackoverflow.com/questions/43753491/include-git-commit-hash-as-string-into-rust-program
    // git show -s --format=%s
    let output = Command::new("git")
        .args(["show", "-s", "--format=%s"])
        .output()
        .unwrap();
    let git_commit_msg = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_COMMIT_MSG={}", git_commit_msg);
}
