#![allow(refining_impl_trait)]

use runenui_core::{Element, ElementId, NoHostProtocol, UiApp, WidgetStateTypeId, WidgetTypeId};
use runenui_external_widget_conformance::{
    ChildAction, GenericWidget, ParentAction, PulseButton, PulseState, child_component,
};
use runenui_runtime::{ActivationResult, AppRuntime, PumpBudget};

fn settle_initial_mounted_declarations<App: UiApp>(runtime: &mut AppRuntime<App>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn node_by_authored_id<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    authored_id: &str,
) -> (WidgetTypeId, WidgetStateTypeId) {
    let authored_id = ElementId::new(authored_id).unwrap_or_else(|_| unreachable!());
    let index = runtime.index();
    let node = index
        .node_by_authored_id(&authored_id)
        .unwrap_or_else(|| unreachable!());
    (node.widget_type_id(), node.widget_state_type_id())
}

struct PulseApp;

impl UiApp for PulseApp {
    type State = usize;
    type Action = ParentAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        child_component().map_action(ParentAction::Child)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            ParentAction::Child(ChildAction::Pulse) => *state += 1,
            ParentAction::Reset => *state = 0,
        }
    }
}

#[test]
fn concrete_widget_state_and_action_mapping_identity_are_mounted_and_stable() {
    let mut runtime = AppRuntime::<PulseApp>::mount(0);
    settle_initial_mounted_declarations(&mut runtime);
    assert_eq!(
        node_by_authored_id(&mut runtime, "external.pulse"),
        (
            WidgetTypeId::of::<PulseButton>(),
            WidgetStateTypeId::of::<PulseState>(),
        )
    );

    let mounted = runtime.index().nodes()[0].id().clone();
    assert!(matches!(
        runtime.activate_node(&mounted),
        ActivationResult::Queued(_)
    ));
    assert!(matches!(
        runtime.activate_node(&mounted),
        ActivationResult::Queued(_)
    ));
    assert_eq!(*runtime.state(), 0);
    assert_eq!(
        runtime
            .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        2
    );
    assert_eq!(*runtime.state(), 2);
    assert_eq!(
        node_by_authored_id(&mut runtime, "external.pulse"),
        (
            WidgetTypeId::of::<PulseButton>(),
            WidgetStateTypeId::of::<PulseState>(),
        )
    );
}

#[derive(Debug, Eq, PartialEq)]
enum NestedAction {
    Outer(ParentAction),
}

struct NestedMappingApp;

impl UiApp for NestedMappingApp {
    type State = Option<NestedAction>;
    type Action = NestedAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        child_component()
            .map_action(ParentAction::Child)
            .map_action(NestedAction::Outer)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        *state = Some(action);
    }
}

#[test]
fn nested_recursive_mapping_preserves_non_clone_action_and_widget_state_identity() {
    let mut runtime = AppRuntime::<NestedMappingApp>::mount(None);
    settle_initial_mounted_declarations(&mut runtime);
    assert_eq!(
        node_by_authored_id(&mut runtime, "external.pulse"),
        (
            WidgetTypeId::of::<PulseButton>(),
            WidgetStateTypeId::of::<PulseState>(),
        )
    );
    let mounted = runtime.index().nodes()[0].id().clone();
    assert!(matches!(
        runtime.activate_node(&mounted),
        ActivationResult::Queued(_)
    ));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(
        runtime.state(),
        &Some(NestedAction::Outer(ParentAction::Child(ChildAction::Pulse)))
    );
}

#[derive(Debug)]
struct GenericApp<T>(core::marker::PhantomData<T>);

impl<T: core::fmt::Debug + Default + 'static> UiApp for GenericApp<T> {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(GenericWidget(T::default())).id("generic")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn generic_external_widget_instantiations_have_distinct_mounted_type_identity() {
    let mut u8_runtime = AppRuntime::<GenericApp<u8>>::mount(());
    let mut u16_runtime = AppRuntime::<GenericApp<u16>>::mount(());
    let u8_identity = node_by_authored_id(&mut u8_runtime, "generic");
    let u16_identity = node_by_authored_id(&mut u16_runtime, "generic");

    assert_eq!(u8_identity.0, WidgetTypeId::of::<GenericWidget<u8>>());
    assert_eq!(u16_identity.0, WidgetTypeId::of::<GenericWidget<u16>>());
    assert_ne!(u8_identity.0, u16_identity.0);
    assert_eq!(u8_identity.1, WidgetStateTypeId::of::<()>());
    assert_eq!(u16_identity.1, WidgetStateTypeId::of::<()>());
}
