//! Transaction-local lowering of `RunenUI` layout into `Taffy`'s low-level kernels.
//!
//! The runtime owns the node mapping, measurement bridge, cache lifetime and
//! publication geometry. Taffy is used only for the algorithms themselves.

use runenui_core::{
    ComputedStyle, ContentAlignment, EdgeInsets, FlexBasis, FlexDirection, FlexWrap, ItemAlignment,
    LayoutBound, LayoutContainer, LayoutDimension, LayoutPosition, LayoutStyle, LogicalPoint,
    LogicalRect, LogicalSize, MainAxisAlignment, OverflowPolicy, Typography, WidgetAvailableSpace,
    WidgetMeasure, WidgetMeasureInput, WidgetMeasuredSize,
};
use runenui_text::{TextConstraints, TextLayoutError, TextLayoutState, TextRequest, TextSystem};
use taffy::{
    CacheTree,
    compute::{
        compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
        compute_leaf_layout, compute_root_layout,
    },
    geometry::{Line, Point, Rect, Size},
    prelude::{
        AvailableSpace, BoxSizing, Dimension, Display, LengthPercentage, LengthPercentageAuto,
        Position, Style,
    },
    style::Overflow,
    tree::{
        Baselines, Cache, Layout, LayoutBlockContainer, LayoutFlexboxContainer,
        LayoutGridContainer, LayoutInput, LayoutOutput, LayoutPartialTree, NodeId,
        TraversePartialTree,
    },
};

use super::resolve::{ResolvedSurfaceNode, ResolvedSurfaceTree};
use super::{LayoutOverflow, SurfaceLayoutNode, SurfaceLayoutReport};
use crate::{AxisLimit, LayoutConstraints};

pub(super) fn layout_resolved_surface<Action>(
    resolved_tree: &ResolvedSurfaceTree,
    mounted_tree: &crate::mounted::MountedTree<Action>,
    root_constraints: LayoutConstraints,
    text_system: &mut TextSystem,
    prior_text_layouts: Option<&[TextLayoutState]>,
) -> Result<
    (
        LogicalSize,
        Vec<LogicalRect>,
        SurfaceLayoutReport,
        Vec<TextLayoutState>,
    ),
    TextLayoutError,
> {
    #[cfg(test)]
    super::cache::note_layout_phase_execution();
    let mut kernel = LayoutKernel::new(
        resolved_tree,
        mounted_tree,
        text_system,
        prior_text_layouts,
        root_constraints,
    );
    let root = NodeId::from(0usize);
    compute_root_layout(&mut kernel, root, available_space(root_constraints));
    kernel.finish(root_constraints)
}

struct LayoutKernel<'a, Action> {
    resolved: &'a ResolvedSurfaceTree,
    mounted: &'a crate::mounted::MountedTree<Action>,
    text_system: &'a mut TextSystem,
    caches: Vec<Cache>,
    layouts: Vec<Layout>,
    text_layouts: Vec<TextLayoutState>,
    text_candidates: Vec<Vec<TextCandidate>>,
    diagnostics: Vec<Vec<runenui_core::WidgetDiagnostic>>,
    intrinsic_sizes: Vec<LogicalSize>,
    custom_intrinsic_sizes: Vec<Option<LogicalSize>>,
    text_error: Option<TextLayoutError>,
    root_constraints: LayoutConstraints,
}

#[derive(Clone)]
struct TextCandidate {
    state: TextLayoutState,
    output_size: LogicalSize,
}

impl<'a, Action> LayoutKernel<'a, Action> {
    fn new(
        resolved: &'a ResolvedSurfaceTree,
        mounted: &'a crate::mounted::MountedTree<Action>,
        text_system: &'a mut TextSystem,
        prior_text_layouts: Option<&[TextLayoutState]>,
        root_constraints: LayoutConstraints,
    ) -> Self {
        let count = resolved.nodes().len();
        let text_layouts = prior_text_layouts
            .filter(|states| states.len() == count)
            .map_or_else(|| vec![TextLayoutState::new(); count], ToOwned::to_owned);
        let mut diagnostics = vec![Vec::new(); count];
        for (index, node) in resolved.nodes().iter().enumerate() {
            if !matches!(
                node.layout().container(),
                LayoutContainer::Block
                    | LayoutContainer::Flex(_)
                    | LayoutContainer::Grid(_)
                    | LayoutContainer::Overlay(_)
            ) {
                diagnostics[index].push(runenui_core::WidgetDiagnostic::new(
                    "runenui.layout.container-unsupported",
                    "layout container variant is not supported by this runtime",
                ));
            }
            for (axis, placement) in [
                ("row", node.layout().grid_item().placement().row()),
                ("column", node.layout().grid_item().placement().column()),
            ] {
                if let Some(line) = placement.start()
                    && i16::try_from(line.get()).is_err()
                {
                    diagnostics[index].push(runenui_core::WidgetDiagnostic::new(
                        "runenui.layout.grid-line-unsupported",
                        format!(
                            "authored Grid {axis} line {} is outside the supported layout range",
                            line.get()
                        ),
                    ));
                }
            }
        }
        Self {
            resolved,
            mounted,
            text_system,
            caches: vec![Cache::new(); count],
            layouts: vec![Layout::default(); count],
            text_layouts,
            text_candidates: vec![Vec::new(); count],
            diagnostics,
            intrinsic_sizes: vec![LogicalSize::ZERO; count],
            custom_intrinsic_sizes: vec![None; count],
            text_error: None,
            root_constraints,
        }
    }

