//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.
//!
//! Incompatible configuration is absent from erased [`Element`] values:
//!
//! ```compile_fail
//! use runenui_core::{View, text};
//! let _ = text("label").disabled().into_element();
//! ```
//!
//! Identifier literal macros apply the same Unicode-aware grammar as dynamic
//! constructors and reject invalid literals during compilation:
//!
//! ```compile_fail
//! let _ = runenui_core::element_id!("\u{00A0}");
//! ```
//!
//! ```compile_fail
//! let _ = runenui_core::element_key!("name\u{2003}");
//! ```
//!
//! ```compile_fail
//! let _ = runenui_core::token_id!("name\u{0085}value");
//! ```
//!
//! Mounted widget state cannot be forged by consumers:
//!
//! ```compile_fail
//! use runenui_core::__runtime::MountedWidgetState;
//! let _ = MountedWidgetState {
//!     value: Box::new(()),
//! };
//! ```
//!
//! Routed protocol identities, time, sequences, origins, payloads, and callback
//! contexts expose no public field-level construction authority:
//!
//! ```compile_fail
//! use runenui_core::MountedNodeId;
//! let _ = MountedNodeId { slot: 1, generation: 1 };
//! ```
//!
//! ```compile_fail
//! use runenui_core::MonotonicInstant;
//! let _ = MonotonicInstant(1);
//! ```
//!
//! ```compile_fail
//! use runenui_core::WorkSequence;
//! let _ = WorkSequence(1);
//! ```
//!
//! ```compile_fail
//! use runenui_core::{CommandDerivation, CommandOrigin, EventSource};
//! let _ = CommandOrigin {
//!     source: EventSource::Automation,
//!     derivation: CommandDerivation::Delegated,
//! };
//! ```
//!
//! Delegated origins are issued only by the checked core event bridge:
//!
//! ```compile_fail
//! use runenui_core::{CommandOrigin, EventSource};
//! let _ = CommandOrigin::delegated(EventSource::Automation);
//! ```
//!
//! ```compile_fail
//! use runenui_core::{CommandOrigin, SemanticCommand, SemanticCommandEvent};
//! let _ = SemanticCommandEvent {
//!     command: SemanticCommand::Activate,
//!     origin: CommandOrigin::programmatic(),
//! };
//! ```
//!
//! ```compile_fail
//! use runenui_core::EventContext;
//! let _ = EventContext::<()> {};
//! ```
//!
//! Event contexts cannot be constructed or consumed by downstream code:
//!
//! ```compile_fail
//! use runenui_core::EventContext;
//! let _ = EventContext::<()>::new;
//! ```
//!
//! ```compile_fail
//! use runenui_core::EventContext;
//! let _ = EventContext::<()>::into_output;
//! ```
//!
//! Transient elements cannot execute mounted lifecycle or capabilities:
//!
//! ```compile_fail
//! use runenui_core::{View, text};
//! let element = text("temporary").into_element();
//! let _ = element.create_widget_state();
//! ```
//!
//! Public built-in view builders cannot bypass their validated conversion path:
//!
//! ```compile_fail
//! use runenui_core::{Element, text};
//! let _ = Element::<()>::new(text("Title"));
//! ```
//!
//! ```compile_fail
//! use runenui_core::{Element, button};
//! let _ = Element::<()>::new(button::<()>("Save"));
//! ```
//!
//! A structurally childless widget cannot use the container authoring path:
//!
//! ```compile_fail
//! use runenui_core::{Widget, children, container, text};
//! #[derive(Debug)]
//! struct Leaf;
//! impl Widget<()> for Leaf {
//!     type State = ();
//!     fn create_state(&self) {}
//! }
//! let _ = container(Leaf, children![text("child")]);
//! ```
//!
//! Gap remains a child-bearing container property, not a generic element setter:
//!
//! ```compile_fail
//! use runenui_core::{View, text};
//! let _ = text("leaf").into_element().gap(4_u16);
//! ```
//!
//! The removed physical-phase button callback has no compatibility alias:
//!
//! ```compile_fail
//! use runenui_core::button;
//! let _ = button("Save").on_press(());
//! ```
//!
//! Semantic activation creates fresh owned actions without a clone bound:
//!
//! ```
//! use runenui_core::button;
//! struct Action;
//! let _ = button("Save").on_activate(|| Action);
//! ```

