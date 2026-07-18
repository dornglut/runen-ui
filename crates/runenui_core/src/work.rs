//! Durable authored work identity and host-neutral work descriptions.

use std::{any::Any, error::Error, fmt, future::Future, hash::Hash, pin::Pin, time::Duration};

use crate::identity::{IdentifierError, validate_identifier};

/// Validation failure for an authored [`WorkKey`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkKeyError {
    Empty,
    WhitespaceOnly,
    SurroundingWhitespace,
    ControlCharacter,
}

impl From<IdentifierError> for WorkKeyError {
    fn from(value: IdentifierError) -> Self {
        match value {
            IdentifierError::Empty => Self::Empty,
            IdentifierError::WhitespaceOnly => Self::WhitespaceOnly,
            IdentifierError::SurroundingWhitespace => Self::SurroundingWhitespace,
            IdentifierError::ControlCharacter => Self::ControlCharacter,
        }
    }
}

impl fmt::Display for WorkKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        IdentifierError::from(*self).fmt(formatter)
    }
}

impl From<WorkKeyError> for IdentifierError {
    fn from(value: WorkKeyError) -> Self {
        match value {
            WorkKeyError::Empty => Self::Empty,
            WorkKeyError::WhitespaceOnly => Self::WhitespaceOnly,
            WorkKeyError::SurroundingWhitespace => Self::SurroundingWhitespace,
            WorkKeyError::ControlCharacter => Self::ControlCharacter,
        }
    }
}

impl Error for WorkKeyError {}

/// Validated durable owner-local intent for cancellation and replacement.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkKey(Box<str>);

impl WorkKey {
    /// Validates and owns a work key.
    ///
    /// # Errors
    ///
    /// Returns a dedicated error for invalid authored identifier text.
    pub fn new(value: impl Into<String>) -> Result<Self, WorkKeyError> {
        let value = value.into();
        validate_identifier(&value).map_err(WorkKeyError::from)?;
        Ok(Self(value.into_boxed_str()))
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for WorkKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("WorkKey")
            .field(&self.as_str())
            .finish()
    }
}

impl fmt::Display for WorkKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for WorkKey {
    type Err = WorkKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Public work family used only to describe same-owner keyed cancellation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WorkFamily {
    LocalTask,
    SendTask,
    Timer,
    HostRequest,
}

/// One-shot or repeating monotonic timer description.
#[must_use]
pub struct TimerEffect<Action> {
    pub(crate) key: Option<WorkKey>,
    pub(crate) delay: Duration,
    pub(crate) interval: Option<Duration>,
    pub(crate) action: Box<dyn FnMut() -> Action>,
}

#[doc(hidden)]
pub type TimerEffectParts<Action> = (
    Option<WorkKey>,
    Duration,
    Option<Duration>,
    Box<dyn FnMut() -> Action>,
);

impl<Action> TimerEffect<Action> {
    pub fn once(delay: Duration, action: impl FnOnce() -> Action + 'static) -> Self {
        let mut action = Some(action);
        Self {
            key: None,
            delay,
            interval: None,
            action: Box::new(move || {
                action
                    .take()
                    .map_or_else(|| unreachable!(), |action| action())
            }),
        }
    }

    /// Creates a repeating timer. Zero intervals are rejected by the runtime.
    pub fn repeating(interval: Duration, action: impl FnMut() -> Action + 'static) -> Self {
        Self {
            key: None,
            delay: interval,
            interval: Some(interval),
            action: Box::new(action),
        }
    }

    pub fn keyed(mut self, key: WorkKey) -> Self {
        self.key = Some(key);
        self
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_key(&self) -> Option<&WorkKey> {
        self.key.as_ref()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_into_parts(self) -> TimerEffectParts<Action> {
        (self.key, self.delay, self.interval, self.action)
    }
}

#[doc(hidden)]
pub type LocalFuture<Action> = Pin<Box<dyn Future<Output = Option<Action>>>>;
#[doc(hidden)]
pub type SendOutput = Box<dyn Any + Send>;
#[doc(hidden)]
pub type SendFuture = Pin<Box<dyn Future<Output = SendOutput> + Send>>;

/// Runtime-neutral local task description.
#[doc(hidden)]
pub struct LocalTaskEffect<Action> {
    pub key: Option<WorkKey>,
    pub future: LocalFuture<Action>,
}

/// Runtime-neutral send-capable task and UI-thread mapper description.
#[doc(hidden)]
pub struct SendTaskEffect<Action> {
    pub key: Option<WorkKey>,
    pub future: SendFuture,
    pub map: Box<dyn FnOnce(SendOutput) -> Action>,
    pub start_failure: Option<Box<dyn FnOnce(SendTaskStartFailure) -> Action>>,
}

/// Structured recoverable send-executor start refusal.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendTaskStartFailure {
    Unavailable,
    Full,
    Closed,
    Rejected,
}

#[cfg(test)]
mod tests {
    use super::{WorkKey, WorkKeyError};

    #[test]
    fn work_key_uses_canonical_unicode_identifier_grammar() {
        assert_eq!(
            WorkKey::new("timer.å").map(|key| key.to_string()),
            Ok("timer.å".into())
        );
        assert_eq!(WorkKey::new(""), Err(WorkKeyError::Empty));
        assert_eq!(WorkKey::new("  "), Err(WorkKeyError::WhitespaceOnly));
        assert_eq!(
            WorkKey::new(" timer"),
            Err(WorkKeyError::SurroundingWhitespace)
        );
        assert_eq!(
            WorkKey::new("bad\u{0085}"),
            Err(WorkKeyError::ControlCharacter)
        );
    }
}
