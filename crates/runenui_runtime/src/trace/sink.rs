use core::fmt;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};

use super::TraceSinkDeliveryOutcome;

/// One complete versioned JSONL record line delivered by the subordinate sink.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceJsonlLine(String);

impl TraceJsonlLine {
    pub(super) fn new(value: String) -> Self {
        Self(value)
    }

    /// Returns the encoded JSON object without a trailing newline.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the owned encoded JSON object.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Nonblocking receive outcome for the bounded subordinate trace sink.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceSinkReceiveError {
    Empty,
    Closed,
}

impl fmt::Display for TraceSinkReceiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("trace sink is currently empty"),
            Self::Closed => formatter.write_str("trace sink is closed"),
        }
    }
}

impl std::error::Error for TraceSinkReceiveError {}

/// Receiving end of the configured bounded trace sink.
///
/// The runtime never invokes consumer code. It only performs nonblocking sends
/// into the bounded queue; consumers drain this handle on their own schedule.
pub struct TraceSinkReceiver {
    receiver: Receiver<TraceJsonlLine>,
}

impl TraceSinkReceiver {
    /// Attempts to receive one JSONL record without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`TraceSinkReceiveError::Empty`] when no record is currently
    /// buffered and [`TraceSinkReceiveError::Closed`] after runtime delivery
    /// authority has ended and the buffer is empty.
    pub fn try_recv(&self) -> Result<TraceJsonlLine, TraceSinkReceiveError> {
        self.receiver.try_recv().map_err(|error| match error {
            TryRecvError::Empty => TraceSinkReceiveError::Empty,
            TryRecvError::Disconnected => TraceSinkReceiveError::Closed,
        })
    }
}

impl fmt::Debug for TraceSinkReceiver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TraceSinkReceiver { .. }")
    }
}

pub(super) struct TraceSink {
    sender: Option<SyncSender<TraceJsonlLine>>,
    receiver: Option<TraceSinkReceiver>,
}

impl TraceSink {
    pub(super) fn bounded(capacity: usize) -> Self {
        let (sender, receiver) = sync_channel(capacity);
        Self {
            sender: Some(sender),
            receiver: Some(TraceSinkReceiver { receiver }),
        }
    }

    pub(super) fn take_receiver(&mut self) -> Option<TraceSinkReceiver> {
        self.receiver.take()
    }

    pub(super) fn try_deliver(&mut self, line: TraceJsonlLine) -> TraceSinkDeliveryOutcome {
        let Some(sender) = self.sender.as_ref() else {
            return TraceSinkDeliveryOutcome::Closed;
        };
        match sender.try_send(line) {
            Ok(()) => TraceSinkDeliveryOutcome::Delivered,
            Err(TrySendError::Full(_)) => TraceSinkDeliveryOutcome::Full,
            Err(TrySendError::Disconnected(_)) => {
                self.sender = None;
                TraceSinkDeliveryOutcome::Closed
            }
        }
    }

    pub(super) fn close(&mut self) {
        self.sender = None;
    }

    pub(super) const fn is_open(&self) -> bool {
        self.sender.is_some()
    }
}

impl fmt::Debug for TraceSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraceSink")
            .field("open", &self.sender.is_some())
            .field("receiver_available", &self.receiver.is_some())
            .finish()
    }
}
