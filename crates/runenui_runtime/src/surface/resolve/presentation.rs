use std::{collections::HashMap, sync::Arc};

use runenui_core::{
    __runtime::transform_rect_aabb, LogicalRect, LogicalTransform, OverflowPolicy, SceneShape,
    SurfacePresentation, SurfacePresentationAlignment, SurfacePresentationAnchor,
    SurfacePresentationPlacement, SurfacePresentationSide, WidgetDiagnostic,
};

use crate::{
    scene::SceneClip,
    surface::{
        SurfacePresentationSnapshot, SurfaceScrollProjection,
        cache::{
            CachedLayoutFacts, CachedPresentationFacts, PresentationNodeFacts,
            PresentationNodeFactsInit,
        },
    },
};

use super::{
    CachedEffectiveFacts, PresentationGeometryError, ScrollChromePresentationContext,
    ScrollChromePresentationInput, SurfaceTopologyNode, SurfaceTopologySnapshot, intersect_rects,
    resolve_present_scroll_chrome,
};

#[derive(Clone, Copy)]
enum AnchorError {
    MissingOwner,
    UnavailableOwner,
    Unrepresentable,
}

fn diagnostic(error: AnchorError) -> WidgetDiagnostic {
    match error {
        AnchorError::MissingOwner => WidgetDiagnostic::new(
            "runenui.presentation.owner-required",
            "same-surface presentation roots require one logical mounted owner",
        ),
        AnchorError::UnavailableOwner => WidgetDiagnostic::new(
            "runenui.presentation.anchor-unavailable",
            "presentation owner geometry is unavailable in this exact surface publication",
        ),
        AnchorError::Unrepresentable => WidgetDiagnostic::new(
            "runenui.presentation.anchor-unrepresentable",
            "presentation anchor geometry cannot be represented in finite surface coordinates",
        ),
    }
}

fn zero_rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 0.0, 0.0)
        .unwrap_or_else(|_| unreachable!("zero presentation rectangle is valid"))
}

fn unpublished(stack_root: Option<usize>, why: Option<WidgetDiagnostic>) -> PresentationNodeFacts {
    let zero = zero_rect();
    PresentationNodeFacts::new(PresentationNodeFactsInit {
        owner_to_surface: LogicalTransform::IDENTITY,
        content_to_surface: LogicalTransform::IDENTITY,
        owner_bounds: zero,
        visible_bounds: zero,
        inherited_clips: Arc::from(Vec::<SceneClip>::new()),
        content_clips: Arc::from(Vec::<SceneClip>::new()),
        published: false,
        stack_root,
        root_snapshot: None,
        diagnostics: Arc::from(why.into_iter().collect::<Vec<_>>()),
    })
}

fn surface_rect(layout: &CachedLayoutFacts) -> Result<LogicalRect, PresentationGeometryError> {
    LogicalRect::try_new(0.0, 0.0, layout.size.width(), layout.size.height())
        .map_err(|_| PresentationGeometryError)
}

fn resolve_anchor(
    authored: &SurfacePresentation,
    owner: Option<&PresentationNodeFacts>,
    surface: LogicalRect,
) -> Result<LogicalRect, AnchorError> {
    match authored.anchor() {
        SurfacePresentationAnchor::OwnerBounds => {
            let owner = owner.ok_or(AnchorError::MissingOwner)?;
            if !owner.published() || owner.owner_to_surface().inverse().is_none() {
                return Err(AnchorError::UnavailableOwner);
            }
            Ok(owner.owner_bounds())
        }
        SurfacePresentationAnchor::OwnerRect(rect) => {
            let owner = owner.ok_or(AnchorError::MissingOwner)?;
            if !owner.published() || owner.owner_to_surface().inverse().is_none() {
                return Err(AnchorError::UnavailableOwner);
            }
            transform_rect_aabb(owner.owner_to_surface(), rect).ok_or(AnchorError::Unrepresentable)
        }
        SurfacePresentationAnchor::SurfacePoint(point) => {
            LogicalRect::try_new(point.x(), point.y(), 0.0, 0.0)
                .map_err(|_| AnchorError::Unrepresentable)
        }
        SurfacePresentationAnchor::SurfaceViewport => Ok(surface),
        _ => Err(AnchorError::Unrepresentable),
    }
}