    fn style_for(&self, node: NodeId) -> Style<String> {
        let index = node_index(node);
        let mut style = lower_style(
            self.resolved.nodes()[index].layout(),
            self.resolved.nodes()[index].resolution().computed_style(),
        );
        if index == 0 {
            style.min_size.width =
                root_min_bound(style.min_size.width, self.root_constraints.horizontal());
            style.min_size.height =
                root_min_bound(style.min_size.height, self.root_constraints.vertical());
            style.max_size.width =
                root_max_bound(style.max_size.width, self.root_constraints.horizontal());
            style.max_size.height =
                root_max_bound(style.max_size.height, self.root_constraints.vertical());
        }
        if let Some(size) = self.custom_intrinsic_sizes[index] {
            apply_custom_intrinsic_minimum(
                &mut style,
                size,
                resolved_padding(&self.resolved.nodes()[index]),
            );
        }
        style
    }

    fn measure_leaf(&mut self, node: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let index = node_index(node);
        let resolved = &self.resolved.nodes()[index];
        let mounted = self
            .mounted
            .node(resolved.id())
            .unwrap_or_else(|| unreachable!("layout node remains mounted"));
        let style = self.style_for(node);
        let padding = resolved_padding(resolved);
        let widget_input = widget_measure_input(inputs, padding);
        let measurement = mounted.widget.measure(&mounted.state, widget_input);
        let mut baselines = Baselines::NONE;
        let mut text_state = None;
        let size = match measurement {
            Ok(WidgetMeasure::Measured(measured)) => {
                baselines = baselines_from_widget(measured, padding);
                self.custom_intrinsic_sizes[index] = Some(measured.size());
                measured.size()
            }
            Ok(WidgetMeasure::Text { content }) => {
                self.custom_intrinsic_sizes[index] = None;
                let typography = resolved
                    .resolution()
                    .computed_style()
                    .typography()
                    .cloned()
                    .unwrap_or_else(Typography::default);
                let constraints =
                    text_constraints(inputs.available_space.width, widget_input.known_width());
                let request = TextRequest::new(content, typography, constraints);
                let mut state = self.text_layouts[index].clone();
                match self.text_system.layout_text(&mut state, &request) {
                    Ok(outcome) => {
                        let artifact = outcome.artifact();
                        let text_size = artifact.size();
                        baselines = text_baselines(artifact, padding);
                        text_state = Some(state);
                        text_size
                    }
                    Err(error) => {
                        self.text_error = Some(error);
                        LogicalSize::ZERO
                    }
                }
            }
            Ok(WidgetMeasure::Unsupported { reason }) => {
                self.custom_intrinsic_sizes[index] = None;
                self.diagnostics[index].push(runenui_core::WidgetDiagnostic::new(
                    "runenui.measurement.unsupported",
                    format!("unsupported widget measurement capability: {reason}"),
                ));
                LogicalSize::ZERO
            }
            Err(error) => {
                self.custom_intrinsic_sizes[index] = None;
                self.diagnostics[index].push(runenui_core::WidgetDiagnostic::new(
                    "runenui.measurement.failed",
                    format!("widget measurement failed: {error:?}"),
                ));
                LogicalSize::ZERO
            }
            Ok(_) => {
                self.custom_intrinsic_sizes[index] = None;
                self.diagnostics[index].push(runenui_core::WidgetDiagnostic::new(
                    "runenui.measurement.unsupported",
                    "widget measurement capability is not supported by this runtime",
                ));
                LogicalSize::ZERO
            }
        };
        let width = size.width();
        let height = size.height();
        self.intrinsic_sizes[index] = size;
        let mut output =
            compute_leaf_layout(inputs, &style, |_, _| 0.0, |_, _| Size { width, height });
        output.baselines = baselines;
        if let Some(state) = text_state {
            self.text_candidates[index].push(TextCandidate {
                state,
                output_size: logical_size(output.size.width, output.size.height),
            });
        }
        output
    }

