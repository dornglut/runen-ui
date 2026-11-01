#![allow(refining_impl_trait, clippy::panic)]

use std::{task::Poll, time::Instant};

use runenui_core::{
    CommandOrigin, EditIntent, EditResolution, Effects, Element, NoHostProtocol, ProducerBudget,
    SemanticCommand, SemanticEditableMode, SubscriptionSet, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, UiApp, UpdateOutput,
    View, WorkKey, WorkResidencyBudget, WorkStartBudget, column, text_field,
};
use runenui_runtime::{AppRuntime, PumpBudget, RuntimeConfig, RuntimeStatus, TraceConfig};

struct State {
    fields: usize,
    subscriptions: usize,
    text: String,
    selection: usize,
    revision: u64,
    noops: usize,
}

enum Action {
    Edit(Box<EditIntent>),
    Noop,
}

struct Form;

const fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(
        TextDocumentId::new(475),
        TextDocumentRevision::new(revision),
    )
}

impl UiApp for Form {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(state: &State) -> ProducerBudget {
        // Column and fields each queue a declaration reconciliation. Each
        // application subscription has a start and retains its exact root.
        ProducerBudget::new(state.fields + 1 + state.subscriptions, 0)
            .with_reconciliation(state.fields + 1, 0, state.fields + 1)
            .with_subscriptions(state.subscriptions, 0)
            .with_work_starts(WorkStartBudget::new(state.subscriptions, 0, 0, 0, 0))
            .with_work_residency(WorkResidencyBudget::new(0, 0, 0, state.subscriptions, 0, 0))
    }

    fn action_producer_budget(state: &State, _: &Action) -> ProducerBudget {
        // Field count and subscription keys never change; edits have no
        // generated actions, starts, or mounted subscription invalidations.
        ProducerBudget::new(1, 0)
            .with_reconciliation(0, state.fields + 1, 0)
            .with_subscriptions(state.subscriptions, 0)
    }

    fn interaction_producer_budget(state: &State) -> ProducerBudget {
        // These plain fields configure no callback producers. One runtime
        // edit/activation/scroll output and two service starts suffice. Two
        // old service outcomes can coexist with two replacement outcome pairs:
        // One possible default, 4 + 4 outcome credits, and cleanup retry access
        // coexist until actual retirement: ten credits, independent of fields.
        ProducerBudget::new(10, 3)
            .with_application_outputs(0)
            .with_reconciliation(0, state.fields + 1, 0)
            .with_subscriptions(state.subscriptions, 0)
            .with_work_starts(WorkStartBudget::new(0, 0, 0, 0, 2))
            .with_work_residency(WorkResidencyBudget::new(0, 0, 0, 0, 0, 2))
    }

    fn root(state: &State) -> Element<Action> {
        let snapshot = snapshot(state.revision);
        let affinity = if state.selection == state.text.len() && !state.text.is_empty() {
            TextAffinity::Upstream
        } else {
            TextAffinity::Downstream
        };
        let position = TextPosition::new(snapshot, &state.text, state.selection, affinity)
            .unwrap_or_else(|_| unreachable!());
        column(
            (0..state.fields)
                .map(|index| {
                    text_field(
                        snapshot,
                        state.text.clone(),
                        TextSelection::collapsed(position),
                        SemanticEditableMode::SingleLine,
                        |intent| Action::Edit(Box::new(intent)),
                    )
                    .unwrap_or_else(|_| unreachable!())
                    .key(format!("field.{index}"))
                })
                .collect::<Vec<_>>(),
        )
        .into_element()
    }

    fn subscriptions(state: &State, subscriptions: &mut SubscriptionSet<Action>) {
        for index in 0..state.subscriptions {
            let _ = subscriptions.try_local(
                WorkKey::new(format!("source.{index}")).unwrap_or_else(|_| unreachable!()),
                0,
                |_: &mut core::task::Context<'_>| Poll::Pending,
            );
        }
    }

    fn update(state: &mut State, action: Action) -> UpdateOutput<Action, NoHostProtocol> {
        match action {
            Action::Noop => {
                state.noops += 1;
                UpdateOutput::effects(Effects::none())
            }
            Action::Edit(intent) => {
                let span = intent.replacement();
                state
                    .text
                    .replace_range(span.start()..span.end(), intent.replacement_text());
                state.revision += 1;
                state.selection = intent.proposed_selection().active();
                UpdateOutput::edit(EditResolution::accepted(
                    intent.request().clone(),
                    snapshot(state.revision),
                ))
            }
        }
    }
}

const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);

