// Emit the Chromium/Blink link flags needed to produce an executable.
// The openui-sys crate uses `cargo:rustc-link-arg` which only applies to
// itself. Binaries that depend on openui must re-emit the link arguments.

fn main() {
    let chromium_src = std::env::var("CHROMIUM_SRC").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME not set");
        format!("{home}/chromium/src")
    });

    let out_release = format!("{chromium_src}/out/Release");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("example must be inside examples/ inside the workspace");

    let deps_path = workspace_dir.join("openui_deps.txt");
    let deps_content = std::fs::read_to_string(&deps_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", deps_path.display()));

    // Wrap all static archives in --start-group / --end-group so the linker
    // resolves circular dependencies between Chromium's static libraries.
    println!("cargo:rustc-link-arg=-Wl,--start-group");

    for line in deps_content.lines() {
        let dep = line.trim();
        if dep.is_empty() || dep.starts_with('#') {
            continue;
        }
        println!("cargo:rustc-link-arg={out_release}/{dep}");
    }

    println!("cargo:rustc-link-arg=-Wl,--end-group");

    // Clang compiler-rt builtins.
    println!(
        "cargo:rustc-link-arg={chromium_src}/third_party/llvm-build/\
         Release+Asserts/lib/clang/23/lib/x86_64-unknown-linux-gnu/\
         libclang_rt.builtins.a"
    );

    // Rerun triggers.
    println!("cargo:rerun-if-changed={}", deps_path.display());
    println!("cargo:rerun-if-env-changed=CHROMIUM_SRC");
}
