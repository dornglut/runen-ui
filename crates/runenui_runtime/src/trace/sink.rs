use core::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{Receiver, SendError, Sender, TryRecvError, channel},
};

use runenui_core::__runtime::RuntimeNamespace;

use super::{TraceRecord, TraceSinkDeliveryOutcome, encode_record_json};

/// One complete versioned JSONL record line delivered by the subordinate sink.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceJsonlLine(String);

impl TraceJsonlLine {
    pub(super) const fn new(value: String) -> Self {
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
/// Runtime mutation only hands off immutable canonical records. JSON encoding
/// happens here when the consumer drains the receiver, so sink serialization
/// never runs inside a mutable runtime transaction.
pub struct TraceSinkReceiver {
    runtime: RuntimeNamespace,
    receiver: Receiver<Arc<TraceRecord>>,
    queued: Arc<AtomicUsize>,
    receiver_closed: Arc<AtomicBool>,
}

impl TraceSinkReceiver {
    /// Attempts to receive and encode one JSONL record without blocking.
    ///
    /// # Errors
    ///
    /// Returns [`TraceSinkReceiveError::Empty`] when no record is currently
    /// buffered and [`TraceSinkReceiveError::Closed`] after runtime delivery
    /// authority has ended and the buffer is empty.
    pub fn try_recv(&self) -> Result<TraceJsonlLine, TraceSinkReceiveError> {
        match self.receiver.try_recv() {
            Ok(record) => {
                let previous = self.queued.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous != 0);
                Ok(TraceJsonlLine::new(encode_record_json(
                    &self.runtime,
                    record.as_ref(),
                )))
            }
            Err(TryRecvError::Empty) => Err(TraceSinkReceiveError::Empty),
            Err(TryRecvError::Disconnected) => Err(TraceSinkReceiveError::Closed),
        }
    }
}

impl Drop for TraceSinkReceiver {
    fn drop(&mut self) {
        self.receiver_closed.store(true, Ordering::Release);
    }
}

impl fmt::Debug for TraceSinkReceiver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TraceSinkReceiver { .. }")
    }
}

pub(super) struct TraceSinkPermit {
    sender: Sender<Arc<TraceRecord>>,
    queued: Arc<AtomicUsize>,
}

impl TraceSinkPermit {
    pub(super) fn deliver(self, record: Arc<TraceRecord>) -> Result<(), Arc<TraceRecord>> {
        self.sender.send(record).map_err(|SendError(record)| {
            let previous = self.queued.fetch_sub(1, Ordering::AcqRel);
            debug_assert!(previous != 0);
            record
        })
    }
}

pub(super) struct TraceSink {
    sender: Option<Sender<Arc<TraceRecord>>>,
    receiver: Option<TraceSinkReceiver>,
    queued: Arc<AtomicUsize>,
    receiver_closed: Arc<AtomicBool>,
    capacity: usize,
}

impl TraceSink {
    pub(super) fn bounded(capacity: usize, runtime: RuntimeNamespace) -> Self {
        let (sender, receiver) = channel();
        let queued = Arc::new(AtomicUsize::new(0));
        let receiver_closed = Arc::new(AtomicBool::new(false));
        Self {
            sender: Some(sender),
            receiver: Some(TraceSinkReceiver {
                runtime,
                receiver,
                queued: Arc::clone(&queued),
                receiver_closed: Arc::clone(&receiver_closed),
            }),
            queued,
            receiver_closed,
            capacity,
        }
    }

    pub(super) const fn take_receiver(&mut self) -> Option<TraceSinkReceiver> {
        self.receiver.take()
    }

    pub(super) fn reserve_delivery(&mut self) -> Result<TraceSinkPermit, TraceSinkDeliveryOutcome> {
        if self.sender.is_none() || self.receiver_closed.load(Ordering::Acquire) {
            self.sender = None;
            return Err(TraceSinkDeliveryOutcome::Closed);
        }

        let mut queued = self.queued.load(Ordering::Acquire);
        loop {
            if queued >= self.capacity {
                return Err(TraceSinkDeliveryOutcome::Full);
            }
            match self.queued.compare_exchange_weak(
                queued,
                queued + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(observed) => queued = observed,
            }
        }

        let sender = self
            .sender
            .as_ref()
            .unwrap_or_else(|| unreachable!("open sink retains its sender"))
            .clone();
        Ok(TraceSinkPermit {
            sender,
            queued: Arc::clone(&self.queued),
        })
    }

    pub(super) fn close(&mut self) {
        self.sender = None;
    }

    pub(super) fn is_open(&self) -> bool {
        self.sender.is_some() && !self.receiver_closed.load(Ordering::Acquire)
    }

    pub(super) fn state_eq(&self, other: &Self) -> bool {
        self.capacity == other.capacity
            && self.is_open() == other.is_open()
            && self.receiver.is_some() == other.receiver.is_some()
            && self.queued.load(Ordering::Acquire) == other.queued.load(Ordering::Acquire)
            && self.receiver_closed.load(Ordering::Acquire)
                == other.receiver_closed.load(Ordering::Acquire)
    }
}

impl fmt::Debug for TraceSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraceSink")
            .field("open", &self.is_open())
            .field("receiver_available", &self.receiver.is_some())
            .field("queued", &self.queued.load(Ordering::Acquire))
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}
