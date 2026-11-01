//! #541 research-only compile probe for the existing independent `UiApp` contract.
//!
//! This probes a type-safe fluent UI facade shape against the real `UiApp`
//! associated-type contract. The local facade deliberately has NO native
//! run implementation and is not a shipped API.

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


// A local *type-checking* facade only. It deliberately does not create a window,
// provide a scheduler, or pretend a native run method exists.
#[derive(Default)]
struct FluentApp {
    window_title: Option<&'static str>,
}

impl FluentApp {
    fn new() -> Self {
        Self::default()
    }

    fn window(mut self, title: &'static str) -> Self {
        self.window_title = Some(title);
        self
    }

    fn ui<A: UiApp<State = A>>(self, state: A) -> ConfiguredUi<A> {
        ConfiguredUi {
            window_title: self.window_title,
            state,
        }
    }

    // An app whose State differs from its UiApp implementation still has a
    // fully typed explicit path; the ordinary self-state path needs no marker.
    fn ui_with_state<A: UiApp>(self, state: A::State) -> ConfiguredLegacyUi<A> {
        ConfiguredLegacyUi {
            window_title: self.window_title,
            runtime: PreparedApp::<A>::new(state, RuntimeConfig::default()),
        }
    }
}

struct ConfiguredUi<A: UiApp<State = A>> {
    window_title: Option<&'static str>,
    state: A,
}

impl<A: UiApp<State = A>> ConfiguredUi<A> {
    fn into_parts(self) -> (Option<&'static str>, A) {
        (self.window_title, self.state)
    }
}

struct ConfiguredLegacyUi<A: UiApp> {
    window_title: Option<&'static str>,
    runtime: PreparedApp<A>,
}

impl<A: UiApp> ConfiguredLegacyUi<A> {
    fn into_runtime(self) -> (Option<&'static str>, AppRuntime<A>) {
        (self.window_title, self.runtime.mount())
    }
}

// The UI model itself implements the existing real UiApp trait.
// There is no additional CounterApp marker or synthetic UI runtime.
#[derive(Debug, Eq, PartialEq)]
struct Counter {
    count: i32,
}

impl Counter {
    const fn new() -> Self {
        Self { count: 0 }
    }
}

enum CounterAction {
    Increment,
}

impl UiApp for Counter {
    type State = Self;
    type Action = CounterAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        text(format!("Count: {}", state.count))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            CounterAction::Increment => state.count += 1,
        }
    }
}

#[test]
fn ordinary_self_state_ui_model_infers_app_type_without_a_marker() {
    let (title, model) = FluentApp::new()
        .window("Counter")
        .ui(Counter::new())
        .into_parts();
    assert_eq!(title, Some("Counter"));

    let mut runtime = AppRuntime::<Counter>::mount(model);
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    assert!(runtime.submit_action(CounterAction::Increment).is_ok());
    assert!(runtime.pump(PumpBudget::new(64, 64, 64, 64)).is_ok());
    assert_eq!(runtime.state().count, 1);
}

#[test]
fn advanced_marker_and_separate_state_remains_explicitly_type_safe() {
    let (title, runtime) = FluentApp::new()
        .window("Number")
        .ui_with_state::<NumberApp>(42)
        .into_runtime();
    assert_eq!(title, Some("Number"));
    assert_eq!(*runtime.state(), 42);
}
