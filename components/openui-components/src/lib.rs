//! shadcn/ui components for Open UI.
//!
//! `openui-components` is a working name. Components are written with the
//! public `openui` crate only, with inline styles, until typed style rules
//! and theme tokens land in the engine. Each component's pixels are compared
//! with shadcn/ui rendered by Chromium; see `components/README.md` for the
//! harness and `docs/progress/tracks/components.md` for measured results.

mod separator;
mod theme;

pub use separator::{Orientation, Separator, SeparatorProps};
pub use theme::Theme;

use std::cell::Cell;

thread_local! {
    static CURRENT_THEME: Cell<Theme> = const { Cell::new(Theme::LIGHT) };
}

/// The theme components read while they build their views.
///
/// This stands in for the cascade of shadcn's CSS custom properties: the
/// `.dark` class on the root swaps every token at once. It is replaced by
/// typed theme tokens once they exist.
pub fn current_theme() -> Theme {
    CURRENT_THEME.with(Cell::get)
}

/// Build views with `theme` as the current theme, restoring the previous one
/// afterwards.
pub fn with_theme<R>(theme: Theme, build: impl FnOnce() -> R) -> R {
    struct Restore(Theme);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT_THEME.with(|current| current.set(self.0));
        }
    }
    let _restore = Restore(CURRENT_THEME.with(|current| current.replace(theme)));
    build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_theme_restores_the_previous_theme() {
        assert_eq!(current_theme(), Theme::LIGHT);
        let inner = with_theme(Theme::DARK, current_theme);
        assert_eq!(inner, Theme::DARK);
        assert_eq!(current_theme(), Theme::LIGHT);
    }
}
