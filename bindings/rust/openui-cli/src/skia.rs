//! The Skia build profile apps must use to render like the qualified engine.
//!
//! Open UI's pixels depend on how Skia is compiled. CI builds it from source
//! with these variables (`.github/workflows/ci.yml`); a test below fails if
//! the two drift apart. Apps get them through `.cargo/config.toml`, so plain
//! `cargo build` produces the same Skia as `openui dev` and `openui build`.

/// Compile-time Skia options, as in CI's `SKIA_GN_ARGS`.
pub const GN_ARGS: &str = concat!(
    r#"extra_cflags += [ "-DSK_SUPPORT_LEGACY_CONIC_CHOP", "#,
    r#""-DSK_ENABLE_LEGACY_SHADERCONTEXT", "#,
    r#""-DSK_USE_FREETYPE_EMBOLDEN", "-DSK_GAMMA_EXPONENT=1.2", "#,
    r#""-DSK_GAMMA_CONTRAST=0.2", "-USK_GAMMA_APPLY_TO_A8", "-fno-strict-aliasing", "-msse3" ]"#,
);

/// Variables every platform sets.
pub const ENV: &[(&str, &str)] = &[
    ("FORCE_SKIA_BUILD", "1"),
    ("SKIA_GN_ARGS", GN_ARGS),
    ("SKIA_NINJA_COMMAND", "ninja"),
    // Deep layouts need more than libtest's and std's 2 MiB thread stacks.
    ("RUST_MIN_STACK", "4194304"),
];

/// Linux CI compiles Skia with clang rather than the system C compiler.
pub const LINUX_ENV: &[(&str, &str)] = &[
    ("CC", "clang"),
    ("CXX", "clang++"),
    ("CLANGCC", "clang"),
    ("CLANGCXX", "clang++"),
];

/// `.cargo/config.toml` for an app created on `os` (`std::env::consts::OS`).
pub fn cargo_config(os: &str) -> String {
    let mut config = String::from(
        "# Written by `openui new`. Open UI's pixels depend on how Skia is\n\
         # compiled; these match the framework's CI. Variables already set in\n\
         # the environment take precedence.\n\
         [env]\n",
    );
    let platform = if os == "linux" { LINUX_ENV } else { &[] };
    for (name, value) in ENV.iter().chain(platform) {
        // TOML literal strings keep the quotes in GN_ARGS as they are.
        debug_assert!(!value.contains('\''));
        config.push_str(&format!("{name} = '{value}'\n"));
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    const CI: &str = include_str!("../../../../.github/workflows/ci.yml");

    /// The `env:` block of a CI job, as `(name, value)` pairs, with folded
    /// (`>-`) values joined by spaces as YAML does.
    fn job_env(job: &str) -> Vec<(String, String)> {
        let start = CI
            .find(&format!("\n  {job}:\n"))
            .unwrap_or_else(|| panic!("CI job {job} not found"));
        let lines: Vec<&str> = CI[start..].lines().skip(2).collect();
        let env_at = lines
            .iter()
            .position(|line| *line == "    env:")
            .expect("job has an env block");
        let mut env = Vec::new();
        let mut index = env_at + 1;
        while let Some(line) = lines.get(index) {
            if !line.starts_with("      ") {
                break;
            }
            index += 1;
            let entry = line.trim();
            if entry.starts_with('#') {
                continue;
            }
            let (name, value) = entry.split_once(':').expect("env entry");
            let value = value.trim();
            let value = if value == ">-" {
                let mut folded = Vec::new();
                while let Some(next) = lines.get(index) {
                    if !next.starts_with("        ") {
                        break;
                    }
                    folded.push(next.trim());
                    index += 1;
                }
                folded.join(" ")
            } else {
                value.trim_matches('"').to_owned()
            };
            env.push((name.to_owned(), value));
        }
        env
    }

    fn assert_job_sets(job: &str, expected: &[(&str, &str)]) {
        let env = job_env(job);
        for (name, value) in expected {
            let actual = env
                .iter()
                .find(|(key, _)| key == name)
                .unwrap_or_else(|| panic!("CI job {job} does not set {name}"));
            assert_eq!(actual.1, *value, "{name} differs from CI job {job}");
        }
    }

    #[test]
    fn profile_matches_both_ci_platforms() {
        assert_job_sets("rust-parity", ENV);
        assert_job_sets("rust-parity", LINUX_ENV);
        assert_job_sets("windows-msvc", ENV);
    }

    #[test]
    fn cargo_config_sets_the_profile_as_literal_strings() {
        let windows = cargo_config("windows");
        assert!(windows.starts_with("# Written by `openui new`"));
        assert!(windows.contains(&format!("SKIA_GN_ARGS = '{GN_ARGS}'\n")));
        assert!(!windows.contains("CC = "));
        let linux = cargo_config("linux");
        assert!(linux.contains("CC = 'clang'\nCXX = 'clang++'\n"));
    }
}