    #[allow(clippy::too_many_lines)]
    fn finish(
        mut self,
        root_constraints: LayoutConstraints,
    ) -> Result<
        (
            LogicalSize,
            Vec<LogicalRect>,
            SurfaceLayoutReport,
            Vec<TextLayoutState>,
        ),
        TextLayoutError,
    > {
        if let Some(error) = self.text_error {
            return Err(error);
        }
        let count = self.resolved.nodes().len();
        let mut bounds = vec![
            LogicalRect::new(
                LogicalPoint::new(0.0, 0.0)
                    .unwrap_or_else(|_| unreachable!("zero is a valid logical point")),
                LogicalSize::ZERO
            );
            count
        ];
        let mut reports = Vec::with_capacity(count);
        let mut absolute = vec![Point { x: 0.0, y: 0.0 }; count];
        for index in 0..count {
            let layout = if layout_is_valid(self.layouts[index]) {
                self.layouts[index]
            } else {
                self.diagnostics[index].push(runenui_core::WidgetDiagnostic::new(
                    "runenui.layout.invalid",
                    "layout algorithm produced a non-finite or negative geometry value",
                ));
                Layout::default()
            };
            let parent_origin = self.resolved.nodes()[index]
                .parent()
                .and_then(|id| self.resolved.position(id))
                .map_or(Point { x: 0.0, y: 0.0 }, |p| absolute[p]);
            let candidate_origin = Point {
                x: parent_origin.x + layout.location.x,
                y: parent_origin.y + layout.location.y,
            };
            absolute[index] = if candidate_origin.x.is_finite() && candidate_origin.y.is_finite() {
                candidate_origin
            } else {
                self.diagnostics[index].push(runenui_core::WidgetDiagnostic::new(
                    "runenui.layout.invalid",
                    "layout algorithm produced a non-finite absolute position",
                ));
                Point { x: 0.0, y: 0.0 }
            };
            let node_size = logical_size(layout.size.width, layout.size.height);
            bounds[index] = LogicalRect::try_new(
                absolute[index].x,
                absolute[index].y,
                layout.size.width,
                layout.size.height,
            )
            .unwrap_or_else(|_| {
                LogicalRect::new(
                    LogicalPoint::new(0.0, 0.0)
                        .unwrap_or_else(|_| unreachable!("zero is a valid logical point")),
                    LogicalSize::ZERO,
                )
            });
            let node = &self.resolved.nodes()[index];
            let outer = constraints_for_node(index, root_constraints, node_size);
            let padding = resolved_padding(node);
            let content = content_constraints(outer, padding);
            let mut desired_content = self.intrinsic_sizes[index];
            for child_id in node.children() {
                if let Some(child_index) = self.resolved.position(child_id) {
                    let child_layout = self.layouts[child_index];
                    desired_content = logical_size(
                        desired_content.width().max(
                            child_layout.location.x + child_layout.size.width
                                - padding.left().get(),
                        ),
                        desired_content.height().max(
                            child_layout.location.y + child_layout.size.height
                                - padding.top().get(),
                        ),
                    );
                }
            }
            let desired_outer = logical_size(
                desired_content.width() + padding.left().get() + padding.right().get(),
                desired_content.height() + padding.top().get() + padding.bottom().get(),
            );
            let overflow = LayoutOverflow::new(
                exceeds_max(desired_content.width(), content.horizontal().max())
                    || desired_outer.width() > node_size.width(),
                exceeds_max(desired_content.height(), content.vertical().max())
                    || desired_outer.height() > node_size.height(),
            );
            let scrollable_extent = logical_size(
                layout.scrollable_overflow_rect.right - layout.scrollable_overflow_rect.left,
                layout.scrollable_overflow_rect.bottom - layout.scrollable_overflow_rect.top,
            );
            reports.push(
                SurfaceLayoutNode::new(
                    node.id().clone(),
                    node.parent().cloned(),
                    node.authored_id().cloned(),
                    [outer, content],
                    [desired_content, desired_outer, node_size],
                    overflow,
                )
                .with_extents(node_size, desired_content, scrollable_extent)
                .with_diagnostics(std::mem::take(&mut self.diagnostics[index])),
            );
        }
        for index in 0..count {
            let final_layout = self.layouts[index];
            let final_size = logical_size(final_layout.size.width, final_layout.size.height);
            if let Some(candidate) = self.text_candidates[index]
                .iter()
                .rev()
                .find(|candidate| candidate.output_size == final_size)
                .cloned()
            {
                self.text_layouts[index] = candidate.state;
            } else {
                self.text_layouts[index].clear();
            }
        }
        let size = bounds.first().map_or(LogicalSize::ZERO, |b| b.size());
        Ok((
            size,
            bounds,
            SurfaceLayoutReport::new(reports),
            self.text_layouts,
        ))
    }
}

