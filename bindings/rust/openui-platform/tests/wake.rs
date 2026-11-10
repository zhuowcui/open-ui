//! A `LoopWaker` woken on another thread reaches an idle event loop: the
//! window draws once and the loop waits; only the wake can end the test.
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
        ActionRequest, LoopWaker, PlatformApplication, PlatformEvent, SoftwareFrame, TreeUpdate,
        WindowCommand, WindowController, WindowId, WindowOptions,
    };
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    struct Still {
        viewport: Option<ViewportMetrics>,
        presented: Arc<AtomicU32>,
    }

    impl PlatformApplication for Still {
        fn event(&mut self, event: PlatformEvent) -> Result<(), String> {
            match event {
                PlatformEvent::Resized(viewport) => self.viewport = Some(viewport),
                PlatformEvent::Presented { .. } => {
                    self.presented.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
            Ok(())
        }

        fn render(&mut self, _time_ms: f64) -> Result<SoftwareFrame, String> {
            let viewport = self.viewport.ok_or("rendered before the first resize")?;
            let (width, height) = viewport.physical_size();
            Ok(SoftwareFrame {
                width,
                height,
                viewport,
                stride: width as usize * 4,
                pixels: vec![0xff; (width * height * 4) as usize],
            })
        }

        fn exit_requested(&self) -> bool {
            false
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

    struct Controller {
        opened: bool,
        presented: Arc<AtomicU32>,
        /// Set by the waking thread just before it wakes the loop.
        woken: Arc<AtomicBool>,
        /// When the task turn saw `woken`.
        seen: Option<Instant>,
        closing: bool,
        wakes_sent: Arc<AtomicU32>,
    }

    impl WindowController for Controller {
        fn started(&mut self, waker: LoopWaker) {
            let presented = self.presented.clone();
            let woken = self.woken.clone();
            let sent = self.wakes_sent.clone();
            std::thread::spawn(move || {
                // Wait until the window has drawn and the loop is idle.
                while presented.load(Ordering::SeqCst) == 0 {
                    std::thread::sleep(Duration::from_millis(10));
                }
                std::thread::sleep(Duration::from_millis(300));
                woken.store(true, Ordering::SeqCst);
                // Several wakes before the loop runs coalesce.
                for _ in 0..5 {
                    waker.wake();
                    sent.fetch_add(1, Ordering::SeqCst);
                }
            });
        }

        fn take_commands(&mut self) -> Result<Vec<WindowCommand>, String> {
            if !self.opened {
                self.opened = true;
                return Ok(vec![WindowCommand::Open {
                    id: WindowId(1),
                    options: WindowOptions {
                        title: "wake".into(),
                        width: 200.0,
                        height: 120.0,
                        backend: openui_platform::BackendPreference::Software,
                        ..WindowOptions::default()
                    },
                    application: Box::new(Still {
                        viewport: None,
                        presented: self.presented.clone(),
                    }),
                }]);
            }
            if self.seen.is_some() && !self.closing {
                self.closing = true;
                return Ok(vec![WindowCommand::Close(WindowId(1))]);
            }
            Ok(Vec::new())
        }

        fn run_tasks(&mut self) -> Result<bool, String> {
            if self.seen.is_none() && self.woken.load(Ordering::SeqCst) {
                self.seen = Some(Instant::now());
            }
            Ok(false)
        }
    }

    pub fn run() {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(60));
            eprintln!("wake: timed out after 60 s; the wake did not reach the loop");
            std::process::exit(1);
        });
        let presented = Arc::new(AtomicU32::new(0));
        let woken = Arc::new(AtomicBool::new(false));
        let sent = Arc::new(AtomicU32::new(0));
        openui_platform::run_windows(Controller {
            opened: false,
            presented: presented.clone(),
            woken,
            seen: None,
            closing: false,
            wakes_sent: sent.clone(),
        })
        .expect("the event loop ran without error");
        assert!(presented.load(Ordering::SeqCst) >= 1);
        println!(
            "wake: ok ({} frame(s) presented, {} wakes sent)",
            presented.load(Ordering::SeqCst),
            sent.load(Ordering::SeqCst)
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
        eprintln!(
            "wake: OPENUI_REQUIRE_DISPLAY is set but no native runtime or display is available"
        );
        std::process::exit(1);
    }
    println!("wake: skipped (no native runtime or display)");
}
