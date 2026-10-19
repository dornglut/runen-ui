#![allow(refining_impl_trait)]

// #475 qualified-admission proof using the completed #478 consumer. This file deliberately does NOT install a new
// runtime admission path. The small pure stage below is the strongest useful
// application-only preparation possible with the current public UiApp protocol;
// the real TextField/mounted-widget test exposes its missing transaction scope.
use std::{cell::RefCell, mem::size_of, rc::Rc, time::Instant};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CompositionEvent, Element, ElementId, EventContext,
    EventPhase, FocusEventKind, NoHostProtocol, SemanticCommand, SemanticEditableMode,
    TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextField,
    TextPosition, TextSelection, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    WidgetUpdateContext, column, container,
};
use runenui_runtime::{
    AppRuntime, MountedNodeId, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    RuntimeTerminalReason,
};

enum Action {
    Disable(Box<u64>),
    Follower,
    MountedEffect,
    AncestorEffect,
}

struct State {
    enabled: bool,
    revision: usize,
    token: Box<u64>,
    processed_followers: usize,
    processed_mounted: usize,
    processed: Vec<&'static str>,
    observed: Rc<RefCell<Vec<&'static str>>>,
}

#[derive(Debug)]
struct Ancestor {
    observed: Rc<RefCell<Vec<&'static str>>>,
}

impl Widget<Action> for Ancestor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if event
            .as_focus()
            .is_some_and(|focus| focus.kind() == FocusEventKind::Out)
        {
            self.observed.borrow_mut().push("ancestor-focus-out");
            if context.phase() == EventPhase::Capture || context.phase() == EventPhase::Bubble {
                context.emit(Action::AncestorEffect);
            }
        }
        if matches!(event, UiEvent::Composition(CompositionEvent::Cancel(_))) {
            self.observed.borrow_mut().push("ancestor-ime-cancel");
            if matches!(context.phase(), EventPhase::Capture | EventPhase::Bubble) {
                context.emit(Action::AncestorEffect);
            }
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for Ancestor {}

#[derive(Debug)]
struct RetainedProbe {
    revision: usize,
    observed: Rc<RefCell<Vec<&'static str>>>,
}

// A retained widget state contains an owned, non-Clone receiver. Staging may
// not duplicate, replay or restore this state through a generic Clone bound.
struct NonCloneMountedState {
    revision: Box<usize>,
    _receiver: std::sync::mpsc::Receiver<()>,
}

impl Widget<Action> for RetainedProbe {
    type State = NonCloneMountedState;

    fn create_state(&self) -> Self::State {
        let (_sender, receiver) = std::sync::mpsc::channel();
        NonCloneMountedState {
            revision: Box::new(self.revision),
            _receiver: receiver,
        }
    }

    fn update(&self, local: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *local.revision != self.revision {
            *local.revision = self.revision;
            self.observed.borrow_mut().push("mounted-update");
            context.emit(Action::MountedEffect);
        }
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(1), TextDocumentRevision::new(1));
        let selection = TextSelection::collapsed(
            TextPosition::new(snapshot, "", 0, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("valid empty document caret")),
        );
        let field = TextField::new(
            snapshot,
            "",
            selection,
            SemanticEditableMode::SingleLine,
            |_| Action::MountedEffect,
        )
        .unwrap_or_else(|_| unreachable!("valid TextField"))
        .disabled(!state.enabled)
        .id("editor")
        .key("editor");
        column(vec![
            container(
                Ancestor {
                    observed: Rc::clone(&state.observed),
                },
                vec![
                    field.into_element(),
                    Element::new(RetainedProbe {
                        revision: state.revision,
                        observed: Rc::clone(&state.observed),
                    })
                    .key("retained")
                    .into_element(),
                ],
            )
            .key("ancestor")
            .into_element(),
            Element::new(RetainedProbe {
                revision: 0,
                observed: Rc::clone(&state.observed),
            })
            .id("other")
            .focusable(true),
        ])
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Disable(payload) => {
                state.enabled = false;
                state.revision += 1;
                *state.token += *payload;
                state.observed.borrow_mut().push("application-update");
            }
            Action::Follower => {
                state.processed_followers += 1;
                state.processed.push("follower");
            }
            Action::MountedEffect => {
                state.processed_mounted += 1;
                state.processed.push("mounted");
            }
            Action::AncestorEffect => state.processed.push("ancestor"),
        }
    }
}