impl<Action> TraversePartialTree for LayoutKernel<'_, Action> {
    type ChildIter<'a>
        = std::vec::IntoIter<NodeId>
    where
        Self: 'a;
    fn child_ids(&self, parent: NodeId) -> Self::ChildIter<'_> {
        self.resolved.nodes()[usize::from(parent)]
            .children()
            .iter()
            .filter_map(|id| self.resolved.position(id))
            .map(NodeId::from)
            .collect::<Vec<_>>()
            .into_iter()
    }
    fn child_count(&self, parent: NodeId) -> usize {
        self.resolved.nodes()[usize::from(parent)].children().len()
    }
    fn get_child_id(&self, parent: NodeId, child_index: usize) -> NodeId {
        let child = &self.resolved.nodes()[usize::from(parent)].children()[child_index];
        NodeId::from(
            self.resolved
                .position(child)
                .unwrap_or_else(|| unreachable!("Taffy child remains in the resolved tree")),
        )
    }
}

impl<Action> CacheTree for LayoutKernel<'_, Action> {
    fn cache_get(&mut self, node: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.caches[usize::from(node)].get(input)
    }
    fn cache_store(&mut self, node: NodeId, input: &LayoutInput, output: LayoutOutput) {
        self.caches[usize::from(node)].store(input, output);
    }
    fn cache_clear(&mut self, node: NodeId) {
        self.caches[usize::from(node)].clear();
    }
}

impl<Action> LayoutPartialTree for LayoutKernel<'_, Action> {
    type CoreContainerStyle<'a>
        = Style<String>
    where
        Self: 'a;
    type CustomIdent = String;
    fn get_core_container_style(&self, node: NodeId) -> Self::CoreContainerStyle<'_> {
        self.style_for(node)
    }
    fn set_unrounded_layout(&mut self, node: NodeId, layout: &Layout) {
        self.layouts[usize::from(node)] = *layout;
    }
    #[allow(clippy::match_same_arms)]
    fn compute_child_layout(&mut self, node: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let index = node_index(node);
        if self.child_count(node) == 0 {
            return self.measure_leaf(node, inputs);
        }
        let style = self.resolved.nodes()[index].layout();
        // Container algorithms size from their children, but an open widget may
        // still contribute an intrinsic border-box minimum. Evaluate that
        // capability once for this request without making it a second layout
        // authority.
        let _ = self.measure_leaf(node, inputs);
        match style.container() {
            LayoutContainer::Flex(_) => {
                compute_cached_layout(self, node, inputs, |tree, node, inputs| {
                    compute_flexbox_layout(tree, node, inputs)
                })
            }
            LayoutContainer::Grid(_) | LayoutContainer::Overlay(_) => {
                compute_cached_layout(self, node, inputs, |tree, node, inputs| {
                    compute_grid_layout(tree, node, inputs)
                })
            }
            LayoutContainer::Block => {
                compute_cached_layout(self, node, inputs, |tree, node, inputs| {
                    compute_block_layout(tree, node, inputs, None)
                })
            }
            _ => LayoutOutput::HIDDEN,
        }
    }
}

impl<Action> LayoutFlexboxContainer for LayoutKernel<'_, Action> {
    type FlexboxContainerStyle<'a>
        = Style<String>
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = Style<String>
    where
        Self: 'a;
    fn get_flexbox_container_style(&self, node: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.style_for(node)
    }
    fn get_flexbox_child_style(&self, child: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.style_for(child)
    }
}

impl<Action> LayoutGridContainer for LayoutKernel<'_, Action> {
    type GridContainerStyle<'a>
        = Style<String>
    where
        Self: 'a;
    type GridItemStyle<'a>
        = Style<String>
    where
        Self: 'a;
    fn get_grid_container_style(&self, node: NodeId) -> Self::GridContainerStyle<'_> {
        self.style_for(node)
    }
    fn get_grid_child_style(&self, child: NodeId) -> Self::GridItemStyle<'_> {
        let mut style = self.style_for(child);
        let child_index = usize::from(child);
        let is_overlay_child = self.resolved.nodes()[child_index]
            .parent()
            .and_then(|parent| self.resolved.position(parent))
            .is_some_and(|parent| {
                matches!(
                    self.resolved.nodes()[parent].layout().container(),
                    LayoutContainer::Overlay(_)
                )
            });
        if is_overlay_child {
            style.grid_row = taffy::prelude::line::<Line<taffy::prelude::GridPlacement<String>>>(1);
            style.grid_column =
                taffy::prelude::line::<Line<taffy::prelude::GridPlacement<String>>>(1);
        }
        style.align_self = self.resolved.nodes()[child_index]
            .layout()
            .grid_item()
            .align_self()
            .map(align);
        style.justify_self = self.resolved.nodes()[child_index]
            .layout()
            .grid_item()
            .justify_self()
            .map(align);
        style
    }
}

impl<Action> LayoutBlockContainer for LayoutKernel<'_, Action> {
    type BlockContainerStyle<'a>
        = Style<String>
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = Style<String>
    where
        Self: 'a;
    fn get_block_container_style(&self, node: NodeId) -> Self::BlockContainerStyle<'_> {
        self.style_for(node)
    }
    fn get_block_child_style(&self, child: NodeId) -> Self::BlockItemStyle<'_> {
        self.style_for(child)
    }
}

