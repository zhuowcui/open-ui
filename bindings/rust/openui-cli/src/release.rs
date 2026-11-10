//! `openui build`: a release binary and its assets in a `dist/` folder.

use std::fs;
use std::path::{Path, PathBuf};

use crate::args::BuildOptions;
use crate::cargo::{self, Build};
use crate::Result;

pub fn run(options: &BuildOptions) -> Result<()> {
    let app = cargo::app(options.manifest_path.as_deref())?;
    let bin = app.bin(options.bin.as_deref())?;
    let executable = cargo::build(&Build {
        app: &app,
        bin: &bin,
        release: !options.debug,
        locked: options.locked,
    })?;

    let out = options
        .out
        .clone()
        .unwrap_or_else(|| app.directory().join("dist"));
    let folder = out.join(dist_name(
        &app.name,
        &app.version,
        std::env::consts::OS,
        std::env::consts::ARCH,
    ));
    if folder.exists() {
        fs::remove_dir_all(&folder)
            .map_err(|error| format!("cannot replace {}: {error}", folder.display()))?;
    }
    fs::create_dir_all(&folder)
        .map_err(|error| format!("cannot create {}: {error}", folder.display()))?;
    let file_name = executable
        .file_name()
        .ok_or_else(|| format!("{} has no file name", executable.display()))?;
    copy_file(&executable, &folder.join(file_name))?;
    let assets = app.directory().join(&app.assets);
    if assets.is_dir() {
        copy_tree(&assets, &folder.join(&app.assets))?;
    }
    println!("{}", folder.display());
    Ok(())
}

fn dist_name(name: &str, version: &str, os: &str, arch: &str) -> String {
    format!("{name}-{version}-{os}-{arch}")
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    fs::copy(from, to).map(drop).map_err(|error| {
        format!(
            "cannot copy {} to {}: {error}",
            from.display(),
            to.display()
        )
    })
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).map_err(|error| format!("cannot create {}: {error}", to.display()))?;
    let entries =
        fs::read_dir(from).map_err(|error| format!("cannot read {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read {}: {error}", from.display()))?;
        let source: PathBuf = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            copy_file(&source, &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dist_folders_name_the_package_version_and_platform() {
        assert_eq!(
            dist_name("notes", "0.1.0", "windows", "x86_64"),
            "notes-0.1.0-windows-x86_64"
        );
    }

    #[test]
    fn asset_trees_are_copied_recursively() {
        let root = std::env::temp_dir().join(format!("openui-dist-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("assets/fonts")).unwrap();
        fs::write(root.join("assets/logo.svg"), "<svg/>").unwrap();
        fs::write(root.join("assets/fonts/a.ttf"), [0_u8, 1, 2]).unwrap();
        copy_tree(&root.join("assets"), &root.join("dist/assets")).unwrap();
        assert_eq!(
            fs::read(root.join("dist/assets/logo.svg")).unwrap(),
            b"<svg/>"
        );
        assert_eq!(
            fs::read(root.join("dist/assets/fonts/a.ttf")).unwrap(),
            [0, 1, 2]
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
