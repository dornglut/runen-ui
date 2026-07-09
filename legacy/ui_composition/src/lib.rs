//! App-neutral UI composition definitions and ratified structural state.

#![forbid(unsafe_code)]

pub mod content;
pub mod definition;
pub mod diagnostic;
pub mod fixture;
pub mod history;
pub mod identity;
pub mod persistence;
pub mod promotion;
pub mod state;
pub mod transaction;
pub mod validation;

pub use content::*;
pub use definition::*;
pub use diagnostic::*;
pub use fixture::*;
pub use history::*;
pub use identity::*;
pub use persistence::prelude::*;
pub use promotion::*;
pub use state::*;
pub use transaction::*;
