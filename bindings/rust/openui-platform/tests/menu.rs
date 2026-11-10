//! A window's native menu bar shows its entries, and choosing an item
//! reaches that window's application.
//!
//! The test reads the menu through Win32, as an assistive tool would, then
//! posts the item's command as a click does. winit needs the main thread, so
//! this test has its own `main`. Other platforms draw their own menus.

#[cfg(target_os = "windows")]
mod native {
    use openui_geometry::ViewportMetrics;
    use openui_platform::{
        ActionRequest, MenuEntry, NativeWindow, PlatformApplication, PlatformEvent, SoftwareFrame,
        TreeUpdate, WindowCommand, WindowController, WindowId, WindowOptions,
    };
    use std::cell::RefCell;
    use std::rc::Rc;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMenu, GetMenuItemCount, GetMenuItemID, GetMenuState, GetMenuStringW, GetSubMenu,
        PostMessageW, MF_BYPOSITION, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, WM_COMMAND,
    };

    struct Still {
        viewport: Option<ViewportMetrics>,
        chosen: Rc<RefCell<Vec<u32>>>,
        presented: Rc<RefCell<bool>>,
    }

    impl PlatformApplication for Still {
        fn event(&mut self, event: PlatformEvent) -> Result<(), String> {
            match event {
                PlatformEvent::Resized(viewport) => self.viewport = Some(viewport),
                PlatformEvent::Presented { .. } => *self.presented.borrow_mut() = true,
                _ => {}
            }
            Ok(())
        }

        fn menu_selected(&mut self, id: u32) -> Result<(), String> {
            self.chosen.borrow_mut().push(id);
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

    fn entries(save_enabled: bool) -> Vec<MenuEntry> {
        vec![
            MenuEntry::Submenu {
                label: "&File".into(),
                enabled: true,
                entries: vec![
                    MenuEntry::Item {
                        id: 0,
                        label: "&Save".into(),
                        accelerator: Some("Ctrl+S".into()),
                        enabled: save_enabled,
                        checked: None,
                    },
                    MenuEntry::Separator,
                    MenuEntry::Item {
                        id: 1,
                        label: "&Wrap".into(),
                        accelerator: None,
                        enabled: true,
                        checked: Some(true),
                    },
                ],
            },
            MenuEntry::Submenu {
                label: "&Help".into(),
                enabled: true,
                entries: vec![MenuEntry::Item {
                    id: 2,
                    label: "&About".into(),
                    accelerator: None,
                    enabled: true,
                    checked: None,
                }],
            },
        ]
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Step {
        Start,
        Opened,
        Checked,
        Chosen,
        Replaced,
        Done,
    }

    struct Controller {
        step: Step,
        hwnd: Option<HWND>,
        chosen: Rc<RefCell<Vec<u32>>>,
        presented: Rc<RefCell<bool>>,
        report: Rc<RefCell<Vec<String>>>,
    }

    fn label(menu: isize, position: u32) -> String {
        let mut text = [0u16; 128];
        // SAFETY: `text` holds 128 UTF-16 units and `menu` is a live menu.
        let length =
            unsafe { GetMenuStringW(menu as _, position, text.as_mut_ptr(), 128, MF_BYPOSITION) };
        String::from_utf16_lossy(&text[..length.max(0) as usize])
    }

    impl Controller {
        /// Read the menu bar as `label [flags]` lines.
        fn read_menu(&self) -> Vec<String> {
            let hwnd = self.hwnd.expect("the window opened");
            let mut lines = Vec::new();
            // SAFETY: the handles come from the live window and its menu.
            unsafe {
                let bar = GetMenu(hwnd);
                assert!(!bar.is_null(), "the window has a menu bar");
                for top in 0..GetMenuItemCount(bar) as u32 {
                    lines.push(label(bar as isize, top));
                    let submenu = GetSubMenu(bar, top as i32);
                    for position in 0..GetMenuItemCount(submenu) as u32 {
                        let state = GetMenuState(submenu, position, MF_BYPOSITION);
                        let mut line = format!("  {}", label(submenu as isize, position));
                        if state & MF_SEPARATOR != 0 {
                            lines.push("  ---".into());
                            continue;
                        }
                        if state & MF_GRAYED != 0 {
                            line.push_str(" [disabled]");
                        }
                        if state & MF_CHECKED != 0 {
                            line.push_str(" [checked]");
                        }
                        lines.push(line);
                    }
                }
            }
            lines
        }

        /// Post the command of the item at `position` in submenu `top`.
        fn choose(&self, top: i32, position: i32) {
            let hwnd = self.hwnd.expect("the window opened");
            // SAFETY: the handles come from the live window and its menu.
            unsafe {
                let submenu = GetSubMenu(GetMenu(hwnd), top);
                let command = GetMenuItemID(submenu, position);
                assert_ne!(command, u32::MAX, "the item has a command");
                PostMessageW(hwnd, WM_COMMAND, command as usize, 0);
            }
        }
    }

    impl WindowController for Controller {
        fn window_opened(&mut self, _id: WindowId, native: NativeWindow) {
            self.hwnd = native.win32().map(|hwnd| hwnd as HWND);
        }

        fn take_commands(&mut self) -> Result<Vec<WindowCommand>, String> {
            let id = WindowId(1);
            Ok(match self.step {
                Step::Start => {
                    self.step = Step::Opened;
                    vec![
                        WindowCommand::Open {
                            id,
                            options: WindowOptions {
                                title: "Open UI menu test".into(),
                                width: 360.0,
                                height: 200.0,
                                ..WindowOptions::default()
                            },
                            application: Box::new(Still {
                                viewport: None,
                                chosen: self.chosen.clone(),
                                presented: self.presented.clone(),
                            }),
                        },
                        WindowCommand::SetMenu(id, entries(false)),
                    ]
                }
                Step::Opened if *self.presented.borrow() => {
                    self.report.borrow_mut().extend(self.read_menu());
                    self.step = Step::Checked;
                    self.choose(0, 2);
                    Vec::new()
                }
                Step::Checked if !self.chosen.borrow().is_empty() => {
                    self.choose(1, 0);
                    self.step = Step::Chosen;
                    Vec::new()
                }
                Step::Chosen if self.chosen.borrow().len() == 2 => {
                    self.step = Step::Replaced;
                    vec![WindowCommand::SetMenu(id, entries(true))]
                }
                Step::Replaced => {
                    let menu = self.read_menu();
                    self.report
                        .borrow_mut()
                        .push(format!("replaced: {}", menu[1].trim()));
                    self.step = Step::Done;
                    vec![
                        WindowCommand::SetMenu(id, Vec::new()),
                        WindowCommand::Close(id),
                    ]
                }
                _ => Vec::new(),
            })
        }
    }

    pub fn run() {
        let chosen = Rc::new(RefCell::new(Vec::new()));
        let report = Rc::new(RefCell::new(Vec::new()));
        let watchdog = std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(60));
            eprintln!("menu: timed out");
            std::process::exit(1);
        });
        drop(watchdog);
        openui_platform::run_windows(Controller {
            step: Step::Start,
            hwnd: None,
            chosen: chosen.clone(),
            presented: Rc::new(RefCell::new(false)),
            report: report.clone(),
        })
        .expect("the event loop ran without error");
        let report = report.borrow();
        assert_eq!(
            report[..7],
            [
                "&File",
                "  &Save\tCtrl+S [disabled]",
                "  ---",
                "  &Wrap [checked]",
                "&Help",
                "  &About",
                "replaced: &Save\tCtrl+S",
            ]
        );
        assert_eq!(*chosen.borrow(), [1, 2], "choices reached the window's app");
        println!(
            "menu: ok ({} entries read, choices {:?})",
            report.len(),
            chosen.borrow()
        );
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    native::run();
    #[cfg(not(target_os = "windows"))]
    println!("menu: skipped (native menu bars are Windows only)");
}
