#![allow(refining_impl_trait)]

use core::{num::NonZeroUsize, time::Duration};

use runenui_core::{
    CommandOrigin, Dialog, LayoutDimension, LayoutStyle, LogicalLength, LogicalPoint,
    NoHostProtocol, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, Popover,
    PresentationDismissReason, PresentationFocusPolicy, PresentationOutsidePointerPolicy,
    SemanticCommand, SemanticRole, SurfacePresentation, SurfacePresentationAnchor,
    SurfacePresentationPlacement, SurfacePresentationSide, Tooltip, TooltipTrigger, UiApp, View,
    button, column,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    ShowTooltip,
    HideTooltip,
    DismissDialog(PresentationDismissReason),
    DismissPopover(PresentationDismissReason),
    ActivateBackground,
}

#[derive(Clone, Debug)]
struct Model {
    tooltip_visible: bool,
    dialog_open: bool,
    popover_open: bool,
    show_count: usize,
    hide_count: usize,
    dismissals: Vec<(bool, PresentationDismissReason)>,
    background_activations: usize,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            tooltip_visible: false,
            dialog_open: false,
            popover_open: false,
            show_count: 0,
            hide_count: 0,
            dismissals: Vec::new(),
            background_activations: 0,
        }
    }
}

struct PresentationApp;

fn fixed(width: u16, height: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::length(LogicalLength::from(width)))
        .with_height(LayoutDimension::length(LogicalLength::from(height)))
}

impl UiApp for PresentationApp {
    type State = Model;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let button = button("Explain")
            .id("help-button")
            .with_layout(fixed(80, 24))
            .on_activate(|| Action::ActivateBackground);
        let button = if state.tooltip_visible {
            button.described_by("help-tooltip")
        } else {
            button
        };
        let mut trigger = TooltipTrigger::new(
            button,
            Duration::from_millis(400),
            || Action::ShowTooltip,
            || Action::HideTooltip,
        )
        .id("help-trigger");

        if state.tooltip_visible {
            trigger = trigger.with_tooltip(Tooltip::new("Helpful details").id("help-tooltip"));
        }

        let mut children = vec![trigger.into_element()];
        if state.popover_open {
            children.push(
                Popover::new(
                    vec![button("Popover action")
                        .id("popover-button")
                        .with_layout(fixed(70, 20))
                        .on_activate(|| Action::ActivateBackground)],
                    SurfacePresentation::new(SurfacePresentationPlacement::new(
                        SurfacePresentationSide::Bottom,
                    ))
                    .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                        LogicalPoint::new(110.0, 50.0)
                            .unwrap_or_else(|_| unreachable!("finite test position")),
                    ))
                    .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
                    .dismiss_on_cancel_or_back(true),
                )
                .id("test-popover")
                .on_dismiss(Action::DismissPopover)
                .into_element(),
            );
        }
        if state.dialog_open {
            children.push(
                Dialog::new(
                    "Settings",
                    vec![button("Inside")
                        .id("dialog-button")
                        .with_layout(fixed(90, 24))
                        .on_activate(|| Action::ActivateBackground)],
                )
                .id("test-dialog")
                .on_dismiss(Action::DismissDialog)
                .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::ShowTooltip => {
                state.tooltip_visible = true;
                state.show_count += 1;
            }
            Action::HideTooltip => {
                state.tooltip_visible = false;
                state.hide_count += 1;
            }
            Action::DismissDialog(reason) => {
                state.dialog_open = false;
                state.dismissals.push((true, reason));
            }
            Action::DismissPopover(reason) => {
                state.popover_open = false;
                state.dismissals.push((false, reason));
            }
            Action::ActivateBackground => state.background_activations += 1,
        }
    }
}

fn settle(harness: &mut TestHarness<PresentationApp>) {
    assert_eq!(
        harness.run_until_idle(SettleBudget::new(
            NonZeroUsize::new(12).unwrap_or(NonZeroUsize::MIN),
            PumpBudget::new(128, 128, 128, 128),
        ))
        .outcome(),
        SettleOutcome::Idle
    );
}

fn node_center(harness: &TestHarness<PresentationApp>, name: &str) -> LogicalPoint {
    let published = harness
        .publication()
        .unwrap_or_else(|| unreachable!("test publication exists"));
    let node = published
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == name))
        .unwrap_or_else(|| unreachable!("authored test node is published"));
    let rect = node.bounds();
    LogicalPoint::new(
        rect.width().mul_add(0.5, rect.x()),
        rect.height().mul_add(0.5, rect.y()),
    )
    .unwrap_or_else(|_| unreachable!("published node has finite geometry"))
}

fn move_pointer(harness: &mut TestHarness<PresentationApp>, position: LogicalPoint) {
    let context = harness
        .input_context()
        .unwrap_or_else(|_| unreachable!("test has displayed surface"))
        .clone();
    harness
        .submit_pointer(PointerEvent::new(
            PointerId::new(332).unwrap_or_else(|| unreachable!("pointer is nonzero")),
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            position,
            context,
        ))
        .unwrap_or_else(|_| unreachable!("pointer ingress is valid"));
    settle(harness);
}

