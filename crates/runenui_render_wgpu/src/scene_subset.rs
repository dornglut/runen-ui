use core::{error::Error, fmt};

use runenui_core::{Color, LogicalRect, LogicalTransform, ResourceKind, SceneOpacity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnsupportedSceneSemantic {
    Image,
    UnknownPrimitive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SceneValidationError {
    UnsupportedResourceKind {
        resource_kind: ResourceKind,
    },
    UnsupportedItem {
        item_index: usize,
        semantic: UnsupportedSceneSemantic,
    },
}

impl fmt::Display for SceneValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedResourceKind { resource_kind } => write!(
                formatter,
                "renderer does not support scene resource kind {resource_kind:?}"
            ),
            Self::UnsupportedItem {
                item_index,
                semantic,
            } => write!(
                formatter,
                "renderer rejects unsupported scene item {item_index}: {semantic:?}"
            ),
        }
    }
}

impl Error for SceneValidationError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SupportedFillRect {
    pub(crate) rect: LogicalRect,
    pub(crate) color: Color,
    pub(crate) opacity: SceneOpacity,
    pub(crate) local_to_surface: LogicalTransform,
}
