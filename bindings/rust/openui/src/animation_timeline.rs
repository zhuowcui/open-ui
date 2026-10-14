//! Borrowed application inputs for the shared native animation source setter.
use crate::{Document, Element, Error};
use openui_engine::AnimationTimelineSource;
use openui_style::{TimelineAxis, ViewTimelineInset};
use std::rc::Rc;

/// A native clock binding copied during a reassignment call.
///
/// Equivalent source descriptors denote the same native binding. This carrier
/// does not create distinct browser Timeline objects or retain its Element
/// borrows. Attachment range remains owned by the existing animation.
#[derive(Clone, Copy)]
pub enum AnimationTimelineBinding<'a> {
    Document,
    Scroll {
        source: &'a Element,
        axis: TimelineAxis,
    },
    View {
        subject: &'a Element,
        axis: TimelineAxis,
    },
    ViewWithInset {
        subject: &'a Element,
        axis: TimelineAxis,
        inset: ViewTimelineInset,
    },
}

impl AnimationTimelineBinding<'_> {
    pub(crate) fn checked_source(
        self,
        document: &Document,
    ) -> Result<AnimationTimelineSource, Error> {
        let element = match self {
            Self::Document => return Ok(AnimationTimelineSource::Document),
            Self::Scroll { source, .. } => source,
            Self::View { subject, .. } | Self::ViewWithInset { subject, .. } => subject,
        };
        if !Rc::ptr_eq(&element.document.inner, &document.inner) {
            return Err(openui_engine::EngineError::WrongDocument.into());
        }
        Ok(match self {
            Self::Document => unreachable!("handled before owner check"),
            Self::Scroll { source, axis } => AnimationTimelineSource::Scroll {
                source: source.handle,
                axis,
            },
            Self::View { subject, axis } => AnimationTimelineSource::View {
                subject: subject.handle,
                axis,
            },
            Self::ViewWithInset {
                subject,
                axis,
                inset,
            } => AnimationTimelineSource::ViewWithInset {
                subject: subject.handle,
                axis,
                inset,
            },
        })
    }
}