fn aligned(
    start: f32,
    anchor_extent: f32,
    extent: f32,
    alignment: SurfacePresentationAlignment,
) -> Result<f32, PresentationGeometryError> {
    let value = match alignment {
        SurfacePresentationAlignment::Start => start,
        SurfacePresentationAlignment::Center => (anchor_extent - extent).mul_add(0.5, start),
        SurfacePresentationAlignment::End => start + anchor_extent - extent,
        _ => return Err(PresentationGeometryError),
    };
    value
        .is_finite()
        .then_some(value)
        .ok_or(PresentationGeometryError)
}

fn candidate_rect(
    anchor: LogicalRect,
    size: runenui_core::LogicalSize,
    candidate: SurfacePresentationPlacement,
) -> Result<LogicalRect, PresentationGeometryError> {
    let (width, height, gap) = (size.width(), size.height(), candidate.gap().get());
    let (x, y) = match candidate.side() {
        SurfacePresentationSide::Top => (
            aligned(anchor.x(), anchor.width(), width, candidate.alignment())?,
            anchor.y() - gap - height,
        ),
        SurfacePresentationSide::Bottom => (
            aligned(anchor.x(), anchor.width(), width, candidate.alignment())?,
            anchor.max_y() + gap,
        ),
        SurfacePresentationSide::Left => (
            anchor.x() - gap - width,
            aligned(anchor.y(), anchor.height(), height, candidate.alignment())?,
        ),
        SurfacePresentationSide::Right => (
            anchor.max_x() + gap,
            aligned(anchor.y(), anchor.height(), height, candidate.alignment())?,
        ),
        SurfacePresentationSide::Center => (
            (anchor.width() - width).mul_add(0.5, anchor.x()),
            (anchor.height() - height).mul_add(0.5, anchor.y()),
        ),
        _ => return Err(PresentationGeometryError),
    };
    let offset = candidate.offset();
    LogicalRect::try_new(x + offset.x(), y + offset.y(), width, height)
        .map_err(|_| PresentationGeometryError)
}

fn contained(rect: LogicalRect, surface: LogicalRect) -> bool {
    rect.x() >= surface.x()
        && rect.y() >= surface.y()
        && rect.max_x() <= surface.max_x()
        && rect.max_y() <= surface.max_y()
}

fn intersection_area(rect: LogicalRect, surface: LogicalRect) -> f32 {
    let width = (rect.max_x().min(surface.max_x()) - rect.x().max(surface.x())).max(0.0);
    let height = (rect.max_y().min(surface.max_y()) - rect.y().max(surface.y())).max(0.0);
    width * height
}

fn clamp(
    rect: LogicalRect,
    surface: LogicalRect,
) -> Result<LogicalRect, PresentationGeometryError> {
    let x = if rect.width() <= surface.width() {
        rect.x().clamp(surface.x(), surface.max_x() - rect.width())
    } else {
        surface.x()
    };
    let y = if rect.height() <= surface.height() {
        rect.y().clamp(surface.y(), surface.max_y() - rect.height())
    } else {
        surface.y()
    };
    LogicalRect::try_new(x, y, rect.width(), rect.height()).map_err(|_| PresentationGeometryError)
}

fn choose(
    authored: &SurfacePresentation,
    anchor: LogicalRect,
    size: runenui_core::LogicalSize,
    surface: LogicalRect,
) -> Result<(usize, SurfacePresentationPlacement, LogicalRect), PresentationGeometryError> {
    let mut fallback = Vec::with_capacity(authored.candidates().len());
    for (index, candidate) in authored.candidates().iter().copied().enumerate() {
        let rect = candidate_rect(anchor, size, candidate)?;
        if contained(rect, surface) {
            return Ok((index, candidate, rect));
        }
        fallback.push((index, candidate, rect, intersection_area(rect, surface)));
    }
    let (index, candidate, rect, _) = fallback
        .into_iter()
        .max_by(|left, right| {
            left.3
                .total_cmp(&right.3)
                .then_with(|| right.0.cmp(&left.0))
        })
        .ok_or(PresentationGeometryError)?;
    Ok((index, candidate, clamp(rect, surface)?))
}

struct LoopState {
    nodes: Vec<PresentationNodeFacts>,
    inherited_scroll: Vec<(f32, f32)>,
    child_offsets: Vec<(f32, f32)>,
    projection_offsets: Vec<(f32, f32)>,
    child_clips: Vec<Vec<SceneClip>>,
    child_clip_bounds: Vec<Vec<LogicalRect>>,
}

