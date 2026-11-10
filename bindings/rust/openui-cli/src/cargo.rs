//! Running Cargo and reading its JSON output. The CLI never builds anything
//! itself.

use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::Result;

/// An absolute path with symlinks resolved, without the `\\?\` prefix
/// Windows adds, so it reads normally in messages and Cargo arguments.
pub fn canonical(path: &Path) -> Result<PathBuf> {
    let resolved = path
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", path.display()))?;
    let text = resolved.to_string_lossy();
    Ok(match text.strip_prefix(r"\\?\") {
        // Only drive paths; UNC paths keep the prefix they need.
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => resolved,
    })
}

/// `cargo`, honoring `CARGO` when the CLI itself runs under Cargo.
pub fn command() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")))
}

/// Resolve dependencies and settle `Cargo.lock` without building.
pub fn resolve(manifest: &Path) -> Result<()> {
    let manifest = canonical(manifest)?;
    let status = command()
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .current_dir(manifest.parent().expect("a manifest has a parent"))
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo could not resolve the dependencies of {}",
            manifest.display()
        ))
    }
}

/// The app package the CLI operates on.
#[derive(Debug)]
pub struct App {
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub bins: Vec<String>,
    /// `[package.metadata.openui] assets`, relative to the package.
    pub assets: PathBuf,
    /// `[package.metadata.openui] watch`, relative to the package.
    pub watch: Vec<PathBuf>,
}

impl App {
    pub fn directory(&self) -> &Path {
        self.manifest_path
            .parent()
            .expect("a manifest has a parent")
    }

    /// The binary to run or package: `requested`, or the only one.
    pub fn bin(&self, requested: Option<&str>) -> Result<String> {
        match requested {
            Some(name) if self.bins.iter().any(|bin| bin == name) => Ok(name.to_owned()),
            Some(name) => Err(format!("{} has no binary named `{name}`", self.name)),
            None => match self.bins.as_slice() {
                [only] => Ok(only.clone()),
                [] => Err(format!("{} has no binary target", self.name)),
                several => Err(format!(
                    "{} has several binaries ({}); choose one with --bin",
                    self.name,
                    several.join(", ")
                )),
            },
        }
    }
}

/// Read the app package from `cargo metadata`. Without `manifest`, the
/// package is the one in the current directory.
pub fn app(manifest: Option<&Path>) -> Result<App> {
    let manifest = match manifest {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir()
            .map_err(|error| format!("cannot read the current directory: {error}"))?
            .join("Cargo.toml"),
    };
    if !manifest.is_file() {
        return Err(format!(
            "{} not found; run this in an app directory or pass --manifest-path",
            manifest.display()
        ));
    }
    let output = command()
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&manifest)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    if !output.status.success() {
        return Err(format!("cargo metadata failed for {}", manifest.display()));
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot parse cargo metadata: {error}"))?;
    let manifest = canonical(&manifest)?;
    parse_app(&metadata, &manifest)
}

fn parse_app(metadata: &Value, manifest: &Path) -> Result<App> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata has no packages")?;
    let package = packages
        .iter()
        .find(|package| {
            package["manifest_path"]
                .as_str()
                .and_then(|path| canonical(Path::new(path)).ok())
                .is_some_and(|path| path == manifest)
        })
        .ok_or_else(|| {
            format!(
                "{} is not a package manifest (a workspace root needs --manifest-path of the app)",
                manifest.display()
            )
        })?;
    let text = |value: &Value, what: &str| {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("cargo metadata has no {what}"))
    };
    let bins = package["targets"]
        .as_array()
        .ok_or("cargo metadata has no targets")?
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        })
        .map(|target| text(&target["name"], "target name"))
        .collect::<Result<Vec<_>>>()?;
    let openui = &package["metadata"]["openui"];
    let assets = openui["assets"].as_str().unwrap_or("assets").into();
    let watch = match &openui["watch"] {
        Value::Null => Vec::new(),
        Value::Array(paths) => paths
            .iter()
            .map(|path| text(path, "watch path").map(PathBuf::from))
            .collect::<Result<_>>()?,
        _ => return Err("[package.metadata.openui] watch must be an array of paths".into()),
    };
    Ok(App {
        name: text(&package["name"], "package name")?,
        version: text(&package["version"], "package version")?,
        manifest_path: manifest.to_path_buf(),
        bins,
        assets,
        watch,
    })
}