#[test]
fn tooltip_delay_is_mounted_and_visibility_remains_app_owned() {
    let mut harness = TestHarness::<PresentationApp>::mount(Model::default());
    assert!(harness.publish().is_ok());
    let button = node_center(&harness, "help-button");
    move_pointer(&mut harness, button);
    assert!(!harness.state().tooltip_visible);
    assert_eq!(harness.state().show_count, 0);
    assert!(harness.last_timer_start_outcome().is_some());

    assert!(harness.advance_time(Duration::from_millis(399)).is_ok());
    settle(&mut harness);
    assert_eq!(harness.state().show_count, 0);

    assert!(harness.advance_time(Duration::from_millis(1)).is_ok());
    settle(&mut harness);
    assert_eq!(harness.state().show_count, 1);
    assert!(harness.state().tooltip_visible);
    assert!(harness.last_timer_firing_outcome().is_some());

    assert!(harness.publish().is_ok());
    assert!(
        harness
            .unique_semantic_target(
                &SemanticQuery::new()
                    .with_role(SemanticRole::Tooltip)
                    .with_name("Helpful details"),
            )
            .is_ok()
    );

    move_pointer(
        &mut harness,
        LogicalPoint::new(250.0, 250.0)
            .unwrap_or_else(|_| unreachable!("finite test point")),
    );
    assert!(!harness.state().tooltip_visible);
    assert_eq!(harness.state().hide_count, 1);
    assert!(harness.publish().is_ok());
    assert!(
        harness
            .query_semantics(&SemanticQuery::new().with_role(SemanticRole::Tooltip))
            .unwrap_or_else(|_| unreachable!("semantic query is valid"))
            .is_empty()
    );
}

#[test]
fn pointer_leave_cancels_delayed_tooltip_show_without_late_action() {
    let mut harness = TestHarness::<PresentationApp>::mount(Model::default());
    assert!(harness.publish().is_ok());
    let point = node_center(&harness, "help-button");
    move_pointer(&mut harness, point);
    move_pointer(
        &mut harness,
        LogicalPoint::new(250.0, 250.0)
            .unwrap_or_else(|_| unreachable!("finite outside point")),
    );
    assert!(harness.advance_time(Duration::from_secs(1)).is_ok());
    settle(&mut harness);
    assert_eq!(harness.state().show_count, 0);
    assert!(!harness.state().tooltip_visible);
}

#[test]
fn dialog_and_popover_dismissal_actions_are_application_owned() {
    let mut state = Model::default();
    state.dialog_open = true;
    let mut dialog = TestHarness::<PresentationApp>::mount(state);
    assert!(dialog.publish().is_ok());
    assert!(
        dialog
            .unique_semantic_target(
                &SemanticQuery::new()
                    .with_role(SemanticRole::Dialog)
                    .with_name("Settings"),
            )
            .is_ok()
    );
    let id = dialog
        .publication()
        .unwrap_or_else(|| unreachable!("dialog published"))
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "test-dialog")
        })
        .unwrap_or_else(|| unreachable!("dialog has exact mounted identity"))
        .id()
        .clone();
    dialog
        .submit_command(
            id,
            SemanticCommand::PresentationDismiss(PresentationDismissReason::CancelOrBack),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("ordinary mounted command admitted"));
    settle(&mut dialog);
    assert!(!dialog.state().dialog_open);
    assert_eq!(
        dialog.state().dismissals,
        [(true, PresentationDismissReason::CancelOrBack)]
    );

    let mut state = Model::default();
    state.popover_open = true;
    let mut popover = TestHarness::<PresentationApp>::mount(state);
    assert!(popover.publish().is_ok());
    let id = popover
        .publication()
        .unwrap_or_else(|| unreachable!("popover published"))
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "test-popover")
        })
        .unwrap_or_else(|| unreachable!("popover has exact mounted identity"))
        .id()
        .clone();
    popover
        .submit_command(
            id,
            SemanticCommand::PresentationDismiss(PresentationDismissReason::OutsidePointer),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("ordinary mounted command admitted"));
    settle(&mut popover);
    assert_eq!(
        popover.state().dismissals,
        [(false, PresentationDismissReason::OutsidePointer)]
    );
    assert!(!popover.state().popover_open);
}

#[test]
fn tooltip_projection_cannot_become_modal_or_enter_focus() {
    let element = Tooltip::<Action>::new("Safe")
        .with_presentation(
            SurfacePresentation::new(SurfacePresentationPlacement::new(
                SurfacePresentationSide::Right,
            ))
            .modal(true)
            .with_outside_pointer(PresentationOutsidePointerPolicy::Block)
            .with_focus_policy(PresentationFocusPolicy::EnterAndRestore),
        )
        .into_element();
    let config = element
        .surface_presentation_config()
        .unwrap_or_else(|| unreachable!("tooltip is a presented element"));
    assert!(!config.is_modal());
    assert_eq!(
        config.outside_pointer(),
        PresentationOutsidePointerPolicy::Ignore
    );
    assert_eq!(config.focus_policy(), PresentationFocusPolicy::Preserve);
}
