//! Reusable native translation and accessibility projection for winit hosts.
//!
//! This crate deliberately does not own a window, event loop, runtime pump,
//! redraw/publication policy, renderer, presentation lifecycle, or application
//! behavior. Hosts retain those authorities and use these adapters only to
//! translate native facts into existing `RunenUI` contracts.

pub mod accessibility;
pub mod device_identity;
pub mod keyboard_input;
pub mod mouse_input;

#[cfg(test)]
struct DemoApp;

#[cfg(test)]
impl runenui_core::UiApp for DemoApp {
    type State = ();
    type Action = ();
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_state: &Self::State) -> impl runenui_core::View<Self::Action> {
        runenui_core::text("winit adapter test fixture")
    }

    fn update(
        _state: &mut Self::State,
        _action: Self::Action,
    ) -> impl runenui_core::IntoEffects<Self::Action, Self::HostProtocol> {
    }
}
