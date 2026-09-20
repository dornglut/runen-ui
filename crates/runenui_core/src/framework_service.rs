//! Host-neutral requests for framework-owned services.

use core::fmt;
use std::{num::NonZeroU32, sync::Arc};

use crate::{
    CompositionGeneration, EditingSessionGeneration, LogicalRect, MountedNodeId, SurfaceId,
    TextDocumentSnapshot, TextSelection, WorkSequence,
};

/// Lifecycle phase of one host-originated drag/drop offer.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DragDropPhase {
    Hover,
    Drop,
    Cancel,
}

/// Neutral class of a drag/drop payload; never includes a filename or native handle.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DragDropPayloadKind {
    Files,
    Text,
    Opaque,
}

/// Bounded metadata for a drag/drop payload held by the host.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DragDropPayloadMetadata {
    kind: DragDropPayloadKind,
    item_count: NonZeroU32,
    total_bytes: Option<u64>,
}

impl DragDropPayloadMetadata {
    #[must_use]
    pub const fn new(
        kind: DragDropPayloadKind,
        item_count: NonZeroU32,
        total_bytes: Option<u64>,
    ) -> Self {
        Self {
            kind,
            item_count,
            total_bytes,
        }
    }

    #[must_use]
    pub const fn kind(self) -> DragDropPayloadKind {
        self.kind
    }

    #[must_use]
    pub const fn item_count(self) -> NonZeroU32 {
        self.item_count
    }

    #[must_use]
    pub const fn total_bytes(self) -> Option<u64> {
        self.total_bytes
    }
}

/// Clipboard operation requested by an editable-text default.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClipboardWritePurpose {
    Copy,
    Cut,
}

/// Host-neutral cursor vocabulary.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CursorShape {
    Default,
    Text,
    Pointer,
    Crosshair,
    Move,
    NotAllowed,
    Wait,
    Progress,
    ResizeHorizontal,
    ResizeVertical,
    ResizeDiagonalNorthwestSoutheast,
    ResizeDiagonalNortheastSouthwest,
    Grab,
    Grabbing,
}

/// Classification attached to text returned by a host clipboard.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClipboardClassification {
    Unclassified,
    Public,
    Sensitive,
}

/// Typed framework-service failure. Platform error strings and payloads are omitted.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameworkServiceFailure {
    Unavailable,
    PermissionDenied,
    UserActivationRequired,
    Rejected,
    Failed,
}

/// One bounded, typed host-neutral service request.
#[non_exhaustive]
#[derive(Clone, PartialEq)]
pub enum FrameworkServiceRequest {
    ClipboardReadText {
        max_bytes: usize,
    },
    ClipboardWriteText {
        text: Arc<str>,
        purpose: ClipboardWritePurpose,
    },
    InputMethod {
        enabled: bool,
        candidate_area: Option<LogicalRect>,
        composition: Option<CompositionGeneration>,
    },
    Cursor {
        shape: CursorShape,
        visible: bool,
    },
    DragDrop {
        source: WorkSequence,
        phase: DragDropPhase,
        payload: DragDropPayloadMetadata,
        accepted: bool,
    },
}

impl FrameworkServiceRequest {
    #[must_use]
    pub const fn response_kind(&self) -> FrameworkServiceResponseKind {
        match self {
            Self::ClipboardReadText { .. } => FrameworkServiceResponseKind::ClipboardReadText,
            Self::ClipboardWriteText { .. } => FrameworkServiceResponseKind::ClipboardWriteText,
            Self::InputMethod { .. } => FrameworkServiceResponseKind::InputMethod,
            Self::Cursor { .. } => FrameworkServiceResponseKind::Cursor,
            Self::DragDrop { .. } => FrameworkServiceResponseKind::DragDrop,
        }
    }

    /// Returns the text only for a clipboard write request.
    #[must_use]
    pub fn clipboard_write_text(&self) -> Option<&str> {
        match self {
            Self::ClipboardWriteText { text, .. } => Some(text),
            _ => None,
        }
    }
}

/// Exact runtime facts that scope one framework-service request.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameworkServiceBinding {
    owner: MountedNodeId,
    surface: SurfaceId,
    surface_context: Option<crate::SurfaceInputContext>,
    editing_session: Option<EditingSessionGeneration>,
    document_snapshot: Option<TextDocumentSnapshot>,
    selection: Option<TextSelection>,
    composition: Option<CompositionGeneration>,
}

impl FrameworkServiceBinding {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        owner: MountedNodeId,
        surface: SurfaceId,
        surface_context: Option<crate::SurfaceInputContext>,
        editing_session: Option<EditingSessionGeneration>,
        document_snapshot: Option<TextDocumentSnapshot>,
        selection: Option<TextSelection>,
        composition: Option<CompositionGeneration>,
    ) -> Self {
        Self {
            owner,
            surface,
            surface_context,
            editing_session,
            document_snapshot,
            selection,
            composition,
        }
    }

    #[must_use]
    pub const fn owner(&self) -> &MountedNodeId {
        &self.owner
    }

    #[must_use]
    pub const fn surface(&self) -> &SurfaceId {
        &self.surface
    }

    #[must_use]
    pub const fn surface_context(&self) -> Option<&crate::SurfaceInputContext> {
        self.surface_context.as_ref()
    }

    #[must_use]
    pub const fn editing_session(&self) -> Option<&EditingSessionGeneration> {
        self.editing_session.as_ref()
    }

    #[must_use]
    pub const fn document_snapshot(&self) -> Option<TextDocumentSnapshot> {
        self.document_snapshot
    }

    #[must_use]
    pub const fn selection(&self) -> Option<TextSelection> {
        self.selection
    }

    #[must_use]
    pub const fn composition(&self) -> Option<&CompositionGeneration> {
        self.composition.as_ref()
    }
}

