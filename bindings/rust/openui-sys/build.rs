fn main() {
    let chromium_src = std::env::var("CHROMIUM_SRC").unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME not set");
        format!("{home}/chromium/src")
    });

    let out_release = format!("{chromium_src}/out/Release");

    // Verify the complete library exists.
    let complete_lib = format!("{out_release}/obj/openui/libopenui_complete.a");
    assert!(
        std::path::Path::new(&complete_lib).exists(),
        "Missing {complete_lib}. Build with: autoninja -C out/Release openui_complete"
    );

    // Emit link flags for test/bench binaries compiled from this crate.
    openui_build::link();

    // Rerun triggers.
    println!("cargo:rerun-if-changed={complete_lib}");
    println!("cargo:rerun-if-env-changed=CHROMIUM_SRC");
}
