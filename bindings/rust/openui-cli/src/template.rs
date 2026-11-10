//! Project templates embedded in the binary, and the parts of a generated
//! `Cargo.toml` that come from the framework's own workspace.

use std::path::Path;

/// One file of a template: its path in the new project and its text, with
/// `{{placeholder}}`s.
pub struct TemplateFile {
    pub path: &'static str,
    pub text: &'static str,
}

pub struct Template {
    pub name: &'static str,
    pub description: &'static str,
    /// Extra lines for the steps `new` prints after creating the project.
    pub next_steps: &'static str,
    pub files: &'static [TemplateFile],
}

macro_rules! template_file {
    ($template:literal, $path:literal, $source:literal) => {
        TemplateFile {
            path: $path,
            text: include_str!(concat!("../templates/", $template, "/", $source)),
        }
    };
}

pub const TEMPLATES: &[Template] = &[
    Template {
        name: "minimal",
        description: "One window with a reactive counter",
        next_steps: "",
        files: &[
            template_file!("minimal", "Cargo.toml", "Cargo.toml.tmpl"),
            template_file!("minimal", ".gitignore", "gitignore.tmpl"),
            template_file!("minimal", "src/main.rs", "src/main.rs.tmpl"),
        ],
    },
    Template {
        name: "default",
        description: "A menu bar, file dialog, HTTP request and headless UI tests",
        next_steps: "Record the UI test's first snapshots, look at them and commit them:\n  \
                     OPENUI_UPDATE_SNAPSHOTS=1 cargo test",
        files: &[
            template_file!("default", "Cargo.toml", "Cargo.toml.tmpl"),
            template_file!("default", ".gitignore", "gitignore.tmpl"),
            template_file!("default", "assets/README.md", "assets/README.md"),
            template_file!("default", "src/main.rs", "src/main.rs.tmpl"),
            template_file!("default", "src/lib.rs", "src/lib.rs.tmpl"),
            template_file!("default", "src/app.rs", "src/app.rs.tmpl"),
            template_file!("default", "src/menu.rs", "src/menu.rs.tmpl"),
            template_file!("default", "tests/ui.rs", "tests/ui.rs.tmpl"),
        ],
    },
];

pub fn find(name: &str) -> Result<&'static Template, String> {
    TEMPLATES
        .iter()
        .find(|template| template.name == name)
        .ok_or_else(|| {
            let known: Vec<_> = TEMPLATES
                .iter()
                .map(|template| format!("{} ({})", template.name, template.description))
                .collect();
            format!("unknown template `{name}`; available: {}", known.join(", "))
        })
}

/// Replace each `{{key}}` with its value. Every placeholder in the text must
/// have a value, so a template typo fails instead of reaching an app.
pub fn render(text: &str, values: &[(&str, &str)]) -> Result<String, String> {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or("unterminated `{{` in template")?;
        let key = &after[..end];
        let value = values
            .iter()
            .find(|(name, _)| *name == key)
            .ok_or_else(|| format!("template placeholder `{{{{{key}}}}}` has no value"))?;
        output.push_str(value.1);
        rest = &after[end + 2..];
    }
    output.push_str(rest);
    Ok(output)
}

/// The workspace manifest and lockfile the CLI was built from.
pub const WORKSPACE_MANIFEST: &str = include_str!("../../Cargo.toml");
pub const WORKSPACE_LOCKFILE: &str = include_str!("../../Cargo.lock");

/// Where generated apps get the framework from.
pub const FRAMEWORK_GIT: &str = "https://github.com/zhuowcui/open-ui";
/// The git reference generated apps pin. `main` until the first alpha tag.
pub const FRAMEWORK_GIT_BRANCH: &str = "main";

pub enum FrameworkSource<'a> {
    Git,
    /// The `bindings/rust` directory of a local checkout.
    Path(&'a Path),
}

impl FrameworkSource<'_> {
    /// A TOML inline table locating `relative` (a path inside
    /// `bindings/rust`) in this source.
    fn locate(&self, relative: &str) -> String {
        match self {
            FrameworkSource::Git => {
                format!("git = \"{FRAMEWORK_GIT}\", branch = \"{FRAMEWORK_GIT_BRANCH}\"")
            }
            FrameworkSource::Path(workspace) => {
                format!("path = '{}'", toml_path(&workspace.join(relative)))
            }
        }
    }

    /// The app's dependency line on the public crate.
    pub fn dependency(&self) -> String {
        format!("openui = {{ {}, features = [\"desktop\"] }}", self.source())
    }

    /// Where the public crate is, as the start of a TOML inline table, for
    /// templates that choose their own features.
    pub fn source(&self) -> String {
        self.locate("openui")
    }

    /// The app's `[patch.crates-io]` table. Cargo applies a workspace's
    /// patches only inside that workspace, so an app repeats the framework's:
    /// git patches as they are, and vendored crates from this source.
    pub fn patch_table(&self) -> Result<String, String> {
        let mut table = String::from(
            "# The framework's dependency patches, which Cargo does not apply to\n\
             # dependents. Keep them equal to the framework's workspace.\n\
             [patch.crates-io]\n",
        );
        for (name, entry) in workspace_patches()? {
            let entry = match entry.strip_prefix("path = \"") {
                Some(path) => {
                    let path = path
                        .strip_suffix('"')
                        .ok_or_else(|| format!("unexpected patch entry for {name}: {entry}"))?;
                    self.locate(path)
                }
                None => entry.to_owned(),
            };
            table.push_str(&format!("{name} = {{ {entry} }}\n"));
        }
        Ok(table)
    }
}