fn state() -> State {
    State {
        enabled: true,
        revision: 0,
        token: Box::new(7),
        processed_followers: 0,
        processed_mounted: 0,
        processed: Vec::new(),
        observed: Rc::new(RefCell::new(Vec::new())),
    }
}

fn target(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = ElementId::new(name).unwrap_or_else(|_| unreachable!("known test id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted test target"))
        .id()
        .clone()
}

fn settle(runtime: &mut AppRuntime<App>) {
    let report = runtime
        .pump(PumpBudget::new(usize::MAX, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("public pump"))
        .report()
        .to_owned();
    assert!(report.is_quiescent());
}

// No Clone, global snapshot, arbitrary callback replay or mutation occurs in
// this application-only pure preview. This is NOT runtime queue admission.
struct Preview {
    action: Action,
    next_revision: usize,
    required_slots: usize,
}

fn try_prepare(
    state: &State,
    action: Action,
    waiting: usize,
    capacity: usize,
    declared_slots: usize,
) -> Result<Preview, Action> {
    let next_revision = if matches!(action, Action::Disable(_)) {
        state.revision + 1
    } else {
        state.revision
    };
    let fits = waiting
        .checked_add(1)
        .and_then(|used| used.checked_add(declared_slots))
        .is_some_and(|used| used <= capacity);
    if !fits {
        return Err(action);
    }
    Ok(Preview {
        action,
        next_revision,
        required_slots: declared_slots,
    })
}

#[test]
fn pure_preview_can_refuse_the_same_nonclone_payload_without_state_mutation() {
    let mut live = state();
    let action = Action::Disable(Box::new(11));
    let original_payload_address = match &action {
        Action::Disable(payload) => std::ptr::from_ref(payload.as_ref()),
        _ => unreachable!("disable action"),
    };
    let start = Instant::now();
    let refused = try_prepare(&live, action, 15, 16, 4)
        .err()
        .unwrap_or_else(|| unreachable!("no capacity for source plus declared slots"));
    let elapsed = start.elapsed();
    let returned_payload_address = match &refused {
        Action::Disable(payload) => std::ptr::from_ref(payload.as_ref()),
        _ => unreachable!("same payload variant"),
    };
    assert_eq!(original_payload_address, returned_payload_address);
    assert_eq!(*live.token, 7);
    assert!(live.enabled);
    assert_eq!(live.revision, 0);
    assert!(live.observed.borrow().is_empty());

    let prepared = try_prepare(&live, refused, 0, 16, 4)
        .ok()
        .unwrap_or_else(|| unreachable!("available preview should admit"));
    assert_eq!(prepared.required_slots, 4);
    assert_eq!(prepared.next_revision, 1);
    // Only application mutation is committed. This preview has not prepared
    // any Widget::update, IME, FocusOut, trace or queued follow-on work.
    App::update(&mut live, prepared.action);
    assert!(!live.enabled);
    assert_eq!(*live.token, 18);
    assert_eq!(live.revision, 1);
    eprintln!(
        "UI478 preview: refusal_ns={} preview_header_bytes={} payload_box_bytes={} declared_slots={}",
        elapsed.as_nanos(),
        size_of::<Preview>(),
        size_of::<u64>(),
        4,
    );
}

#[test]
fn actual_textfield_ime_and_nonclone_mounted_update_escape_preview_scope() {
    // Current accepted main, NOT the unmerged live-FocusOut correction #476.
    // This is an adversarial negative integration proof, not a claim that a
    // separate preview can intercept the runtime's accepted FIFO head.
    for (capacity, allowance, followers, should_poison) in [
        (16, 4, 0, false),
        (16, 4, 4, false),
        (16, 4, 15, true),
        (4096, 1024, 0, false),
        (4096, 1024, 2049, true),
    ] {
        let mut runtime = AppRuntime::<App>::mount_with_config(
            state(),
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(capacity)
                    .with_transaction_outputs(allowance),
            ),
        );
        settle(&mut runtime);
        let editor = target(&mut runtime, "editor");
        runtime
            .submit_command(
                editor,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("editor focus submitted"));
        settle(&mut runtime);
        runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("IME composition started"));
        settle(&mut runtime);

        runtime
            .submit_action(Action::Disable(Box::new(11)))
            .unwrap_or_else(|_| unreachable!("disablement accepted"));
        for _ in 0..followers {
            runtime
                .submit_action(Action::Follower)
                .unwrap_or_else(|_| unreachable!("queued follower accepted"));
        }
        let start = Instant::now();
        let report = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one accepted action"))
            .report()
            .to_owned();
        let elapsed = start.elapsed();
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(*runtime.state().token, 18);
        assert!(!runtime.state().enabled);
        let seen = runtime.state().observed.borrow();
        assert!(seen.contains(&"application-update"));
        assert!(seen.contains(&"mounted-update"));
        drop(seen);
        eprintln!(
            "UI478 runtime: C={} R={} followers={} remaining={} status={:?} elapsed_ns={} state_header_bytes={} queued_payload_lower_bound_bytes={}",
            capacity,
            allowance,
            followers,
            report.remaining_queued_envelopes(),
            runtime.status(),
            elapsed.as_nanos(),
            size_of::<State>(),
            report.remaining_queued_envelopes() * size_of::<Action>(),
        );
        if should_poison {
            assert_eq!(
                runtime.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned),
                "accepted action mutates non-Clone app and mounted state before failed admission",
            );
        } else {
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            assert_eq!(runtime.state().processed_mounted, 0);
            settle(&mut runtime);
            assert_eq!(runtime.state().processed_mounted, 1);
            assert_eq!(runtime.state().processed_followers, followers);
            let processed = &runtime.state().processed;
            assert_eq!(&processed[..followers], vec!["follower"; followers]);
            let first_ancestor = processed
                .iter()
                .position(|event| *event == "ancestor")
                .unwrap_or_else(|| unreachable!("IME cancel must route through ancestor"));
            let mounted = processed
                .iter()
                .position(|event| *event == "mounted")
                .unwrap_or_else(|| unreachable!("compatible mounted update effect"));
            assert!(
                first_ancestor < mounted,
                "cleanup outputs precede update outputs"
            );
        }
    }
}

