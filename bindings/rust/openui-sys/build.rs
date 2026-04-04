fn main() {
    let chromium_src = std::env::var("CHROMIUM_SRC").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME not set");
        format!("{home}/chromium/src")
    });

    let sysroot = format!("{chromium_src}/build/linux/debian_bullseye_amd64-sysroot");
    let out_release = format!("{chromium_src}/out/Release");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = std::path::Path::new(&manifest_dir)
        .parent()
        .expect("openui-sys must be inside a workspace directory");

    let deps_path = workspace_dir.join("openui_deps.txt");
    let deps_content = std::fs::read_to_string(&deps_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", deps_path.display()));

    let system_libs_path = workspace_dir.join("system_libs.txt");
    let system_libs_content = std::fs::read_to_string(&system_libs_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", system_libs_path.display()));

    // Sysroot for the linker.
    println!("cargo:rustc-link-arg=--sysroot={sysroot}");

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

    // Clang compiler-rt builtins (not part of the Chromium deps list).
    println!(
        "cargo:rustc-link-arg={chromium_src}/third_party/llvm-build/\
         Release+Asserts/lib/clang/23/lib/x86_64-unknown-linux-gnu/\
         libclang_rt.builtins.a"
    );

    // System shared library search path (inside the sysroot).
    println!("cargo:rustc-link-search=native={sysroot}/usr/lib/x86_64-linux-gnu");

    // System shared libraries.
    for line in system_libs_content.lines() {
        let entry = line.trim();
        if entry.is_empty() || entry.starts_with('#') {
            continue;
        }
        let lib = entry.strip_prefix("-l").unwrap_or(entry);
        if !lib.is_empty() {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
    }

    // Rerun triggers.
    println!("cargo:rerun-if-changed={}", deps_path.display());
    println!("cargo:rerun-if-changed={}", system_libs_path.display());
    println!("cargo:rerun-if-env-changed=CHROMIUM_SRC");
}
