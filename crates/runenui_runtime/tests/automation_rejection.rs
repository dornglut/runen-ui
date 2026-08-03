#![allow(refining_impl_trait)]

use runenui_core::{ElementId, NoHostProtocol, SemanticCommand, UiApp, View, button};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeStatus, SubmitAutomationErrorKind, SubmitCommandErrorKind,
    TraceRecordKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Activated,
}

struct App;

impl UiApp for App {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        button("target")
            .id("target")
            .key("target")
            .on_activate(|| Action::Activated)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Activated => *state += 1,
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn authored_id() -> ElementId {
    ElementId::new("target").unwrap_or_else(|_| unreachable!("valid authored ID"))
}

#[test]
fn trace_exhaustion_rejects_automation_without_terminalizing_or_consuming_authority() {
    let mut runtime = AppRuntime::<App>::mount(0);
    settle(&mut runtime);
    let trace_len = runtime.trace().len();
    runtime.__seed_next_trace_sequence_for_test(0);

    let authored = authored_id();
    let Err(error) = runtime.submit_automation_command(authored.clone(), SemanticCommand::Activate)
    else {
        unreachable!("trace exhaustion rejects automation ingress");
    };
    assert!(matches!(
        error.kind(),
        SubmitAutomationErrorKind::Command(SubmitCommandErrorKind::TraceSequenceExhausted)
    ));
    assert_eq!(error.into_request(), (authored, SemanticCommand::Activate));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state(), &0);
    assert_eq!(runtime.trace().len(), trace_len);
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );

    runtime.__seed_next_trace_sequence_for_test(100);
    runtime
        .submit_automation_command(authored_id(), SemanticCommand::Activate)
        .unwrap_or_else(|_| unreachable!("runtime remains usable after rejection"));
    settle(&mut runtime);
    assert_eq!(runtime.state(), &1);
}

#[test]
fn work_sequence_exhaustion_rejects_after_resolution_without_terminalizing() {
    let mut runtime = AppRuntime::<App>::mount(0);
    settle(&mut runtime);
    runtime.__seed_next_work_sequence_for_test(0);

    let authored = authored_id();
    let Err(error) = runtime.submit_automation_command(authored.clone(), SemanticCommand::Activate)
    else {
        unreachable!("work-sequence exhaustion rejects automation ingress");
    };
    assert!(matches!(
        error.kind(),
        SubmitAutomationErrorKind::Command(SubmitCommandErrorKind::WorkSequenceExhausted)
    ));
    assert_eq!(error.into_request(), (authored, SemanticCommand::Activate));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state(), &0);
    assert!(
        runtime
            .trace()
            .records()
            .any(|record| matches!(record.kind(), TraceRecordKind::AutomationResolutionUnique))
    );
    assert!(
        !runtime
            .trace()
            .records()
            .any(|record| matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted))
    );
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );

    runtime.__seed_next_work_sequence_for_test(100);
    runtime
        .submit_automation_command(authored_id(), SemanticCommand::Activate)
        .unwrap_or_else(|_| unreachable!("runtime remains usable after rejection"));
    settle(&mut runtime);
    assert_eq!(runtime.state(), &1);
}
