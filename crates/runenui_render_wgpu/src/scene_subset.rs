use core::{error::Error, fmt};

use runenui_core::{
    Color, LogicalRect, LogicalTransform, PaintPrimitive, ResourceKind, SceneOpacity,
};
use runenui_runtime::{PaintPublication, SceneCapabilities};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedSceneSemantic {
    StrokeRect,
    Image,
    ShapedTextRun,
    UnknownPrimitive,
    NonIdentityTransform,
    NonEmptyClips,
    NonUnitItemOpacity,
    NonOpaqueFillColor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneValidationError {
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
pub struct SupportedFillRect {
    pub(crate) rect: LogicalRect,
    pub(crate) color: Color,
}

/// Validates the complete publication before any target or GPU work begins.
///
/// `SceneCapabilities` remains the canonical resource-kind check. The following
/// item walk is deliberately renderer-owned and describes only this temporary
/// implementation checkpoint, not runtime scene capability authority.
pub fn validate_scene_subset(
    publication: &PaintPublication,
) -> Result<Vec<SupportedFillRect>, SceneValidationError> {
    let requirements = publication.scene().requirements();
    let unsupported_resource_kind = SceneCapabilities::default()
        .check_requirements(&requirements)
        .err()
        .map(|error| SceneValidationError::UnsupportedResourceKind {
            resource_kind: error.resource_kind(),
        });

    let fill_rects = publication
        .scene()
        .items()
        .iter()
        .enumerate()
        .map(|(item_index, item)| {
            let (rect, color) = match item.primitive() {
                PaintPrimitive::FillRect { rect, color } => (*rect, *color),
                PaintPrimitive::StrokeRect { .. } => {
                    return Err(unsupported(
                        item_index,
                        UnsupportedSceneSemantic::StrokeRect,
                    ));
                }
                PaintPrimitive::Image(_) => {
                    return Err(unsupported(item_index, UnsupportedSceneSemantic::Image));
                }
                PaintPrimitive::ShapedTextRun(_) => {
                    return Err(unsupported(
                        item_index,
                        UnsupportedSceneSemantic::ShapedTextRun,
                    ));
                }
                _ => {
                    return Err(unsupported(
                        item_index,
                        UnsupportedSceneSemantic::UnknownPrimitive,
                    ));
                }
            };

            if item.local_to_surface() != LogicalTransform::IDENTITY {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonIdentityTransform,
                ));
            }
            if !item.clips().is_empty() {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonEmptyClips,
                ));
            }
            if item.opacity() != SceneOpacity::OPAQUE {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonUnitItemOpacity,
                ));
            }
            if color.alpha() != u8::MAX {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonOpaqueFillColor,
                ));
            }

            Ok(SupportedFillRect { rect, color })
        })
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(error) = unsupported_resource_kind {
        return Err(error);
    }
    Ok(fill_rects)
}

const fn unsupported(
    item_index: usize,
    semantic: UnsupportedSceneSemantic,
) -> SceneValidationError {
    SceneValidationError::UnsupportedItem {
        item_index,
        semantic,
    }
}
