#![allow(refining_impl_trait)]

// #478 proof-only experiment. This file deliberately does NOT install a new
// runtime admission path. The small pure stage below is the strongest useful
// application-only preparation possible with the current public UiApp protocol;
// the real TextField/mounted-widget test exposes its missing transaction scope.
use std::{
    cell::RefCell,
    mem::size_of,
    rc::Rc,
    time::Instant,
};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CompositionEvent, Element, ElementId, EventContext, EventPhase,
    FocusEventKind, NoHostProtocol, SemanticCommand, SemanticEditableMode, TextAffinity,
    TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextField, TextPosition,
    TextSelection, UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetUpdateContext, column,
    container,
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

impl Widget<Action> for RetainedProbe {
    // The actual mounted state is deliberately not Clone.
    type State = Box<usize>;

    fn create_state(&self) -> Self::State {
        Box::new(self.revision)
    }

    fn update(&self, local: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if **local != self.revision {
            **local = self.revision;
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
        let snapshot = TextDocumentSnapshot::new(
            TextDocumentId::new(1),
            TextDocumentRevision::new(1),
        );
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

fn target(runtime: &AppRuntime<App>, name: &str) -> MountedNodeId {
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
        Action::Disable(payload) => &**payload as *const u64,
        _ => unreachable!("disable action"),
    };
    let start = Instant::now();
    let refused = match try_prepare(&live, action, 15, 16, 4) {
        Ok(_) => panic!("no capacity for source plus complete declared slots"),
        Err(action) => action,
    };
    let elapsed = start.elapsed();
    let returned_payload_address = match &refused {
        Action::Disable(payload) => &**payload as *const u64,
        _ => unreachable!("same payload variant"),
    };
    assert_eq!(original_payload_address, returned_payload_address);
    assert_eq!(*live.token, 7);
    assert!(live.enabled);
    assert_eq!(live.revision, 0);
    assert!(live.observed.borrow().is_empty());

    let prepared = match try_prepare(&live, refused, 0, 16, 4) {
        Ok(prepared) => prepared,
        Err(_) => panic!("available preview should admit"),
    };
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
        runtime
            .submit_command(
                target(&runtime, "editor"),
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
                .expect("IME cancel must route through ancestor");
            let mounted = processed
                .iter()
                .position(|event| *event == "mounted")
                .expect("compatible mounted update effect");
            assert!(first_ancestor < mounted, "cleanup outputs precede update outputs");
        }
    }
}
