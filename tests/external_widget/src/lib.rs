//! Genuine downstream consumer of only `RunenUI`'s public APIs.

#![forbid(unsafe_code)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Axis, ChildLayout, ChildLayoutWidget, Container, EdgeInsets, Element, LogicalLength, View,
    Views, Widget, WidgetActivation, WidgetDiagnostic, WidgetLifecycle, WidgetLifecycleContext,
    WidgetLifecycleRequest, WidgetMeasure, WidgetPaintProof, WidgetSemanticProof, WidgetTextKind,
    button, children, column, container, text,
};
use runenui_runtime::UiApp;

#[derive(Debug, Eq, PartialEq)]
pub enum ChildAction {
    Pulse,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ParentAction {
    Child(ChildAction),
    Reset,
}

#[derive(Debug, Default)]
pub struct PulseState {
    lifecycle_count: usize,
}

#[derive(Debug)]
pub struct PulseButton {
    label: String,
    enabled: bool,
    action: Option<ChildAction>,
}

impl PulseButton {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            action: Some(ChildAction::Pulse),
        }
    }

    #[must_use]
    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

impl Widget<ChildAction> for PulseButton {
    type State = PulseState;

    fn create_state(&self) -> Self::State {
        PulseState::default()
    }

    fn activation(&self) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }

    fn activate(&mut self) -> Option<ChildAction> {
        if self.enabled {
            self.action.take()
        } else {
            None
        }
    }

    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(80.0).unwrap_or_default(),
            height: LogicalLength::new(24.0).unwrap_or_default(),
        }
    }

    fn paint(&self) -> WidgetPaintProof {
        WidgetPaintProof::new("pulse", format!("label={:?}", self.label))
    }

    fn semantics(&self) -> WidgetSemanticProof {
        WidgetSemanticProof::new("pulse-button", self.label.clone())
            .with_enabled(self.enabled)
            .with_action("pulse")
    }

    fn diagnostics(&self) -> Vec<WidgetDiagnostic> {
        vec![WidgetDiagnostic::new(
            "external.pulse.ready",
            format!("{} is ready", self.label),
        )]
    }

    fn lifecycle(
        &self,
        state: &mut Self::State,
        event: WidgetLifecycle,
        context: &mut WidgetLifecycleContext,
    ) {
        state.lifecycle_count += 1;
        context.request(WidgetLifecycleRequest::Diagnostic(WidgetDiagnostic::new(
            "external.pulse.lifecycle",
            format!("{}:{event:?}", state.lifecycle_count),
        )));
    }
}

#[must_use]
pub fn child_component() -> Element<ChildAction> {
    Element::new(PulseButton::new("Pulse"))
        .id("external.pulse")
        .key("pulse-key")
        .padding(EdgeInsets::all(LogicalLength::from(2_u16)))
}

#[must_use]
pub fn parent_view() -> Element<ParentAction> {
    custom_column(children![
        text("External widget"),
        child_component().map_action(ParentAction::Child),
        button("Reset")
            .id("external.reset")
            .on_press(ParentAction::Reset),
    ])
    .id("external.panel")
    .key("panel-key")
    .into_element()
}

#[derive(Debug)]
pub struct CustomColumn;

impl<Action> Widget<Action> for CustomColumn {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn paint(&self) -> WidgetPaintProof {
        WidgetPaintProof::new("external-panel", "axis=Vertical")
    }
    fn semantics(&self) -> WidgetSemanticProof {
        WidgetSemanticProof::new("group", "External panel")
    }
    fn diagnostics(&self) -> Vec<WidgetDiagnostic> {
        vec![WidgetDiagnostic::new(
            "external.panel.ready",
            "external child-layout widget is ready",
        )]
    }
}

