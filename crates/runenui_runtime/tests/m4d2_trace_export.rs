#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;

use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, EventContext, NoHostProtocol, SemanticCommand, UiApp,
    UiEvent, View, Widget, WidgetEventOutput, WidgetTextInput,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, TraceConfig, TracePayloadCapture, TraceRecord,
    TraceRecordKind, TraceSinkDeliveryOutcome, TraceSinkReceiveError,
};

#[derive(Clone, Copy)]
enum TestAction {
    Increment,
}

#[derive(Debug)]
struct TextTarget;

impl Widget<TestAction> for TextTarget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        _: &UiEvent,
        _: &mut EventContext<'_, TestAction>,
    ) -> WidgetEventOutput {
        WidgetEventOutput::none()
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
}

struct TestApp;

impl UiApp for TestApp {
    type State = usize;
    type Action = TestAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        Element::new(TextTarget)
            .id("target")
            .key("target")
            .focusable(true)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            TestAction::Increment => *state += 1,
        }
    }
}

fn settle(runtime: &mut AppRuntime<TestApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn mounted(config: TraceConfig) -> AppRuntime<TestApp> {
    let mut runtime = AppRuntime::<TestApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_trace_config(config),
    );
    settle(&mut runtime);
    runtime
}

fn focus(runtime: &mut AppRuntime<TestApp>) {
    let authored = runenui_core::ElementId::new("target")
        .unwrap_or_else(|_| unreachable!("fixture id is valid"));
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("fixture target is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("fixture target accepts focus"));
    settle(runtime);
}

fn record_for_work(runtime: &AppRuntime<TestApp>, work: runenui_runtime::WorkSequence) -> &TraceRecord {
    runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.work_sequence() == Some(work)
        })
        .unwrap_or_else(|| unreachable!("accepted action owns one canonical trace record"))
}

fn canonical_signature(
    runtime: &AppRuntime<TestApp>,
) -> Vec<(u64, TraceRecordKind, Option<u64>, Option<u64>)> {
    runtime
        .trace()
        .records()
        .map(|record| {
            (
                record.sequence().get(),
                record.kind().clone(),
                record.work_sequence().map(runenui_runtime::WorkSequence::get),
                record.causal_parent().map(runenui_runtime::TraceSequence::get),
            )
        })
        .collect()
}

fn deterministic_execution() -> String {
    let mut runtime = mounted(TraceConfig::new(128));
    runtime
        .submit_action(TestAction::Increment)
        .unwrap_or_else(|_| unreachable!("action is admitted"));
    settle(&mut runtime);
    runtime.trace().export_jsonl()
}

#[test]
fn trace_export_01_jsonl_v1_is_versioned_and_byte_stable() {
    let first = deterministic_execution();
    let second = deterministic_execution();

    assert_eq!(first, second);
    let mut lines = first.lines();
    let header = lines
        .next()
        .unwrap_or_else(|| unreachable!("export owns one header line"));
    assert!(header.starts_with(
        "{\"schema\":\"runenui.trace\",\"version\":1,\"dropped_before_sequence\":null,\"retained_records\":"
    ));
    let records: Vec<_> = lines.collect();
    assert!(!records.is_empty());
    assert!(records.iter().all(|line| line.starts_with(
        "{\"schema\":\"runenui.trace.record\",\"version\":1,\"sequence\":"
    )));
}

#[test]
fn trace_export_02_text_is_redacted_by_default_and_full_only_by_explicit_policy() {
    const SECRET: &str = "text-secret";

    let mut redacted = mounted(TraceConfig::new(128));
    focus(&mut redacted);
    redacted
        .submit_text(
            CommittedTextEvent::new(SECRET, None)
                .unwrap_or_else(|_| unreachable!("fixture text is valid")),
        )
        .unwrap_or_else(|_| unreachable!("text-capable focus accepts text"));
    let accepted = redacted
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::CommittedTextSubmissionAccepted))
        .unwrap_or_else(|| unreachable!("committed text acceptance is traced"));
    let input = accepted
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("committed text owns input context"));
    assert_eq!(input.captured_text(), None);
    assert!(!redacted.trace().export_jsonl().contains(SECRET));

    let config = TraceConfig::new(128).with_payload_capture(TracePayloadCapture::FullText);
    let mut full = mounted(config);
    focus(&mut full);
    full.submit_text(
        CommittedTextEvent::new(SECRET, None)
            .unwrap_or_else(|_| unreachable!("fixture text is valid")),
    )
    .unwrap_or_else(|_| unreachable!("text-capable focus accepts text"));
    let accepted = full
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::CommittedTextSubmissionAccepted))
        .unwrap_or_else(|| unreachable!("committed text acceptance is traced"));
    assert_eq!(
        accepted.context().input().and_then(|input| input.captured_text()),
        Some(SECRET)
    );
    assert!(full.trace().export_jsonl().contains(SECRET));
}