#[test]
fn real_textfield_routed_ime_and_focusout_outputs_follow_existing_fifo() {
    // Normal focus transfer has a live old route on accepted main. This is
    // distinct from the compatible-disablement FocusOut still blocked in #475.
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state(),
        RuntimeConfig::default().with_limits(
            RuntimeLimits::default()
                .with_waiting_envelopes(16)
                .with_transaction_outputs(4),
        ),
    );
    settle(&mut runtime);
    let editor = target(&mut runtime, "editor");
    let next = target(&mut runtime, "other");
    runtime
        .submit_command(
            editor,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("editor focused"));
    settle(&mut runtime);
    runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("active IME"));
    settle(&mut runtime);

    runtime
        .submit_command(
            next,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("transfer accepted"));
    for _ in 0..2 {
        runtime
            .submit_action(Action::Follower)
            .unwrap_or_else(|_| unreachable!("older FIFO follower"));
    }
    let report = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("single routed transition"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state().processed.len(), 0);
    assert_eq!(
        &*runtime.state().observed.borrow(),
        &[
            "ancestor-ime-cancel",
            "ancestor-ime-cancel",
            "ancestor-focus-out",
            "ancestor-focus-out",
        ],
    );
    assert_eq!(report.remaining_queued_envelopes(), 6);
    settle(&mut runtime);
    assert_eq!(
        runtime.state().processed,
        vec![
            "follower", "follower", "ancestor", "ancestor", "ancestor", "ancestor",
        ]
    );
}

