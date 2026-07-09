//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.

#![forbid(unsafe_code)]

pub mod prelude;

/// Marker type for the future typed UI description tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Element<Action> {
    _action: core::marker::PhantomData<fn() -> Action>,
}

impl<Action> Element<Action> {
    #[must_use]
    pub const fn marker() -> Self {
        Self {
            _action: core::marker::PhantomData,
        }
    }
}