// Preserve the old generic two-output callback boundary as an independent
// negative control. Plain TextFields no longer declare these optional outputs.
#[derive(Debug)]
struct WideCallback;
impl runenui_core::Widget<Action> for WideCallback {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        (): &(),
    ) -> runenui_core::WidgetProducerBudget {
        runenui_core::WidgetProducerBudget::new(
            usize::from(phase == runenui_core::WidgetProducerPhase::Event) * 2,
        )
    }
}
struct WideForm;
impl UiApp for WideForm {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(state: &State) -> ProducerBudget {
        Form::initial_producer_budget(state)
    }
    fn action_producer_budget(state: &State, action: &Action) -> ProducerBudget {
        Form::action_producer_budget(state, action)
    }
    fn interaction_producer_budget(state: &State) -> ProducerBudget {
        // Keep the independent generic callback bound for the negative control.
        ProducerBudget::new(8 * state.fields + 8, 8 * state.fields + 4)
            .with_application_outputs(0)
            .with_reconciliation(0, state.fields + 1, 0)
            .with_work_starts(WorkStartBudget::new(0, 0, 0, 0, 2))
            .with_work_residency(WorkResidencyBudget::new(0, 0, 0, 0, 0, 2))
    }
    fn root(state: &State) -> Element<Action> {
        column(
            (0..state.fields)
                .map(|_| Element::new(WideCallback).focusable(true))
                .collect::<Vec<_>>(),
        )
        .into_element()
    }
    fn update(state: &mut State, action: Action) -> UpdateOutput<Action, NoHostProtocol> {
        Form::update(state, action)
    }
}

#[test]
fn measure_128_field_transaction_limit_boundary() {
    for output_limit in [1024, 1025, 1026, 1028, 2048] {
        let mut runtime = AppRuntime::<WideForm>::mount_with_config(
            State {
                fields: 128,
                subscriptions: 0,
                text: String::new(),
                selection: 0,
                revision: 0,
                noops: 0,
            },
            RuntimeConfig::default().with_limits(
                runenui_runtime::RuntimeLimits::default()
                    .with_waiting_envelopes(4096)
                    .with_transaction_outputs(output_limit),
            ),
        )
        .unwrap_or_else(|error| panic!("diagnostic mount: {error}"));
        runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
        let owner = runtime.index().nodes()[1].id().clone();
        let start = Instant::now();
        let admitted = runtime.submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        );
        let admission_us = start.elapsed().as_micros();
        if output_limit < 1026 {
            let Err(error) = admitted else {
                unreachable!("whole-tree callback bound exceeds transaction limit");
            };
            assert_eq!(
                error.kind(),
                runenui_runtime::SubmitCommandErrorKind::InvalidProducerProfile
            );
            assert!(runtime.focus().focused_node().is_none());
            eprintln!(
                "U475_128_BOUNDARY R={output_limit} C=4096 admission_us={admission_us} refused={:?}",
                error.kind()
            );
        } else {
            admitted.unwrap_or_else(|error| panic!("diagnostic focus: {error:?}"));
            runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
            assert_eq!(runtime.focus().focused_node(), Some(&owner));
            eprintln!(
                "U475_128_WIDE_CALLBACK_BOUNDARY R={output_limit} C=4096 admission_us={admission_us} focus=committed"
            );
        }
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        runtime.shutdown().unwrap_or_else(|_| unreachable!());
    }
}

