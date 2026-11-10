//! `openui doctor`: check the tools needed to build Skia from source and
//! run apps, and say how to get the missing ones.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::Result;

/// Oldest Rust the framework supports (the workspace `rust-version`).
const MIN_RUST: (u32, u32) = (1, 85);

struct Check {
    name: &'static str,
    result: std::result::Result<String, String>,
}

pub fn run() -> Result<()> {
    let checks = checks();
    let mut missing = 0;
    for check in &checks {
        match &check.result {
            Ok(found) => println!("  ok       {:<10} {found}", check.name),
            Err(fix) => {
                missing += 1;
                println!("  missing  {:<10} {fix}", check.name);
            }
        }
    }
    if missing == 0 {
        println!("\nEverything needed to build Open UI apps was found.");
        Ok(())
    } else {
        Err(format!("{missing} required tool(s) missing"))
    }
}

fn checks() -> Vec<Check> {
    let mut checks = vec![
        Check {
            name: "rustc",
            result: rustc(),
        },
        Check {
            name: "git",
            result: version_of("git", &["--version"], "install Git"),
        },
        Check {
            name: "python",
            result: version_of("python3", &["--version"], "")
                .or_else(|_| version_of("python", &["--version"], ""))
                .map_err(|_| "install Python 3; Skia's build scripts need it".into()),
        },
        Check {
            name: "ninja",
            result: version_of(
                "ninja",
                &["--version"],
                "install Ninja; Skia builds with it",
            ),
        },
        Check {
            name: "clang",
            result: version_of(
                "clang",
                &["--version"],
                "install LLVM/clang; Skia and its Rust bindings need it",
            ),
        },
        Check {
            name: "libclang",
            result: libclang(),
        },
    ];
    if cfg!(windows) {
        checks.push(Check {
            name: "msvc",
            result: msvc(),
        });
    }
    checks
}

/// The first line a tool prints for `args`, or `fix` if it cannot run.
fn version_of(program: &str, args: &[&str], fix: &str) -> std::result::Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| fix.to_owned())?;
    if !output.status.success() {
        return Err(fix.to_owned());
    }
    let text = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    Ok(String::from_utf8_lossy(&text)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned())
}

fn rustc() -> std::result::Result<String, String> {
    let version = version_of(
        "rustc",
        &["--version"],
        "install Rust from https://rustup.rs",
    )?;
    match parse_rust_version(&version) {
        Some(found) if found >= MIN_RUST => Ok(version),
        Some(_) => Err(format!(
            "{version} is older than {}.{}; run `rustup update`",
            MIN_RUST.0, MIN_RUST.1
        )),
        None => Err(format!("cannot read the version in `{version}`")),
    }
}

/// `(major, minor)` from `rustc --version` output such as
/// `rustc 1.94.1 (e408947bf 2026-03-25)`.
fn parse_rust_version(text: &str) -> Option<(u32, u32)> {
    let version = text.split_whitespace().nth(1)?;
    let mut parts = version.split(['.', '-']);
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

/// rust-skia's bindgen loads libclang from `LIBCLANG_PATH` or a default
/// location.
fn libclang() -> std::result::Result<String, String> {
    let file = if cfg!(windows) {
        "libclang.dll"
    } else {
        "libclang.so"
    };
    let mut candidates: Vec<PathBuf> = std::env::var_os("LIBCLANG_PATH")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    if cfg!(windows) {
        candidates.push(PathBuf::from(r"C:\Program Files\LLVM\bin"));
    } else {
        candidates.extend(
            (14..=22)
                .rev()
                .map(|version| PathBuf::from(format!("/usr/lib/llvm-{version}/lib"))),
        );
        candidates.push(PathBuf::from("/usr/lib64"));
        candidates.push(PathBuf::from("/usr/lib"));
    }
    candidates
        .iter()
        .find(|directory| has_libclang(directory, file))
        .map(|directory| directory.display().to_string())
        .ok_or_else(|| {
            format!("{file} not found; install libclang and set LIBCLANG_PATH to its directory")
        })
}

fn has_libclang(directory: &Path, file: &str) -> bool {
    directory.join(file).is_file()
        || std::fs::read_dir(directory).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&format!("{file}."))
            })
        })
}

/// The MSVC C++ build tools, found through Visual Studio's installer.
fn msvc() -> std::result::Result<String, String> {
    let fix = "install the Visual Studio C++ build tools (Desktop development with C++)";
    let vswhere =
        Path::new(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    let output = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ])
        .output()
        .map_err(|_| fix.to_owned())?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() && !path.is_empty() {
        Ok(path)
    } else {
        Err(fix.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_versions_are_read_from_rustc_output() {
        assert_eq!(
            parse_rust_version("rustc 1.94.1 (e408947bf 2026-03-25)"),
            Some((1, 94))
        );
        assert_eq!(parse_rust_version("rustc 1.85.0-nightly"), Some((1, 85)));
        assert_eq!(parse_rust_version("rustc"), None);
        assert!((1, 84) < MIN_RUST && (1, 85) >= MIN_RUST);
    }

    #[test]
    fn the_running_toolchain_passes_the_rust_check() {
        assert!(rustc().is_ok());
    }
}
