//! Renderer-neutral visible proof models for generic interaction replay.

use crate::WidgetId;

use super::fixture::{MountedInteractionControl, MountedInteractionFixture};
use super::inspector::{InteractionInspectorView, InteractionReportView};
use super::report::InteractionFormationReport;
use super::state_mapping::{
    folded_current_states, push_marker, visible_state_from_fact, visible_state_from_outcome,
    visible_state_from_transition,
};

/// Renderer-neutral visible proof for Phase 12 generic interaction.
///
/// This model is not a product UI and does not add a story/gallery framework.
/// It is the static proof surface that an existing gallery or static mount path
/// can render: a main view, inspector view, and report/event view formed from
/// the same descriptor-backed replay report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionVisualProof {
    /// Stable proof identifier.
    pub proof_id: String,

    /// Main visible proof view containing mounted controls.
    pub main_view: InteractionVisualMainView,

    /// Inspector visible proof view for the selected control.
    pub inspector_view: InteractionInspectorView,

    /// Report/event visible proof view.
    pub report_view: InteractionReportView,
}

impl InteractionVisualProof {
    /// Projects a descriptor-backed interaction replay into visible proof data.
    ///
    /// The proof contains three renderer-neutral views: main, inspector, and
    /// report/event. It is proof evidence rather than product UI.
    pub fn from_fixture_report(
        proof_id: impl Into<String>,
        fixture: &MountedInteractionFixture,
        report: &InteractionFormationReport,
        selected_widget: WidgetId,
    ) -> Self {
        let selected_control = fixture.control(selected_widget);
        Self {
            proof_id: proof_id.into(),
            main_view: InteractionVisualMainView::from_fixture_report(fixture, report),
            inspector_view: InteractionInspectorView::from_fixture_report(selected_control, report),
            report_view: InteractionReportView::from_report(report),
        }
    }
}

/// Main-view controls and markers that a static story can render visibly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionVisualMainView {
    /// Fixture/story id shown by the main proof view.
    pub mounted_story_id: String,

    /// Mounted controls and their observed/current visual state evidence.
    pub controls: Vec<InteractionVisualControl>,
}

impl InteractionVisualMainView {
    fn from_fixture_report(
        fixture: &MountedInteractionFixture,
        report: &InteractionFormationReport,
    ) -> Self {
        Self {
            mounted_story_id: fixture.mounted_story_id.clone(),
            controls: fixture
                .controls
                .iter()
                .map(|control| InteractionVisualControl::from_report(control, report))
                .collect(),
        }
    }

    /// Finds one visible proof control by widget id.
    pub fn control(&self, widget_id: WidgetId) -> Option<&InteractionVisualControl> {
        self.controls
            .iter()
            .find(|control| control.widget_id == widget_id)
    }
}

/// One visible mounted control in the Phase 12 proof model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionVisualControl {
    /// Mounted widget id represented by this proof control.
    pub widget_id: WidgetId,

    /// Package control kind id represented by this proof control.
    pub control_kind_id: String,

    /// Human-readable label rendered by proof adapters.
    pub label: String,

    /// Visual evidence observed at any point during replay.
    pub observed_markers: Vec<InteractionVisualMarker>,

    /// Final reusable state after replay transitions are folded in order.
    pub current_states: Vec<InteractionVisibleState>,
}

