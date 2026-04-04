/// Emit all Chromium/Blink link flags for an Open UI binary.
///
/// Call this from the `build.rs` of any binary crate that depends on `openui`.
/// It emits `cargo:rustc-link-arg` directives for:
///
/// * `libopenui_complete.a` — the complete static library containing openui_lib
///   plus all transitive Chromium deps (built with `complete_static_lib = true`
///   in GN so that source_set objects are bundled in).
/// * `libchromium_rust_deps.a` — object code extracted from Chromium's .rlib
///   files (fontations, crabby-avif, etc.) which are compiled with Chromium's
///   Rust 1.95-dev toolchain.
/// * Chromium's Rust sysroot rlibs — the standard library matching Chromium's
///   Rust toolchain, required by the .rlib objects above.
/// * Chromium's `libc++.a` and `libc++abi.a`.
/// * `libclang_rt.builtins.a` — compiler-rt builtins.
/// * System shared libraries (X11, fontconfig, freetype, etc.).
///
/// All archives are wrapped in `--start-group` / `--end-group` so the linker
/// resolves circular dependencies. `--allow-multiple-definition` is used
/// because our Rust 1.94 std and Chromium's Rust 1.95 std share one symbol
/// (`rust_eh_personality`); the linker picks the first definition.
pub fn link() {
    let chromium_src = std::env::var("CHROMIUM_SRC").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME not set");
        format!("{home}/chromium/src")
    });

    let sysroot = format!("{chromium_src}/build/linux/debian_bullseye_amd64-sysroot");
    let out_release = format!("{chromium_src}/out/Release");

    let complete_lib = format!("{out_release}/obj/openui/libopenui_complete.a");
    assert!(
        std::path::Path::new(&complete_lib).exists(),
        "Missing {complete_lib}. Build with: autoninja -C out/Release openui_complete"
    );

    let libcxx = format!(
        "{out_release}/obj/buildtools/third_party/libc++/libc++.a"
    );
    let libcxxabi = format!(
        "{out_release}/obj/buildtools/third_party/libc++abi/libc++abi.a"
    );

    let clang_rt = format!(
        "{chromium_src}/third_party/llvm-build/Release+Asserts/lib/clang/23/\
         lib/x86_64-unknown-linux-gnu/libclang_rt.builtins.a"
    );

    // Sysroot for the linker.
    println!("cargo:rustc-link-arg=--sysroot={sysroot}");

    // Allow duplicate definitions between our Rust std and Chromium's Rust std.
    // Only `rust_eh_personality` overlaps — linker picks the first definition.
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");

    // All static archives in one group to resolve circular dependencies.
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    println!("cargo:rustc-link-arg={complete_lib}");

    // Chromium's Rust subsystem object code (fontations, crabbyavif, etc.)
    // extracted from .rlib files and bundled into a single archive.
    let rust_deps = format!("{out_release}/obj/openui/libchromium_rust_deps.a");
    if std::path::Path::new(&rust_deps).exists() {
        println!("cargo:rustc-link-arg={rust_deps}");

        // Chromium's Rust sysroot — standard library matching Chromium's
        // Rust 1.95-dev toolchain. The .rlib objects above reference symbols
        // from this sysroot that differ from our Rust 1.94 std (different
        // symbol hashes due to ABI instability between versions).
        let chromium_rust_sysroot = format!(
            "{out_release}/local_rustc_sysroot/lib/rustlib/\
             x86_64-unknown-linux-gnu/lib"
        );
        if std::path::Path::new(&chromium_rust_sysroot).is_dir() {
            for entry in std::fs::read_dir(&chromium_rust_sysroot).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) == Some("rlib") {
                    println!("cargo:rustc-link-arg={}", path.display());
                }
            }
        }
    }

    println!("cargo:rustc-link-arg={libcxx}");
    println!("cargo:rustc-link-arg={libcxxabi}");
    println!("cargo:rustc-link-arg={clang_rt}");
    println!("cargo:rustc-link-arg=-Wl,--end-group");

    // Stub library for the Rust alloc shim and any remaining symbols.
    // The stubs source is always alongside this crate's lib.rs.
    let stubs_src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("chromium_rust_stubs.c");
    if stubs_src.exists() {
        let stubs_obj = format!(
            "{}/chromium_rust_stubs.o",
            std::env::var("OUT_DIR").unwrap()
        );
        let status = std::process::Command::new("cc")
            .args(["-c", "-o", &stubs_obj])
            .arg(&stubs_src)
            .arg(format!("--sysroot={sysroot}"))
            .status()
            .expect("failed to compile chromium_rust_stubs.c");
        assert!(status.success(), "chromium_rust_stubs.c compilation failed");
        println!("cargo:rustc-link-arg={stubs_obj}");
        println!("cargo:rerun-if-changed={}", stubs_src.display());
    }

    // System shared libraries.
    let system_libs = [
        "X11",
        "Xcomposite",
        "Xdamage",
        "Xext",
        "Xfixes",
        "Xi",
        "Xrandr",
        "Xrender",
        "Xtst",
        "asound",
        "atk-1.0",
        "atk-bridge-2.0",
        "atomic",
        "atspi",
        "cairo",
        "dbus-1",
        "dl",
        "drm",
        "expat",
        "ffi_pic",
        "fontconfig",
        "freetype",
        "gbm",
        "gio-2.0",
        "glib-2.0",
        "gmodule-2.0",
        "gobject-2.0",
        "gthread-2.0",
        "harfbuzz",
        "m",
        "nspr4",
        "nss3",
        "nssutil3",
        "pango-1.0",
        "pangocairo-1.0",
        "pci",
        "plc4",
        "plds4",
        "pthread",
        "resolv",
        "rt",
        "smime3",
        "udev",
        "uuid",
        "xcb",
        "xkbcommon",
        "xshmfence",
    ];
    println!(
        "cargo:rustc-link-arg=-L{sysroot}/usr/lib/x86_64-linux-gnu"
    );
    for lib in &system_libs {
        println!("cargo:rustc-link-arg=-l{lib}");
    }

    // Rerun triggers.
    println!("cargo:rerun-if-changed={complete_lib}");
    println!("cargo:rerun-if-env-changed=CHROMIUM_SRC");
}