/// The `[patch.crates-io]` entries of the workspace manifest, as
/// `(crate, inline table contents)`.
pub fn workspace_patches() -> Result<Vec<(&'static str, &'static str)>, String> {
    let section = WORKSPACE_MANIFEST
        .split("\n[patch.crates-io]\n")
        .nth(1)
        .ok_or("the workspace manifest has no [patch.crates-io] table")?;
    let mut patches = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line
            .split_once(" = ")
            .ok_or_else(|| format!("unexpected patch line: {line}"))?;
        let entry = value
            .strip_prefix("{ ")
            .and_then(|value| value.strip_suffix(" }"))
            .ok_or_else(|| format!("patch for {name} is not a one-line inline table"))?;
        patches.push((name, entry));
    }
    Ok(patches)
}

/// A path as a TOML literal string body: forward slashes, without Windows'
/// verbatim prefix.
pub fn toml_path(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    text.strip_prefix("//?/").map(str::to_owned).unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_replaces_placeholders_and_rejects_unknown_ones() {
        assert_eq!(
            render("a {{x}} b {{y}}{{x}}", &[("x", "1"), ("y", "2")]).unwrap(),
            "a 1 b 21"
        );
        assert!(render("{{missing}}", &[]).unwrap_err().contains("missing"));
        assert!(render("{{open", &[]).is_err());
    }

    #[test]
    fn every_template_renders_with_the_values_new_supplies() {
        for template in TEMPLATES {
            for file in template.files {
                render(file.text, &crate::new::sample_values())
                    .unwrap_or_else(|error| panic!("{}/{}: {error}", template.name, file.path));
            }
        }
        assert!(find("minimal").is_ok());
        assert!(find("default").is_ok());
        assert!(find("nope").err().unwrap().contains("minimal"));
    }

    #[test]
    fn patch_table_repeats_every_workspace_patch() {
        let patches = workspace_patches().unwrap();
        let names: Vec<_> = patches.iter().map(|(name, _)| *name).collect();
        for expected in ["skia-safe", "skia-bindings", "skrifa", "zbus_xml"] {
            assert!(
                names.contains(&expected),
                "{expected} missing from {names:?}"
            );
        }

        let git = FrameworkSource::Git.patch_table().unwrap();
        let local = FrameworkSource::Path(Path::new("/src/open-ui/bindings/rust"))
            .patch_table()
            .unwrap();
        for (name, entry) in &patches {
            if entry.starts_with("git = ") {
                let line = format!("{name} = {{ {entry} }}\n");
                assert!(git.contains(&line) && local.contains(&line), "{line}");
            } else {
                let path = entry.trim_start_matches("path = \"").trim_end_matches('"');
                assert!(git.contains(&format!(
                    "{name} = {{ git = \"{FRAMEWORK_GIT}\", branch = \"{FRAMEWORK_GIT_BRANCH}\" }}\n"
                )));
                assert!(local.contains(&format!(
                    "{name} = {{ path = '/src/open-ui/bindings/rust/{path}' }}\n"
                )));
            }
        }
        assert_eq!(git.matches(" = { ").count(), patches.len());
    }

    #[test]
    fn dependency_selects_the_desktop_runtime() {
        assert_eq!(
            FrameworkSource::Git.dependency(),
            format!(
                "openui = {{ git = \"{FRAMEWORK_GIT}\", branch = \"main\", features = [\"desktop\"] }}"
            )
        );
        assert_eq!(
            toml_path(Path::new(r"\\?\E:\open-ui\bindings\rust")),
            "E:/open-ui/bindings/rust"
        );
    }

    #[test]
    fn embedded_lockfile_is_the_workspace_lockfile() {
        assert!(WORKSPACE_LOCKFILE.contains("\nname = \"openui\"\n"));
        assert!(WORKSPACE_LOCKFILE.contains("\nname = \"skia-bindings\"\n"));
    }
}
