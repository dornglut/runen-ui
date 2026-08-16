//! Deterministic public-only testing ergonomics for `RunenUI`.
//!
//! This crate is a downstream consumer of `runenui_core` and `runenui_runtime`.
//! It owns no runtime behavior and exposes no private mounted or semantic-owner
//! mutation seam. Every mutation delegates to ordinary public runtime ingress.
//!
//! Semantic test targets cannot be created from a bare semantic identity. They
//! must be scoped by membership in an exact committed snapshot:
//!
//! ```compile_fail
//! use runenui_testing::SemanticTarget;
//! let _ = SemanticTarget::from_node_id;
//! ```
//!
//! Snapshot-scoped semantic targets expose no semantic-to-mounted routing shortcut:
//!
//! ```compile_fail
//! use runenui_testing::SemanticTarget;
//! let _ = SemanticTarget::mounted_node_id;
//! ```
//!
//! Settle requests have no unbounded convenience constructor:
//!
//! ```compile_fail
//! use runenui_testing::SettleBudget;
//! let _ = SettleBudget::unbounded;
//! ```

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
