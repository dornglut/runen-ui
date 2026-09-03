use super::{
    LayoutOverflow, SurfaceLayoutNode, SurfaceLayoutReport,
    arrange::SurfaceArrangementBuilder,
    resolve::{ResolvedSurfaceNode, ResolvedSurfaceTree},
};
use crate::{AxisConstraints, AxisLimit, LayoutConstraints, LogicalPoint};
use runenui_core::{
    Axis, ChildLayout, EdgeInsets, LogicalLength, LogicalRect, LogicalSize, Typography,
    WidgetDiagnostic, WidgetMeasure,
};
use runenui_text::{TextConstraints, TextLayoutError, TextLayoutState, TextRequest, TextSystem};

pub(super) fn layout_resolved_surface(
    resolved_tree: &ResolvedSurfaceTree,
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
    let mut measured_layout = MeasuredSurfaceLayout::new(resolved_tree, prior_text_layouts);
    let root = resolved_tree
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("mounted publication has a root"));
    let mut measurer = SurfaceMeasurer::new(text_system);
    let frame_size =
        measurer.measure_node(resolved_tree, &mut measured_layout, root, root_constraints)?;

    let bounds = {
        let mut arranger = SurfaceArrangementBuilder::new(&measured_layout);
        let origin = LogicalPoint::new(0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("the logical origin is finite"));
        arranger.push_node(resolved_tree, root, origin);
        arranger.into_nodes()
    };
    let (report, text_layouts) = measured_layout.into_parts();
    Ok((frame_size, bounds, report, text_layouts))
}

pub(super) struct MeasuredSurfaceLayout {
    nodes: Vec<SurfaceLayoutNode>,
    measured: Vec<bool>,
    text_layouts: Vec<TextLayoutState>,
}

impl MeasuredSurfaceLayout {
    fn new(resolved: &ResolvedSurfaceTree, prior_text_layouts: Option<&[TextLayoutState]>) -> Self {
        let node_count = resolved.nodes().len();
        let text_layouts = match prior_text_layouts {
            Some(states) if states.len() == node_count => states.to_vec(),
            Some(states) => {
                debug_assert_eq!(
                    states.len(),
                    node_count,
                    "retained text-layout state remains topology aligned"
                );
                vec![TextLayoutState::new(); node_count]
            }
            None => vec![TextLayoutState::new(); node_count],
        };
        Self {
            nodes: resolved
                .nodes()
                .iter()
                .map(|node| SurfaceLayoutNode::placeholder(node.id().clone()))
                .collect(),
            measured: vec![false; node_count],
            text_layouts,
        }
    }

    fn is_measured(&self, position: usize) -> bool {
        self.measured[position]
    }

    pub(super) fn node(&self, position: usize) -> &SurfaceLayoutNode {
        debug_assert!(self.is_measured(position));
        &self.nodes[position]
    }

    fn text_layout_mut(&mut self, position: usize) -> &mut TextLayoutState {
        &mut self.text_layouts[position]
    }

    fn clear_text_layout(&mut self, position: usize) {
        self.text_layouts[position].clear();
    }

    fn record(&mut self, index: usize, node: SurfaceLayoutNode) -> LogicalSize {
        debug_assert!(index < self.nodes.len());
        let size = node.constrained_outer_size();
        self.nodes[index] = node;
        self.measured[index] = true;
        size
    }

    fn into_parts(self) -> (SurfaceLayoutReport, Vec<TextLayoutState>) {
        debug_assert!(self.measured.iter().all(|measured| *measured));
        (SurfaceLayoutReport::new(self.nodes), self.text_layouts)
    }
}

struct SurfaceMeasurer<'a> {
    text_system: &'a mut TextSystem,
}

impl<'a> SurfaceMeasurer<'a> {
    const fn new(text_system: &'a mut TextSystem) -> Self {
        Self { text_system }
    }

