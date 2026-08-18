use core::num::NonZeroUsize;

use runenui_core::{
    CommandOrigin, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, PhysicalKey, PointerButton, PointerButtons, SemanticAction,
    SemanticCommand, SemanticRole,
};
use runenui_runtime::{
    LogicalPoint, MountedNodeId, PointerDeviceKind, PointerId, PointerPhase, PumpBudget,
    SemanticUpdateResult,
};
use runenui_testing::{
    SemanticQuery, SemanticTarget, SettleBudget, SettleOutcome, TestHarness,
};

#[path = "../../../examples/counter/src/app.rs"]
mod app;
#[path = "../../../examples/counter/src/ui.rs"]
mod ui;

use app::{Counter, CounterApp};

#[derive(Clone, Copy, Debug)]
enum ActivationOrigin {
    SemanticAction,
    Pointer,
    Keyboard,
    Automation,
    Programmatic,
}

struct CounterTargets {
    semantic: SemanticTarget,
    mounted: MountedNodeId,
    point: LogicalPoint,
}

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn settle(harness: &mut TestHarness<CounterApp>) {
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
}

fn authored_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("Counter authored ID is valid"))
}

const fn enter_down() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Enter,
        LogicalKey::Enter,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

fn kind_count(kinds: &[&str], expected: &str) -> usize {
    kinds.iter().filter(|kind| **kind == expected).count()
}

fn increment_targets(harness: &TestHarness<CounterApp>) -> CounterTargets {
    let increment_query = SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("+")
        .with_supported_action(SemanticAction::Activate);
    let semantic = harness
        .unique_semantic_target(&increment_query)
        .unwrap_or_else(|error| {
            unreachable!("Counter increment semantic target is unique: {error:?}")
        });

    let Some((mounted, point)) = (|| {
        let publication = harness.publication()?;
        let authored = authored_id("counter.increment");
        let node = publication
            .frame()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored))?;
        let bounds = node.bounds();
        let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0).ok()?;
        assert_eq!(
            publication.frame().hit_test_id(point),
            Some(node.id().clone())
        );
        Some((node.id().clone(), point))
    })() else {
        unreachable!("published Counter increment has exact public frame identity and bounds")
    };

    CounterTargets {
        semantic,
        mounted,
        point,
    }
}

fn submit_pointer_activation(harness: &mut TestHarness<CounterApp>, point: LogicalPoint) {
    let pointer_id =
        PointerId::new(1).unwrap_or_else(|| unreachable!("pointer identity is non-zero"));
    let down = harness
        .pointer_event(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            point,
        )
        .unwrap_or_else(|_| unreachable!("published Counter accepts pointer context"))
        .with_changed_button(PointerButton::Primary)
        .with_buttons(PointerButtons::new([PointerButton::Primary]));
    harness
        .submit_pointer(down)
        .unwrap_or_else(|error| unreachable!("Counter pointer down is accepted: {error:?}"));
    settle(harness);
    assert_eq!(harness.state().count, 0);

    let up = harness
        .pointer_event(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            point,
        )
        .unwrap_or_else(|_| unreachable!("published Counter accepts pointer context"))
        .with_changed_button(PointerButton::Primary);
    harness
        .submit_pointer(up)
        .unwrap_or_else(|error| unreachable!("Counter pointer up is accepted: {error:?}"));
}

fn submit_activation(
    harness: &mut TestHarness<CounterApp>,
    origin: ActivationOrigin,
    targets: CounterTargets,
) {
    match origin {
        ActivationOrigin::SemanticAction => harness
            .submit_semantic_action(&targets.semantic, SemanticAction::Activate)
            .unwrap_or_else(|error| {
                unreachable!("Counter semantic activation is accepted: {error:?}")
            }),
        ActivationOrigin::Pointer => submit_pointer_activation(harness, targets.point),
        ActivationOrigin::Keyboard => {
            harness
                .submit_command(
                    targets.mounted,
                    SemanticCommand::RequestFocus,
                    CommandOrigin::programmatic(),
                )
                .unwrap_or_else(|error| {
                    unreachable!("Counter increment focus request is accepted: {error:?}")
                });
            settle(harness);
            harness
                .submit_keyboard(enter_down())
                .unwrap_or_else(|error| unreachable!("Counter Enter is accepted: {error:?}"));
        }
        ActivationOrigin::Automation => harness
            .submit_automation_command(
                authored_id("counter.increment"),
                SemanticCommand::Activate,
            )
            .unwrap_or_else(|error| {
                unreachable!("Counter automation activation is accepted: {error:?}")
            }),
        ActivationOrigin::Programmatic => harness
            .submit_command(
                targets.mounted,
                SemanticCommand::Activate,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|error| {
                unreachable!("Counter programmatic activation is accepted: {error:?}")
            }),
    };
}

fn assert_trace(harness: &TestHarness<CounterApp>, origin: ActivationOrigin) {
    let replay = harness
        .trace_replay()
        .unwrap_or_else(|error| unreachable!("Counter trace must replay for {origin:?}: {error}"));
    assert!(replay.is_complete());
    let kinds = replay
        .records()
        .map(|record| record.kind().as_str())
        .collect::<Vec<_>>();

    assert!(kind_count(&kinds, "command_submission_accepted") >= 1);
    assert!(kind_count(&kinds, "routed_event_started") >= 1);
    assert!(kind_count(&kinds, "semantic_default_applied") >= 1);
    assert_eq!(kind_count(&kinds, "action_submission_accepted"), 1);
    assert_eq!(kind_count(&kinds, "application_state_updated"), 1);

    match origin {
        ActivationOrigin::SemanticAction => {
            assert_eq!(kind_count(&kinds, "semantic_action_bound"), 1);
        }
        ActivationOrigin::Pointer => {
            assert_eq!(kind_count(&kinds, "pointer_submission_accepted"), 2);
            assert_eq!(kind_count(&kinds, "pointer_activate_collected"), 1);
        }
        ActivationOrigin::Keyboard => {
            assert_eq!(kind_count(&kinds, "keyboard_submission_accepted"), 1);
            assert_eq!(kind_count(&kinds, "keyboard_enter_activation_derived"), 1);
        }
        ActivationOrigin::Automation => {
            assert_eq!(kind_count(&kinds, "automation_resolution_unique"), 1);
        }
        ActivationOrigin::Programmatic => {}
    }
}

fn run_origin(origin: ActivationOrigin) {
    let mut harness = TestHarness::<CounterApp>::mount(Counter::new());
    assert!(harness.publish().is_ok());

    let Ok(snapshot) = harness.semantic_snapshot() else {
        unreachable!("explicit Counter publication produces semantics")
    };
    let baseline_surface = snapshot.surface_id().clone();
    let baseline_revision = snapshot.revision();
    let targets = increment_targets(&harness);

    submit_activation(&mut harness, origin, targets);
    settle(&mut harness);
    assert_eq!(
        harness.state().count,
        1,
        "origin {origin:?} must update Counter once"
    );

    assert!(harness.publish().is_ok());
    assert!(matches!(
        harness.semantic_update_from(&baseline_surface, baseline_revision),
        Ok(SemanticUpdateResult::Delta(_))
    ));
    assert_trace(&harness, origin);
}

#[test]
fn counter_converges_semantic_pointer_keyboard_automation_and_programmatic_activation() {
    for origin in [
        ActivationOrigin::SemanticAction,
        ActivationOrigin::Pointer,
        ActivationOrigin::Keyboard,
        ActivationOrigin::Automation,
        ActivationOrigin::Programmatic,
    ] {
        run_origin(origin);
    }
}
