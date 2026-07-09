//! File: domain/ui/ui_surface/src/lib.rs
//! Crate: ui_surface
//! Purpose: Surface semantic contracts for definitions and mounted instances.

pub mod capability;
pub mod definition;
pub mod diagnostics;
pub mod intent;
pub mod mount;
pub mod observation;
pub mod presentation;
pub mod ratification;
pub mod session;
pub mod validation;

pub use capability::*;
pub use definition::*;
pub use diagnostics::*;
pub use intent::*;
pub use mount::*;
pub use observation::*;
pub use presentation::*;
pub use ratification::*;
pub use session::*;
pub use validation::*;
