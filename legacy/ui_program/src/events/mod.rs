//! File: domain/ui/ui_program/src/events/mod.rs
//! Crate: ui_program

pub mod packet;
pub mod payload;
pub mod phase;
pub mod route;

pub use packet::{UiEventPacket, UiEventSourceControlId};
pub use payload::UiEventPayload;
pub use phase::UiEventPhase;
pub use route::{RouteCapability, RouteContractError, RouteId, RouteSchemaVersion};
