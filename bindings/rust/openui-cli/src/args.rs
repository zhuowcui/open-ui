//! Command-line parsing. The grammar is small enough that a hand-written
//! parser costs less than a dependency.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

pub const USAGE: &str = "\
Usage: openui <command> [options]

Commands:
  new <name>   Create a new app in ./<name>
      --template <name>        Template to use (default: minimal)
      --framework-path <dir>   Use a local Open UI checkout instead of git
      --path <dir>             Directory to create (default: ./<name>)
  dev          Build and run the app, then rebuild and restart it on save
      --release                Use the release profile
      --bin <name>             Binary to run when the package has several
      --manifest-path <file>   Cargo.toml of the app (default: ./Cargo.toml)
      -- <args>...             Arguments passed to the app
  build        Build a release binary and copy it with assets to dist/
      --debug                  Use the dev profile
      --locked                 Require Cargo.lock to be up to date
      --out <dir>              Output directory (default: <app>/dist)
      --bin <name>             Binary to package when the package has several
      --manifest-path <file>   Cargo.toml of the app (default: ./Cargo.toml)
  doctor       Check the tools needed to build Skia and apps

  -h, --help     Print this help
  -V, --version  Print the version";

#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Version,
    New(NewOptions),
    Dev(DevOptions),
    Build(BuildOptions),
    Doctor,
}

#[derive(Debug, PartialEq)]
pub struct NewOptions {
    pub name: String,
    pub template: String,
    pub framework_path: Option<PathBuf>,
    pub path: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Default)]
pub struct DevOptions {
    pub release: bool,
    pub bin: Option<String>,
    pub manifest_path: Option<PathBuf>,
    pub app_args: Vec<OsString>,
}

#[derive(Debug, PartialEq, Default)]
pub struct BuildOptions {
    pub debug: bool,
    pub locked: bool,
    pub out: Option<PathBuf>,
    pub bin: Option<String>,
    pub manifest_path: Option<PathBuf>,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Command::Help);
    };
    match text(&command)? {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-V" | "--version" => Ok(Command::Version),
        "new" => parse_new(args),
        "dev" => parse_dev(args),
        "build" => parse_build(args),
        "doctor" => match args.next() {
            None => Ok(Command::Doctor),
            Some(extra) => Err(format!("unexpected argument {extra:?} for doctor")),
        },
        other => Err(format!("unknown command `{other}`")),
    }
}

fn parse_new(mut args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut name = None;
    let mut template = None;
    let mut framework_path = None;
    let mut path = None;
    while let Some(arg) = args.next() {
        match text(&arg)? {
            "--template" => template = Some(value(&mut args, "--template")?),
            "--framework-path" => {
                framework_path = Some(PathBuf::from(os_value(&mut args, "--framework-path")?))
            }
            "--path" => path = Some(PathBuf::from(os_value(&mut args, "--path")?)),
            "-h" | "--help" => return Ok(Command::Help),
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option `{flag}` for new"))
            }
            positional if name.is_none() => name = Some(positional.to_owned()),
            extra => return Err(format!("unexpected argument `{extra}` for new")),
        }
    }
    let name = name.ok_or("new needs an app name: openui new <name>")?;
    Ok(Command::New(NewOptions {
        name,
        template: template.unwrap_or_else(|| "minimal".to_owned()),
        framework_path,
        path,
    }))
}

fn parse_dev(mut args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut options = DevOptions::default();
    while let Some(arg) = args.next() {
        match text(&arg)? {
            "--release" => options.release = true,
            "--bin" => options.bin = Some(value(&mut args, "--bin")?),
            "--manifest-path" => {
                options.manifest_path = Some(PathBuf::from(os_value(&mut args, "--manifest-path")?))
            }
            "--" => {
                options.app_args.extend(args);
                break;
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unexpected argument `{other}` for dev")),
        }
    }
    Ok(Command::Dev(options))
}

fn parse_build(mut args: impl Iterator<Item = OsString>) -> Result<Command, String> {
    let mut options = BuildOptions::default();
    while let Some(arg) = args.next() {
        match text(&arg)? {
            "--debug" => options.debug = true,
            "--locked" => options.locked = true,
            "--out" => options.out = Some(PathBuf::from(os_value(&mut args, "--out")?)),
            "--bin" => options.bin = Some(value(&mut args, "--bin")?),
            "--manifest-path" => {
                options.manifest_path = Some(PathBuf::from(os_value(&mut args, "--manifest-path")?))
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unexpected argument `{other}` for build")),
        }
    }
    Ok(Command::Build(options))
}

fn text(arg: &OsStr) -> Result<&str, String> {
    arg.to_str()
        .ok_or_else(|| format!("argument {arg:?} is not valid Unicode"))
}

fn os_value(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<OsString, String> {
    args.next().ok_or_else(|| format!("{flag} needs a value"))
}

fn value(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<String, String> {
    let value = os_value(args, flag)?;
    text(&value).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn no_arguments_and_help_flags_print_usage() {
        assert_eq!(parse_str(&[]), Ok(Command::Help));
        assert_eq!(parse_str(&["--help"]), Ok(Command::Help));
        assert_eq!(parse_str(&["new", "-h"]), Ok(Command::Help));
        assert_eq!(parse_str(&["-V"]), Ok(Command::Version));
    }

    #[test]
    fn new_takes_a_name_and_options_in_any_order() {
        assert_eq!(
            parse_str(&[
                "new",
                "--framework-path",
                "../open-ui",
                "notes",
                "--template",
                "minimal"
            ]),
            Ok(Command::New(NewOptions {
                name: "notes".into(),
                template: "minimal".into(),
                framework_path: Some(PathBuf::from("../open-ui")),
                path: None,
            }))
        );
        assert!(parse_str(&["new"])
            .unwrap_err()
            .contains("needs an app name"));
        assert!(parse_str(&["new", "a", "b"])
            .unwrap_err()
            .contains("unexpected"));
        assert!(parse_str(&["new", "a", "--template"])
            .unwrap_err()
            .contains("needs a value"));
    }

    #[test]
    fn dev_passes_arguments_after_double_dash_to_the_app() {
        assert_eq!(
            parse_str(&["dev", "--release", "--bin", "app", "--", "--release", "x"]),
            Ok(Command::Dev(DevOptions {
                release: true,
                bin: Some("app".into()),
                manifest_path: None,
                app_args: vec!["--release".into(), "x".into()],
            }))
        );
    }

    #[test]
    fn build_and_doctor_reject_unknown_arguments() {
        assert_eq!(
            parse_str(&["build", "--debug", "--locked", "--out", "out"]),
            Ok(Command::Build(BuildOptions {
                debug: true,
                locked: true,
                out: Some(PathBuf::from("out")),
                ..BuildOptions::default()
            }))
        );
        assert!(parse_str(&["build", "--target", "x"]).is_err());
        assert!(parse_str(&["doctor", "now"]).is_err());
        assert!(parse_str(&["serve"])
            .unwrap_err()
            .contains("unknown command"));
    }
}