fn measure_ime_cycles(runtime: &mut AppRuntime<Form>) -> Vec<u128> {
    let mut latencies = Vec::new();
    for revision in 1..=32 {
        let start = Instant::now();
        let original = runtime.state().text.clone();
        let composition = runtime
            .start_composition(None)
            .unwrap_or_else(|error| panic!("IME start: {error:?}"));
        runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
        runtime
            .submit_composition_update(composition.generation().clone(), "é".into(), None)
            .unwrap_or_else(|_| unreachable!());
        runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
        runtime
            .cancel_composition(composition.generation().clone())
            .unwrap_or_else(|_| unreachable!());
        assert!(
            runtime
                .pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert_eq!(
            runtime.state().text,
            original,
            "preedit never mutates the application document"
        );
        runtime
            .submit_text(
                runenui_core::CommittedTextEvent::new("x", None).unwrap_or_else(|_| unreachable!()),
            )
            .unwrap_or_else(|_| unreachable!());
        assert!(
            runtime
                .pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(runtime.state().revision, revision);
        assert_eq!(runtime.state().text, format!("{original}x"));
        latencies.push(start.elapsed().as_micros());
    }
    latencies.sort_unstable();
    latencies
}

#[test]
fn measure_mounted_form_subscription_and_ime_capacity_without_selecting_production_caps() {
    for (fields, subscriptions, capacity) in
        [(1, 0, 32), (16, 16, 512), (64, 16, 4096), (128, 0, 4096)]
    {
        for trace_capacity in [0, 4096] {
            let started = Instant::now();
            let mut runtime = AppRuntime::<Form>::mount_with_config(
                State {
                    fields,
                    subscriptions,
                    text: String::new(),
                    selection: 0,
                    revision: 0,
                    noops: 0,
                },
                RuntimeConfig::default()
                    .with_queue_capacity(capacity)
                    .with_trace_config(TraceConfig::new(trace_capacity)),
            )
            .unwrap_or_else(|error| panic!("source-derived initial frontier fits: {error}"));
            assert!(
                runtime
                    .pump(FULL)
                    .unwrap_or_else(|_| unreachable!())
                    .report()
                    .is_quiescent()
            );
            let mount_us = started.elapsed().as_micros();
            assert_eq!(runtime.index().nodes().len(), fields + 1);
            let owner = runtime.index().nodes()[1].id().clone();
            let focused = runtime.submit_command(
                owner,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            );
            focused.unwrap_or_else(|error| panic!("declared interaction frontier fits: {error:?}"));
            assert!(
                runtime
                    .pump(FULL)
                    .unwrap_or_else(|_| unreachable!())
                    .report()
                    .is_quiescent()
            );
            let latencies = measure_ime_cycles(&mut runtime);
            let mut accepted = 0;
            let refused = loop {
                match runtime.submit_action(Action::Noop) {
                    Ok(_) => {
                        accepted += 1;
                        assert!(accepted <= capacity);
                    }
                    Err(error) => break error,
                }
            };
            assert_eq!(refused.kind(), runenui_runtime::SubmitActionErrorKind::Full);
            assert!(matches!(refused.into_action(), Action::Noop));
            let drain_started = Instant::now();
            assert!(
                runtime
                    .pump(FULL)
                    .unwrap_or_else(|_| unreachable!())
                    .report()
                    .is_quiescent()
            );
            let drain_us = drain_started.elapsed().as_micros();
            assert_eq!(runtime.state().noops, accepted);
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            eprintln!(
                "U475_WORKLOAD fields={fields} subscriptions={subscriptions} capacity={capacity} trace={trace_capacity} mount_us={mount_us} ime_cycle_p50_us={} ime_cycle_p95_us={} noop_roots={accepted} drain_us={drain_us}",
                latencies[16], latencies[30]
            );
            runtime.shutdown().unwrap_or_else(|_| unreachable!());
        }
    }
}

#[test]
fn sixteen_credits_refuse_an_independent_ime_update_without_spending_cleanup() {
    let mut runtime = AppRuntime::<Form>::mount_with_config(
        State {
            fields: 1,
            subscriptions: 0,
            text: String::new(),
            selection: 0,
            revision: 0,
            noops: 0,
        },
        RuntimeConfig::default().with_queue_capacity(16),
    )
    .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let owner = runtime.index().nodes()[1].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let composition = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let update = runtime
        .submit_composition_update(composition.generation().clone(), "é".into(), None)
        .err()
        .unwrap_or_else(|| unreachable!("independent update cannot fit beside the live frontier"));
    assert_eq!(
        update.kind(),
        runenui_runtime::SubmitCompositionErrorKind::Full
    );
    let event = update.into_event();
    assert_eq!(event.generation(), composition.generation());
    assert_eq!(runtime.state().revision, 0);
    assert_eq!(runtime.state().text, "");
    runtime
        .cancel_composition(composition.generation().clone())
        .unwrap_or_else(|_| unreachable!("cleanup uses retained credits"));
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    eprintln!(
        "U475_SMALL_FORM C=16 retained_frontier=10 independent_update=10 update=Full cleanup=committed"
    );
}

/// Opt-in release-mode diagnostic, not a timing assertion or acceptance limit.
#[test]
#[ignore = "run by producer-ledger-performance workflow"]
fn bench_u475_accumulated_noop_root_admission_scaling() {
    const REPEATS: usize = 30;
    for size in [32usize, 256, 1024, 2048] {
        for trace in [0, 512] {
            let mut timings = Vec::with_capacity(REPEATS);
            let mut max_rss_kib = 0usize;
            for _ in 0..REPEATS {
                let mut runtime = AppRuntime::<Form>::mount_with_config(
                    State {
                        fields: 1,
                        subscriptions: 0,
                        text: String::new(),
                        selection: 0,
                        revision: 0,
                        noops: 0,
                    },
                    RuntimeConfig::default()
                        .with_queue_capacity(size)
                        .with_trace_config(TraceConfig::new(trace)),
                )
                .unwrap_or_else(|error| panic!("diagnostic mount: {error}"));
                assert!(
                    runtime
                        .pump(FULL)
                        .unwrap_or_else(|_| unreachable!())
                        .report()
                        .is_quiescent()
                );
                let start = Instant::now();
                for _ in 0..size {
                    runtime
                        .submit_action(Action::Noop)
                        .unwrap_or_else(|error| panic!("one-credit producer fits: {error:?}"));
                }
                timings.push(start.elapsed().as_micros());
                if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                    if let Some(kib) = status
                        .lines()
                        .find_map(|line| line.strip_prefix("VmRSS:"))
                        .and_then(|value| value.split_whitespace().next())
                        .and_then(|value| value.parse::<usize>().ok())
                    {
                        max_rss_kib = max_rss_kib.max(kib);
                    }
                }
                assert!(
                    runtime
                        .pump(FULL)
                        .unwrap_or_else(|_| unreachable!())
                        .report()
                        .is_quiescent()
                );
                assert_eq!(runtime.state().noops, size);
                assert_eq!(runtime.status(), RuntimeStatus::Running);
                runtime.shutdown().unwrap_or_else(|_| unreachable!());
            }
            timings.sort_unstable();
            println!(
                "U475_METRIC,admission,{trace},{size},{REPEATS},{},{},{},{max_rss_kib}",
                timings[REPEATS / 2],
                timings[(95 * REPEATS).div_ceil(100) - 1],
                timings[REPEATS - 1]
            );
        }
    }
}