// #475 PROOF ONLY: one vector of prepaid claims, with no second runtime queue.
// Existing public Widget/UiApp cannot yet be constrained to these claims.
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
struct Credits {
    fifo: usize,
    trace: usize,
    work: usize,
    identities: usize,
    sequences: usize,
}
impl Credits {
    const fn of(
        fifo: usize,
        trace: usize,
        work: usize,
        identities: usize,
        sequences: usize,
    ) -> Self {
        Self {
            fifo,
            trace,
            work,
            identities,
            sequences,
        }
    }
    fn plus(self, rhs: Self) -> Option<Self> {
        Some(Self {
            fifo: self.fifo.checked_add(rhs.fifo)?,
            trace: self.trace.checked_add(rhs.trace)?,
            work: self.work.checked_add(rhs.work)?,
            identities: self.identities.checked_add(rhs.identities)?,
            sequences: self.sequences.checked_add(rhs.sequences)?,
        })
    }
    fn minus(self, rhs: Self) -> Self {
        Self::of(
            self.fifo
                .checked_sub(rhs.fifo)
                .unwrap_or_else(|| unreachable!("fifo")),
            self.trace
                .checked_sub(rhs.trace)
                .unwrap_or_else(|| unreachable!("trace")),
            self.work
                .checked_sub(rhs.work)
                .unwrap_or_else(|| unreachable!("work")),
            self.identities
                .checked_sub(rhs.identities)
                .unwrap_or_else(|| unreachable!("identities")),
            self.sequences
                .checked_sub(rhs.sequences)
                .unwrap_or_else(|| unreachable!("sequences")),
        )
    }
    const fn fits(self, max: Self) -> bool {
        self.fifo <= max.fifo
            && self.trace <= max.trace
            && self.work <= max.work
            && self.identities <= max.identities
            && self.sequences <= max.sequences
    }
}
struct Claim {
    prepaid: Credits,
}
struct CreditModel {
    max: Credits,
    used: Credits,
}
impl CreditModel {
    fn new(capacity: usize, waiting: usize) -> Self {
        assert!(waiting <= capacity);
        Self {
            max: Credits::of(capacity, 50000, 50000, 50000, 50000),
            used: Credits::of(waiting, waiting, 0, 0, waiting),
        }
    }
    fn admit<T>(&mut self, action: T, downstream: Credits) -> Result<(T, Claim), T> {
        let Some(prepaid) = downstream.plus(Credits::of(1, 1, 0, 0, 1)) else {
            return Err(action);
        };
        let Some(total) = self.used.plus(prepaid) else {
            return Err(action);
        };
        if !total.fits(self.max) {
            return Err(action);
        }
        self.used = total;
        Ok((action, Claim { prepaid }))
    }
    fn release(&mut self, claim: &Claim) {
        self.used = self.used.minus(claim.prepaid);
    }
}
fn observed_p95(samples: &mut [u128]) -> u128 {
    samples.sort_unstable();
    samples[95 * samples.len() / 100]
}
fn closed_binary_lineage(depth: usize) -> Option<usize> {
    let (mut total, mut width) = (0usize, 1usize);
    for _ in 0..=depth {
        total = total.checked_add(width)?;
        width = width.checked_mul(2)?;
    }
    Some(total)
}

#[test]
fn qualified_credit_boundary_retains_exact_nonclone_payload() {
    for (c, r, waiting, admitted) in [
        (16, 4, 7, true),
        (16, 4, 8, false),
        (64, 8, 47, true),
        (64, 8, 48, false),
        (4096, 1024, 2047, true),
        (4096, 1024, 2048, false),
    ] {
        let mut ledger = CreditModel::new(c, waiting);
        let action = Action::Disable(Box::new(11));
        let old_ptr = match &action {
            Action::Disable(value) => std::ptr::from_ref(value.as_ref()),
            _ => unreachable!("disable"),
        };
        let outcome = ledger.admit(action, Credits::of(2 * r, 20, 4, 4, 2 * r));
        assert_eq!(outcome.is_ok(), admitted, "C={c} R={r} q={waiting}");
        match outcome {
            Ok((original, held)) => {
                let new_ptr = match &original {
                    Action::Disable(value) => std::ptr::from_ref(value.as_ref()),
                    _ => unreachable!("disable"),
                };
                assert_eq!(old_ptr, new_ptr);
                assert_eq!(ledger.used.fifo, waiting + 1 + 2 * r);
                ledger.release(&held);
            }
            Err(original) => {
                let new_ptr = match &original {
                    Action::Disable(value) => std::ptr::from_ref(value.as_ref()),
                    _ => unreachable!("disable"),
                };
                assert_eq!(old_ptr, new_ptr);
            }
        }
        assert_eq!(ledger.used.fifo, waiting);
    }
}

