//! Runtime-neutral UI definition persistence, migration dry-run, diff, and activation contracts.

mod diagnostics;
mod diff;
mod document;
mod migration;
mod request;
mod validation;

#[cfg(test)]
mod tests;

pub use diagnostics::*;
pub use diff::*;
pub use document::*;
pub use migration::*;
pub use request::*;
pub use validation::{UiPersistenceActivationValidationReport, validate_persistence_activation};
