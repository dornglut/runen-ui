use runenui_core::{
    CommandOrigin, ElementId, NoHostProtocol, SemanticCommand, UiApp, View, button,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, SubmitCommandError, TraceConfig, TraceRecordKind,
};
use runenui_winit::controller_input::{
    ControllerButton, ControllerInputOutcome, ControllerInputProfile, ControllerInputState,
    ControllerTransition,
};

struct HostApp;

impl UiApp for HostApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        button("external controller target")
            .on_activate(|| ())
            .id("external.controller.target")
            .key("external.controller.target")
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

fn host_target(runtime: &mut AppRuntime<HostApp>) -> runenui_runtime::MountedNodeId {
    let id = ElementId::new("external.controller.target")
        .unwrap_or_else(|_| unreachable!("fixture element ID is valid"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("public host target is mounted"))
        .id()
        .clone()
}

#[test]
fn reference_host_normalizes_controller_transitions_through_public_runtime()
-> Result<(), SubmitCommandError> {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(128));
    let mut runtime = AppRuntime::<HostApp>::mount_with_config(0, config);
    let target = host_target(&mut runtime);
    let mut controller = ControllerInputState::new(ControllerInputProfile::Navigation);

    for (transition, expected_repeat) in [
        (ControllerTransition::Pressed, false),
        (ControllerTransition::Repeated, true),
    ] {
        let ControllerInputOutcome::Submit(command) =
            controller.transition(ControllerButton::Accept, transition)
        else {
            unreachable!("the selected logical accept control maps to activation");
        };
        assert_eq!(command.profile(), ControllerInputProfile::Navigation);
        assert_eq!(command.command(), SemanticCommand::Activate);
        assert_eq!(command.origin(), CommandOrigin::controller());
        assert_eq!(command.is_repeat(), expected_repeat);
        runtime.submit_command(target.clone(), command.command(), command.origin())?;
        runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    }

    assert_eq!(
        controller.transition(ControllerButton::Accept, ControllerTransition::Cancelled),
        ControllerInputOutcome::Cancelled(ControllerButton::Accept)
    );
    assert_eq!(*runtime.state(), 2);
    assert_eq!(
        runtime.focus().modality(),
        Some(runenui_runtime::InputModality::Controller)
    );
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(record.kind(), TraceRecordKind::RoutedEventStarted))
            .map(runenui_runtime::TraceRecord::command_origin)
            .collect::<Vec<_>>(),
        [
            Some(CommandOrigin::controller()),
            Some(CommandOrigin::controller())
        ]
    );
    assert!(
        runtime
            .trace()
            .export_jsonl()
            .contains("\"source\":\"controller\"")
    );
    Ok(())
}