#[allow(clippy::field_reassign_with_default, clippy::match_same_arms)]
#[allow(clippy::too_many_lines)]
fn lower_style(layout: &LayoutStyle, computed: &ComputedStyle) -> Style<String> {
    let mut style = Style::<String>::default();
    style.display = match layout.container() {
        LayoutContainer::Flex(_) => Display::Flex,
        LayoutContainer::Grid(_) | LayoutContainer::Overlay(_) => Display::Grid,
        _ => Display::Block,
    };
    style.box_sizing = BoxSizing::BorderBox;
    style.position = match layout.position() {
        LayoutPosition::Absolute(_) => Position::Absolute,
        _ => Position::Relative,
    };
    style.inset = match layout.position() {
        LayoutPosition::Absolute(insets) => Rect {
            top: offset(insets.top()),
            right: offset(insets.right()),
            bottom: offset(insets.bottom()),
            left: offset(insets.left()),
        },
        _ => Rect {
            top: LengthPercentageAuto::auto(),
            right: LengthPercentageAuto::auto(),
            bottom: LengthPercentageAuto::auto(),
            left: LengthPercentageAuto::auto(),
        },
    };
    style.size = Size {
        width: dimension(layout.width()),
        height: dimension(layout.height()),
    };
    style.min_size = Size {
        width: bound(layout.min_width()),
        height: bound(layout.min_height()),
    };
    style.max_size = Size {
        width: bound(layout.max_width()),
        height: bound(layout.max_height()),
    };
    style.margin = edge_auto(layout.margin());
    style.padding = edge_length(computed.padding().unwrap_or(EdgeInsets::ZERO));
    style.gap = Size {
        width: LengthPercentage::length(layout.gap().horizontal().get()),
        height: LengthPercentage::length(layout.gap().vertical().get()),
    };
    style.overflow = Point {
        x: overflow(layout.overflow().horizontal()),
        y: overflow(layout.overflow().vertical()),
    };
    style.scrollbar_width = 0.0;
    match layout.container() {
        LayoutContainer::Flex(container) => {
            style.flex_direction = flex_direction(container.direction());
            style.flex_wrap = flex_wrap(container.wrap());
            style.justify_content = Some(justify(container.justify_content()));
            style.align_items = Some(align(container.align_items()));
            style.align_content = Some(content(container.align_content()));
        }
        LayoutContainer::Grid(container) => {
            style.grid_template_columns = container
                .columns()
                .iter()
                .map(|track| taffy::prelude::GridTemplateComponent::Single(track_function(track)))
                .collect();
            style.grid_template_rows = container
                .rows()
                .iter()
                .map(|track| taffy::prelude::GridTemplateComponent::Single(track_function(track)))
                .collect();
            style.grid_auto_columns = vec![track_function(&container.auto_columns())];
            style.grid_auto_rows = vec![track_function(&container.auto_rows())];
            style.grid_auto_flow = match container.auto_flow() {
                runenui_core::GridAutoFlow::Row => taffy::prelude::GridAutoFlow::Row,
                runenui_core::GridAutoFlow::Column => taffy::prelude::GridAutoFlow::Column,
            };
            style.align_items = Some(align(container.align_items()));
            style.justify_items = Some(align(container.justify_items()));
            style.align_content = Some(content(container.align_content()));
            style.justify_content = Some(content(container.justify_content()));
        }
        LayoutContainer::Overlay(container) => {
            style.grid_template_columns = vec![taffy::prelude::GridTemplateComponent::Single(
                taffy::prelude::auto(),
            )];
            style.grid_template_rows = vec![taffy::prelude::GridTemplateComponent::Single(
                taffy::prelude::auto(),
            )];
            style.align_items = Some(overlay_align(container.vertical()));
            style.justify_items = Some(overlay_align(container.horizontal()));
        }
        _ => {}
    }
    style.flex_basis = match layout.flex_item().basis() {
        FlexBasis::Content => Dimension::content(),
        FlexBasis::Length(value) => Dimension::length(value.get()),
        FlexBasis::Percent(value) => Dimension::percent(value.get()),
        _ => Dimension::auto(),
    };
    style.flex_grow = layout.flex_item().grow().get();
    style.flex_shrink = layout.flex_item().shrink().get();
    style.align_self = layout.flex_item().align_self().map(align);
    style.grid_row = grid_placement(layout.grid_item().placement().row());
    style.grid_column = grid_placement(layout.grid_item().placement().column());
    style
}

fn node_index(node: NodeId) -> usize {
    usize::from(node)
}

