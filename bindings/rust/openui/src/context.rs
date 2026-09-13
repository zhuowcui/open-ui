//! Thread-local render context used by `view!` and reactive effects.

use crate::Document;
use std::cell::RefCell;

thread_local! {
    static RENDER_DOC: RefCell<Option<Document>> = const { RefCell::new(None) };
}

/// Run a closure with a cloneable, lifetime-safe current document.
pub fn with_document<R>(document: &Document, operation: impl FnOnce() -> R) -> R {
    RENDER_DOC.with(|current| {
        let previous = current.replace(Some(document.clone()));
        struct Restore<'a> {
            slot: &'a RefCell<Option<Document>>,
            previous: Option<Document>,
        }
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.slot.replace(self.previous.take());
            }
        }
        let _restore = Restore {
            slot: current,
            previous,
        };
        operation()
    })
}

/// Clone the current document for use by generated view code.
pub fn current_document() -> Document {
    RENDER_DOC.with(|current| {
        current
            .borrow()
            .clone()
            .expect("current_document() called outside with_document()")
    })
}
