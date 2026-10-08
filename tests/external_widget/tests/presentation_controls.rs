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
    OpenDialog,
    ShowTooltip,
    HideTooltip,
    DismissDialog(PresentationDismissReason),
    DismissPopover(PresentationDismissReason),
    ActivateBackground,
}

#[derive(Clone, Debug, Default)]
struct Model {
    tooltip_visible: bool,
    dialog_open: bool,
    popover_open: bool,
    show_count: usize,
    hide_count: usize,
    dismissals: Vec<(bool, PresentationDismissReason)>,
    background_activations: usize,
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
        let help_button = button("Explain")
            .id("help-button")
            .with_layout(fixed(80, 24))
            .on_activate(|| Action::ActivateBackground);
        let help_button = if state.tooltip_visible {
            help_button.described_by("help-tooltip")
        } else {
            help_button
        };
        let mut trigger = TooltipTrigger::new(
            help_button,
            Duration::from_millis(400),
            || Action::ShowTooltip,
            || Action::HideTooltip,
        )
        .id("help-trigger");

        if state.tooltip_visible {
            trigger = trigger.with_tooltip(Tooltip::new("Helpful details").id("help-tooltip"));
        }

        let mut children = vec![
            trigger.into_element(),
            button("Elsewhere")
                .id("focus-elsewhere")
                .with_layout(fixed(70, 24))
                .on_activate(|| Action::ActivateBackground)
                .into_element(),
        ];
        if state.popover_open {
            children.push(
                Popover::new(
                    vec![
                        button("Popover action")
                            .id("popover-button")
                            .with_layout(fixed(70, 20))
                            .on_activate(|| Action::ActivateBackground),
                    ],
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
                    vec![
                        button("Inside")
                            .id("dialog-button")
                            .with_layout(fixed(90, 24))
                            .on_activate(|| Action::ActivateBackground),
                    ],
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
            Action::OpenDialog => state.dialog_open = true,
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
        harness
            .run_until_idle(SettleBudget::new(
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
    let semantic = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("published semantic snapshot exists"));
    let owner = semantic
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Button && node.name() == Some("Explain"))
        .unwrap_or_else(|| unreachable!("semantic Button is published"));
    let tooltip = semantic
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Tooltip)
        .unwrap_or_else(|| unreachable!("semantic Tooltip is published"));
    assert!(owner.relationships().iter().any(|relationship| {
        relationship.kind() == runenui_core::SemanticRelationshipKind::DescribedBy
            && relationship.target() == tooltip.id()
    }));

    move_pointer(
        &mut harness,
        LogicalPoint::new(250.0, 250.0).unwrap_or_else(|_| unreachable!("finite test point")),
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
fn tooltip_focus_activation_uses_same_mounted_timer_and_blur_hides() {
    let mut harness = TestHarness::<PresentationApp>::mount(Model::default());
    assert!(harness.publish().is_ok());
    let first = harness
        .publication()
        .unwrap_or_else(|| unreachable!("focus fixture has a publication"))
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "help-button")
        })
        .unwrap_or_else(|| unreachable!("help button is mounted"))
        .id()
        .clone();
    harness
        .submit_command(
            first,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is accepted"));
    settle(&mut harness);
    assert!(!harness.state().tooltip_visible);
    assert!(harness.last_timer_start_outcome().is_some());

    assert!(harness.advance_time(Duration::from_millis(400)).is_ok());
    settle(&mut harness);
    assert!(harness.state().tooltip_visible);
    assert_eq!(harness.state().show_count, 1);
    assert!(harness.publish().is_ok());

    let elsewhere = harness
        .publication()
        .unwrap_or_else(|| unreachable!("mounted surface is published"))
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "focus-elsewhere")
        })
        .unwrap_or_else(|| unreachable!("the second focusable button is mounted"))
        .id()
        .clone();
    harness
        .submit_command(
            elsewhere,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("new focus request is accepted"));
    settle(&mut harness);
    assert!(!harness.state().tooltip_visible);
    assert_eq!(harness.state().hide_count, 1);
}

#[test]
fn pointer_leave_cancels_delayed_tooltip_show_without_late_action() {
    let mut harness = TestHarness::<PresentationApp>::mount(Model::default());
    assert!(harness.publish().is_ok());
    let point = node_center(&harness, "help-button");
    move_pointer(&mut harness, point);
    move_pointer(
        &mut harness,
        LogicalPoint::new(250.0, 250.0).unwrap_or_else(|_| unreachable!("finite outside point")),
    );
    assert!(harness.advance_time(Duration::from_secs(1)).is_ok());
    settle(&mut harness);
    assert_eq!(harness.state().show_count, 0);
    assert!(!harness.state().tooltip_visible);
}