const fn dimension(value: LayoutDimension) -> Dimension {
    match value {
        LayoutDimension::Length(v) => Dimension::length(v.get()),
        LayoutDimension::Percent(v) => Dimension::percent(v.get()),
        LayoutDimension::MinContent => Dimension::min_content(),
        LayoutDimension::MaxContent => Dimension::max_content(),
        LayoutDimension::Fill => Dimension::stretch(),
        _ => Dimension::auto(),
    }
}
const fn bound(value: LayoutBound) -> LengthPercentageAuto {
    match value {
        LayoutBound::Length(v) => LengthPercentageAuto::length(v.get()),
        LayoutBound::Percent(v) => LengthPercentageAuto::percent(v.get()),
        _ => LengthPercentageAuto::auto(),
    }
}
fn root_min_bound(
    authored: LengthPercentageAuto,
    constraints: crate::AxisConstraints,
) -> LengthPercentageAuto {
    if constraints.max().is_unbounded() && constraints.min() == runenui_core::LogicalLength::ZERO {
        return authored;
    }
    let minimum = constraints.min().get();
    let authored = authored
        .resolve_to_option(root_percentage_basis(constraints.max()), |_, _| 0.0)
        .unwrap_or(0.0);
    LengthPercentageAuto::length(authored.max(minimum))
}

fn root_max_bound(
    authored: LengthPercentageAuto,
    constraints: crate::AxisConstraints,
) -> LengthPercentageAuto {
    match constraints.max() {
        AxisLimit::Finite(maximum) => {
            let authored = authored
                .resolve_to_option(maximum.get(), |_, _| 0.0)
                .unwrap_or_else(|| maximum.get());
            LengthPercentageAuto::length(authored.min(maximum.get()))
        }
        AxisLimit::Unbounded => authored,
    }
}

const fn root_percentage_basis(value: AxisLimit) -> f32 {
    match value {
        AxisLimit::Finite(value) => value.get(),
        AxisLimit::Unbounded => 0.0,
    }
}

fn apply_custom_intrinsic_minimum(
    style: &mut Style<String>,
    content_size: LogicalSize,
    padding: EdgeInsets,
) {
    let border_box = Size {
        width: content_size.width() + padding.left().get() + padding.right().get(),
        height: content_size.height() + padding.top().get() + padding.bottom().get(),
    };
    if style.min_size.width.is_auto() {
        style.min_size.width = LengthPercentageAuto::length(border_box.width);
    }
    if style.min_size.height.is_auto() {
        style.min_size.height = LengthPercentageAuto::length(border_box.height);
    }
}

