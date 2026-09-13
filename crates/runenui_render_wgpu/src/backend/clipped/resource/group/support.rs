//! Renderer-private symbolic neutral effect-support reconstruction.
//!
//! ADR 0015 separates ordinary-shadow support from realized color alpha. This
//! module therefore retains only renderer-neutral publication geometry and exact
//! set operations. It deliberately carries no brush/foreground alpha, image
//! payload alpha, item/group opacity, antialiasing, MSDF samples, raster scale,
//! target extent, or cache/device state. Disposable raster realization consumes
//! this symbolic support directly; nested groups still propagate the expression
//! rather than rendered pixels.

use std::{collections::HashMap, sync::Arc};

use runenui_core::{
    LogicalPoint, LogicalRect, LogicalTransform, PaintPrimitive, ResourceRef, ScenePath,
    SceneShape, StrokeStyle,
};
use runenui_runtime::{PaintScene, PaintSceneItem, SceneClip};

use crate::scene_subset::UnsupportedSceneSemantic;

use super::super::super::shaped_outline::{
    OutlineResolveFailure, UnsupportedOutlineKind, resolve_positioned_paths,
};
use super::super::{PublicationRenderError, UnsupportedShapedGlyphKind};

/// Renderer-private scalar facts frozen from one runtime-published ordinary shadow.
///
/// The renderer does not retain the authored/runtime `DropShadow` vocabulary as a
/// second behavior authority. Only the geometry needed by ADR 0015's neutral
/// support operation crosses this private realization seam. Shadow color is
/// intentionally absent because it cannot change neutral support.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NeutralShadowFacts {
    spread: f64,
    offset_x: f64,
    offset_y: f64,
    blur_square_half_extent: f64,
}

impl NeutralShadowFacts {
    pub(super) fn new(offset_x: f32, offset_y: f32, sigma: f32, spread: f32) -> Self {
        Self {
            spread: f64::from(spread),
            offset_x: f64::from(offset_x),
            offset_y: f64::from(offset_y),
            blur_square_half_extent: f64::from(sigma) * 3.0,
        }
    }
}

/// One direct primitive's alpha-independent neutral support authority.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum NeutralPrimitiveSupport {
    Fill {
        shape: SceneShape,
        local_to_surface: LogicalTransform,
    },
    Stroke {
        shape: SceneShape,
        style: StrokeStyle,
        local_to_surface: LogicalTransform,
    },
    Image {
        destinations: Arc<[LogicalRect]>,
        local_to_surface: LogicalTransform,
    },
    ShapedText {
        item_index: usize,
        resource: ResourceRef,
        origin: LogicalPoint,
        local_to_surface: LogicalTransform,
    },
    ShapedTextPaths {
        paths: Arc<[ScenePath]>,
        local_to_surface: LogicalTransform,
    },
}

/// Exact symbolic neutral-support set used by ordinary-shadow preparation.
///
/// `Shadow` denotes ADR 0015's support operation only: signed Euclidean spread,
/// then offset, then Minkowski sum with the closed axis-aligned `3 * sigma`
/// square. Shadow color is intentionally absent because alpha cannot shrink
/// neutral support. `Clip` is outside child-plus-shadow union, matching group
/// effect ordering.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum NeutralSupport {
    Empty,
    Primitive(NeutralPrimitiveSupport),
    Union(Arc<[Arc<Self>]>),
    Shadow {
        source: Arc<Self>,
        spread: f64,
        offset_x: f64,
        offset_y: f64,
        blur_square_half_extent: f64,
    },
    Clip {
        source: Arc<Self>,
        clips: Arc<[SceneClip]>,
    },
}

