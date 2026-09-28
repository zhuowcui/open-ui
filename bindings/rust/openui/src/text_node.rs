//! Mutable text-node handles.

use crate::document::{Document, DocumentInner};
use crate::style::Error;
use openui_engine::{NodeHandle, WeakNode};
use std::rc::{Rc, Weak};

pub struct TextNode {
    pub(crate) document: Document,
    pub(crate) handle: NodeHandle,
    remove_on_drop: bool,
}

#[derive(Clone)]
pub struct WeakTextNode {
    document: Weak<DocumentInner>,
    handle: WeakNode,
}

impl WeakTextNode {
    pub fn upgrade(&self) -> Option<TextNode> {
        let inner = self.document.upgrade()?;
        let document = Document { inner };
        let handle = document
            .with_engine(|engine| self.handle.upgrade(engine))
            .ok()?
            .ok()?;
        Some(TextNode::from_handle(document, handle, false))
    }
}

impl TextNode {
    pub(crate) fn from_handle(
        document: Document,
        handle: NodeHandle,
        remove_on_drop: bool,
    ) -> Self {
        Self {
            document,
            handle,
            remove_on_drop,
        }
    }

    pub fn downgrade(&self) -> WeakTextNode {
        WeakTextNode {
            document: Rc::downgrade(&self.document.inner),
            handle: self.handle.downgrade(),
        }
    }

    pub fn set_data(&self, data: &str) -> Result<(), Error> {
        self.document
            .with_engine_mut(|engine| engine.set_text(self.handle, data.to_owned()))
    }

    pub fn data(&self) -> Result<String, Error> {
        Ok(self
            .document
            .with_engine(|engine| engine.text_data(self.handle).map(str::to_owned))??)
    }

    pub fn parent(&self) -> Result<Option<crate::Element>, Error> {
        let parent = self
            .document
            .with_engine(|engine| engine.parent(self.handle))??;
        Ok(parent.map(|handle| crate::Element::from_handle(self.document.clone(), handle)))
    }

    pub fn remove(mut self) -> Result<(), Error> {
        self.remove_on_drop = false;
        self.document.remove_node(self.handle)
    }
}

impl Drop for TextNode {
    fn drop(&mut self) {
        if self.remove_on_drop {
            let _ = self.document.remove_node(self.handle);
        }
    }
}