    fn measure_node(
        &mut self,
        resolved_tree: &ResolvedSurfaceTree,
        measured_layout: &mut MeasuredSurfaceLayout,
        node: &ResolvedSurfaceNode,
        outer_constraints: LayoutConstraints,
    ) -> Result<LogicalSize, TextLayoutError> {
        if measured_layout.is_measured(node.position) {
            return Ok(measured_layout.node(node.position).constrained_outer_size());
        }

        let padding = resolved_padding(node);
        let content_constraints = content_constraints(outer_constraints, padding);
        let mut diagnostics = Vec::new();
        let intrinsic_size = match node.measurement() {
            WidgetMeasure::Text {
                content,
                minimum_width,
                minimum_height,
                ..
            } => {
                let measured_text =
                    self.measure_text_content(measured_layout, node, content, content_constraints)?;
                apply_minimum(measured_text, *minimum_width, *minimum_height)
            }
            WidgetMeasure::Fixed { width, height } => {
                measured_layout.clear_text_layout(node.position);
                LogicalSize::new(*width, *height)
            }
            WidgetMeasure::Unsupported { reason } => {
                measured_layout.clear_text_layout(node.position);
                diagnostics.push(WidgetDiagnostic::new(
                    "runenui.measurement.unsupported",
                    format!("unsupported widget measurement capability: {reason}"),
                ));
                zero_size()
            }
            _ => {
                measured_layout.clear_text_layout(node.position);
                diagnostics.push(WidgetDiagnostic::new(
                    "runenui.measurement.unrecognized",
                    "widget measurement capability is not recognized by this runtime version",
                ));
                zero_size()
            }
        };

        let child_content_size = if let Some(child_layout) = node.child_layout() {
            let axis = child_layout_axis(child_layout, &mut diagnostics);
            self.measure_child_layout_content(
                resolved_tree,
                measured_layout,
                node,
                axis,
                content_constraints,
            )?
        } else {
            zero_size()
        };
        debug_assert!(node.child_layout().is_some() || node.children().is_empty());
        let desired_content_size = component_max_size(intrinsic_size, child_content_size);
        let constrained_content_size = content_constraints.constrain(desired_content_size);
        let desired_outer_size = expand_size_by_padding(constrained_content_size, padding);
        let constrained_outer_size = outer_constraints.constrain(desired_outer_size);
        let overflow = layout_overflow(
            desired_content_size,
            content_constraints,
            desired_outer_size,
            outer_constraints,
        );
        let measured = SurfaceLayoutNode::new(
            node.id().clone(),
            node.parent().cloned(),
            node.authored_id().cloned(),
            [outer_constraints, content_constraints],
            [
                desired_content_size,
                desired_outer_size,
                constrained_outer_size,
            ],
            overflow,
        )
        .with_diagnostics(diagnostics);

        Ok(measured_layout.record(node.position, measured))
    }

    fn measure_text_content(
        &mut self,
        measured_layout: &mut MeasuredSurfaceLayout,
        node: &ResolvedSurfaceNode,
        content: &str,
        content_constraints: LayoutConstraints,
    ) -> Result<LogicalSize, TextLayoutError> {
        let typography = node
            .resolution()
            .computed_style()
            .typography()
            .cloned()
            .unwrap_or_else(Typography::default);
        let request = TextRequest::new(
            content,
            typography,
            text_constraints_from_layout(content_constraints),
        );
        let outcome = self
            .text_system
            .layout_text(measured_layout.text_layout_mut(node.position), &request)?;
        Ok(sanitize_size(outcome.artifact().size()))
    }

    fn measure_child_layout_content(
        &mut self,
        resolved_tree: &ResolvedSurfaceTree,
        measured_layout: &mut MeasuredSurfaceLayout,
        node: &ResolvedSurfaceNode,
        axis: Axis,
        content_constraints: LayoutConstraints,
    ) -> Result<LogicalSize, TextLayoutError> {
        let child_constraints = child_constraints(axis, content_constraints);
        let gap = node.layout().gap().get();
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for (measured_child_count, child_id) in node.children().iter().enumerate() {
            let child = resolved_tree.node(child_id);
            let child_size =
                self.measure_node(resolved_tree, measured_layout, child, child_constraints)?;

            if measured_child_count > 0 {
                match axis {
                    Axis::Vertical => height = finite_sum(height, gap),
                    Axis::Horizontal => width = finite_sum(width, gap),
                }
            }
            match axis {
                Axis::Vertical => {
                    width = width.max(child_size.width());
                    height = finite_sum(height, child_size.height());
                }
                Axis::Horizontal => {
                    width = finite_sum(width, child_size.width());
                    height = height.max(child_size.height());
                }
            }
        }

        Ok(logical_size_from_arithmetic(width, height))
    }
}

const fn text_constraints_from_layout(constraints: LayoutConstraints) -> TextConstraints {
    match constraints.horizontal().max() {
        AxisLimit::Finite(max) => TextConstraints::limited(max),
        AxisLimit::Unbounded => TextConstraints::unbounded(),
    }
}

fn apply_minimum(
    size: LogicalSize,
    minimum_width: LogicalLength,
    minimum_height: LogicalLength,
) -> LogicalSize {
    logical_size_from_arithmetic(
        max_extent(size.width(), minimum_width.get()),
        max_extent(size.height(), minimum_height.get()),
    )
}

const fn zero_size() -> LogicalSize {
    LogicalSize::new(LogicalLength::ZERO, LogicalLength::ZERO)
}

fn component_max_size(left: LogicalSize, right: LogicalSize) -> LogicalSize {
    logical_size_from_arithmetic(
        left.width().max(right.width()),
        left.height().max(right.height()),
    )
}

fn child_layout_axis(child_layout: ChildLayout, diagnostics: &mut Vec<WidgetDiagnostic>) -> Axis {
    if let ChildLayout::Linear { axis } = child_layout {
        axis
    } else {
        diagnostics.push(WidgetDiagnostic::new(
            "runenui.child-layout.unrecognized",
            "child layout capability is not recognized; using vertical linear fallback",
        ));
        Axis::Vertical
    }
}