impl InteractionVisualControl {
    fn from_report(
        control: &MountedInteractionControl,
        report: &InteractionFormationReport,
    ) -> Self {
        let mut observed_markers = vec![InteractionVisualMarker::new(
            InteractionVisibleState::Normal,
        )];
        if !control.enabled {
            push_marker(&mut observed_markers, InteractionVisibleState::Disabled);
        }
        if control.read_only {
            push_marker(&mut observed_markers, InteractionVisibleState::ReadOnly);
        }
        for transition in report
            .state_transitions
            .iter()
            .filter(|transition| transition.target == control.widget_id)
        {
            if let Some(state) = visible_state_from_transition(&transition.state) {
                push_marker(&mut observed_markers, state);
            }
        }
        for fact in report
            .runtime_facts
            .iter()
            .filter(|fact| fact.target == control.widget_id)
        {
            if let Some(state) = visible_state_from_fact(&fact.fact) {
                push_marker(&mut observed_markers, state);
            }
        }
        for outcome in report
            .semantic_outcomes
            .iter()
            .filter(|outcome| outcome.target == control.widget_id)
        {
            if let Some(state) = visible_state_from_outcome(&outcome.outcome) {
                push_marker(&mut observed_markers, state);
            }
        }
        if report
            .suppressed_events
            .iter()
            .any(|event| event.target == control.widget_id)
        {
            push_marker(&mut observed_markers, InteractionVisibleState::Suppressed);
        }

        Self {
            widget_id: control.widget_id,
            control_kind_id: control.control_kind_id.clone(),
            label: control.label.clone(),
            observed_markers,
            current_states: folded_current_states(control, report),
        }
    }

    /// Returns true when the replay observed a marker for this control.
    pub fn has_marker(&self, state: InteractionVisibleState) -> bool {
        self.observed_markers
            .iter()
            .any(|marker| marker.state == state)
    }

    /// Returns true when the folded final state contains this state.
    pub fn has_current_state(&self, state: InteractionVisibleState) -> bool {
        self.current_states.contains(&state)
    }
}

/// A named visible marker, such as hovered, pressed, or text-intent probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionVisualMarker {
    /// Stable visible state represented by this marker.
    pub state: InteractionVisibleState,

    /// Human-readable marker label.
    pub label: String,
}

impl InteractionVisualMarker {
    /// Creates a marker with the state's stable label.
    pub fn new(state: InteractionVisibleState) -> Self {
        Self {
            label: state.as_str().to_owned(),
            state,
        }
    }
}

/// Visible proof states used by the renderer-neutral Phase 12 proof surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InteractionVisibleState {
    /// Normal baseline proof marker.
    Normal,

    /// Pointer hover was observed.
    Hovered,

    /// Pointer press was observed.
    Pressed,

    /// Pointer capture was observed.
    Captured,

    /// Focus was observed.
    Focused,

    /// Keyboard-visible focus was observed.
    FocusVisible,

    /// Active/navigation state was observed.
    Active,

    /// Disabled state or suppression target was observed.
    Disabled,

    /// Read-only state was observed.
    ReadOnly,

    /// Suppressed interaction evidence was observed.
    Suppressed,

    /// Activation was requested by a declared reusable requirement.
    ActivationRequested,

    /// Action intent was emitted by a declared reusable requirement.
    ActionIntent,

    /// List active-item intent was emitted.
    ListActiveItemIntent,

    /// Tree node intent was emitted.
    TreeNodeIntent,

    /// Table cell or row intent was emitted.
    TableCellOrRowIntent,

    /// Text intent was observed as a probe, without editing.
    TextIntentProbe,

    /// Read-only text intent was observed as a probe, without editing.
    ReadOnlyTextIntentProbe,
}

impl InteractionVisibleState {
    /// Returns the stable report/proof label for this visible state.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Hovered => "hovered",
            Self::Pressed => "pressed",
            Self::Captured => "captured",
            Self::Focused => "focused",
            Self::FocusVisible => "focus-visible",
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::ReadOnly => "read-only",
            Self::Suppressed => "suppressed",
            Self::ActivationRequested => "activation-requested",
            Self::ActionIntent => "action-intent",
            Self::ListActiveItemIntent => "list-active-item-intent",
            Self::TreeNodeIntent => "tree-node-intent",
            Self::TableCellOrRowIntent => "table-cell-or-row-intent",
            Self::TextIntentProbe => "text-intent-probe",
            Self::ReadOnlyTextIntentProbe => "read-only-text-intent-probe",
        }
    }
}
