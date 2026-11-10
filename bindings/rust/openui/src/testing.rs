//! Headless UI tests: mount a view, find elements, click and type, run
//! tasks and compare the rendered pixels with accepted snapshots.
//!
//! Enable the `testing` feature for tests only:
//!
//! ```toml
//! [dev-dependencies]
//! openui = { ..., features = ["testing"] }
//! ```
//!
//! ```no_run
//! use openui::prelude::*;
//! use openui::testing::TestApp;
//!
//! let count = create_signal(0);
//! let mut app = TestApp::new(LogicalSize::new(200.0, 100.0)).mount(move || view! {
//!     <button on:click={move |_| count.set(count.get() + 1)}>"Add"</button>
//! });
//! app.click(&app.by_text("Add"));
//! assert_eq!(count.get(), 1);
//! openui::assert_snapshot!(app, "after-one-click");
//! ```
//!
//! Helpers panic with a description of what was missing, as test
//! assertions do.
//!
//! [`assert_snapshot!`](crate::assert_snapshot) compares exact RGBA with
//! `tests/snapshots/<name>.<os>.png` in the calling crate. Snapshots are
//! per operating system because system fonts differ. A missing snapshot
//! fails unless `OPENUI_UPDATE_SNAPSHOTS=1`, which writes or replaces it, so
//! CI never accepts a new image silently. A mismatch writes the rendering
//! next to the snapshot as `<name>.<os>.actual.png`.

use crate::app::{HeadlessApp, LogicalSize};
use crate::element::Element;
use crate::events::{KeyEventType, Modifiers, MouseButton, MouseEventType};
use crate::view_node::IntoView;
use crate::Document;
use openui_engine::ViewportMetrics;
use std::path::{Path, PathBuf};

/// The environment variable that lets snapshot assertions write snapshots.
pub const UPDATE_SNAPSHOTS: &str = "OPENUI_UPDATE_SNAPSHOTS";

/// A headless app under test.
pub struct TestApp {
    app: HeadlessApp,
}

impl TestApp {
    /// An app with a viewport of `size` CSS pixels at scale 1.
    pub fn new(size: LogicalSize) -> Self {
        let viewport = ViewportMetrics::from_logical_size(size.width, size.height, 1.0)
            .expect("a positive, finite test viewport");
        Self {
            app: HeadlessApp::new(viewport).expect("a headless document"),
        }
    }

    /// Mount `view` as the app's root and run the tasks it starts.
    pub fn mount<V: IntoView>(mut self, view: impl FnOnce() -> V) -> Self {
        self.app.mount(view).expect("the view mounts");
        self.run_until_idle();
        self
    }

    pub fn document(&self) -> &Document {
        self.app.document()
    }

    pub fn headless(&mut self) -> &mut HeadlessApp {
        &mut self.app
    }

    /// The element with id `id`.
    pub fn by_id(&self, id: &str) -> Element {
        self.document()
            .element_by_id(id)
            .expect("the document answers")
            .unwrap_or_else(|| panic!("no element has id `{id}`"))
    }

    /// The innermost element whose text, with surrounding white space
    /// trimmed, is `text`.
    pub fn by_text(&self, text: &str) -> Element {
        self.find_text(text)
            .unwrap_or_else(|| panic!("no element has the text `{text}`"))
    }

    /// Like [`by_text`](Self::by_text), but `None` when nothing matches.
    pub fn find_text(&self, text: &str) -> Option<Element> {
        innermost_with_text(&self.document().body(), text.trim())
    }

    /// Press and release the primary button at the centre of `element`, as
    /// a user would. The click reaches whatever is drawn there.
    pub fn click(&mut self, element: &Element) {
        let rect = element
            .bounding_rect()
            .expect("the document answers")
            .unwrap_or_else(|| panic!("the element to click has no box"));
        let (x, y) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        let document = self.document().clone();
        for kind in [
            MouseEventType::Move,
            MouseEventType::Down,
            MouseEventType::Up,
        ] {
            document
                .dispatch_mouse_event(kind, x, y, MouseButton::Left, Modifiers::NONE)
                .expect("the click dispatches");
        }
        self.run_until_idle();
    }

    /// Type `text` into the focused element, one key per character.
    pub fn type_text(&mut self, text: &str) {
        let document = self.document().clone();
        let mut buffer = [0; 4];
        for character in text.chars() {
            let text = &*character.encode_utf8(&mut buffer);
            let code = key_code_of(character);
            document
                .dispatch_key_input(
                    KeyEventType::Down,
                    code,
                    Some(text),
                    Some(text),
                    Modifiers::NONE,
                )
                .expect("the key dispatches");
            document
                .dispatch_key_input(KeyEventType::Up, code, Some(text), None, Modifiers::NONE)
                .expect("the key dispatches");
        }
        self.run_until_idle();
    }

