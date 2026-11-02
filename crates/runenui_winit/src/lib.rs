//! Reusable native translation and accessibility projection for winit hosts.
//!
//! This crate deliberately does not own a window, event loop, runtime pump,
//! redraw/publication policy, renderer, presentation lifecycle, or application
//! behavior. Hosts retain those authorities and use these adapters only to
//! translate native facts into existing `RunenUI` contracts.

pub mod accessibility;
pub mod controller_input;
pub mod device_identity;
pub mod keyboard_input;
pub mod mouse_input;
pub mod native_mapping;
pub mod text_input;
pub mod touch_input;

#[cfg(test)]
struct DemoApp;

#[cfg(test)]
impl runenui_core::UiApp for DemoApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One mounted owner, regardless of the number of projected semantic nodes.
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(1, 1, 1)
    }
    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(1, 1, 1)
    }

    type State = ();
    type Action = ();
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_state: &Self::State) -> impl runenui_core::View<Self::Action> {
        runenui_core::text("winit adapter test fixture")
    }

    fn update(
        _state: &mut Self::State,
        _action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}
