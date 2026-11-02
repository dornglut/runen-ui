//! Native-independent proof that the host clock and wake are installed before
//! the canonical initial application transaction, without any second scheduler.

#![allow(refining_impl_trait)]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use runenui_core::{Effects, IntoEffects, NoHostProtocol, UiApp, View, text};
use runenui_runtime::{AppRuntime, MonotonicClock, MonotonicInstant, PumpBudget, RuntimeConfig};

struct InitialActionApp;

impl UiApp for InitialActionApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("host startup")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::action(())
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

struct SampleClock {
    readings: Arc<AtomicUsize>,
}

impl MonotonicClock for SampleClock {
    fn now(&self) -> MonotonicInstant {
        self.readings.fetch_add(1, Ordering::SeqCst);
        MonotonicInstant::ZERO
    }
}

#[test]
fn initial_application_work_observes_preinstalled_host_clock_and_wake() {
    let readings = Arc::new(AtomicUsize::new(0));
    let wakes = Arc::new(AtomicUsize::new(0));
    let wake_count = Arc::clone(&wakes);

    let mut runtime = AppRuntime::<InitialActionApp>::mount_with_host_adapters(
        0,
        RuntimeConfig::default(),
        SampleClock {
            readings: Arc::clone(&readings),
        },
        move || {
            wake_count.fetch_add(1, Ordering::SeqCst);
        },
    );

    // These observations must hold *before* the caller can pump or set a
    // transport after mounting: an initial transaction already happened.
    assert!(readings.load(Ordering::SeqCst) > 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    assert_eq!(*runtime.state(), 0);

    assert!(runtime.pump(PumpBudget::new(64, 64, 64, 64)).is_ok());
    assert_eq!(*runtime.state(), 1);
}