pub struct Build<'a> {
    pub app: &'a App,
    pub bin: &'a str,
    pub release: bool,
    pub locked: bool,
}

/// Build one binary and return its path. Cargo renders diagnostics to
/// stderr; the JSON on stdout gives the executable.
pub fn build(build: &Build) -> Result<PathBuf> {
    let mut command = command();
    // Cargo reads `.cargo/config.toml`, which carries the Skia build profile,
    // from the working directory rather than from the manifest's.
    command
        .args([
            "build",
            "--message-format=json-render-diagnostics",
            "--bin",
            build.bin,
        ])
        .arg("--manifest-path")
        .arg(&build.app.manifest_path)
        .current_dir(build.app.directory());
    if build.release {
        command.arg("--release");
    }
    if build.locked {
        command.arg("--locked");
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut executable = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| format!("cannot read cargo output: {error}"))?;
        if let Some(path) = artifact_executable(&line, &build.app.manifest_path, build.bin) {
            executable = Some(path);
        }
    }
    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for cargo: {error}"))?;
    if !status.success() {
        return Err("build failed".into());
    }
    executable.ok_or_else(|| format!("cargo built no executable for `{}`", build.bin))
}

/// The executable in a `compiler-artifact` message for `bin` of the package
/// at `manifest`, if this line is one.
fn artifact_executable(line: &str, manifest: &Path, bin: &str) -> Option<PathBuf> {
    let message: Value = serde_json::from_str(line).ok()?;
    if message["reason"] != "compiler-artifact" || message["target"]["name"] != bin {
        return None;
    }
    let artifact_manifest = canonical(Path::new(message["manifest_path"].as_str()?)).ok()?;
    if artifact_manifest != manifest {
        return None;
    }
    message["executable"].as_str().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PathBuf {
        canonical(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap()
    }

    #[test]
    fn canonical_paths_have_no_verbatim_prefix() {
        let path = manifest();
        assert!(path.is_absolute() && path.is_file());
        assert!(!path.to_string_lossy().starts_with(r"\\?\"));
    }

    #[test]
    fn app_is_read_from_this_package() {
        let app = app(Some(&manifest())).unwrap();
        assert_eq!(app.name, "openui-cli");
        assert_eq!(app.bins, ["openui"]);
        assert_eq!(app.bin(None).unwrap(), "openui");
        assert!(app.bin(Some("other")).is_err());
        assert_eq!(app.assets, PathBuf::from("assets"));
        assert!(app.watch.is_empty());
    }

    #[test]
    fn openui_metadata_and_several_binaries_are_read() {
        let manifest = manifest();
        let metadata = serde_json::json!({ "packages": [{
            "name": "notes",
            "version": "0.1.0",
            "manifest_path": manifest,
            "targets": [
                { "name": "notes", "kind": ["bin"] },
                { "name": "tool", "kind": ["bin"] },
                { "name": "notes", "kind": ["lib"] },
            ],
            "metadata": { "openui": { "assets": "static", "watch": ["shaders"] } },
        }]});
        let app = parse_app(&metadata, &manifest).unwrap();
        assert_eq!(app.bins, ["notes", "tool"]);
        assert!(app.bin(None).unwrap_err().contains("--bin"));
        assert_eq!(app.bin(Some("tool")).unwrap(), "tool");
        assert_eq!(app.assets, PathBuf::from("static"));
        assert_eq!(app.watch, [PathBuf::from("shaders")]);
    }

    #[test]
    fn executables_come_from_matching_artifact_messages() {
        let manifest = manifest();
        let message = |reason: &str, name: &str| {
            serde_json::json!({
                "reason": reason,
                "manifest_path": manifest,
                "target": { "name": name },
                "executable": "/target/debug/openui",
            })
            .to_string()
        };
        assert_eq!(
            artifact_executable(&message("compiler-artifact", "openui"), &manifest, "openui"),
            Some(PathBuf::from("/target/debug/openui"))
        );
        assert_eq!(
            artifact_executable(&message("compiler-artifact", "other"), &manifest, "openui"),
            None
        );
        assert_eq!(
            artifact_executable(&message("build-finished", "openui"), &manifest, "openui"),
            None
        );
        assert_eq!(artifact_executable("not json", &manifest, "openui"), None);
    }
}
