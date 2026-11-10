//! `openui new`: write a new app from an embedded template.

use std::fs;
use std::path::{Path, PathBuf};

use crate::args::NewOptions;
use crate::cargo;
use crate::skia;
use crate::template::{self, FrameworkSource};
use crate::Result;

pub fn run(options: &NewOptions) -> Result<()> {
    validate_name(&options.name)?;
    let template = template::find(&options.template)?;
    let directory = options
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(&options.name));
    ensure_empty(&directory)?;

    let workspace = options
        .framework_path
        .as_deref()
        .map(framework_workspace)
        .transpose()?;
    let source = match &workspace {
        Some(workspace) => FrameworkSource::Path(workspace),
        None => FrameworkSource::Git,
    };
    let crate_name = options.name.replace('-', "_");
    let app_id = format!("com.example.{crate_name}");
    let dependency = source.dependency();
    let framework_source = source.source();
    let patch_table = source.patch_table()?;
    let values = [
        ("name", options.name.as_str()),
        ("crate_name", crate_name.as_str()),
        ("app_id", app_id.as_str()),
        ("framework_dependency", dependency.as_str()),
        ("framework_source", framework_source.as_str()),
        ("patch_table", patch_table.as_str()),
    ];

    for file in template.files {
        write(
            &directory.join(file.path),
            &template::render(file.text, &values)?,
        )?;
    }
    write(
        &directory.join(".cargo/config.toml"),
        &skia::cargo_config(std::env::consts::OS),
    )?;
    // Start from the framework's reviewed dependency versions; Cargo drops
    // the entries the app does not use when it resolves below.
    write(&directory.join("Cargo.lock"), template::WORKSPACE_LOCKFILE)?;

    println!(
        "Created `{}` from the {} template in {}",
        options.name,
        template.name,
        directory.display()
    );
    if workspace.is_none() {
        println!("Resolving dependencies; the first time fetches the framework from git.");
    }
    cargo::resolve(&directory.join("Cargo.toml"))?;
    println!(
        "\nNext:\n  cd {}\n  openui dev\n\nThe first build compiles Skia from source and takes a while; \
         `openui doctor` checks the tools it needs.",
        directory.display()
    );
    if !template.next_steps.is_empty() {
        println!("\n{}", template.next_steps);
    }
    Ok(())
}

/// Package names Cargo accepts, kept to ASCII so the name is also a safe
/// window title and directory name.
fn validate_name(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let valid = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(())
    } else {
        Err(format!(
            "`{name}` is not a valid app name: use ASCII letters, digits, `-` and `_`, starting with a letter"
        ))
    }
}

fn ensure_empty(directory: &Path) -> Result<()> {
    match fs::read_dir(directory) {
        Ok(mut entries) => match entries.next() {
            Some(_) => Err(format!(
                "{} already exists and is not empty",
                directory.display()
            )),
            None => Ok(()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot read {}: {error}", directory.display())),
    }
}

/// Accept a checkout's root or its `bindings/rust` directory and return the
/// latter, absolute.
fn framework_workspace(path: &Path) -> Result<PathBuf> {
    let candidates = [path.join("bindings/rust"), path.to_path_buf()];
    let workspace = candidates
        .iter()
        .find(|candidate| candidate.join("openui/Cargo.toml").is_file())
        .ok_or_else(|| {
            format!(
                "{} is not an Open UI checkout (no bindings/rust/openui/Cargo.toml)",
                path.display()
            )
        })?;
    cargo::canonical(workspace)
}

fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Values for every placeholder `run` fills, for template tests.
#[cfg(test)]
pub fn sample_values() -> Vec<(&'static str, &'static str)> {
    vec![
        ("name", "sample-app"),
        ("crate_name", "sample_app"),
        ("app_id", "com.example.sample_app"),
        ("framework_dependency", "openui = { path = 'x' }"),
        ("framework_source", "path = 'x'"),
        ("patch_table", "[patch.crates-io]"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_cargo_package_rules() {
        for name in ["notes", "my-app", "app_2"] {
            assert!(validate_name(name).is_ok(), "{name}");
        }
        for name in ["", "2app", "-app", "my app", "app/x", "café"] {
            assert!(validate_name(name).is_err(), "{name}");
        }
    }

    #[test]
    fn framework_path_accepts_the_checkout_root_or_its_workspace() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let root = workspace.parent().unwrap().parent().unwrap();
        let expected = cargo::canonical(workspace).unwrap();
        assert_eq!(framework_workspace(root).unwrap(), expected);
        assert_eq!(framework_workspace(workspace).unwrap(), expected);
        assert!(framework_workspace(&workspace.join("openui-cli")).is_err());
    }

    #[test]
    fn existing_non_empty_directories_are_refused() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(ensure_empty(manifest_dir).is_err());
        assert!(ensure_empty(&manifest_dir.join("does-not-exist")).is_ok());
    }
}
