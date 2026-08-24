use core::{error::Error, fmt};

use runenui_core::{
    Color, LogicalRect, LogicalTransform, PaintPrimitive, ResourceKind, SceneOpacity,
};
use runenui_runtime::{PaintPublication, PaintSceneItem, SceneCapabilities};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedSceneSemantic {
    StrokeRect,
    Image,
    ShapedTextRun,
    UnknownPrimitive,
    NonEmptyClips,
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
    pub(crate) opacity: SceneOpacity,
    pub(crate) local_to_surface: LogicalTransform,
}

/// One renderer-admitted literal rectangle item represented by a color-bearing
/// rectangle plus an optional inner rectangle that must be excluded.
///
/// Ordinary fills and centered strokes whose inset collapses have no inner
/// exclusion. A non-collapsed stroke retains the exact accepted f32 inset from
/// the independent M6 literal-paint oracle rather than reconstructing a backend
/// line primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SupportedLiteralRect {
    pub(crate) fill: SupportedFillRect,
    pub(crate) stroke_inset: Option<LogicalRect>,
}

pub(crate) fn publication_resource_error(
    publication: &PaintPublication,
) -> Option<SceneValidationError> {
    let requirements = publication.scene().requirements();
    SceneCapabilities::default()
        .check_requirements(&requirements)
        .err()
        .map(|error| SceneValidationError::UnsupportedResourceKind {
            resource_kind: error.resource_kind(),
        })
}

/// Validates one literal fill/stroke primitive without applying the temporary
/// base-renderer clip/stroke subset gate.
///
/// A centered stroke follows the accepted M6 independent compositor exactly:
/// zero width/area and checked derived-rectangle overflow have no coverage;
/// otherwise the color geometry is the checked expanded rectangle and a valid
/// non-collapsed inset is retained as an exclusion mask.
pub(crate) const fn validate_literal_rect_item(
    item_index: usize,
    item: &PaintSceneItem,
) -> Result<Option<SupportedLiteralRect>, SceneValidationError> {
    match item.primitive() {
        PaintPrimitive::FillRect { rect, color } => Ok(Some(SupportedLiteralRect {
            fill: supported_fill_rect(item, *rect, *color),
            stroke_inset: None,
        })),
        PaintPrimitive::StrokeRect { rect, color, width } => Ok(supported_stroke_rect(
            item,
            *rect,
            *color,
            width.get(),
        )),
        PaintPrimitive::Image(_) => Err(unsupported(item_index, UnsupportedSceneSemantic::Image)),
        PaintPrimitive::ShapedTextRun(_) => Err(unsupported(
            item_index,
            UnsupportedSceneSemantic::ShapedTextRun,
        )),
        _ => Err(unsupported(
            item_index,
            UnsupportedSceneSemantic::UnknownPrimitive,
        )),
    }
}

const fn supported_fill_rect(
    item: &PaintSceneItem,
    rect: LogicalRect,
    color: Color,
) -> SupportedFillRect {
    SupportedFillRect {
        rect,
        color,
        opacity: item.opacity(),
        local_to_surface: item.local_to_surface(),
    }
}

const fn supported_stroke_rect(
    item: &PaintSceneItem,
    rect: LogicalRect,
    color: Color,
    width: f32,
) -> Option<SupportedLiteralRect> {
    if width == 0.0 || rect.width() == 0.0 || rect.height() == 0.0 {
        return None;
    }

    let half = width / 2.0;
    let expanded = match LogicalRect::try_new(
        rect.x() - half,
        rect.y() - half,
        rect.width() + width,
        rect.height() + width,
    ) {
        Ok(expanded) => expanded,
        Err(_) => return None,
    };

    let stroke_inset = if rect.width() <= width || rect.height() <= width {
        None
    } else {
        match LogicalRect::try_new(
            rect.x() + half,
            rect.y() + half,
            rect.width() - width,
            rect.height() - width,
        ) {
            Ok(inset) => Some(inset),
            Err(_) => return None,
        }
    };

    Some(SupportedLiteralRect {
        fill: supported_fill_rect(item, expanded, color),
        stroke_inset,
    })
}

pub(crate) const fn validate_fill_rect_item(
    item_index: usize,
    item: &PaintSceneItem,
) -> Result<SupportedFillRect, SceneValidationError> {
    if matches!(item.primitive(), PaintPrimitive::StrokeRect { .. }) {
        return Err(unsupported(
            item_index,
            UnsupportedSceneSemantic::StrokeRect,
        ));
    }
    match validate_literal_rect_item(item_index, item) {
        Ok(Some(literal)) => Ok(literal.fill),
        Ok(None) => Err(unsupported(
            item_index,
            UnsupportedSceneSemantic::StrokeRect,
        )),
        Err(error) => Err(error),
    }
}

/// Validates the complete publication before any target or GPU work begins.
///
/// `SceneCapabilities` remains the canonical resource-kind check. The following
/// item walk is deliberately renderer-owned and describes only this temporary
/// implementation checkpoint, not runtime scene capability authority.
pub fn validate_scene_subset(
    publication: &PaintPublication,
) -> Result<Vec<SupportedFillRect>, SceneValidationError> {
    let unsupported_resource_kind = publication_resource_error(publication);

    let fill_rects = publication
        .scene()
        .items()
        .iter()
        .enumerate()
        .map(|(item_index, item)| {
            let fill = validate_fill_rect_item(item_index, item)?;
            if !item.clips().is_empty() {
                return Err(unsupported(
                    item_index,
                    UnsupportedSceneSemantic::NonEmptyClips,
                ));
            }
            Ok(fill)
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