#![forbid(unsafe_code)]

mod application;
mod builtins;
mod computed_style;
mod effects;
mod element;
mod event;
mod event_context;
mod identity;
mod layout;
pub mod prelude;
mod runtime_protocol;
mod style;
mod style_resolution;
mod style_tokens;
mod subscription;
mod value;
mod widget_context;
mod widget_erasure;
mod widget_mapping;
mod work;

include!("element_macros.rs");
include!("identity_macros.rs");
include!("token_macros.rs");

pub use application::{
    HostProtocol, NoHostCommand, NoHostProtocol, NoHostResponse, NoHostResponseKind, UiApp,
};
pub use builtins::{Button, Container, Text, button, column, container, row, text};
pub use computed_style::ComputedStyle;
pub use effects::{Effects, IntoEffects};
pub use element::{
    AuthoringDiagnostic, ChildLayout, ChildLayoutWidget, Element, View, Views, Widget,
    WidgetActivation, WidgetActivationOutput, WidgetDiagnostic, WidgetMeasure, WidgetPaintProof,
    WidgetSemanticProof, WidgetStateTypeId, WidgetTextKind, WidgetTypeId,
};
pub use event::{
    CommandDerivation, CommandOrigin, EventPhase, EventSource, SemanticCommand,
    SemanticCommandEvent, UiEvent, WidgetEventOutput,
};
pub use event_context::EventContext;
/// Unstable safe bridge from transient core elements to the mounted runtime.
///
/// This namespace is public only because core and runtime are separate Rust
/// crates. It is doc-hidden, outside the prelude, unsupported for application
/// use, semver-exempt before 1.0, and may change without compatibility support.
#[doc(hidden)]
pub mod __runtime {
    pub use crate::effects::{Effect, HostRequestEffect, MountedEffect};
    pub use crate::event_context::{EventContextOutput, RoutedEventOutput};
    pub use crate::runtime_protocol::RuntimeNamespace;
    pub use crate::subscription::{ErasedSendSubscriptionSource, Subscription, SubscriptionSource};
    pub use crate::widget_erasure::{
        ElementParts, ElementRuntimeParts, MountedWidget, MountedWidgetState, WidgetBridgeError,
    };
    pub use crate::work::{LocalTaskEffect, SendFuture, SendOutput, SendTaskEffect};
}
#[doc(hidden)]
pub use identity::is_valid_identifier_literal;
pub use identity::{ElementId, ElementKey, IdentifierError, IntoElementId, IntoElementKey};
pub use layout::{Axis, LayoutStyle};
pub use runtime_protocol::{
    MonotonicInstant, MonotonicTimeError, MountedNodeId, SemanticNodeId, WorkSequence,
};
pub use style::{
    Color, ColorToken, ColorValue, EdgeInsets, Radius, RadiusToken, RadiusValue, SpacingToken,
    SpacingValue, StyleIntent, TokenId,
};
pub use style_resolution::{
    StyleFieldProvenance, StyleProvenance, StyleResolution, UnresolvedStyleToken,
    resolve_literal_style, resolve_style,
};
pub use style_tokens::{DuplicateTokenDefinition, StyleTokens, TokenFamily};
pub use subscription::{
    LocalSubscriptionSource, SendSubscriptionSink, SendSubscriptionSinkError,
    SendSubscriptionSource, SendSubscriptionStartOutcome, SubscriptionSet,
};
pub use value::{LogicalLength, LogicalLengthError};
pub use widget_context::{
    WidgetActivationContext, WidgetInvalidation, WidgetMountContext, WidgetUnmountContext,
    WidgetUnmountReason, WidgetUpdateContext,
};
pub use work::{SendTaskStartFailure, TimerEffect, WorkFamily, WorkKey, WorkKeyError};