pub(super) const fn child_layout_axis_without_diagnostic(child_layout: ChildLayout) -> Axis {
    match child_layout {
        ChildLayout::Linear { axis } => axis,
        _ => Axis::Vertical,
    }
}

const fn max_extent(left: f32, right: f32) -> f32 {
    if left > right { left } else { right }
}

pub(super) fn resolved_padding(node: &ResolvedSurfaceNode) -> EdgeInsets {
    node.resolution()
        .computed_style()
        .padding()
        .unwrap_or(EdgeInsets::ZERO)
}

fn content_constraints(
    outer_constraints: LayoutConstraints,
    padding: EdgeInsets,
) -> LayoutConstraints {
    LayoutConstraints::new(
        content_axis_constraints(outer_constraints.horizontal(), horizontal_padding(padding)),
        content_axis_constraints(outer_constraints.vertical(), vertical_padding(padding)),
    )
}

fn child_constraints(axis: Axis, content_constraints: LayoutConstraints) -> LayoutConstraints {
    match axis {
        Axis::Vertical => LayoutConstraints::new(
            loose_axis(content_constraints.horizontal()),
            AxisConstraints::unbounded(),
        ),
        Axis::Horizontal => LayoutConstraints::new(
            AxisConstraints::unbounded(),
            loose_axis(content_constraints.vertical()),
        ),
    }
}

fn loose_axis(axis: AxisConstraints) -> AxisConstraints {
    match axis.max() {
        AxisLimit::Finite(max) => AxisConstraints::loose(max),
        AxisLimit::Unbounded => AxisConstraints::unbounded(),
    }
}

fn content_axis_constraints(axis: AxisConstraints, padding: f32) -> AxisConstraints {
    let max = match axis.max() {
        AxisLimit::Finite(max) => AxisLimit::Finite(logical_extent_from_arithmetic(
            subtract_extent(max.get(), padding),
        )),
        AxisLimit::Unbounded => AxisLimit::Unbounded,
    };

    AxisConstraints::new(
        logical_extent_from_arithmetic(subtract_extent(axis.min().get(), padding)),
        max,
    )
}

fn expand_size_by_padding(size: LogicalSize, padding: EdgeInsets) -> LogicalSize {
    logical_size_from_arithmetic(
        finite_sum(size.width(), horizontal_padding(padding)),
        finite_sum(size.height(), vertical_padding(padding)),
    )
}

const fn sanitize_size(size: LogicalSize) -> LogicalSize {
    size
}

fn layout_overflow(
    desired_content_size: LogicalSize,
    content_constraints: LayoutConstraints,
    desired_outer_size: LogicalSize,
    outer_constraints: LayoutConstraints,
) -> LayoutOverflow {
    LayoutOverflow::new(
        axis_overflow(
            desired_content_size.width(),
            content_constraints.horizontal(),
            desired_outer_size.width(),
            outer_constraints.horizontal(),
        ),
        axis_overflow(
            desired_content_size.height(),
            content_constraints.vertical(),
            desired_outer_size.height(),
            outer_constraints.vertical(),
        ),
    )
}

fn axis_overflow(
    desired_content: f32,
    content_constraints: AxisConstraints,
    desired_outer: f32,
    outer_constraints: AxisConstraints,
) -> bool {
    exceeds_finite_max(desired_content, content_constraints.max())
        || exceeds_finite_max(desired_outer, outer_constraints.max())
}

fn exceeds_finite_max(desired: f32, maximum: AxisLimit) -> bool {
    matches!(maximum, AxisLimit::Finite(max) if desired > max.get())
}

fn horizontal_padding(padding: EdgeInsets) -> f32 {
    finite_sum(padding.left().get(), padding.right().get())
}

fn vertical_padding(padding: EdgeInsets) -> f32 {
    finite_sum(padding.top().get(), padding.bottom().get())
}

fn subtract_extent(value: f32, amount: f32) -> f32 {
    let difference = value - amount;
    if difference > 0.0 { difference } else { 0.0 }
}

pub(super) fn finite_sum(left: f32, right: f32) -> f32 {
    finite_saturating_add(extent_from_arithmetic(left), extent_from_arithmetic(right))
}

pub(super) fn finite_saturating_add(left: f32, right: f32) -> f32 {
    let sum = left + right;
    if sum.is_finite() {
        sum
    } else if left.is_sign_negative() && right.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

fn extent_from_arithmetic(value: f32) -> f32 {
    debug_assert!(
        !value.is_nan() && value >= 0.0,
        "internal extent arithmetic must be non-negative and not NaN"
    );
    if value.is_finite() && value > 0.0 {
        value
    } else if value == f32::INFINITY {
        f32::MAX
    } else {
        0.0
    }
}

pub(super) fn logical_extent_from_arithmetic(value: f32) -> LogicalLength {
    LogicalLength::new(extent_from_arithmetic(value)).unwrap_or_default()
}

fn logical_size_from_arithmetic(width: f32, height: f32) -> LogicalSize {
    LogicalSize::new(
        logical_extent_from_arithmetic(width),
        logical_extent_from_arithmetic(height),
    )
}