impl NeutralSupport {
    /// Reconstructs one published item's exact neutral support without observing
    /// any realized source alpha or item opacity. Shaped text keeps only its retained
    /// resource identity until an actual shadow consumes that support.
    pub(super) fn from_item(
        item_index: usize,
        item: &PaintSceneItem,
    ) -> Result<Arc<Self>, UnsupportedSceneSemantic> {
        let local_to_surface = item.local_to_surface();
        let primitive = match item.primitive() {
            PaintPrimitive::Fill { shape, .. } => NeutralPrimitiveSupport::Fill {
                shape: shape.clone(),
                local_to_surface,
            },
            PaintPrimitive::Stroke { shape, style, .. } => NeutralPrimitiveSupport::Stroke {
                shape: shape.clone(),
                style: *style,
                local_to_surface,
            },
            PaintPrimitive::Image(image) => {
                let patch_count = image
                    .resolved_patch_count()
                    .ok_or(UnsupportedSceneSemantic::Image)?;
                let destinations = (0..patch_count)
                    .map(|patch_index| {
                        image.resolved_patch(patch_index).map_or_else(
                            || unreachable!("runtime-resolved image patch count is exact"),
                            |(_, destination)| destination,
                        )
                    })
                    .collect::<Vec<_>>();
                if destinations.is_empty() {
                    return Ok(Arc::new(Self::Empty));
                }
                NeutralPrimitiveSupport::Image {
                    destinations: destinations.into(),
                    local_to_surface,
                }
            }
            PaintPrimitive::ShapedTextRun(run) => NeutralPrimitiveSupport::ShapedText {
                item_index,
                resource: run.resource_ref().clone(),
                origin: run.origin(),
                local_to_surface,
            },
            _ => return Err(UnsupportedSceneSemantic::UnknownPrimitive),
        };
        Ok(Self::clipped(
            Arc::new(Self::Primitive(primitive)),
            item.clips(),
        ))
    }

    /// Replaces one support root by resolving only shaped-text nodes reachable from
    /// a support set that is actually consumed by an ordinary shadow. The immutable
    /// retained shaped resource is read directly from this exact publication;
    /// atlas/MSDF state never participates. Shared symbolic nodes are memoized so
    /// sibling/ancestor reuse does not manufacture duplicate geometry interpretations.
    pub(super) fn resolve_shaped_text(
        scene: &PaintScene,
        source: Arc<Self>,
    ) -> Result<Arc<Self>, PublicationRenderError> {
        let mut memo = HashMap::<*const Self, Arc<Self>>::new();
        let resolved = Self::resolve_shaped_text_inner(scene, &source, &mut memo)?;
        drop(source);
        Ok(resolved)
    }

    fn resolve_shaped_text_inner(
        scene: &PaintScene,
        source: &Arc<Self>,
        memo: &mut HashMap<*const Self, Arc<Self>>,
    ) -> Result<Arc<Self>, PublicationRenderError> {
        let key = Arc::as_ptr(source);
        if let Some(resolved) = memo.get(&key) {
            return Ok(Arc::clone(resolved));
        }

        let resolved = match source.as_ref() {
            Self::Primitive(NeutralPrimitiveSupport::ShapedText {
                item_index,
                resource,
                origin,
                local_to_surface,
            }) => {
                let shaped = scene.shaped_text_resource(resource).ok_or(
                    PublicationRenderError::ShapedTextResourceUnavailable {
                        item_index: *item_index,
                    },
                )?;
                let paths = resolve_positioned_paths(shaped, *origin)
                    .map_err(|failure| shaped_outline_failure(*item_index, failure))?;
                if paths.is_empty() {
                    Arc::new(Self::Empty)
                } else {
                    Arc::new(Self::Primitive(NeutralPrimitiveSupport::ShapedTextPaths {
                        paths: paths.into(),
                        local_to_surface: *local_to_surface,
                    }))
                }
            }
            Self::Empty | Self::Primitive(_) => Arc::clone(source),
            Self::Union(members) => {
                let resolved_members = members
                    .iter()
                    .map(|member| Self::resolve_shaped_text_inner(scene, member, memo))
                    .collect::<Result<Vec<_>, _>>()?;
                Self::union(resolved_members)
            }
            Self::Shadow {
                source: shadow_source,
                spread,
                offset_x,
                offset_y,
                blur_square_half_extent,
            } => Arc::new(Self::Shadow {
                source: Self::resolve_shaped_text_inner(scene, shadow_source, memo)?,
                spread: *spread,
                offset_x: *offset_x,
                offset_y: *offset_y,
                blur_square_half_extent: *blur_square_half_extent,
            }),
            Self::Clip { source, clips } => Arc::new(Self::Clip {
                source: Self::resolve_shaped_text_inner(scene, source, memo)?,
                clips: Arc::clone(clips),
            }),
        };
        memo.insert(key, Arc::clone(&resolved));
        Ok(resolved)
    }