impl LoopState {
    fn with_capacity(count: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(count),
            inherited_scroll: Vec::with_capacity(count),
            child_offsets: Vec::with_capacity(count),
            projection_offsets: Vec::with_capacity(count),
            child_clips: Vec::with_capacity(count),
            child_clip_bounds: Vec::with_capacity(count),
        }
    }

    fn push_unpublished(
        &mut self,
        stack_root: Option<usize>,
        why: Option<WidgetDiagnostic>,
        inherited_scroll: (f32, f32),
    ) {
        self.inherited_scroll.push(inherited_scroll);
        self.child_offsets.push(inherited_scroll);
        self.projection_offsets.push((0.0, 0.0));
        self.child_clips.push(Vec::new());
        self.child_clip_bounds.push(Vec::new());
        self.nodes.push(unpublished(stack_root, why));
    }
}

fn local_scroll(node: &SurfaceTopologyNode, scroll: &SurfaceScrollProjection) -> (f32, f32) {
    let local = scroll.offset(&node.id);
    (
        if node.overflow.horizontal() == OverflowPolicy::Scroll {
            local.0
        } else {
            0.0
        },
        if node.overflow.vertical() == OverflowPolicy::Scroll {
            local.1
        } else {
            0.0
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn append_scroll_clip(
    node: &SurfaceTopologyNode,
    layout_node: &crate::surface::SurfaceLayoutNode,
    owner_to_surface: LogicalTransform,
    inherited_clip_bounds: &mut Vec<LogicalRect>,
    content_clips: &mut Vec<SceneClip>,
) -> Result<(), PresentationGeometryError> {
    if node.overflow.horizontal() != OverflowPolicy::Scroll
        && node.overflow.vertical() != OverflowPolicy::Scroll
    {
        return Ok(());
    }
    let viewport = layout_node.scroll_viewport_extent();
    let rect = LogicalRect::try_new(0.0, 0.0, viewport.width(), viewport.height())
        .unwrap_or_else(|_| unreachable!("published viewport extent is valid"));
    let bounds = transform_rect_aabb(owner_to_surface, rect).ok_or(PresentationGeometryError)?;
    content_clips.push(SceneClip::new(SceneShape::rect(rect), owner_to_surface));
    inherited_clip_bounds.push(bounds);
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub(in crate::surface) fn resolve_presentation(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    effective: &CachedEffectiveFacts,
    scroll: &SurfaceScrollProjection,
) -> Result<CachedPresentationFacts, PresentationGeometryError> {
    let count = topology.nodes.len();
    if layout.bounds.len() != count
        || effective.nodes.len() != count
        || layout.report.nodes().len() != count
        || layout.scroll_chrome.len() != count
    {
        return Err(PresentationGeometryError);
    }

    let positions = topology
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.clone(), position))
        .collect::<HashMap<_, _>>();
    let surface = surface_rect(layout)?;
    let hard_surface_clip = SceneClip::new(SceneShape::rect(surface), LogicalTransform::IDENTITY);
    let chrome_context = ScrollChromePresentationContext {
        topology,
        layout,
        scroll,
    };
    let mut state = LoopState::with_capacity(count);

    for (position, ((bounds, effective), node)) in layout
        .bounds
        .iter()
        .zip(&effective.nodes)
        .zip(&topology.nodes)
        .enumerate()
    {
        let layout_node = layout
            .report
            .nodes()
            .get(position)
            .filter(|layout_node| layout_node.id() == &node.id)
            .ok_or(PresentationGeometryError)?;
        let parent = node
            .parent
            .as_ref()
            .and_then(|parent| positions.get(parent).copied());
        let parent_presentation = parent.and_then(|parent| state.nodes.get(parent));

        if let Some(authored) = node.surface_presentation.as_ref() {
            let stack_root = Some(position);
            let Some(owner) = parent_presentation else {
                state.push_unpublished(
                    stack_root,
                    Some(diagnostic(AnchorError::MissingOwner)),
                    (0.0, 0.0),
                );
                continue;
            };
            if !owner.published() {
                state.push_unpublished(
                    stack_root,
                    Some(diagnostic(AnchorError::UnavailableOwner)),
                    (0.0, 0.0),
                );
                continue;
            }
            let anchor = match resolve_anchor(authored, Some(owner), surface) {
                Ok(value) => value,
                Err(error) => {
                    state.push_unpublished(stack_root, Some(diagnostic(error)), (0.0, 0.0));
                    continue;
                }
            };
            let (candidate_index, candidate, placed) =
                choose(authored, anchor, bounds.size(), surface)?;
            let projection_offset = (placed.x() - bounds.x(), placed.y() - bounds.y());
            if !projection_offset.0.is_finite() || !projection_offset.1.is_finite() {
                return Err(PresentationGeometryError);
            }
            state.inherited_scroll.push((0.0, 0.0));
            state.projection_offsets.push(projection_offset);

            let transform = effective
                .computed_style()
                .presentation()
                .map_or(Ok(LogicalTransform::IDENTITY), |value| {
                    value.resolve_in_box(bounds.size())
                })
                .map_err(|_| PresentationGeometryError)?;
            let placement = LogicalTransform::translation(placed.x(), placed.y())
                .map_err(|_| PresentationGeometryError)?;
            let owner_to_surface = transform
                .then(placement)
                .map_err(|_| PresentationGeometryError)?;
            let local_bounds = LogicalRect::try_new(0.0, 0.0, bounds.width(), bounds.height())
                .unwrap_or_else(|_| unreachable!("layout size is valid"));
            let owner_bounds = transform_rect_aabb(owner_to_surface, local_bounds)
                .ok_or(PresentationGeometryError)?;
            let visible_bounds = intersect_rects(owner_bounds, surface);
            let (scroll_x, scroll_y) = local_scroll(node, scroll);
            let content_translation = LogicalTransform::translation(-scroll_x, -scroll_y)
                .map_err(|_| PresentationGeometryError)?;
            let content_to_surface = content_translation
                .then(owner_to_surface)
                .map_err(|_| PresentationGeometryError)?;
            state.child_offsets.push((scroll_x, scroll_y));

            let inherited_clips = vec![hard_surface_clip.clone()];
            let mut inherited_clip_bounds = vec![surface];
            let mut content_clips = inherited_clips.clone();
            append_scroll_clip(
                node,
                layout_node,
                owner_to_surface,
                &mut inherited_clip_bounds,
                &mut content_clips,
            )?;
            let snapshot = SurfacePresentationSnapshot::new(
                candidate_index,
                candidate,
                anchor,
                placed,
                visible_bounds,
            );
            state.nodes.push(PresentationNodeFacts::new(PresentationNodeFactsInit {
                owner_to_surface,
                content_to_surface,
                owner_bounds,
                visible_bounds,
                inherited_clips: Arc::from(inherited_clips),
                content_clips: Arc::from(content_clips.clone()),
                published: true,
                stack_root,
                root_snapshot: Some(snapshot),
                diagnostics: Arc::from(Vec::<WidgetDiagnostic>::new()),
            }));
            state.child_clips.push(content_clips);
            state.child_clip_bounds.push(inherited_clip_bounds);
            continue;
        }

        let stack_root = parent_presentation.and_then(PresentationNodeFacts::stack_root);
        if parent_presentation.is_some_and(|parent| !parent.published()) {
            let inherited = parent.map_or((0.0, 0.0), |parent| state.child_offsets[parent]);
            state.push_unpublished(stack_root, None, inherited);
            continue;
        }
        let inherited = parent.map_or((0.0, 0.0), |parent| state.child_offsets[parent]);
        let projection = parent.map_or((0.0, 0.0), |parent| state.projection_offsets[parent]);
        state.projection_offsets.push(projection);

        let transform = effective
            .computed_style()
            .presentation()
            .map_or(Ok(LogicalTransform::IDENTITY), |value| {
                value.resolve_in_box(bounds.size())
            })
            .map_err(|_| PresentationGeometryError)?;
        if let Some(chrome) = layout.scroll_chrome[position].filter(|chrome| chrome.present()) {
            let presented = resolve_present_scroll_chrome(
                &chrome_context,
                ScrollChromePresentationInput {
                    node_presentation: transform,
                    position,
                    bounds: *bounds,
                    projection: chrome,
                },
                &state.nodes,
                &state.inherited_scroll,
            )?;
            state.inherited_scroll.push(presented.child_offset);
            state.child_offsets.push(presented.child_offset);
            state.nodes.push(presented.presentation);
            state.child_clips.push(presented.child_clips);
            state.child_clip_bounds.push(presented.child_clip_bounds);
            continue;
        }

        state.inherited_scroll.push(inherited);
        let inherited_clips = parent
            .map(|parent| state.child_clips[parent].clone())
            .unwrap_or_default();
        let mut inherited_clip_bounds = parent
            .map(|parent| state.child_clip_bounds[parent].clone())
            .unwrap_or_default();
        let placement = LogicalTransform::translation(
            bounds.x() - inherited.0 + projection.0,
            bounds.y() - inherited.1 + projection.1,
        )
        .map_err(|_| PresentationGeometryError)?;
        let owner_to_surface = transform
            .then(placement)
            .map_err(|_| PresentationGeometryError)?;
        let local_bounds = LogicalRect::try_new(0.0, 0.0, bounds.width(), bounds.height())
            .unwrap_or_else(|_| unreachable!("layout size is valid"));
        let owner_bounds =
            transform_rect_aabb(owner_to_surface, local_bounds).ok_or(PresentationGeometryError)?;
        let visible_bounds = inherited_clip_bounds
            .iter()
            .fold(owner_bounds, |visible, clip| {
                intersect_rects(visible, *clip)
            });
        let (scroll_x, scroll_y) = local_scroll(node, scroll);
        let content_translation = LogicalTransform::translation(-scroll_x, -scroll_y)
            .map_err(|_| PresentationGeometryError)?;
        let content_to_surface = content_translation
            .then(owner_to_surface)
            .map_err(|_| PresentationGeometryError)?;
        let child_offset = (inherited.0 + scroll_x, inherited.1 + scroll_y);
        if !child_offset.0.is_finite() || !child_offset.1.is_finite() {
            return Err(PresentationGeometryError);
        }
        state.child_offsets.push(child_offset);

        let mut content_clips = inherited_clips.clone();
        append_scroll_clip(
            node,
            layout_node,
            owner_to_surface,
            &mut inherited_clip_bounds,
            &mut content_clips,
        )?;
        state.nodes.push(PresentationNodeFacts::new(PresentationNodeFactsInit {
            owner_to_surface,
            content_to_surface,
            owner_bounds,
            visible_bounds,
            inherited_clips: Arc::from(inherited_clips),
            content_clips: Arc::from(content_clips.clone()),
            published: true,
            stack_root,
            root_snapshot: None,
            diagnostics: Arc::from(Vec::<WidgetDiagnostic>::new()),
        }));
        state.child_clips.push(content_clips);
        state.child_clip_bounds.push(inherited_clip_bounds);
    }
    Ok(CachedPresentationFacts { nodes: state.nodes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use runenui_core::{LogicalLength, LogicalSize};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rect is valid"))
    }

    fn size(width: u16, height: u16) -> LogicalSize {
        LogicalSize::new(LogicalLength::from(width), LogicalLength::from(height))
    }

    #[test]
    fn authored_order_selects_first_full_fit_before_later_candidates() {
        let authored = SurfacePresentation::new(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Bottom,
        ))
        .with_fallback(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Top,
        ));
        let (index, placement, resolved) = choose(
            &authored,
            rect(20.0, 20.0, 20.0, 20.0),
            size(10, 10),
            rect(0.0, 0.0, 80.0, 80.0),
        )
        .unwrap_or_else(|_| unreachable!("fixture placement resolves"));
        assert_eq!(index, 0);
        assert_eq!(placement.side(), SurfacePresentationSide::Bottom);
        assert_eq!(resolved, rect(25.0, 40.0, 10.0, 10.0));
    }

    #[test]
    fn greatest_visible_area_wins_then_clamps_when_no_candidate_fully_fits() {
        let authored = SurfacePresentation::new(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Right,
        ))
        .with_fallback(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Left,
        ));
        let (index, placement, resolved) = choose(
            &authored,
            rect(35.0, 15.0, 10.0, 10.0),
            size(40, 20),
            rect(0.0, 0.0, 60.0, 40.0),
        )
        .unwrap_or_else(|_| unreachable!("fixture placement resolves"));
        assert_eq!(index, 1, "left has greater visible intersection than right");
        assert_eq!(placement.side(), SurfacePresentationSide::Left);
        assert_eq!(resolved, rect(0.0, 10.0, 40.0, 20.0));
    }

    #[test]
    fn equal_visible_area_keeps_authored_order_and_oversized_axis_pins_to_surface_start() {
        let authored = SurfacePresentation::new(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Top,
        ))
        .with_fallback(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Bottom,
        ));
        let (index, placement, resolved) = choose(
            &authored,
            rect(20.0, 10.0, 20.0, 20.0),
            size(80, 20),
            rect(0.0, 0.0, 60.0, 40.0),
        )
        .unwrap_or_else(|_| unreachable!("fixture placement resolves"));
        assert_eq!(index, 0);
        assert_eq!(placement.side(), SurfacePresentationSide::Top);
        assert_eq!(
            resolved.x(),
            0.0,
            "oversized horizontal axis pins to surface start"
        );
        assert_eq!(resolved.y(), 0.0);
        assert_eq!(resolved.width(), 80.0);
    }
}