impl<Action> ChildLayoutWidget<Action> for CustomColumn {
    fn child_layout(&self) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[must_use]
pub fn custom_column<Action>(children: impl Views<Action>) -> Container<Action> {
    container(CustomColumn, children)
}

#[derive(Debug)]
pub struct CustomRow;

impl<Action> Widget<Action> for CustomRow {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn paint(&self) -> WidgetPaintProof {
        WidgetPaintProof::new("external-row", "axis=Horizontal")
    }
    fn semantics(&self) -> WidgetSemanticProof {
        WidgetSemanticProof::new("group", "External row")
    }
}

impl<Action> ChildLayoutWidget<Action> for CustomRow {
    fn child_layout(&self) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Horizontal,
        }
    }
}

#[derive(Debug)]
pub struct MinimumPanel;

impl<Action> Widget<Action> for MinimumPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(180.0).unwrap_or_default(),
            height: LogicalLength::new(60.0).unwrap_or_default(),
        }
    }
}

impl<Action> ChildLayoutWidget<Action> for MinimumPanel {
    fn child_layout(&self) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[derive(Debug)]
pub struct TextMinimumPanel;

impl<Action> Widget<Action> for TextMinimumPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: "external text intrinsic minimum".to_owned(),
            kind: WidgetTextKind::Text,
            minimum_width: LogicalLength::ZERO,
            minimum_height: LogicalLength::ZERO,
        }
    }
}

impl<Action> ChildLayoutWidget<Action> for TextMinimumPanel {
    fn child_layout(&self) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Horizontal,
        }
    }
}

#[derive(Debug)]
pub struct UnsupportedMinimumPanel;

impl<Action> Widget<Action> for UnsupportedMinimumPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::Unsupported {
            reason: "external child-layout intrinsic proof",
        }
    }
}

impl<Action> ChildLayoutWidget<Action> for UnsupportedMinimumPanel {
    fn child_layout(&self) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[must_use]
pub fn custom_row<Action>(children: impl Views<Action>) -> Container<Action> {
    container(CustomRow, children)
}

#[must_use]
pub fn minimum_panel<Action>(children: impl Views<Action>) -> Container<Action> {
    container(MinimumPanel, children)
}

#[must_use]
pub fn text_minimum_panel<Action>(children: impl Views<Action>) -> Container<Action> {
    container(TextMinimumPanel, children)
}

#[must_use]
pub fn unsupported_minimum_panel<Action>(children: impl Views<Action>) -> Container<Action> {
    container(UnsupportedMinimumPanel, children)
}

#[must_use]
pub fn diagnostic_panel() -> Element<ParentAction> {
    custom_column(children![
        text("first").id("external.duplicate").key("duplicate-key"),
        custom_column(children![
            text("nested").id("external.duplicate"),
            button("Nested action")
                .id("external.nested")
                .on_press(ChildAction::Pulse)
                .into_element()
                .map_action(ParentAction::Child),
        ])
        .key("duplicate-key"),
        text("third"),
        text("fourth"),
    ])
    .id("external.diagnostic-panel")
    .into_element()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutCase {
    BuiltInColumn,
    ExternalColumn,
    ExternalRow,
    FixedMinimum,
    TextMinimum,
    UnsupportedMinimum,
    NestedExternal,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LayoutAction {
    Activate,
}

#[derive(Debug)]
pub struct LayoutState {
    pub case: LayoutCase,
    pub activations: usize,
}

fn layout_children() -> Vec<Element<LayoutAction>> {
    vec![
        text("layout child").id("layout.label").into_element(),
        button("Activate")
            .id("layout.action")
            .key("action-key")
            .on_press(LayoutAction::Activate)
            .into_element(),
        text("tail").id("layout.tail").into_element(),
    ]
}

#[must_use]
pub fn layout_case_view(case: LayoutCase) -> Element<LayoutAction> {
    match case {
        LayoutCase::BuiltInColumn => column(layout_children())
            .id("layout.root")
            .gap(3_u16)
            .into_element(),
        LayoutCase::ExternalColumn => custom_column(layout_children())
            .id("layout.root")
            .gap(5_u16)
            .into_element(),
        LayoutCase::ExternalRow => custom_row(layout_children())
            .id("layout.root")
            .gap(7_u16)
            .into_element(),
        LayoutCase::FixedMinimum => minimum_panel(layout_children())
            .id("layout.root")
            .gap(4_u16)
            .into_element(),
        LayoutCase::TextMinimum => text_minimum_panel(layout_children())
            .id("layout.root")
            .gap(6_u16)
            .into_element(),
        LayoutCase::UnsupportedMinimum => unsupported_minimum_panel(layout_children())
            .id("layout.root")
            .gap(8_u16)
            .into_element(),
        LayoutCase::NestedExternal => custom_column(children![
            text("nested head").id("layout.label"),
            custom_row(children![
                button("Activate")
                    .id("layout.action")
                    .key("action-key")
                    .on_press(LayoutAction::Activate),
                text("nested tail").id("layout.tail"),
            ])
            .id("layout.nested")
            .gap(11_u16),
        ])
        .id("layout.root")
        .gap(13_u16)
        .into_element(),
    }
}

pub struct LayoutConformanceApp;

impl UiApp for LayoutConformanceApp {
    type State = LayoutState;
    type Action = LayoutAction;

    fn root(state: &Self::State) -> Element<Self::Action> {
        layout_case_view(state.case)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            LayoutAction::Activate => state.activations += 1,
        }
    }
}

pub struct ConformanceApp;

impl UiApp for ConformanceApp {
    type State = usize;
    type Action = ParentAction;

    fn root(_: &Self::State) -> Element<Self::Action> {
        parent_view()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            ParentAction::Child(ChildAction::Pulse) => *state += 1,
            ParentAction::Reset => *state = 0,
        }
    }
}

#[derive(Debug)]
pub struct GenericWidget<T>(pub T);

impl<T: core::fmt::Debug + 'static> Widget<()> for GenericWidget<T> {
    type State = ();
    fn create_state(&self) -> Self::State {}
}

