//! File: domain/ui/ui_testing/src/lib.rs
//! Crate: ui_testing

mod adaptive_composition_fixture;
mod architecture_fixture;
mod assertions;
mod composition_fixture;
mod errors;
mod headless_fixture;
mod program_fixture;
mod reproducibility;

pub use adaptive_composition_fixture::{
    AdaptiveCompositionFixtureRun, adaptive_composition_conformance_fixtures,
    run_adaptive_composition_conformance_fixtures,
};
pub use architecture_fixture::UiArchitectureFixture;
pub use assertions::{DiagnosticAssertion, DiagnosticExpectation, SourceMapAssertion};
pub use composition_fixture::{
    composition_conformance_fixtures, run_composition_conformance_fixtures,
};
pub use errors::UiTestingError;
pub use headless_fixture::{HeadlessFixture, HeadlessFixtureRun};
pub use reproducibility::ReproducibilityAssertion;

#[cfg(test)]
mod tests;