#[test]
fn dialog_default_modal_entry_and_exact_focus_restoration_use_shared_runtime() {
    let mut harness = TestHarness::<PresentationApp>::mount(Model::default());
    assert!(harness.publish().is_ok());
    let trigger = harness
        .publication()
        .unwrap_or_else(|| unreachable!("baseline is published"))
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "help-button")
        })
        .unwrap_or_else(|| unreachable!("owner button is mounted"))
        .id()
        .clone();
    assert!(
        harness
            .submit_command(
                trigger,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .is_ok()
    );
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    let original_focus = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("focused baseline snapshot exists"))
        .focused()
        .cloned()
        .unwrap_or_else(|| unreachable!("original owner must be focused"));

    assert!(harness.submit_action(Action::OpenDialog).is_ok());
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("dialog snapshot exists"));
    let inside = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Button && node.name() == Some("Inside"))
        .unwrap_or_else(|| unreachable!("Dialog has one focusable child"));
    assert_eq!(
        snapshot.focused(),
        Some(inside.id()),
        "default modal Dialog must enter its first eligible child"
    );
    let dialog = harness
        .publication()
        .unwrap_or_else(|| unreachable!("dialog is published"))
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "test-dialog")
        })
        .unwrap_or_else(|| unreachable!("Dialog retains mounted identity"))
        .id()
        .clone();
    let dismissal = SemanticCommand::PresentationDismiss(PresentationDismissReason::CancelOrBack);
    assert!(
        harness
            .submit_command(dialog, dismissal, CommandOrigin::programmatic())
            .is_ok()
    );
    settle(&mut harness);
    assert!(!harness.state().dialog_open);
    assert!(harness.publish().is_ok());
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_eq!(
        harness
            .semantic_snapshot()
            .unwrap_or_else(|_| unreachable!("closed dialog has a snapshot"))
            .focused(),
        Some(&original_focus),
        "the exact previous owner must be restored by the shared runtime"
    );
}

#[test]
fn dialog_and_popover_dismissal_actions_are_application_owned() {
    let mut dialog = TestHarness::<PresentationApp>::mount(Model {
        dialog_open: true,
        ..Model::default()
    });
    assert!(dialog.publish().is_ok());
    settle(&mut dialog);
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
    let semantic = dialog
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("dialog semantic snapshot exists"));
    assert_eq!(
        semantic
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::Dialog)
            .unwrap_or_else(|| unreachable!("Dialog semantic node exists"))
            .state()
            .modal(),
        Some(true)
    );
    assert!(
        semantic.focused().is_some(),
        "modal Dialog enters ordinary focus scope"
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

    let mut popover = TestHarness::<PresentationApp>::mount(Model {
        popover_open: true,
        ..Model::default()
    });
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
fn common_presentation_builder_overrides_constructor_and_preserves_control_invariants() {
    let original = SurfacePresentation::new(SurfacePresentationPlacement::new(
        SurfacePresentationSide::Bottom,
    ))
    .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
    .modal(true)
    .with_focus_policy(PresentationFocusPolicy::EnterAndRestore);
    let override_policy = SurfacePresentation::new(SurfacePresentationPlacement::new(
        SurfacePresentationSide::Right,
    ))
    .with_outside_pointer(PresentationOutsidePointerPolicy::Ignore)
    .with_focus_policy(PresentationFocusPolicy::Preserve);

    let popover = Popover::<Action>::new(vec![button("Inside")], original)
        .surface_presentation(override_policy.clone())
        .into_element();
    let popover_config = popover
        .surface_presentation_config()
        .unwrap_or_else(|| unreachable!("popover is presented"));
    assert_eq!(
        popover_config.candidates()[0].side(),
        SurfacePresentationSide::Right,
        "standard common-node builder must replace constructor policy"
    );
    assert!(!popover_config.is_modal());
    assert_eq!(
        popover_config.outside_pointer(),
        PresentationOutsidePointerPolicy::Ignore
    );

    let dialog = Dialog::<Action>::new("Preferences", vec![button("Inside")])
        .surface_presentation(override_policy)
        .into_element();
    assert!(
        !dialog
            .surface_presentation_config()
            .unwrap_or_else(|| unreachable!("dialog is presented"))
            .is_modal()
    );
    assert!(
        dialog.focus_scope_config().is_none(),
        "nonmodal override must not retain a stale modal focus trap"
    );
}

#[test]
fn tooltip_projection_cannot_become_modal_or_enter_focus() {
    let element = Tooltip::<Action>::new("Safe")
        .surface_presentation(
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

#[test]
fn nonmodal_dialog_preserves_explicit_runtime_policy_without_focus_trap() {
    let element = Dialog::<Action>::new("Nonmodal", vec![button("Action")])
        .surface_presentation(
            SurfacePresentation::new(SurfacePresentationPlacement::new(
                SurfacePresentationSide::Center,
            ))
            .modal(false)
            .with_focus_policy(PresentationFocusPolicy::Preserve),
        )
        .into_element();
    assert!(
        !element
            .surface_presentation_config()
            .unwrap_or_else(|| unreachable!("Dialog is presented"))
            .is_modal()
    );
    assert!(element.focus_scope_config().is_none());
}