#[test]
fn complete_recursive_and_independent_route_guarantees_have_finite_limits() {
    assert_eq!(closed_binary_lineage(4), Some(31));
    assert_eq!(closed_binary_lineage(11), Some(4095));
    assert_eq!(closed_binary_lineage(12), Some(8191));
    for (c, depth) in [(16, 3), (64, 5), (4096, 11)] {
        let count = closed_binary_lineage(depth).unwrap_or_else(|| unreachable!("bounded"));
        let mut ledger = CreditModel::new(c, 0);
        let (_, held) = ledger
            .admit(
                Action::Follower,
                Credits::of(count - 1, count, count, count, count),
            )
            .unwrap_or_else(|_| unreachable!("exact capacity"));
        assert_eq!(ledger.used.fifo, count);
        ledger.release(&held);
        let larger = closed_binary_lineage(depth + 1).unwrap_or_else(|| unreachable!("bounded"));
        assert!(
            ledger
                .admit(
                    Action::Follower,
                    Credits::of(larger - 1, larger, larger, larger, larger)
                )
                .is_err()
        );
        assert_eq!(ledger.used.fifo, 0);
    }
    let mut ledger = CreditModel::new(4096, 0);
    assert!(
        ledger
            .admit(
                Action::Follower,
                Credits::of(4 * 1024, 10, 10, 10, 4 * 1024)
            )
            .is_err()
    );
    let (_, held) = ledger
        .admit(Action::Follower, Credits::of(10, 10, 10, 10, 10))
        .unwrap_or_else(|_| unreachable!("small closed budget"));
    assert_eq!(ledger.used.fifo, 11);
    ledger.release(&held);
    ledger.max.trace = 0;
    assert!(
        ledger
            .admit(Action::Follower, Credits::of(0, 0, 0, 0, 0))
            .is_err()
    );
    // Observed custom ancestor emits capture+bubble for IME and FocusOut.
    // Claiming 0/1/2 routed outputs based on TextField itself is unsound.
}

#[test]
fn measure_test_only_credit_cost_without_conflating_live_pump_latency() {
    let mut accept_ns = Vec::new();
    let mut reject_ns = Vec::new();
    for _ in 0..2_000 {
        let mut ledger = CreditModel::new(16, 0);
        let start = Instant::now();
        let (_, held) = ledger
            .admit(Action::Follower, Credits::of(8, 20, 4, 4, 8))
            .unwrap_or_else(|_| unreachable!("fits"));
        accept_ns.push(start.elapsed().as_nanos());
        ledger.release(&held);

        let mut full = CreditModel::new(16, 8);
        let start = Instant::now();
        assert!(
            full.admit(Action::Follower, Credits::of(8, 20, 4, 4, 8))
                .is_err()
        );
        reject_ns.push(start.elapsed().as_nanos());
    }
    let mut real_ns = Vec::new();
    for _ in 0..20 {
        let mut runtime = AppRuntime::<App>::mount_with_config(
            state(),
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(16)
                    .with_transaction_outputs(4),
            ),
        );
        settle(&mut runtime);
        let editor = target(&mut runtime, "editor");
        runtime
            .submit_command(
                editor,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("focus"));
        settle(&mut runtime);
        runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("IME"));
        settle(&mut runtime);
        runtime
            .submit_action(Action::Disable(Box::new(11)))
            .unwrap_or_else(|_| unreachable!("accepted"));
        for _ in 0..4 {
            runtime
                .submit_action(Action::Follower)
                .unwrap_or_else(|_| unreachable!("follower"));
        }
        let start = Instant::now();
        let report = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("pump"))
            .report()
            .to_owned();
        real_ns.push(start.elapsed().as_nanos());
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
    }
    eprintln!(
        "UI475 measurements debug-hosted: 2000 checks / 20 real pumps, p95_accept_ns={} p95_reject_ns={} p95_mutating_ime_pump_ns={} credit_header_bytes={} claim_bytes={}; accounting NOT wired to runtime",
        observed_p95(&mut accept_ns),
        observed_p95(&mut reject_ns),
        observed_p95(&mut real_ns),
        size_of::<CreditModel>(),
        size_of::<Claim>()
    );
}
