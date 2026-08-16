//! Deterministic public-only testing ergonomics for `RunenUI`.
//!
//! This crate is a downstream consumer of `runenui_core` and `runenui_runtime`.
//! It owns no runtime behavior and exposes no private mounted or semantic-owner
//! mutation seam. Every mutation delegates to ordinary public runtime ingress.

#![forbid(unsafe_code)]

mod harness;
mod semantic;
mod settle;
mod surface;

pub use harness::{
    HarnessSemanticQueryError, HarnessSurfaceCommandError, MissingPublication, TestHarness,
};
pub use semantic::{
    SemanticQuery, SemanticQueryMatches, SemanticTarget, SemanticTargetError,
    UniqueSemanticQueryError, query_semantics,
};
pub use settle::{SettleBudget, SettleOutcome, SettleReport};
pub use surface::{DEFAULT_TEST_SURFACE_SIZE, TestSurfaceConfig, TestSurfaceConfigError};
