//! Two native windows on one event loop: both open and present, closing one
//! keeps the other running, opening an open id focuses it, and the loop
//! exits after the last window closes.
//!
//! winit needs the main thread, so this test has its own `main`. Without a
//! display it reports a skip, unless `OPENUI_REQUIRE_DISPLAY` is set.

#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
mod native {
    use openui_geometry::ViewportMetrics;
    use openui_platform::{
        ActionRequest, BackendPreference, PlatformApplication, PlatformEvent, SoftwareFrame,
        TreeUpdate, WindowCommand, WindowController, WindowId, WindowOptions,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Log {
        events: Vec<(u64, &'static str)>,
        presented: [u64; 3],
        closed: Vec<u64>,
        dropped: Vec<u64>,
        open_counts_seen: Vec<usize>,
    }

    struct Painter {
        id: u64,
        log: Rc<RefCell<Log>>,
        viewport: Option<ViewportMetrics>,
    }

    impl Drop for Painter {
        fn drop(&mut self) {
            self.log.borrow_mut().dropped.push(self.id);
        }
    }

    impl PlatformApplication for Painter {
        fn event(&mut self, event: PlatformEvent) -> Result<(), String> {
            let name = match event {
                PlatformEvent::BackendChanged(_) => "backend",
                PlatformEvent::Resized(viewport) => {
                    self.viewport = Some(viewport);
                    "resized"
                }
                PlatformEvent::Presented { .. } => {
                    self.log.borrow_mut().presented[self.id as usize] += 1;
                    "presented"
                }
                PlatformEvent::CloseRequested => "close",
                _ => return Ok(()),
            };
            self.log.borrow_mut().events.push((self.id, name));
            Ok(())
        }

        fn render(&mut self, _time_ms: f64) -> Result<SoftwareFrame, String> {
            let viewport = self.viewport.ok_or("rendered before the first resize")?;
            let (width, height) = (viewport.physical_width(), viewport.physical_height());
            let shade = if self.id == 1 { 0x40 } else { 0xc0 };
            Ok(SoftwareFrame {
                width,
                height,
                viewport,
                stride: width as usize * 4,
                pixels: [shade, 0x80, 0xff - shade, 0xff].repeat((width * height) as usize),
            })
        }

        fn exit_requested(&self) -> bool {
            false
        }

        /// Keep presenting, so the survivor shows the loop still runs.
        fn is_animating(&self) -> bool {
            true
        }

        fn accessibility_update(&mut self) -> Result<TreeUpdate, String> {
            Ok(TreeUpdate {
                nodes: Vec::new(),
                tree: None,
                tree_id: accesskit::TreeId::ROOT,
                focus: accesskit::NodeId(0),
            })
        }

        fn accessibility_action(&mut self, _request: ActionRequest) -> Result<(), String> {
            Ok(())
        }
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Step {
        Start,
        BothOpen,
        ClosingFirst,
        /// Window 1 closed when window 2 had presented this many frames.
        FirstClosed(u64),
        Done,
    }

    /// Opens two windows; once both have presented, closes the first; once
    /// it is gone and the second has presented again, closes the second.
    struct Script {
        step: Step,
        log: Rc<RefCell<Log>>,
        backend: BackendPreference,
    }

    impl Script {
        fn open(&self, id: u64, title: &str) -> WindowCommand {
            WindowCommand::Open {
                id: WindowId(id),
                options: WindowOptions {
                    title: title.into(),
                    width: 320.0,
                    height: 200.0,
                    backend: self.backend,
                    min_size: Some((160.0, 100.0)),
                    ..WindowOptions::default()
                },
                application: Box::new(Painter {
                    id,
                    log: self.log.clone(),
                    viewport: None,
                }),
            }
        }
    }

    impl WindowController for Script {
        fn take_commands(&mut self) -> Result<Vec<WindowCommand>, String> {
            let presented = self.log.borrow().presented;
            Ok(match self.step {
                Step::Start => {
                    self.step = Step::BothOpen;
                    vec![self.open(1, "first"), self.open(2, "second")]
                }
                Step::BothOpen if presented[1] > 0 && presented[2] > 0 => {
                    self.step = Step::ClosingFirst;
                    vec![
                        // Already open: focuses window 2 and drops this app.
                        self.open(2, "duplicate"),
                        WindowCommand::SetTitle(WindowId(2), "second, retitled".into()),
                        WindowCommand::Close(WindowId(1)),
                    ]
                }
                Step::FirstClosed(frames) if presented[2] > frames => {
                    self.step = Step::Done;
                    vec![WindowCommand::Close(WindowId(2))]
                }
                _ => Vec::new(),
            })
        }

        fn window_closed(&mut self, id: WindowId) -> Result<(), String> {
            let mut log = self.log.borrow_mut();
            log.closed.push(id.0);
            if id.0 == 1 {
                self.step = Step::FirstClosed(log.presented[2]);
            }
            Ok(())
        }

        fn exit_requested(&self, open_windows: usize) -> bool {
            let mut log = self.log.borrow_mut();
            if log.open_counts_seen.last() != Some(&open_windows) {
                log.open_counts_seen.push(open_windows);
            }
            open_windows == 0
        }
    }

    pub fn run() {
        let backend = match std::env::var("OUI_BACKEND").as_deref() {
            Ok("opengl") => BackendPreference::OpenGl,
            Ok("software") => BackendPreference::Software,
            _ => BackendPreference::Auto,
        };
        // A hung event loop must fail the test rather than the CI job.
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(60));
            eprintln!("multi_window: timed out after 60 s");
            std::process::exit(1);
        });
        let log = Rc::new(RefCell::new(Log::default()));
        openui_platform::run_windows(Script {
            step: Step::Start,
            log: log.clone(),
            backend,
        })
        .expect("the event loop ran without error");

        let log = log.borrow();
        for id in [1, 2] {
            let names: Vec<_> = log
                .events
                .iter()
                .filter(|(window, _)| *window == id)
                .map(|(_, name)| *name)
                .collect();
            assert_eq!(names[..2], ["backend", "resized"], "window {id}: {names:?}");
            assert!(names.contains(&"presented"), "window {id}: {names:?}");
        }
        assert_eq!(log.closed, [1, 2], "windows close one at a time");
        // The duplicate open's app is dropped first, unused; each window's
        // app is dropped when it closes.
        assert_eq!(log.dropped, [2, 1, 2]);
        assert_eq!(
            log.open_counts_seen
                .iter()
                .copied()
                .filter(|&n| n > 0)
                .max(),
            Some(2)
        );
        assert!(
            log.open_counts_seen.contains(&1),
            "{:?}",
            log.open_counts_seen
        );
        assert_eq!(log.open_counts_seen.last(), Some(&0));
        println!(
            "multi_window: ok ({:?} backend, frames presented {:?})",
            backend,
            &log.presented[1..]
        );
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
        eprintln!("multi_window: OPENUI_REQUIRE_DISPLAY is set but no native runtime or display is available");
        std::process::exit(1);
    }
    println!("multi_window: skipped (no native runtime or display)");
}
