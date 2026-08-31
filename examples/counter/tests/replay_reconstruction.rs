#![allow(refining_impl_trait)]

#[path = "../src/app.rs"]
mod app;
#[path = "../src/ui.rs"]
mod ui;

use app::{Counter, CounterApp};
use runenui_core::{
    ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
    LogicalKey, LogicalLength, PhysicalKey, SemanticCommand, StyleEnvironment,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceReplay, TraceReplayRecord,
    TraceReplaySequence,
};

const SURFACE_SIZE: LogicalSize = LogicalSize::new(
    match LogicalLength::new(240.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
    match LogicalLength::new(160.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
);

fn settle(runtime: &mut AppRuntime<CounterApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "counter did not settle: {report:?}");
}

fn authored_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("counter authored id is valid"))
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

fn kind<'a>(replay: &'a TraceReplay, name: &str) -> Option<&'a TraceReplayRecord> {
    replay
        .records()
        .find(|record| record.kind().as_str() == name)
}

fn descends_from(
    replay: &TraceReplay,
    mut child: TraceReplaySequence,
    ancestor: TraceReplaySequence,
) -> bool {
    loop {
        if child == ancestor {
            return true;
        }
        let Some(record) = replay.record(child) else {
            return false;
        };
        let Some(parent) = record.causal_parent() else {
            return false;
        };
        child = parent;
    }
}

fn reconstructs_counter_activation(replay: &TraceReplay) -> bool {
    let Some(automation) = kind(replay, "automation_resolution_unique") else {
        return false;
    };
    let Some(focus_command) = replay.records().find(|record| {
        record.kind().as_str() == "command_submission_accepted"
            && record.causal_parent() == Some(automation.sequence())
    }) else {
        return false;
    };
    let Some(keyboard) = replay.records().find(|record| {
        record.kind().as_str() == "keyboard_submission_accepted"
            && record.sequence() > focus_command.sequence()
    }) else {
        return false;
    };
    let Some(derived) = replay.records().find(|record| {
        record.kind().as_str() == "keyboard_enter_activation_derived"
            && descends_from(replay, record.sequence(), keyboard.sequence())
    }) else {
        return false;
    };
    let Some(command) = replay.records().find(|record| {
        record.kind().as_str() == "command_submission_accepted"
            && record.causal_parent() == Some(derived.sequence())
    }) else {
        return false;
    };
    let Some(route_started) = replay.records().find(|record| {
        record.kind().as_str() == "routed_event_started"
            && descends_from(replay, record.sequence(), command.sequence())
    }) else {
        return false;
    };
    let Some(default_applied) = replay.records().find(|record| {
        record.kind().as_str() == "semantic_default_applied"
            && descends_from(replay, record.sequence(), route_started.sequence())
    }) else {
        return false;
    };
    let Some(routed_action) = replay.records().find(|record| {
        record.kind().as_str() == "routed_action_collected"
            && descends_from(replay, record.sequence(), command.sequence())
    }) else {
        return false;
    };
    if !descends_from(replay, routed_action.sequence(), default_applied.sequence()) {
        return false;
    }
    let Some(action) = replay.records().find(|record| {
        record.kind().as_str() == "action_submission_accepted"
            && record.causal_parent() == Some(routed_action.sequence())
    }) else {
        return false;
    };
    let Some(action_work) = action.work_sequence() else {
        return false;
    };
    let Some(transaction) = replay.records().find(|record| {
        record.kind().as_str() == "application_action_transaction_started"
            && record.work_sequence() == Some(action_work)
            && record.causal_parent() == Some(action.sequence())
    }) else {
        return false;
    };
    let Some(updated) = replay.records().find(|record| {
        record.kind().as_str() == "application_state_updated"
            && record.work_sequence() == Some(action_work)
            && record.causal_parent() == Some(action.sequence())
    }) else {
        return false;
    };
    let Some(reconciled) = replay.records().find(|record| {
        record.kind().as_str() == "tree_reconciled"
            && record.work_sequence() == Some(action_work)
            && record.causal_parent() == Some(action.sequence())
    }) else {
        return false;
    };
    if updated.sequence() <= transaction.sequence() || reconciled.sequence() <= updated.sequence() {
        return false;
    }
    let Some(redraw) = replay.records().find(|record| {
        record.kind().as_str() == "redraw_requested"
            && record.causal_parent() == Some(reconciled.sequence())
    }) else {
        return false;
    };
    replay.records().any(|record| {
        record.kind().as_str() == "surface_published"
            && record.causal_parent() == Some(redraw.sequence())
    })
}

fn counter_jsonl() -> String {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    settle(&mut runtime);

    runtime
        .submit_automation_command(
            authored_id("counter.increment"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("automation focuses the increment control"));
    settle(&mut runtime);

    runtime
        .submit_keyboard(enter_down())
        .unwrap_or_else(|_| unreachable!("focused increment accepts raw Enter"));
    settle(&mut runtime);
    assert_eq!(runtime.state().count, 1);

    let style_environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(&style_environment, SURFACE_SIZE);
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("counter replay publication is admitted"));
    assert!(!publication.frame().nodes().is_empty());

    runtime.trace().export_jsonl()
}

#[test]
fn replay_02_counter_reconstructs_from_owned_jsonl_after_live_runtime_is_gone() {
    let jsonl = counter_jsonl();
    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| unreachable!("Counter export must replay: {error}"));
    assert!(replay.is_complete());
    assert!(reconstructs_counter_activation(&replay));
}

#[test]
fn replay_02_structurally_valid_divergent_projection_fails_counter_reconstruction() {
    let jsonl = counter_jsonl();
    let divergent = jsonl.replacen(
        "\"name\":\"application_state_updated\"",
        "\"name\":\"application_state_diverged\"",
        1,
    );
    assert_ne!(
        divergent, jsonl,
        "fixture must contain the state-update fact"
    );

    let replay = TraceReplay::parse_jsonl(&divergent).unwrap_or_else(|error| {
        unreachable!("unknown v1 kind remains structurally valid: {error}")
    });
    assert!(replay.is_complete());
    assert!(!reconstructs_counter_activation(&replay));
}
