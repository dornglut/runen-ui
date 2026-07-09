//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/mod.rs
//! Purpose: Retained UI runtime entrypoint module wiring.

mod entry;
mod focus;
mod graph_canvas;
mod helpers;
mod popup;
mod scroll_metrics;

#[cfg(test)]
mod tests;

pub use entry::UiRuntime;
