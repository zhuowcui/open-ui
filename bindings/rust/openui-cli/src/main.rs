//! `openui`: create, run and build Open UI desktop apps.
//!
//! `openui` and `openui-cli` are working names until the product is named.
//! See `docs/alpha/rfcs/0002-cli-and-app-layout.md` for the design.

mod args;
mod cargo;
mod dev;
mod doctor;
mod new;
mod release;
mod skia;
mod template;
mod watch;

use std::process::ExitCode;

use args::Command;

fn main() -> ExitCode {
    let command = match args::parse(std::env::args_os().skip(1)) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("openui: {message}\n\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    let result = match command {
        Command::Help => {
            println!("{}", args::USAGE);
            Ok(())
        }
        Command::Version => {
            println!("openui {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::New(options) => new::run(&options),
        Command::Dev(options) => dev::run(&options),
        Command::Build(options) => release::run(&options),
        Command::Doctor => doctor::run(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("openui: {message}");
            ExitCode::FAILURE
        }
    }
}

/// CLI errors are messages for a person; each carries its own context.
pub type Result<T> = std::result::Result<T, String>;