#[test]
fn trace_export_03_preedit_is_redacted_by_default_and_preserves_checked_range() {
    const PREEDIT: &str = "preedit-secret";
    let range = runenui_core::CompositionRange::new(PREEDIT, 0, 7)
        .unwrap_or_else(|_| unreachable!("fixture range is valid"));

    let mut redacted = mounted(TraceConfig::new(128));
    focus(&mut redacted);
    let start = redacted
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition-capable focus accepts start"));
    redacted
        .submit_composition_update(
            start.generation().clone(),
            PREEDIT.to_owned(),
            Some(range),
        )
        .unwrap_or_else(|_| unreachable!("live composition accepts update"));
    let update = redacted
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::CompositionUpdateSubmitted))
        .unwrap_or_else(|| unreachable!("composition update is traced"));
    let input = update
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("composition update owns input context"));
    assert_eq!(input.captured_text(), None);
    let captured_range = input
        .composition_range()
        .unwrap_or_else(|| unreachable!("checked range is retained"));
    assert_eq!((captured_range.byte_start(), captured_range.byte_end()), (0, 7));
    assert!(!redacted.trace().export_jsonl().contains(PREEDIT));

    let config = TraceConfig::new(128).with_payload_capture(TracePayloadCapture::FullText);
    let mut full = mounted(config);
    focus(&mut full);
    let start = full
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition-capable focus accepts start"));
    full.submit_composition_update(
        start.generation().clone(),
        PREEDIT.to_owned(),
        Some(range),
    )
    .unwrap_or_else(|_| unreachable!("live composition accepts update"));
    let update = full
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::CompositionUpdateSubmitted))
        .unwrap_or_else(|| unreachable!("composition update is traced"));
    assert_eq!(
        update.context().input().and_then(|input| input.captured_text()),
        Some(PREEDIT)
    );
    assert!(full.trace().export_jsonl().contains(PREEDIT));
}

#[test]
fn trace_export_05_08_full_sink_loses_only_external_copy_and_adds_no_sequence() {
    let one = NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("one is non-zero"));
    let mut runtime = mounted(TraceConfig::new(128).with_sink_capacity(one));
    let receiver = runtime
        .take_trace_sink_receiver()
        .unwrap_or_else(|| unreachable!("configured sink exposes one receiver"));

    let _mount_line = receiver
        .try_recv()
        .unwrap_or_else(|_| unreachable!("initial delivered record is buffered"));
    let first_work = runtime
        .submit_action(TestAction::Increment)
        .unwrap_or_else(|_| unreachable!("first action is admitted"));
    let second_work = runtime
        .submit_action(TestAction::Increment)
        .unwrap_or_else(|_| unreachable!("second action is admitted"));

    let first = record_for_work(&runtime, first_work);
    let second = record_for_work(&runtime, second_work);
    assert_eq!(first.sink_delivery(), Some(TraceSinkDeliveryOutcome::Delivered));
    assert_eq!(second.sink_delivery(), Some(TraceSinkDeliveryOutcome::Full));
    assert_eq!(second.sequence().get(), first.sequence().get() + 1);

    let delivered = receiver
        .try_recv()
        .unwrap_or_else(|_| unreachable!("first action copy is buffered"));
    assert!(delivered.as_str().contains("action_submission_accepted"));
    assert_eq!(receiver.try_recv(), Err(TraceSinkReceiveError::Empty));
}

