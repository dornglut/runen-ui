//! Opt-in fluent native application composition for independent `RunenUI` programs.
//!
//! This package does not own UI mounted state, actions, scheduling or publication;
//! it composes the existing public `RunenUI` runtime, native adapter and renderer.
//!
//! The basic native path intentionally supports only `NoHostProtocol` and one
//! native window. Applications requiring host effects or external event-loop
//! ownership must continue using the public `AppRuntime` host contracts.

#![forbid(unsafe_code)]

mod native;

use core::fmt;

use runenui_core::{NoHostProtocol, StyleEnvironment, UiApp};
use runenui_render_wgpu::{
    ResourcePayload, ResourceProvider, ResourceProviderError, ResourceProviderErrorKind,
    ResourceRequest,
};
use runenui_runtime::{FontSourcePolicy, RuntimeConfig};

/// An unmounted, platform-neutral application configuration.
#[derive(Default)]
pub struct App {
    title: Option<String>,
}

impl App {
    /// Creates an unmounted, windowless application description.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the native single-window profile and its display title.
    ///
    /// The title is not a logical `RunenUI` surface key or an operating-system ID.
    #[must_use]
    pub fn window(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Installs a model that implements `RunenUI`'s canonical `UiApp` trait itself.
    ///
    /// `UiApp<State = Self>` permits the model's type to be inferred from the
    /// value. No second application-state/queue/runtime authority is created.
    #[must_use]
    pub fn ui<A: UiApp<State = A, HostProtocol = NoHostProtocol>>(
        self,
        model: A,
    ) -> ConfiguredApp<A> {
        self.ui_with_state::<A>(model)
    }

    /// Installs a separately typed UI application and its explicitly typed state.
    ///
    /// Useful when the canonical `UiApp` implementor is intentionally distinct
    /// from the application state; no attempt to reverse-lookup the marker is made.
    #[must_use]
    pub fn ui_with_state<A: UiApp<HostProtocol = NoHostProtocol>>(
        self,
        state: A::State,
    ) -> ConfiguredApp<A> {
        ConfiguredApp {
            title: self.title,
            state,
            styles: StyleEnvironment::default(),
            resources: Box::new(NoExternalResources),
            runtime_config: RuntimeConfig::default()
                .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
        }
    }
}

/// One unmounted typed UI instance and optional native window intent.
pub struct ConfiguredApp<A: UiApp<HostProtocol = NoHostProtocol>> {
    title: Option<String>,
    state: A::State,
    styles: StyleEnvironment,
    resources: Box<dyn ResourceProvider>,
    runtime_config: RuntimeConfig,
}

impl<A: UiApp<HostProtocol = NoHostProtocol>> ConfiguredApp<A> {
    /// Supplies application-owned styling for native surface publication.
    #[must_use]
    pub fn styles(mut self, styles: StyleEnvironment) -> Self {
        self.styles = styles;
        self
    }

    /// Supplies application-owned resource lookup for image publications.
    #[must_use]
    pub fn resources(mut self, resources: impl ResourceProvider + 'static) -> Self {
        self.resources = Box::new(resources);
        self
    }

    /// Specifies the runtime's exact accepted configuration before mount.
    #[must_use]
    pub const fn runtime_config(mut self, config: RuntimeConfig) -> Self {
        self.runtime_config = config;
        self
    }

