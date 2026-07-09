//! UI-owned app integration proof bridge.
//!
//! This crate proves a narrow ECS-backed UI/app loop without exposing final
//! `engine::App` ergonomics. It must not become a generic app framework.

pub mod action;
pub mod bridge;
pub mod host;
pub mod ids;
pub mod proof;
pub mod report;
pub mod screen;
pub mod source;

pub use action::*;
pub use bridge::*;
pub use host::*;
pub use ids::*;
pub use proof::*;
pub use report::*;
pub use screen::*;
pub use source::*;