const fn edge_length(value: EdgeInsets) -> Rect<LengthPercentage> {
    Rect {
        top: LengthPercentage::length(value.top().get()),
        right: LengthPercentage::length(value.right().get()),
        bottom: LengthPercentage::length(value.bottom().get()),
        left: LengthPercentage::length(value.left().get()),
    }
}
const fn edge_auto(value: EdgeInsets) -> Rect<LengthPercentageAuto> {
    Rect {
        top: LengthPercentageAuto::length(value.top().get()),
        right: LengthPercentageAuto::length(value.right().get()),
        bottom: LengthPercentageAuto::length(value.bottom().get()),
        left: LengthPercentageAuto::length(value.left().get()),
    }
}
const fn offset(value: runenui_core::LayoutOffset) -> LengthPercentageAuto {
    match value {
        runenui_core::LayoutOffset::Length(v) => LengthPercentageAuto::length(v.get()),
        runenui_core::LayoutOffset::Percent(v) => LengthPercentageAuto::percent(v.get()),
        _ => LengthPercentageAuto::auto(),
    }
}
const fn overflow(value: OverflowPolicy) -> Overflow {
    match value {
        OverflowPolicy::Visible => Overflow::Visible,
        OverflowPolicy::Clip => Overflow::Clip,
        OverflowPolicy::Scroll => Overflow::Scroll,
    }
}
const fn flex_direction(value: FlexDirection) -> taffy::prelude::FlexDirection {
    match value {
        FlexDirection::Row => taffy::prelude::FlexDirection::Row,
        FlexDirection::RowReverse => taffy::prelude::FlexDirection::RowReverse,
        FlexDirection::Column => taffy::prelude::FlexDirection::Column,
        FlexDirection::ColumnReverse => taffy::prelude::FlexDirection::ColumnReverse,
    }
}
const fn flex_wrap(value: FlexWrap) -> taffy::prelude::FlexWrap {
    match value {
        FlexWrap::NoWrap => taffy::prelude::FlexWrap::NoWrap,
        FlexWrap::Wrap => taffy::prelude::FlexWrap::Wrap,
        FlexWrap::WrapReverse => taffy::prelude::FlexWrap::WrapReverse,
    }
}
const fn align(value: ItemAlignment) -> taffy::prelude::AlignItems {
    match value {
        ItemAlignment::Stretch => taffy::prelude::AlignItems::STRETCH,
        ItemAlignment::Start => taffy::prelude::AlignItems::START,
        ItemAlignment::End => taffy::prelude::AlignItems::END,
        ItemAlignment::Center => taffy::prelude::AlignItems::CENTER,
        ItemAlignment::Baseline => taffy::prelude::AlignItems::BASELINE,
    }
}
const fn overlay_align(value: runenui_core::OverlayAlignment) -> taffy::prelude::AlignItems {
    match value {
        runenui_core::OverlayAlignment::Stretch => taffy::prelude::AlignItems::STRETCH,
        runenui_core::OverlayAlignment::Start => taffy::prelude::AlignItems::START,
        runenui_core::OverlayAlignment::End => taffy::prelude::AlignItems::END,
        runenui_core::OverlayAlignment::Center => taffy::prelude::AlignItems::CENTER,
    }
}
const fn justify(value: MainAxisAlignment) -> taffy::prelude::JustifyContent {
    match value {
        MainAxisAlignment::Start => taffy::prelude::JustifyContent::START,
        MainAxisAlignment::End => taffy::prelude::JustifyContent::END,
        MainAxisAlignment::Center => taffy::prelude::JustifyContent::CENTER,
        MainAxisAlignment::SpaceBetween => taffy::prelude::JustifyContent::SPACE_BETWEEN,
        MainAxisAlignment::SpaceAround => taffy::prelude::JustifyContent::SPACE_AROUND,
        MainAxisAlignment::SpaceEvenly => taffy::prelude::JustifyContent::SPACE_EVENLY,
    }
}
const fn content(value: ContentAlignment) -> taffy::prelude::AlignContent {
    match value {
        ContentAlignment::Stretch => taffy::prelude::AlignContent::STRETCH,
        ContentAlignment::Start => taffy::prelude::AlignContent::START,
        ContentAlignment::End => taffy::prelude::AlignContent::END,
        ContentAlignment::Center => taffy::prelude::AlignContent::CENTER,
        ContentAlignment::SpaceBetween => taffy::prelude::AlignContent::SPACE_BETWEEN,
        ContentAlignment::SpaceAround => taffy::prelude::AlignContent::SPACE_AROUND,
        ContentAlignment::SpaceEvenly => taffy::prelude::AlignContent::SPACE_EVENLY,
    }
}
fn track_function(track: &runenui_core::GridTrack) -> taffy::prelude::TrackSizingFunction {
    taffy::prelude::minmax(track_min(track.min()), track_max(track.max()))
}
const fn track_min(value: runenui_core::GridTrackMin) -> taffy::prelude::MinTrackSizingFunction {
    match value {
        runenui_core::GridTrackMin::MinContent => {
            taffy::prelude::MinTrackSizingFunction::min_content()
        }
        runenui_core::GridTrackMin::MaxContent => {
            taffy::prelude::MinTrackSizingFunction::max_content()
        }
        runenui_core::GridTrackMin::Length(v) => {
            taffy::prelude::MinTrackSizingFunction::length(v.get())
        }
        runenui_core::GridTrackMin::Percent(v) => {
            taffy::prelude::MinTrackSizingFunction::percent(v.get())
        }
        _ => taffy::prelude::MinTrackSizingFunction::auto(),
    }
}
const fn track_max(value: runenui_core::GridTrackMax) -> taffy::prelude::MaxTrackSizingFunction {
    match value {
        runenui_core::GridTrackMax::MinContent => {
            taffy::prelude::MaxTrackSizingFunction::min_content()
        }
        runenui_core::GridTrackMax::MaxContent => {
            taffy::prelude::MaxTrackSizingFunction::max_content()
        }
        runenui_core::GridTrackMax::Length(v) => {
            taffy::prelude::MaxTrackSizingFunction::length(v.get())
        }
        runenui_core::GridTrackMax::Percent(v) => {
            taffy::prelude::MaxTrackSizingFunction::percent(v.get())
        }
        runenui_core::GridTrackMax::Fraction(v) => {
            taffy::prelude::MaxTrackSizingFunction::fr(v.get())
        }
        _ => taffy::prelude::MaxTrackSizingFunction::auto(),
    }
}
fn grid_placement(
    value: runenui_core::GridAxisPlacement,
) -> Line<taffy::prelude::GridPlacement<String>> {
    let span = value.span().get();
    match value.start() {
        Some(start) => {
            let Ok(line_number) = i16::try_from(start.get()) else {
                return Line::default();
            };
            let line =
                taffy::prelude::line::<Line<taffy::prelude::GridPlacement<String>>>(line_number);
            let end = if span == 1 {
                taffy::prelude::GridPlacement::Auto
            } else {
                taffy::prelude::span::<Line<taffy::prelude::GridPlacement<String>>>(span).start
            };
            Line {
                start: line.start,
                end,
            }
        }
        None if span > 1 => {
            taffy::prelude::span::<Line<taffy::prelude::GridPlacement<String>>>(span)
        }
        None => Line::default(),
    }
}