#[derive(Debug)]
pub struct CountingLayoutPanel {
    measure_calls: Rc<Cell<usize>>,
    child_layout_calls: Rc<Cell<usize>>,
}

impl Widget<()> for CountingLayoutPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        self.measure_calls.set(self.measure_calls.get() + 1);
        WidgetMeasure::default()
    }
}

impl ChildLayoutWidget<()> for CountingLayoutPanel {
    fn child_layout(&self) -> ChildLayout {
        let call = self.child_layout_calls.get() + 1;
        self.child_layout_calls.set(call);
        ChildLayout::Linear {
            axis: if call % 2 == 1 {
                Axis::Vertical
            } else {
                Axis::Horizontal
            },
        }
    }
}

#[derive(Debug)]
struct CountingText {
    calls: Rc<Cell<usize>>,
}

impl Widget<()> for CountingText {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        self.calls.set(self.calls.get() + 1);
        WidgetMeasure::Text {
            content: "counted descriptor".to_owned(),
            kind: WidgetTextKind::ControlLabel,
            minimum_width: LogicalLength::ZERO,
            minimum_height: LogicalLength::ZERO,
        }
    }
}

#[derive(Debug)]
struct CountingFixed {
    calls: Rc<Cell<usize>>,
}

impl Widget<()> for CountingFixed {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        self.calls.set(self.calls.get() + 1);
        WidgetMeasure::Fixed {
            width: LogicalLength::new(20.0).unwrap_or_default(),
            height: LogicalLength::new(7.0).unwrap_or_default(),
        }
    }
}

#[must_use]
pub fn counting_measurement_tree(
    panel_calls: Rc<Cell<usize>>,
    child_layout_calls: Rc<Cell<usize>>,
    text_calls: Rc<Cell<usize>>,
    fixed_calls: Rc<Cell<usize>>,
) -> Element<()> {
    container(
        CountingLayoutPanel {
            measure_calls: panel_calls,
            child_layout_calls,
        },
        children![
            Element::new(CountingText { calls: text_calls }).id("external.counted-text"),
            Element::new(CountingFixed { calls: fixed_calls }).id("external.counted-fixed"),
        ],
    )
    .id("external.counting-panel")
    .into_element()
}

#[derive(Debug)]
pub struct UnsupportedMeasure;

impl Widget<()> for UnsupportedMeasure {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::Unsupported {
            reason: "external proof capability",
        }
    }
}
