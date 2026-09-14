#![allow(refining_impl_trait)]
#![allow(
    clippy::float_cmp,
    reason = "M9 transition proofs require exact accepted endpoint identity at start and explicit disable"
)]

use std::time::Duration;

use runenui_core::{
    Element, MotionEasing, MotionTarget, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, TransitionSpec, UiApp, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, SurfacePublication,
};

#[derive(Clone, Copy)]
struct TransitionState {
    transparent: bool,
    policy_enabled: bool,
    policy_disabled: bool,
}

#[derive(Clone, Copy)]
enum TransitionAction {
    SetTarget(bool),
    SetPolicy(bool),
    DisablePolicy,
}

struct TransitionApp;

impl UiApp for TransitionApp {
    type State = TransitionState;
    type Action = TransitionAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let opacity = if state.transparent {
            SceneOpacity::TRANSPARENT
        } else {
            SceneOpacity::OPAQUE
        };
        let root = text("transition").key("root").opacity(opacity);
        if state.policy_disabled {
            root.transition_disabled(MotionTarget::Opacity)
                .into_element()
        } else if state.policy_enabled {
            root.transition(MotionTarget::Opacity, linear_transition())
                .into_element()
        } else {
            root.into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            TransitionAction::SetTarget(transparent) => state.transparent = transparent,
            TransitionAction::SetPolicy(enabled) => {
                state.policy_enabled = enabled;
                state.policy_disabled = false;
            }
            TransitionAction::DisablePolicy => {
                state.policy_enabled = false;
                state.policy_disabled = true;
            }
        }
    }
}

const fn initial_state() -> TransitionState {
    TransitionState {
        transparent: false,
        policy_enabled: true,
        policy_disabled: false,
    }
}

fn linear_transition() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(100),
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("transition continuity proof spec is valid"))
}

fn publish(
    runtime: &mut AppRuntime<TransitionApp>,
    environment: &StyleEnvironment,
) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("transition continuity publication is admitted"))
}

fn opacity(publication: &SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("transition continuity proof has a root"))
        .computed_style()
        .opacity()
        .get()
}

fn dispatch(runtime: &mut AppRuntime<TransitionApp>, action: TransitionAction) {
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!("transition continuity action is accepted"));
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
}

#[test]
fn interrupted_transition_reverses_from_the_current_same_clock_sample() {
    let mut runtime = AppRuntime::<TransitionApp>::mount(initial_state());
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    assert_eq!(opacity(&initial), 1.0);

    dispatch(&mut runtime, TransitionAction::SetTarget(true));
    let started = publish(&mut runtime, &environment);
    assert_eq!(opacity(&started), 1.0);

    runtime
        .advance_time(Duration::from_millis(40))
        .unwrap_or_else(|_| unreachable!("bounded transition advance is representable"));
    let outbound = publish(&mut runtime, &environment);
    assert!((opacity(&outbound) - 0.6).abs() <= f32::EPSILON);

    dispatch(&mut runtime, TransitionAction::SetTarget(false));
    let reversed = publish(&mut runtime, &environment);
    assert!(
        (opacity(&reversed) - 0.6).abs() <= f32::EPSILON,
        "replacement must begin from the old transition's sample at the exact replacement instant"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded reversal advance is representable"));
    let halfway_back = publish(&mut runtime, &environment);
    assert!((opacity(&halfway_back) - 0.8).abs() <= f32::EPSILON);
}

#[test]
fn removing_policy_keeps_an_unchanged_live_transition_until_completion() {
    let mut runtime = AppRuntime::<TransitionApp>::mount(initial_state());
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    assert_eq!(opacity(&initial), 1.0);

    dispatch(&mut runtime, TransitionAction::SetTarget(true));
    let started = publish(&mut runtime, &environment);
    assert_eq!(opacity(&started), 1.0);

    runtime
        .advance_time(Duration::from_millis(40))
        .unwrap_or_else(|_| unreachable!("bounded transition advance is representable"));
    let before_removal = publish(&mut runtime, &environment);
    assert!((opacity(&before_removal) - 0.6).abs() <= f32::EPSILON);

    dispatch(&mut runtime, TransitionAction::SetPolicy(false));
    let removed = publish(&mut runtime, &environment);
    assert!(
        (opacity(&removed) - 0.6).abs() <= f32::EPSILON,
        "policy absence with an unchanged target must retain the already-accepted transition"
    );

    runtime
        .advance_time(Duration::from_millis(20))
        .unwrap_or_else(|_| unreachable!("bounded retained transition advance is representable"));
    let continued = publish(&mut runtime, &environment);
    assert!((opacity(&continued) - 0.4).abs() <= f32::EPSILON);
}

#[test]
fn explicit_disabled_policy_cancels_the_live_transition_at_the_current_target() {
    let mut runtime = AppRuntime::<TransitionApp>::mount(initial_state());
    let environment = StyleEnvironment::default();

    assert_eq!(opacity(&publish(&mut runtime, &environment)), 1.0);
    dispatch(&mut runtime, TransitionAction::SetTarget(true));
    assert_eq!(opacity(&publish(&mut runtime, &environment)), 1.0);

    runtime
        .advance_time(Duration::from_millis(40))
        .unwrap_or_else(|_| unreachable!("bounded transition advance is representable"));
    let live = publish(&mut runtime, &environment);
    assert!((opacity(&live) - 0.6).abs() <= f32::EPSILON);

    dispatch(&mut runtime, TransitionAction::DisablePolicy);
    let disabled = publish(&mut runtime, &environment);
    assert_eq!(
        opacity(&disabled),
        0.0,
        "an explicit Disabled policy must cancel the live transition and expose the current resolved target"
    );

    runtime
        .advance_time(Duration::from_millis(20))
        .unwrap_or_else(|_| unreachable!("bounded post-disable advance is representable"));
    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
}