#[test]
fn trace_export_06_closed_sink_is_diagnosed_once_then_retired_before_shutdown() {
    let capacity = NonZeroUsize::new(8).unwrap_or_else(|| unreachable!("eight is non-zero"));
    let mut runtime = mounted(TraceConfig::new(128).with_sink_capacity(capacity));
    let receiver = runtime
        .take_trace_sink_receiver()
        .unwrap_or_else(|| unreachable!("configured sink exposes one receiver"));
    drop(receiver);

    let first_work = runtime
        .submit_action(TestAction::Increment)
        .unwrap_or_else(|_| unreachable!("action remains admitted after sink close"));
    let second_work = runtime
        .submit_action(TestAction::Increment)
        .unwrap_or_else(|_| unreachable!("later action remains admitted"));
    assert_eq!(
        record_for_work(&runtime, first_work).sink_delivery(),
        Some(TraceSinkDeliveryOutcome::Closed)
    );
    assert_eq!(record_for_work(&runtime, second_work).sink_delivery(), None);

    let before_shutdown = runtime.trace().len();
    let _ = runtime.shutdown();
    assert!(runtime.trace().len() > before_shutdown);
    let shutdown = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::RuntimeShutdown { .. }))
        .unwrap_or_else(|| unreachable!("shutdown remains canonical"));
    assert_eq!(shutdown.sink_delivery(), None);
}

#[test]
fn trace_export_07_09_10_sink_backpressure_cannot_change_runtime_or_canonical_order() {
    fn execute(config: TraceConfig) -> (usize, Vec<(u64, TraceRecordKind, Option<u64>, Option<u64>)>) {
        let mut runtime = mounted(config);
        runtime
            .submit_action(TestAction::Increment)
            .unwrap_or_else(|_| unreachable!("first action is admitted"));
        runtime
            .submit_action(TestAction::Increment)
            .unwrap_or_else(|_| unreachable!("second action is admitted"));
        settle(&mut runtime);
        (*runtime.state(), canonical_signature(&runtime))
    }

    let without_sink = execute(TraceConfig::new(128));
    let one = NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("one is non-zero"));
    let with_full_sink = execute(TraceConfig::new(128).with_sink_capacity(one));

    assert_eq!(without_sink.0, 2);
    assert_eq!(with_full_sink.0, 2);
    assert_eq!(without_sink.1, with_full_sink.1);
}

#[test]
fn trace_export_10_huge_logical_sink_capacity_does_not_eagerly_allocate() {
    let huge = NonZeroUsize::new(usize::MAX)
        .unwrap_or_else(|| unreachable!("usize max is non-zero"));
    let mut runtime = mounted(TraceConfig::new(128).with_sink_capacity(huge));
    assert!(runtime.take_trace_sink_receiver().is_some());
    assert!(!runtime.trace().is_empty());
}

#[test]
fn capacity_zero_disables_payload_and_sink_diagnostics_without_changing_actions() {
    let huge = NonZeroUsize::new(usize::MAX)
        .unwrap_or_else(|| unreachable!("usize max is non-zero"));
    let config = TraceConfig::new(0)
        .with_payload_capture(TracePayloadCapture::FullText)
        .with_sink_capacity(huge);
    let mut runtime = mounted(config);

    assert!(runtime.trace().is_empty());
    assert!(runtime.take_trace_sink_receiver().is_none());
    runtime
        .submit_action(TestAction::Increment)
        .unwrap_or_else(|_| unreachable!("disabled trace does not change action admission"));
    settle(&mut runtime);
    assert_eq!(runtime.state(), &1);
    assert!(runtime.trace().is_empty());
    assert_eq!(
        runtime.trace().export_jsonl(),
        "{\"schema\":\"runenui.trace\",\"version\":1,\"dropped_before_sequence\":null,\"retained_records\":0}\n"
    );
}

#[test]
fn open_sink_delivers_shutdown_then_closes_after_buffer_drains() {
    let capacity = NonZeroUsize::new(32).unwrap_or_else(|| unreachable!("capacity is non-zero"));
    let mut runtime = mounted(TraceConfig::new(128).with_sink_capacity(capacity));
    let receiver = runtime
        .take_trace_sink_receiver()
        .unwrap_or_else(|| unreachable!("configured sink exposes one receiver"));
    while receiver.try_recv().is_ok() {}

    let _ = runtime.shutdown();
    let mut saw_shutdown = false;
    loop {
        match receiver.try_recv() {
            Ok(line) => saw_shutdown |= line.as_str().contains("runtime_shutdown"),
            Err(TraceSinkReceiveError::Empty) => continue,
            Err(TraceSinkReceiveError::Closed) => break,
        }
    }
    assert!(saw_shutdown);
}
