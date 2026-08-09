#![allow(refining_impl_trait)]

use core::sync::atomic::{AtomicUsize, Ordering};

use runenui_core::{Effects, Element, NoHostProtocol, UiApp, View, button};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, TraceActionCategory, TraceConfig, TraceRecordKind,
};

static LABEL_CALLS: AtomicUsize = AtomicUsize::new(0);

struct OpaqueAction {
    emit_follow_up: bool,
}

struct TraceState {
    updates: usize,
}

struct TraceApp;

impl UiApp for TraceApp {
    type State = TraceState;
    type Action = OpaqueAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        button("Go")
            .on_activate(|| OpaqueAction {
                emit_follow_up: false,
            })
            .into_element()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> Effects<Self::Action, Self::HostProtocol> {
        state.updates += 1;
        if action.emit_follow_up {
            Effects::action(OpaqueAction {
                emit_follow_up: false,
            })
        } else {
            Effects::none()
        }
    }

    fn trace_action_label(action: &Self::Action) -> Option<&'static str> {
        LABEL_CALLS.fetch_add(1, Ordering::Relaxed);
        Some(if action.emit_follow_up {
            "emits-follow-up"
        } else {
            "leaf"
        })
    }
}

fn settle(runtime: &mut AppRuntime<TraceApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

#[test]
fn trace_export_04_public_non_debug_labels_are_optional_and_dormant_when_trace_is_disabled() {
    LABEL_CALLS.store(0, Ordering::Relaxed);
    let mut runtime = AppRuntime::<TraceApp>::mount(TraceState { updates: 0 });
    settle(&mut runtime);

    let direct_work = runtime
        .submit_action(OpaqueAction {
            emit_follow_up: true,
        })
        .unwrap_or_else(|_| unreachable!("direct action is admitted"));
    let direct = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.work_sequence() == Some(direct_work)
        })
        .unwrap_or_else(|| unreachable!("direct action acceptance is traced"));
    let direct_identity = direct
        .context()
        .action()
        .unwrap_or_else(|| unreachable!("direct action owns public trace identity"));
    assert_eq!(
        direct_identity.category(),
        TraceActionCategory::DirectSubmission
    );
    assert_eq!(direct_identity.label(), Some("emits-follow-up"));

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    let effect = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.work_sequence() != Some(direct_work)
                && record.context().action().is_some_and(|identity| {
                    identity.category() == TraceActionCategory::ApplicationEffect
                })
        })
        .unwrap_or_else(|| unreachable!("effect action acceptance is traced"));
    assert_eq!(
        effect
            .context()
            .action()
            .and_then(|identity| identity.label()),
        Some("leaf")
    );
    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("\"label\":\"emits-follow-up\""));
    assert!(jsonl.contains("\"label\":\"leaf\""));
    assert_eq!(LABEL_CALLS.load(Ordering::Relaxed), 2);

    let before_disabled = LABEL_CALLS.load(Ordering::Relaxed);
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(0));
    let mut disabled = AppRuntime::<TraceApp>::mount_with_config(TraceState { updates: 0 }, config);
    disabled
        .submit_action(OpaqueAction {
            emit_follow_up: false,
        })
        .unwrap_or_else(|_| unreachable!("disabled trace does not change action admission"));
    settle(&mut disabled);
    assert_eq!(disabled.state().updates, 1);
    assert!(disabled.trace().is_empty());
    assert_eq!(LABEL_CALLS.load(Ordering::Relaxed), before_disabled);
}