    /// Press and release `key` with no modifiers.
    pub fn press(&mut self, key: Key) {
        self.press_with(Modifiers::NONE, key);
    }

    /// Press and release `key` while holding `modifiers`.
    pub fn press_with(&mut self, modifiers: Modifiers, key: Key) {
        let document = self.document().clone();
        let (code, text) = key.code_and_text();
        let text = text.as_deref();
        document
            .dispatch_key_input(KeyEventType::Down, code, text, text, modifiers)
            .expect("the key dispatches");
        document
            .dispatch_key_input(KeyEventType::Up, code, text, None, modifiers)
            .expect("the key dispatches");
        self.run_until_idle();
    }

    /// Run UI tasks, and the timers and background work they await, until
    /// all are done. Panics after 30 s of unfinished work.
    pub fn run_until_idle(&mut self) {
        self.app.run_until_idle().expect("tasks finish within 30 s");
    }

    /// The current frame as PNG bytes.
    pub fn render_png(&self) -> Vec<u8> {
        self.app.render_png_at(0.0).expect("the frame renders")
    }

    /// Compare the current frame with snapshot `name` in `directory`. Used
    /// by [`assert_snapshot!`](crate::assert_snapshot), which passes the
    /// calling crate's `tests/snapshots`.
    pub fn check_snapshot(
        &self,
        directory: impl AsRef<Path>,
        name: &str,
    ) -> Result<Snapshot, String> {
        let update = std::env::var_os(UPDATE_SNAPSHOTS).is_some_and(|value| !value.is_empty());
        compare_snapshot(directory.as_ref(), name, &self.render_png(), update)
    }
}

/// What a snapshot comparison did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Snapshot {
    /// The frame matched the accepted snapshot exactly.
    Matched,
    /// The frame was written as the snapshot, because updates were allowed.
    Written(PathBuf),
}

/// Compare the frame with `directory/name.<os>.png`.
#[doc(hidden)]
pub fn compare_snapshot(
    directory: &Path,
    name: &str,
    png: &[u8],
    update: bool,
) -> Result<Snapshot, String> {
    let os = std::env::consts::OS;
    let accepted = directory.join(format!("{name}.{os}.png"));
    let actual = directory.join(format!("{name}.{os}.actual.png"));
    let frame = decode(png).map_err(|error| format!("the frame is not a PNG: {error}"))?;
    let write = |path: &Path| {
        std::fs::create_dir_all(directory)
            .and_then(|()| std::fs::write(path, png))
            .map_err(|error| format!("could not write {}: {error}", path.display()))
    };
    let difference = match std::fs::read(&accepted) {
        Ok(bytes) => match decode(&bytes) {
            Ok(expected) => difference(&expected, &frame),
            Err(error) => Some(format!("the snapshot is not a readable PNG: {error}")),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Some("there is no accepted snapshot".to_owned())
        }
        Err(error) => return Err(format!("could not read {}: {error}", accepted.display())),
    };
    match difference {
        None => {
            let _ = std::fs::remove_file(&actual);
            Ok(Snapshot::Matched)
        }
        Some(_) if update => {
            write(&accepted)?;
            let _ = std::fs::remove_file(&actual);
            Ok(Snapshot::Written(accepted))
        }
        Some(difference) => {
            write(&actual)?;
            Err(format!(
                "snapshot `{name}` does not match {}: {difference}. The frame is in {}. \
                 If the change is intended, rerun with {UPDATE_SNAPSHOTS}=1 to accept it.",
                accepted.display(),
                actual.display()
            ))
        }
    }
}

/// Assert that `app`'s current frame matches snapshot `name` in the calling
/// crate's `tests/snapshots` directory. See [`testing`](crate::testing).
#[macro_export]
macro_rules! assert_snapshot {
    ($app:expr, $name:expr $(,)?) => {
        if let Err(message) = $app.check_snapshot(
            ::std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("snapshots"),
            $name,
        ) {
            panic!("{message}");
        }
    };
}

/// A key for [`TestApp::press`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Key {
    /// A character key; it also types the character.
    Char(char),
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    Home,
    End,
    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,
    /// A function key, `Function(1)` to `Function(12)`.
    Function(u8),
}

impl Key {
    fn code_and_text(self) -> (i32, Option<String>) {
        match self {
            Self::Char(character) => (key_code_of(character), Some(character.to_string())),
            Self::Enter => (13, Some("\r".into())),
            Self::Escape => (27, None),
            Self::Tab => (9, Some("\t".into())),
            Self::Space => (32, Some(" ".into())),
            Self::Backspace => (8, None),
            Self::Delete => (46, None),
            Self::Home => (36, None),
            Self::End => (35, None),
            Self::ArrowLeft => (37, None),
            Self::ArrowUp => (38, None),
            Self::ArrowRight => (39, None),
            Self::ArrowDown => (40, None),
            Self::Function(number) => (111 + i32::from(number.clamp(1, 12)), None),
        }
    }
}

