//! #541 research-only compile probe for the existing independent `UiApp` contract.
//!
//! This does not implement a native application host or assert a proposed API
//! exists. The local descriptor is disposable until the owner accepts a design.

use core::marker::PhantomData;

use runenui_core::{NoHostProtocol, UiApp, View, text};
use runenui_runtime::{AppRuntime, PumpBudget, RuntimeConfig, RuntimeStatus};

struct PreparedApp<A: UiApp> {
    state: A::State,
    config: RuntimeConfig,
    app: PhantomData<fn() -> A>,
}

impl<A: UiApp> PreparedApp<A> {
    fn new(state: A::State, config: RuntimeConfig) -> Self {
        Self {
            state,
            config,
            app: PhantomData,
        }
    }

    fn mount(self) -> AppRuntime<A> {
        AppRuntime::<A>::mount_with_config(self.state, self.config)
    }
}

struct NumberApp;

impl UiApp for NumberApp {
    type State = i32;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        text(format!("Number: {state}"))
    }

    fn update(
        _state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

struct MessageApp;

impl UiApp for MessageApp {
    type State = &'static str;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        text(*state)
    }

    fn update(
        _state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn independent_app_descriptors_mount_two_distinct_ui_apps_with_public_runtime_api() {
    let numbers = PreparedApp::<NumberApp>::new(42, RuntimeConfig::default()).mount();
    let message = PreparedApp::<MessageApp>::new("independent", RuntimeConfig::default()).mount();

    assert_eq!(*numbers.state(), 42);
    assert_eq!(*message.state(), "independent");
    assert_eq!(numbers.status(), RuntimeStatus::Running);
    assert_eq!(message.status(), RuntimeStatus::Running);
}

#[test]
fn ordinary_external_host_can_drive_and_close_without_a_native_dependency() {
    let mut runtime =
        PreparedApp::<MessageApp>::new("external host", RuntimeConfig::default()).mount();

    let observation = runtime.pump(PumpBudget::new(64, 64, 64, 64));
    assert!(observation.is_ok());
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    let shutdown = runtime.shutdown();
    assert!(shutdown.is_ok());
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
}