    /// Unions direct child support without introducing painter-order authority.
    /// Runtime ordering remains independently frozen by `PaintSceneEntry`.
    pub(super) fn union(supports: impl IntoIterator<Item = Arc<Self>>) -> Arc<Self> {
        let members = supports
            .into_iter()
            .filter(|support| !matches!(support.as_ref(), Self::Empty))
            .collect::<Vec<_>>();
        match members.as_slice() {
            [] => Arc::new(Self::Empty),
            [single] => Arc::clone(single),
            _ => Arc::new(Self::Union(members.into())),
        }
    }

    /// Builds one group's ADR 0015 output support from already-unioned direct child
    /// support. Every sibling shadow receives the same exact pre-shadow allocation;
    /// no shadow chains from a previous sibling's result.
    pub(super) fn group_from_child(
        child: Arc<Self>,
        shadows: impl IntoIterator<Item = NeutralShadowFacts>,
        clips: &[SceneClip],
    ) -> Arc<Self> {
        if matches!(child.as_ref(), Self::Empty) {
            return child;
        }

        let shadows = shadows.into_iter().collect::<Vec<_>>();
        let mut output_members = Vec::with_capacity(shadows.len().saturating_add(1));
        output_members.push(Arc::clone(&child));
        output_members.extend(
            shadows
                .into_iter()
                .map(|shadow| Self::shadow(Arc::clone(&child), shadow)),
        );
        Self::clipped(Self::union(output_members), clips)
    }

    fn shadow(source: Arc<Self>, shadow: NeutralShadowFacts) -> Arc<Self> {
        if matches!(source.as_ref(), Self::Empty) {
            return source;
        }
        Arc::new(Self::Shadow {
            source,
            spread: shadow.spread,
            offset_x: shadow.offset_x,
            offset_y: shadow.offset_y,
            blur_square_half_extent: shadow.blur_square_half_extent,
        })
    }

    fn clipped(source: Arc<Self>, clips: &[SceneClip]) -> Arc<Self> {
        if clips.is_empty() || matches!(source.as_ref(), Self::Empty) {
            source
        } else {
            Arc::new(Self::Clip {
                source,
                clips: clips.to_vec().into(),
            })
        }
    }
}

