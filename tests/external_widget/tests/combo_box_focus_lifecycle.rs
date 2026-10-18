#![allow(refining_impl_trait)]
//! One real ComboBox focus owner; losing focus dismisses via application action.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, NoHostProtocol, SemanticCommand, UiApp, View,
    button, combo_box, option_item, row,
};
use runenui_runtime::{AppRuntime, MountedNodeId, PumpBudget};
struct State {
    open: bool,
    enabled: bool,
    committed: u8,
    requests: Vec<ComboBoxRequest>,
}
enum Action {
    Popup(ComboBoxRequest),
    Other,
    SetEnabled(bool),
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        row(vec![
            combo_box(
                "Choose",
                ElementId::new("focus.combo.list").unwrap_or_else(|_| unreachable!()),
                [option_item("Alpha", state.committed == 1)
                    .id("focus.combo.alpha")
                    .on_activate(|| Action::Other)],
                Action::Popup,
            )
            .id("focus.combo.owner")
            .open(state.open)
            .enabled(state.enabled)
            .into_element(),
            button("Next")
                .id("focus.other")
                .on_activate(|| Action::Other)
                .into_element(),
        ])
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Popup(request) => {
                state.open = matches!(request, ComboBoxRequest::Open);
                state.requests.push(request);
            }
            Action::SetEnabled(enabled) => state.enabled = enabled,
            Action::Other => {}
        }
    }
}
fn settle(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent()
    );
}
fn id(runtime: &mut AppRuntime<App>, text: &str) -> MountedNodeId {
    let authored = ElementId::new(text).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("focus fixture node exists"))
        .id()
        .clone()
}
fn focus(runtime: &mut AppRuntime<App>, target: MountedNodeId) {
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus command admitted"));
    settle(runtime);
}
#[test]
fn leaving_combo_box_real_focus_closes_popup_only_via_application_fifo() {
    let mut runtime = AppRuntime::<App>::mount(State {
        open: true,
        enabled: true,
        committed: 0,
        requests: Vec::new(),
    });
    settle(&mut runtime);
    let owner = id(&mut runtime, "focus.combo.owner");
    let other = id(&mut runtime, "focus.other");
    focus(&mut runtime, owner.clone());
    assert!(runtime.state().open);
    assert!(runtime.state().requests.is_empty());
    focus(&mut runtime, other.clone());
    assert_eq!(runtime.focus().focused_node(), Some(&other));
    assert!(
        !runtime.state().open,
        "popup follows app's committed Close update"
    );
    assert_eq!(runtime.state().requests, [ComboBoxRequest::Close]);
    assert_eq!(runtime.state().committed, 0);
    assert!(!runtime.index().nodes().iter().any(|node| {
        node.authored_id()
            .is_some_and(|id| id.as_str() == "focus.combo.alpha")
    }));
}

#[test]
fn compatible_disable_closes_app_owned_popup_once_and_reenable_does_not_reopen() {
    for initially_open in [false, true] {
        let mut runtime = AppRuntime::<App>::mount(State {
            open: initially_open,
            enabled: true,
            committed: 0,
            requests: Vec::new(),
        });
        settle(&mut runtime);
        let owner = id(&mut runtime, "focus.combo.owner");
        focus(&mut runtime, owner.clone());

        runtime
            .submit_action(Action::SetEnabled(false))
            .unwrap_or_else(|_| unreachable!("authoring change accepted"));
        let report = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("disable action processed"))
            .report()
            .to_owned();
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(id(&mut runtime, "focus.combo.owner"), owner);
        assert!(!runtime.state().enabled);
        assert_eq!(runtime.state().open, initially_open);
        assert!(runtime.state().requests.is_empty());
        assert_eq!(
            report.remaining_queued_envelopes(),
            usize::from(initially_open),
            "only a previously effective-open popup may queue a Close request",
        );

        settle(&mut runtime);
        assert_eq!(runtime.focus().focused_node(), None);
        assert!(!runtime.state().open);
        assert_eq!(
            runtime.state().requests,
            if initially_open {
                vec![ComboBoxRequest::Close]
            } else {
                Vec::new()
            },
        );
        assert!(!runtime.index().nodes().iter().any(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "focus.combo.alpha")
        }));

        runtime
            .submit_action(Action::SetEnabled(true))
            .unwrap_or_else(|_| unreachable!("reenable accepted"));
        settle(&mut runtime);
        assert!(runtime.state().enabled);
        assert!(
            !runtime.state().open,
            "stale authored Open cannot resurrect popup"
        );
        assert!(!runtime.index().nodes().iter().any(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "focus.combo.alpha")
        }));
        assert_eq!(id(&mut runtime, "focus.combo.owner"), owner);
    }
}

#[test]
fn explicitly_closed_popup_does_not_emit_a_redundant_widget_close() {
    let mut runtime = AppRuntime::<App>::mount(State {
        open: true,
        enabled: true,
        committed: 0,
        requests: Vec::new(),
    });
    settle(&mut runtime);
    runtime
        .submit_action(Action::Popup(ComboBoxRequest::Close))
        .unwrap_or_else(|_| unreachable!("application Close action accepted"));
    settle(&mut runtime);
    assert!(!runtime.state().open);
    assert_eq!(runtime.state().requests, [ComboBoxRequest::Close]);
}
