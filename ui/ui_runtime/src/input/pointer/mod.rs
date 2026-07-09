//! File: domain/ui/ui_runtime/src/input/pointer/mod.rs
//! Purpose: Pointer input dispatch module wiring for ui_runtime.

mod dispatch;
mod graph_canvas;
mod helpers;
mod hover;
mod middle_pan;
mod numeric;
mod popup;
mod press;
mod scroll;
mod scrollbar;

pub use dispatch::dispatch_pointer_event;
