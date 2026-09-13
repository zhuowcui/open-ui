use std::path::PathBuf;

fn main() {
    let map = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("openui.map");
    println!("cargo:rerun-if-changed={}", map.display());
    if std::env::var_os("CARGO_CFG_TARGET_OS").as_deref() == Some(std::ffi::OsStr::new("linux")) {
        println!(
            "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
            map.display()
        );
    }
}
