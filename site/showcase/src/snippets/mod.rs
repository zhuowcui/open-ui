//! Code on the site that opens windows, menus, dialogs or the network, so it
//! cannot be drawn headless. Each file is shown whole on the site
//! (`site/src/components/Snippet.astro`); compiling this binary, and its
//! tests (`cargo test --no-run`, which `render.ps1` runs), checks every one
//! against the checked-out `openui`. Nothing here is called.
#![allow(dead_code)]

pub mod app_minimal;
pub mod async_tasks;
pub mod clipboard;
pub mod dialogs;
pub mod fragments;
pub mod headless;
pub mod http;
pub mod menus;
pub mod testing;
pub mod windows;
