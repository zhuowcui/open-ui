# Framework test app

This app uses the public `openui` Rust API: declarative `view!` markup, flex
layout, a reactive signal, click handlers, the headless renderer, and the Linux
window runner. Its headless mode sends a pointer click, checks that the counter
and rendered pixels change, then writes an 800×600 PNG.

From `bindings/rust`:

```sh
cargo run --locked --package framework-test -- --headless /tmp/openui-framework-test.png
cargo run --locked --package framework-test --features linux -- --window
```

With no arguments, the app runs headlessly and writes `framework-test.png` in
the current directory. Close the window to end the native run.

On the Chromium-equipped maintainer machine, the host C startup files are not
installed. Use the checked-in sysroot config there:

```sh
cargo --config .cargo/config.chromium.toml run --locked --package framework-test --features linux -- --window
```

If `rust-lld` reports missing `Scrt1.o` or `crti.o` elsewhere, install that
host's C runtime development files before using the ordinary command.
