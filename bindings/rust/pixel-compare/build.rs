//! Bind the offline comparison executable to the source used for its build.
use std::{env, ffi::OsString, path::PathBuf, process::Command};

/// Interpreters tried in order. `OPENUI_PYTHON` overrides the search; Linux
/// finds `python3` first, while Windows usually provides `python` or `py -3`.
fn python_candidates() -> Vec<(OsString, Vec<&'static str>)> {
    if let Some(python) = env::var_os("OPENUI_PYTHON") {
        return vec![(python, Vec::new())];
    }
    vec![
        ("python3".into(), Vec::new()),
        ("python".into(), Vec::new()),
        ("py".into(), vec!["-3"]),
    ]
}

/// Accepts only a working Python 3.11 or newer, which rejects launcher stubs
/// such as the Windows Store `python3` alias.
fn usable(program: &OsString, prefix: &[&str]) -> bool {
    Command::new(program)
        .args(prefix)
        .args(["-c", "import sys; sys.exit(sys.version_info < (3, 11))"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn main() {
    println!("cargo:rerun-if-env-changed=OPENUI_PYTHON");
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // Match the Linux main-thread stack; the MSVC default is 1 MiB.
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
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
    let (python, prefix) = python_candidates()
        .into_iter()
        .find(|(program, prefix)| usable(program, prefix))
        .expect("the offline pixel comparison tool requires Python 3.11+ (set OPENUI_PYTHON)");
    let result = Command::new(&python)
        .args(&prefix)
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
