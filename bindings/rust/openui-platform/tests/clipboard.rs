//! The runtime's native clipboard round-trips text and is restored after.
//!
//! winit needs the main thread, so this test has its own `main`. Without a
//! display it reports a skip, unless `OPENUI_REQUIRE_DISPLAY` is set.
//! Wayland lets a client set the clipboard only in response to input, which
//! a headless compositor never sends, so there a failed round trip is
//! reported rather than failing the test.

#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
mod native {
    use openui_platform::{Clipboard, WindowCommand, WindowController};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Controller(Rc<RefCell<Option<Result<String, String>>>>);

    impl WindowController for Controller {
        fn clipboard(&mut self, clipboard: Clipboard) {
            let previous = clipboard.read_text().ok();
            let marker = format!("openui clipboard test {} ✓ ü 中", std::process::id());
            clipboard.write_text(marker.clone());
            let read = clipboard
                .read_text()
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    if text == marker {
                        Ok(text)
                    } else {
                        Err(format!("read back {text:?}"))
                    }
                });
            if let Some(previous) = previous {
                clipboard.write_text(previous);
            }
            *self.0.borrow_mut() = Some(read);
        }

        fn take_commands(&mut self) -> Result<Vec<WindowCommand>, String> {
            Ok(Vec::new())
        }
    }

    pub fn run() {
        let result = Rc::new(RefCell::new(None));
        // With no windows the loop exits as soon as it starts.
        openui_platform::run_windows(Controller(result.clone()))
            .expect("the event loop ran without error");
        let result = result
            .borrow_mut()
            .take()
            .expect("the controller got the clipboard");
        let wayland = cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some();
        match result {
            Ok(text) => println!("clipboard: ok (round-tripped {text:?})"),
            Err(error) if wayland => {
                println!("clipboard: Wayland round trip unavailable without input ({error})")
            }
            Err(error) => panic!("clipboard round trip failed: {error}"),
        }
    }
}

fn main() {
    #[cfg(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    ))]
    {
        let has_display = cfg!(target_os = "windows")
            || std::env::var_os("DISPLAY").is_some()
            || std::env::var_os("WAYLAND_DISPLAY").is_some();
        if has_display {
            native::run();
            return;
        }
    }
    if std::env::var_os("OPENUI_REQUIRE_DISPLAY").is_some() {
        eprintln!(
            "clipboard: OPENUI_REQUIRE_DISPLAY is set but no native runtime or display is available"
        );
        std::process::exit(1);
    }
    println!("clipboard: skipped (no native runtime or display)");
}