    /// Runs a native single-window application to normal closure or typed error.
    ///
    /// # Errors
    ///
    /// Rejects an absent/empty native window declaration, a native event-loop
    /// acquisition/runtime failure, or a fatal host/renderer/shutdown outcome.
    /// Headless/embedded execution continues through the public `AppRuntime`
    /// integration path; this method never silently starts a fake headless loop.
    pub fn run(self) -> Result<(), NativeAppError> {
        let title = self.title.ok_or(NativeAppError::WindowRequired)?;
        if title.trim().is_empty() {
            return Err(NativeAppError::WindowTitleEmpty);
        }
        native::run_native::<A>(
            title,
            self.state,
            self.styles,
            self.resources,
            self.runtime_config,
        )
    }
}

/// Distinguishes native configuration, event-loop and runtime/renderer failures.
#[derive(Debug)]
#[non_exhaustive]
pub enum NativeAppError {
    /// Native execution was requested without selecting a native window.
    WindowRequired,
    /// Native window title was empty.
    WindowTitleEmpty,
    /// Platform event-loop initialization/advancement rejected the operation.
    EventLoop(String),
    /// Native host, renderer, runtime or required shutdown failed.
    Fatal(String),
    /// A primary failure occurred and the required cleanup also failed.
    PrimaryAndCleanup {
        /// The original failure that initiated native termination.
        primary: Box<Self>,
        /// The separate cleanup failure; its observation must not be hidden.
        cleanup: String,
    },
}

impl fmt::Display for NativeAppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowRequired => {
                write!(f, "native application needs a .window(...) declaration")
            }
            Self::WindowTitleEmpty => write!(f, "native application window title must be nonempty"),
            Self::EventLoop(detail) => write!(f, "native event-loop failure: {detail}"),
            Self::Fatal(detail) => write!(f, "native application failure: {detail}"),
            Self::PrimaryAndCleanup { primary, cleanup } => {
                write!(f, "{primary}; cleanup also failed: {cleanup}")
            }
        }
    }
}

impl std::error::Error for NativeAppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PrimaryAndCleanup { primary, .. } => Some(primary.as_ref()),
            _ => None,
        }
    }
}

impl NativeAppError {
    fn combine_exit(primary: Option<Self>, cleanup: Option<String>) -> Result<(), Self> {
        match (primary, cleanup) {
            (None, None) => Ok(()),
            (Some(primary), None) => Err(primary),
            (None, Some(cleanup)) => Err(Self::Fatal(format!(
                "native shutdown observation unavailable: {cleanup}"
            ))),
            (Some(primary), Some(cleanup)) => Err(Self::PrimaryAndCleanup {
                primary: Box::new(primary),
                cleanup,
            }),
        }
    }
}

struct NoExternalResources;

impl ResourceProvider for NoExternalResources {
    fn load(
        &self,
        _resource: &runenui_core::ResourceRef,
        _request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Missing,
            "no external resources registered for this application",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runenui_core::{View, text};

    struct Minimal {
        value: i32,
    }

    impl UiApp for Minimal {
        type State = Self;
        type Action = ();
        type HostProtocol = NoHostProtocol;

        fn root(state: &Self::State) -> impl View<Self::Action> {
            text(format!("Value {}", state.value))
        }

        fn update(
            _: &mut Self::State,
            (): Self::Action,
        ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        }
    }

    #[test]
    fn native_requires_an_explicit_nonempty_window() {
        let no_window = App::new().ui(Minimal { value: 1 }).run();
        assert!(matches!(no_window, Err(NativeAppError::WindowRequired)));
        let empty = App::new().window("").ui(Minimal { value: 1 }).run();
        assert!(matches!(empty, Err(NativeAppError::WindowTitleEmpty)));
    }

    #[test]
    fn native_exit_preserves_primary_and_cleanup_failures_separately() {
        let combined = NativeAppError::combine_exit(
            Some(NativeAppError::EventLoop("event loop failed".to_owned())),
            Some("shutdown observation failed".to_owned()),
        );
        let Err(NativeAppError::PrimaryAndCleanup { primary, cleanup }) = combined else {
            panic!("both failures must be returned as separate typed facts");
        };
        assert!(matches!(*primary, NativeAppError::EventLoop(_)));
        assert_eq!(cleanup, "shutdown observation failed");
    }

    #[test]
    fn typed_fluent_application_is_not_mounted_during_configuration() {
        let model = App::new().window("Counter").ui(Minimal { value: 42 });
        assert_eq!(model.title.as_deref(), Some("Counter"));
        assert_eq!(model.state.value, 42);
    }
}
