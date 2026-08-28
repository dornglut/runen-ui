//! Reusable native translation and accessibility projection for winit hosts.
//!
//! This crate deliberately does not own a window, event loop, runtime pump,
//! redraw/publication policy, renderer, presentation lifecycle, or application
//! behavior. Hosts retain those authorities and use these adapters only to
//! translate native facts into existing RunenUI contracts.

pub mod accessibility;
pub mod device_identity;
pub mod keyboard_input;
pub mod mouse_input;