impl fmt::Debug for FrameworkServiceRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClipboardReadText { max_bytes } => formatter
                .debug_struct("ClipboardReadText")
                .field("max_bytes", max_bytes)
                .finish(),
            Self::ClipboardWriteText { text, purpose } => formatter
                .debug_struct("ClipboardWriteText")
                .field("purpose", purpose)
                .field("text_bytes", &text.len())
                .finish(),
            Self::InputMethod {
                enabled,
                candidate_area,
                composition,
            } => formatter
                .debug_struct("InputMethod")
                .field("enabled", enabled)
                .field("candidate_area", candidate_area)
                .field("has_composition", &composition.is_some())
                .finish(),
            Self::Cursor { shape, visible } => formatter
                .debug_struct("Cursor")
                .field("shape", shape)
                .field("visible", visible)
                .finish(),
            Self::DragDrop {
                source,
                phase,
                payload,
                accepted,
            } => formatter
                .debug_struct("DragDrop")
                .field("source", source)
                .field("phase", phase)
                .field("payload", payload)
                .field("accepted", accepted)
                .finish(),
        }
    }
}

/// Expected typed response for one service request kind.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameworkServiceResponseKind {
    ClipboardReadText,
    ClipboardWriteText,
    InputMethod,
    Cursor,
    DragDrop,
}

/// Text returned by a host clipboard, with payload-redacting diagnostics.
#[derive(Clone, Eq, PartialEq)]
pub struct ClipboardText {
    text: Arc<str>,
    classification: ClipboardClassification,
}

impl ClipboardText {
    #[must_use]
    pub fn new(text: impl Into<Arc<str>>, classification: ClipboardClassification) -> Self {
        Self {
            text: text.into(),
            classification,
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn classification(&self) -> ClipboardClassification {
        self.classification
    }
}

impl fmt::Debug for ClipboardText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClipboardText")
            .field("classification", &self.classification)
            .field("text_bytes", &self.text.len())
            .finish()
    }
}

/// Result returned by the host for one framework-service request.
#[non_exhaustive]
#[derive(Clone, Eq, PartialEq)]
pub enum FrameworkServiceResponse {
    ClipboardReadText(Result<ClipboardText, FrameworkServiceFailure>),
    ClipboardWriteText(Result<(), FrameworkServiceFailure>),
    InputMethod(Result<(), FrameworkServiceFailure>),
    Cursor(Result<(), FrameworkServiceFailure>),
    DragDrop(Result<(), FrameworkServiceFailure>),
}

impl FrameworkServiceResponse {
    #[must_use]
    pub const fn kind(&self) -> FrameworkServiceResponseKind {
        match self {
            Self::ClipboardReadText(_) => FrameworkServiceResponseKind::ClipboardReadText,
            Self::ClipboardWriteText(_) => FrameworkServiceResponseKind::ClipboardWriteText,
            Self::InputMethod(_) => FrameworkServiceResponseKind::InputMethod,
            Self::Cursor(_) => FrameworkServiceResponseKind::Cursor,
            Self::DragDrop(_) => FrameworkServiceResponseKind::DragDrop,
        }
    }
}

impl fmt::Debug for FrameworkServiceResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClipboardReadText(Ok(text)) => formatter
                .debug_tuple("ClipboardReadText")
                .field(text)
                .finish(),
            Self::ClipboardReadText(Err(error)) => formatter
                .debug_tuple("ClipboardReadText")
                .field(error)
                .finish(),
            Self::ClipboardWriteText(result) => formatter
                .debug_tuple("ClipboardWriteText")
                .field(result)
                .finish(),
            Self::InputMethod(result) => {
                formatter.debug_tuple("InputMethod").field(result).finish()
            }
            Self::Cursor(result) => formatter.debug_tuple("Cursor").field(result).finish(),
            Self::DragDrop(result) => formatter.debug_tuple("DragDrop").field(result).finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClipboardClassification, ClipboardText, ClipboardWritePurpose, FrameworkServiceFailure,
        FrameworkServiceRequest, FrameworkServiceResponse, FrameworkServiceResponseKind,
    };

    #[test]
    fn clipboard_protocol_is_typed_and_debug_redacts_payloads() {
        let secret = "sensitive clipboard phrase";
        let request = FrameworkServiceRequest::ClipboardWriteText {
            text: secret.into(),
            purpose: ClipboardWritePurpose::Cut,
        };
        assert_eq!(
            request.response_kind(),
            FrameworkServiceResponseKind::ClipboardWriteText
        );
        assert!(!format!("{request:?}").contains(secret));
        assert_eq!(request.clipboard_write_text(), Some(secret));

        let response = FrameworkServiceResponse::ClipboardReadText(Ok(ClipboardText::new(
            secret,
            ClipboardClassification::Sensitive,
        )));
        assert_eq!(
            response.kind(),
            FrameworkServiceResponseKind::ClipboardReadText
        );
        assert!(!format!("{response:?}").contains(secret));
        assert_eq!(
            FrameworkServiceResponse::ClipboardWriteText(Err(
                FrameworkServiceFailure::PermissionDenied
            ))
            .kind(),
            FrameworkServiceResponseKind::ClipboardWriteText
        );
    }
}
