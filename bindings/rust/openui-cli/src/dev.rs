//! `openui dev`: build and run the app, then rebuild and restart it when its
//! sources change. A failed build leaves the running app alone.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::args::DevOptions;
use crate::cargo::{self, Build};
use crate::watch::Watcher;
use crate::Result;

/// The runtime exits through its normal path when this is set and stdin
/// closes (`openui-platform`'s `DEV_ENV`).
const DEV_ENV: &str = "OPENUI_DEV";
/// How long an app gets to exit after stdin closes before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(2);

pub fn run(options: &DevOptions) -> Result<()> {
    let app = cargo::app(options.manifest_path.as_deref())?;
    let bin = app.bin(options.bin.as_deref())?;
    let watcher = Watcher::new(watched_paths(app.directory(), &app.assets, &app.watch));
    let build = Build {
        app: &app,
        bin: &bin,
        release: options.release,
        locked: false,
    };

    let mut running: Option<Child> = None;
    loop {
        // Snapshot before building: a save during the build triggers one
        // more build as soon as this one ends.
        let sources = watcher.snapshot();
        let started = Instant::now();
        match cargo::build(&build) {
            Ok(executable) => {
                if let Some(child) = running.take() {
                    stop(child);
                }
                eprintln!(
                    "openui: built in {:.1}s, starting {}",
                    started.elapsed().as_secs_f64(),
                    executable.display()
                );
                running = Some(start(&executable, app.directory(), options)?);
            }
            Err(error) => {
                eprintln!("openui: {error}; waiting for changes");
            }
        }
        watcher.wait_for_change(&sources);
        eprintln!("openui: change detected, rebuilding");
    }
}

fn watched_paths(directory: &Path, assets: &Path, extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = ["src", "build.rs", "Cargo.toml"]
        .iter()
        .map(|path| directory.join(path))
        .collect();
    paths.push(directory.join(assets));
    paths.extend(extra.iter().map(|path| directory.join(path)));
    paths
}

fn start(executable: &Path, directory: &Path, options: &DevOptions) -> Result<Child> {
    Command::new(executable)
        .args(&options.app_args)
        .current_dir(directory)
        .env(DEV_ENV, "1")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start {}: {error}", executable.display()))
}

/// Close the app's stdin so it exits through its normal path; kill it if it
/// has not exited after a grace period.
fn stop(mut child: Child) {
    drop(child.stdin.take());
    let deadline = Instant::now() + STOP_GRACE;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => thread::sleep(Duration::from_millis(25)),
        }
    }
    eprintln!("openui: the app did not exit within {STOP_GRACE:?}; stopping it");
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watches_sources_manifest_assets_and_extra_paths() {
        let directory = Path::new("app");
        assert_eq!(
            watched_paths(directory, Path::new("static"), &[PathBuf::from("shaders")]),
            [
                directory.join("src"),
                directory.join("build.rs"),
                directory.join("Cargo.toml"),
                directory.join("static"),
                directory.join("shaders"),
            ]
        );
    }

    #[test]
    fn stop_closes_stdin_then_waits_for_a_graceful_exit() {
        // `sort` reads stdin until it closes and then exits, as a dev-mode
        // app does; it exists on Windows and Linux.
        let child = Command::new("sort")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let started = Instant::now();
        stop(child);
        assert!(started.elapsed() < STOP_GRACE);
    }
}
