#![allow(refining_impl_trait)]

use runenui_core::{Effects, Element, NoHostProtocol, UiApp, View, button};
use runenui_runtime::{AppRuntime, PumpBudget, TraceActionCategory, TraceRecordKind};

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
fn public_trace_reconstructs_non_debug_direct_and_effect_actions() {
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
        .unwrap_or_else(|| unreachable!("accepted action owns public redacted identity"));
    assert_eq!(
        direct_identity.type_name(),
        core::any::type_name::<OpaqueAction>()
    );
    assert_eq!(
        direct_identity.category(),
        TraceActionCategory::DirectSubmission
    );
    assert_eq!(direct.causal_parent(), None);
    let direct_sequence = direct.sequence();

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.state().updates, 1);

    let transaction = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == Some(direct_work)
        })
        .unwrap_or_else(|| unreachable!("direct action starts one application transaction"));
    assert_eq!(transaction.causal_parent(), Some(direct_sequence));

    let effect = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.causal_parent() == Some(transaction.sequence())
                && record.work_sequence() != Some(direct_work)
        })
        .unwrap_or_else(|| unreachable!("update effect action is admitted by the transaction"));
    let effect_identity = effect
        .context()
        .action()
        .unwrap_or_else(|| unreachable!("effect action owns public redacted identity"));
    assert_eq!(
        effect_identity.type_name(),
        core::any::type_name::<OpaqueAction>()
    );
    assert_eq!(
        effect_identity.category(),
        TraceActionCategory::ApplicationEffect
    );
    let effect_work = effect
        .work_sequence()
        .unwrap_or_else(|| unreachable!("accepted effect action owns one work sequence"));
    let effect_sequence = effect.sequence();

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.state().updates, 2);
    let effect_transaction = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == Some(effect_work)
        })
        .unwrap_or_else(|| unreachable!("effect action starts one application transaction"));
    assert_eq!(effect_transaction.causal_parent(), Some(effect_sequence));
}
