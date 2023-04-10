#[cfg(windows)]
pub const NPM: &str = "npm.cmd";

#[cfg(not(windows))]
pub const NPM: &str = "npm";

fn main() {
    // only run if index.js changed:
    println!("cargo:rerun-if-changed=../index.js");
    let webpack = std::process::Command::new(NPM)
        .arg("run")
        .arg("build")
        .status()
        .expect("Failed to run npm");
    assert!(webpack.success());
}