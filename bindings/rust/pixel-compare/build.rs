//! Bind the offline comparison executable to the source used for its build.
use std::{env, path::PathBuf, process::Command};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let output =
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("renderer-build-identity.json");
    let result = Command::new("python3")
        .arg(root.join("tools/qualification/renderer_source_identity.py"))
        .arg("--cargo-output")
        .arg(output)
        .output()
        .expect("the offline pixel comparison tool requires Python 3 and Git");
    assert!(
        result.status.success(),
        "could not record renderer build source: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    print!("{}", String::from_utf8_lossy(&result.stdout));
}