fn resolved_padding(node: &ResolvedSurfaceNode) -> EdgeInsets {
    node.resolution()
        .computed_style()
        .padding()
        .unwrap_or(EdgeInsets::ZERO)
}
fn logical_size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width.max(0.0), height.max(0.0)).unwrap_or(LogicalSize::ZERO)
}
const fn available_space(constraints: LayoutConstraints) -> Size<AvailableSpace> {
    Size {
        width: axis_available(constraints.horizontal().max()),
        height: axis_available(constraints.vertical().max()),
    }
}
const fn axis_available(limit: AxisLimit) -> AvailableSpace {
    match limit {
        AxisLimit::Finite(value) => AvailableSpace::Definite(value.get()),
        AxisLimit::Unbounded => AvailableSpace::MaxContent,
    }
}
fn widget_measure_input(inputs: LayoutInput, padding: EdgeInsets) -> WidgetMeasureInput {
    WidgetMeasureInput::new(
        inputs
            .known_dimensions
            .width
            .map(|v| logical_extent(v - padding.left().get() - padding.right().get())),
        inputs
            .known_dimensions
            .height
            .map(|v| logical_extent(v - padding.top().get() - padding.bottom().get())),
        widget_space(inputs.available_space.width),
        widget_space(inputs.available_space.height),
    )
}
fn widget_space(value: AvailableSpace) -> WidgetAvailableSpace {
    match value {
        AvailableSpace::Definite(v) => WidgetAvailableSpace::definite(logical_extent(v)),
        AvailableSpace::MinContent => WidgetAvailableSpace::MinContent,
        AvailableSpace::MaxContent => WidgetAvailableSpace::MaxContent,
    }
}
fn logical_extent(value: f32) -> runenui_core::LogicalLength {
    runenui_core::LogicalLength::new(value.max(0.0)).unwrap_or_default()
}
fn text_constraints(
    space: AvailableSpace,
    known: Option<runenui_core::LogicalLength>,
) -> TextConstraints {
    match (known, space) {
        (Some(value), _) => TextConstraints::limited(value),
        (None, AvailableSpace::Definite(value)) => TextConstraints::limited(logical_extent(value)),
        (None, AvailableSpace::MinContent) => TextConstraints::min_content(),
        (None, AvailableSpace::MaxContent) => TextConstraints::unbounded(),
    }
}
fn baselines_from_widget(size: WidgetMeasuredSize, padding: EdgeInsets) -> Baselines {
    Baselines {
        first: size.first_baseline().map(|v| v.get() + padding.top().get()),
        last: size.last_baseline().map(|v| v.get() + padding.top().get()),
    }
}
fn text_baselines(artifact: &runenui_text::TextArtifact, padding: EdgeInsets) -> Baselines {
    let lines = artifact.lines();
    Baselines {
        first: lines
            .first()
            .map(|line| line.metrics().baseline() + padding.top().get()),
        last: lines
            .last()
            .map(|line| line.metrics().baseline() + padding.top().get()),
    }
}
const fn constraints_for_node(
    index: usize,
    root: LayoutConstraints,
    size: LogicalSize,
) -> LayoutConstraints {
    if index == 0 {
        root
    } else {
        LayoutConstraints::tight(size)
    }
}
fn content_constraints(outer: LayoutConstraints, padding: EdgeInsets) -> LayoutConstraints {
    LayoutConstraints::new(
        axis_content(
            outer.horizontal(),
            padding.left().get() + padding.right().get(),
        ),
        axis_content(
            outer.vertical(),
            padding.top().get() + padding.bottom().get(),
        ),
    )
}
fn axis_content(axis: crate::AxisConstraints, padding: f32) -> crate::AxisConstraints {
    crate::AxisConstraints::new(
        logical_extent(axis.min().get() - padding),
        match axis.max() {
            AxisLimit::Finite(v) => AxisLimit::Finite(logical_extent(v.get() - padding)),
            AxisLimit::Unbounded => AxisLimit::Unbounded,
        },
    )
}
fn exceeds_max(value: f32, maximum: AxisLimit) -> bool {
    matches!(maximum, AxisLimit::Finite(maximum) if value > maximum.get())
}

fn layout_is_valid(layout: Layout) -> bool {
    layout.location.x.is_finite()
        && layout.location.y.is_finite()
        && layout.size.width.is_finite()
        && layout.size.height.is_finite()
        && layout.size.width >= 0.0
        && layout.size.height >= 0.0
        && layout.scrollable_overflow_rect.left.is_finite()
        && layout.scrollable_overflow_rect.right.is_finite()
        && layout.scrollable_overflow_rect.top.is_finite()
        && layout.scrollable_overflow_rect.bottom.is_finite()
        && layout.scrollable_overflow_rect.right >= layout.scrollable_overflow_rect.left
        && layout.scrollable_overflow_rect.bottom >= layout.scrollable_overflow_rect.top
}
