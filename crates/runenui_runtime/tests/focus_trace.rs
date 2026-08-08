#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, ElementId, EventContext, FocusEventKind, NoHostProtocol,
    SemanticCommand, UiApp, UiEvent, View, Widget, WidgetEventOutput, column,
};
use runenui_runtime::{
    AppRuntime, FocusReason, InputModality, MountedNodeId, PumpBudget, RuntimeConfig,
    RuntimeLimits, TraceDeliveryOutcome, TraceFocusRecordRole, TraceRecord, TraceRecordKind,
    TraceRoutedIntegrityFailure, TraceTarget, WorkSequence,
};

struct App;

impl UiApp for App {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(vec![
            Element::new(FocusProbe {
                overflow_on_focus_out: true,
            })
            .id("a")
            .key("a")
            .focusable(true),
            Element::new(FocusProbe {
                overflow_on_focus_out: false,
            })
            .id("b")
            .key("b")
            .focusable(true),
        ])
        .key("root")
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[derive(Debug)]
struct FocusProbe {
    overflow_on_focus_out: bool,
}

impl Widget<()> for FocusProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if self.overflow_on_focus_out
            && event
                .as_focus()
                .is_some_and(|focus| focus.kind() == FocusEventKind::Out)
        {
            context.emit(());
            context.emit(());
        }
        WidgetEventOutput::none()
    }
}

const fn full_budget() -> PumpBudget {
    PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

fn settle(runtime: &mut AppRuntime<App>) {
    assert!(runtime.pump(full_budget()).is_quiescent());
}

fn id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored = ElementId::new(authored).unwrap_or_else(|_| unreachable!("test id is valid"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("named test node is mounted"))
        .id()
        .clone()
}

fn request_focus(
    runtime: &mut AppRuntime<App>,
    target: MountedNodeId,
    origin: CommandOrigin,
) -> WorkSequence {
    let submission = runtime
        .submit_command(target, SemanticCommand::RequestFocus, origin)
        .unwrap_or_else(|_| unreachable!("live focus command is accepted"));
    let sequence = submission.sequence();
    let _ = runtime.pump(full_budget());
    sequence
}

fn record<'a>(
    records: &[&'a TraceRecord],
    sequence: WorkSequence,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| record.work_sequence() == Some(sequence) && predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("required focus trace fact is retained"))
}

fn assert_causal_ancestor(
    records: &[&TraceRecord],
    descendant: &TraceRecord,
    ancestor: &TraceRecord,
) {
    let mut parent = descendant.causal_parent();
    while parent != Some(ancestor.sequence()) {
        let sequence = parent
            .unwrap_or_else(|| unreachable!("descendant retains the expected focus ancestor"));
        parent = records
            .iter()
            .copied()
            .find(|record| record.sequence() == sequence)
            .unwrap_or_else(|| unreachable!("retained causal parent is present"))
            .causal_parent();
    }
}

#[test]
fn modality_trace_owns_exact_previous_and_current_endpoints() {
    let mut runtime = AppRuntime::<App>::mount(());
    settle(&mut runtime);
    let a = id(&mut runtime, "a");

    let programmatic = request_focus(&mut runtime, a.clone(), CommandOrigin::programmatic());
    let records = runtime.trace().records().collect::<Vec<_>>();
    let first = record(&records, programmatic, |kind| {
        matches!(kind, TraceRecordKind::ModalityChanged)
    });
    assert_eq!(
        first.context().focus_record_role(),
        Some(TraceFocusRecordRole::ModalityChange)
    );
    let first_transition = first
        .context()
        .modality_transition()
        .unwrap_or_else(|| unreachable!("modality record owns exact endpoints"));
    assert_eq!(first_transition.previous(), None);
    assert_eq!(first_transition.current(), InputModality::Programmatic);
    assert_eq!(first.context().target_transition(), None);
    assert_eq!(first.context().route(), None);
    assert_eq!(first.context().delivery(), None);

    let automation = request_focus(&mut runtime, a, CommandOrigin::automation());
    let records = runtime.trace().records().collect::<Vec<_>>();
    let second = record(&records, automation, |kind| {
        matches!(kind, TraceRecordKind::ModalityChanged)
    });
    assert_eq!(
        second.context().focus_record_role(),
        Some(TraceFocusRecordRole::ModalityChange)
    );
    let second_transition = second
        .context()
        .modality_transition()
        .unwrap_or_else(|| unreachable!("modality record owns exact endpoints"));
    assert_eq!(
        second_transition.previous(),
        Some(InputModality::Programmatic)
    );
    assert_eq!(second_transition.current(), InputModality::Automation);
}

#[test]
fn overflowing_focus_out_callback_never_records_false_delivery() {
    let limits = RuntimeLimits::default().with_transaction_outputs(1);
    let mut runtime =
        AppRuntime::<App>::mount_with_config((), RuntimeConfig::default().with_limits(limits));
    settle(&mut runtime);
    let a = id(&mut runtime, "a");
    let b = id(&mut runtime, "b");
    let _ = request_focus(&mut runtime, a.clone(), CommandOrigin::programmatic());

    let sequence = request_focus(&mut runtime, b.clone(), CommandOrigin::programmatic());
    let records = runtime.trace().records().collect::<Vec<_>>();

    let transition = record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::FocusTransitionCommitted {
                reason: FocusReason::ProgrammaticRequest,
            }
        )
    });
    assert_eq!(
        transition.context().focus_record_role(),
        Some(TraceFocusRecordRole::Transition)
    );
    let endpoints = transition
        .context()
        .target_transition()
        .unwrap_or_else(|| unreachable!("committed focus transition owns exact endpoints"));
    assert_eq!(
        endpoints.previous().map(TraceTarget::mounted_node_id),
        Some(&a)
    );
    assert_eq!(
        endpoints.current().map(TraceTarget::mounted_node_id),
        Some(&b)
    );

    let failure = record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::RoutedIntegrityFailed {
                failure: TraceRoutedIntegrityFailure::OutputAllowanceExceeded,
            }
        )
    });
    assert_causal_ancestor(&records, failure, transition);
    assert!(!records.iter().any(|record| {
        record.work_sequence() == Some(sequence)
            && matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved { .. }
            )
            && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
    }));
}