const fn shaped_outline_failure(
    item_index: usize,
    failure: OutlineResolveFailure,
) -> PublicationRenderError {
    match failure {
        OutlineResolveFailure::UnsupportedGlyph { glyph_id, kind } => {
            PublicationRenderError::UnsupportedShapedGlyph {
                item_index,
                glyph_id,
                kind: match kind {
                    UnsupportedOutlineKind::ColrV0 => UnsupportedShapedGlyphKind::ColrV0,
                    UnsupportedOutlineKind::ColrV1 => UnsupportedShapedGlyphKind::ColrV1,
                    UnsupportedOutlineKind::Bitmap => UnsupportedShapedGlyphKind::Bitmap,
                    UnsupportedOutlineKind::Svg => UnsupportedShapedGlyphKind::Svg,
                    UnsupportedOutlineKind::FauxBold => UnsupportedShapedGlyphKind::FauxBold,
                },
            }
        }
        OutlineResolveFailure::InvalidFont => {
            PublicationRenderError::ShapedTextFontInvalid { item_index }
        }
        OutlineResolveFailure::InvalidOutline { glyph_id } => {
            PublicationRenderError::ShapedTextOutlineInvalid {
                item_index,
                glyph_id,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use runenui_core::{Color, DropShadow, LogicalLength, LogicalRect, LogicalTransform};

    use super::{NeutralPrimitiveSupport, NeutralShadowFacts, NeutralSupport};

    fn rect() -> LogicalRect {
        LogicalRect::try_new(0.0, 0.0, 8.0, 6.0)
            .unwrap_or_else(|_| unreachable!("controlled rectangle is valid"))
    }

    fn source() -> Arc<NeutralSupport> {
        Arc::new(NeutralSupport::Primitive(NeutralPrimitiveSupport::Image {
            destinations: Arc::<[LogicalRect]>::from(vec![rect()]),
            local_to_surface: LogicalTransform::IDENTITY,
        }))
    }

    fn shadow(color: Color) -> DropShadow {
        DropShadow::new(
            2.0,
            -3.0,
            LogicalLength::new(4.0).unwrap_or_else(|_| unreachable!("controlled sigma is valid")),
            -1.5,
            color,
        )
        .unwrap_or_else(|_| unreachable!("controlled shadow is valid"))
    }

    fn facts(shadow: DropShadow) -> NeutralShadowFacts {
        NeutralShadowFacts::new(
            shadow.offset_x(),
            shadow.offset_y(),
            shadow.sigma().get(),
            shadow.spread(),
        )
    }

    #[test]
    fn shadow_color_alpha_cannot_change_neutral_support() {
        let transparent =
            NeutralSupport::shadow(source(), facts(shadow(Color::rgba(0x10, 0x20, 0x30, 0x00))));
        let opaque =
            NeutralSupport::shadow(source(), facts(shadow(Color::rgba(0xF0, 0xE0, 0xD0, 0xFF))));
        assert_eq!(transparent, opaque);
    }

    #[test]
    fn sibling_shadows_share_one_pre_shadow_source() {
        let child = source();
        let shadows = [
            facts(shadow(Color::rgba(0x00, 0x00, 0x00, 0x40))),
            facts(
                DropShadow::new(
                    -1.0,
                    5.0,
                    LogicalLength::new(2.0)
                        .unwrap_or_else(|_| unreachable!("controlled sigma is valid")),
                    3.0,
                    Color::rgba(0xFF, 0x00, 0x00, 0x80),
                )
                .unwrap_or_else(|_| unreachable!("controlled shadow is valid")),
            ),
        ];
        let support = NeutralSupport::group_from_child(Arc::clone(&child), shadows, &[]);
        let NeutralSupport::Union(members) = support.as_ref() else {
            unreachable!("controlled child plus two shadows produces a support union");
        };
        assert_eq!(members.len(), 3);
        assert!(Arc::ptr_eq(&members[0], &child));
        for member in &members[1..] {
            let NeutralSupport::Shadow {
                source: shadow_source,
                ..
            } = member.as_ref()
            else {
                unreachable!("controlled sibling remains one symbolic shadow operation");
            };
            assert!(Arc::ptr_eq(shadow_source, &child));
        }
    }

    #[test]
    fn neutral_shadow_support_freezes_spread_offset_and_three_sigma_envelope() {
        let support =
            NeutralSupport::shadow(source(), facts(shadow(Color::rgba(0x00, 0x00, 0x00, 0x80))));
        let (spread, offset_x, offset_y, blur_square_half_extent) = match support.as_ref() {
            NeutralSupport::Shadow {
                spread,
                offset_x,
                offset_y,
                blur_square_half_extent,
                ..
            } => (*spread, *offset_x, *offset_y, *blur_square_half_extent),
            _ => unreachable!("controlled shadow produces one symbolic shadow operation"),
        };
        assert_eq!(spread.to_bits(), (-1.5_f64).to_bits());
        assert_eq!(offset_x.to_bits(), 2.0_f64.to_bits());
        assert_eq!(offset_y.to_bits(), (-3.0_f64).to_bits());
        assert_eq!(blur_square_half_extent.to_bits(), 12.0_f64.to_bits());
    }
}