/// The key code a character's key reports: its upper-case letter or digit,
/// else 0.
fn key_code_of(character: char) -> i32 {
    if character.is_ascii_alphanumeric() {
        character.to_ascii_uppercase() as i32
    } else if character == ' ' {
        32
    } else {
        0
    }
}

fn innermost_with_text(element: &Element, text: &str) -> Option<Element> {
    let mut child = element.first_child().ok().flatten();
    while let Some(current) = child {
        if let Some(found) = innermost_with_text(&current, text) {
            return Some(found);
        }
        child = current.next_sibling().ok().flatten();
    }
    let own = element.text_content().ok()?;
    (own.trim() == text).then(|| element.clone())
}

struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn decode(png: &[u8]) -> Result<Image, png::DecodingError> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buffer)?;
    buffer.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::GrayscaleAlpha => buffer
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        other => unreachable!("the ALPHA transformation leaves no {other:?}"),
    };
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}

/// How `actual` differs from `expected`, or `None` when they are equal.
fn difference(expected: &Image, actual: &Image) -> Option<String> {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Some(format!(
            "the frame is {}x{}, the snapshot {}x{}",
            actual.width, actual.height, expected.width, expected.height
        ));
    }
    let mut first = None;
    let mut count = 0_u64;
    for (index, (want, got)) in expected
        .rgba
        .chunks_exact(4)
        .zip(actual.rgba.chunks_exact(4))
        .enumerate()
    {
        if want != got {
            count += 1;
            first.get_or_insert((index, want.to_vec(), got.to_vec()));
        }
    }
    let (index, want, got) = first?;
    let (x, y) = (index as u32 % actual.width, index as u32 / actual.width);
    Some(format!(
        "{count} pixels differ; the first, at ({x}, {y}), is {got:?} instead of {want:?}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_signal, view};

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("openui-testing-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        directory
    }

    fn app(color: openui_style::Color) -> TestApp {
        TestApp::new(LogicalSize::new(40.0, 20.0)).mount(move || {
            view! { <div style:width="20px" style:height="10px" style:background-color={color}></div> }
        })
    }

    #[test]
    fn snapshots_are_written_only_on_request_and_compared_exactly() {
        let directory = scratch("snapshots");
        let red = app(openui_style::Color::from_hex(0xff0000, false)).render_png();
        let blue = app(openui_style::Color::from_hex(0x0000ff, false)).render_png();
        let os = std::env::consts::OS;

        let missing = compare_snapshot(&directory, "box", &red, false).unwrap_err();
        assert!(missing.contains("no accepted snapshot"), "{missing}");
        assert!(directory.join(format!("box.{os}.actual.png")).exists());

        let accepted = directory.join(format!("box.{os}.png"));
        assert_eq!(
            compare_snapshot(&directory, "box", &red, true),
            Ok(Snapshot::Written(accepted.clone()))
        );
        assert!(!directory.join(format!("box.{os}.actual.png")).exists());
        assert_eq!(
            compare_snapshot(&directory, "box", &red, false),
            Ok(Snapshot::Matched)
        );

        let changed = compare_snapshot(&directory, "box", &blue, false).unwrap_err();
        assert!(changed.contains("200 pixels differ"), "{changed}");
        assert_eq!(
            std::fs::read(&accepted).unwrap(),
            red,
            "a mismatch keeps the snapshot"
        );
        assert_eq!(
            std::fs::read(directory.join(format!("box.{os}.actual.png"))).unwrap(),
            blue
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn lookups_clicks_and_typing_reach_the_document() {
        let clicks = create_signal(0);
        let mut app = TestApp::new(LogicalSize::new(200.0, 120.0)).mount(move || {
            view! {
                <div>
                    <p id="title">"  Notes  "</p>
                    <button on:click={move |_| clicks.set(clicks.get() + 1)}>
                        <span>"Add"</span>
                    </button>
                    <input id="field" />
                </div>
            }
        });
        assert_eq!(app.by_id("title").text_content().unwrap(), "  Notes  ");
        assert_eq!(
            app.by_text("Notes").get_attribute("id").unwrap().as_deref(),
            Some("title")
        );
        assert!(app.find_text("Missing").is_none());
        let add = app.by_text("Add");
        app.click(&add);
        assert_eq!(clicks.get(), 1, "one press and release is one click");

        let field = app.by_id("field");
        app.click(&field);
        app.type_text("hi 1");
        app.press(Key::Backspace);
        app.press(Key::Char('!'));
        assert_eq!(field.control_value().unwrap().as_deref(), Some("hi !"));
    }
}
