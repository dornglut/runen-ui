//! Bounded cross-thread completion ingress and opaque send-task jobs.

#![allow(clippy::redundant_pub_crate)]

use core::fmt;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use runenui_core::{
    __runtime::{SendFuture, SendOutput},
    SendSubscriptionSinkError,
};

use crate::{TraceSequence, TraceWorkIdentity, wake::WakeHandle, work::WorkGeneration};

pub(crate) struct CompletionIngress {
    shared: Arc<Mutex<IngressState>>,
    wake: WakeHandle,
}

struct IngressState {
    capacity: usize,
    closed: bool,
    waiting: VecDeque<CompletionPayload>,
    host_responses: HashMap<WorkGeneration, HostResponseState>,
    send_tasks: HashMap<WorkGeneration, ProducerState>,
    subscriptions: HashMap<WorkGeneration, ProducerState>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProducerState {
    Starting,
    Running,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostResponseState {
    Open,
    DetachedQueued,
    DirectClaimed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostResponseRevocation {
    RemovedOpen,
    RemovedDetached,
    RemovedDirectClaimed,
    Missing,
}

pub(crate) struct CompletionPayload {
    pub(crate) generation: WorkGeneration,
    pub(crate) output: SendOutput,
    pub(crate) kind: CompletionKind,
    pub(crate) trace_identity: TraceWorkIdentity,
    pub(crate) causal_parent: Option<TraceSequence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CompletionKind {
    SendTask,
    Subscription,
    HostResponse,
}

impl CompletionIngress {
    pub(crate) fn new(capacity: usize, wake: WakeHandle) -> Self {
        Self {
            shared: Arc::new(Mutex::new(IngressState {
                capacity,
                closed: false,
                waiting: VecDeque::new(),
                host_responses: HashMap::new(),
                send_tasks: HashMap::new(),
                subscriptions: HashMap::new(),
            })),
            wake,
        }
    }

    pub(crate) fn sender(&self) -> CompletionSender {
        CompletionSender {
            shared: Arc::clone(&self.shared),
            wake: self.wake.clone(),
        }
    }

    pub(crate) fn pop(&self) -> Option<CompletionPayload> {
        lock(&self.shared).waiting.pop_front()
    }

    pub(crate) fn front(
        &self,
    ) -> Option<(
        WorkGeneration,
        CompletionKind,
        TraceWorkIdentity,
        Option<TraceSequence>,
    )> {
        lock(&self.shared).waiting.front().map(|payload| {
            (
                payload.generation,
                payload.kind,
                payload.trace_identity.clone(),
                payload.causal_parent,
            )
        })
    }

    pub(crate) fn len(&self) -> usize {
        lock(&self.shared).waiting.len()
    }

    pub(crate) fn close(&self) {
        let mut state = lock(&self.shared);
        state.closed = true;
        state.waiting.clear();
        state.host_responses.clear();
        state.send_tasks.clear();
        state.subscriptions.clear();
    }

    pub(crate) fn register_host_response(&self, generation: WorkGeneration) {
        lock(&self.shared)
            .host_responses
            .insert(generation, HostResponseState::Open);
    }

    pub(crate) fn host_response_is_open(&self, generation: WorkGeneration) -> bool {
        lock(&self.shared).host_responses.get(&generation) == Some(&HostResponseState::Open)
    }

    pub(crate) fn release_host_response(&self, generation: WorkGeneration) {
        lock(&self.shared).host_responses.remove(&generation);
    }

    pub(crate) fn claim_direct_host_response(&self, generation: WorkGeneration) -> bool {
        let mut state = lock(&self.shared);
        let Some(response_state) = state.host_responses.get_mut(&generation) else {
            return false;
        };
        if *response_state != HostResponseState::Open {
            return false;
        }
        *response_state = HostResponseState::DirectClaimed;
        drop(state);
        true
    }

    pub(crate) fn register_send_task_starting(&self, generation: WorkGeneration) {
        lock(&self.shared)
            .send_tasks
            .insert(generation, ProducerState::Starting);
    }

    pub(crate) fn promote_send_task_running(&self, generation: WorkGeneration) -> bool {
        promote(&mut lock(&self.shared).send_tasks, generation)
    }

    pub(crate) fn register_subscription_starting(&self, generation: WorkGeneration) {
        lock(&self.shared)
            .subscriptions
            .insert(generation, ProducerState::Starting);
    }

    pub(crate) fn promote_subscription_running(&self, generation: WorkGeneration) -> bool {
        promote(&mut lock(&self.shared).subscriptions, generation)
    }

    pub(crate) fn release_subscription(&self, generation: WorkGeneration) {
        let mut state = lock(&self.shared);
        state.subscriptions.remove(&generation);
        state.waiting.retain(|payload| {
            payload.kind != CompletionKind::Subscription || payload.generation != generation
        });
    }

    pub(crate) fn revoke_generation(&self, generation: WorkGeneration) -> HostResponseRevocation {
        let mut state = lock(&self.shared);
        state.send_tasks.remove(&generation);
        state.subscriptions.remove(&generation);
        let host_response = state.host_responses.remove(&generation);
        state
            .waiting
            .retain(|payload| payload.generation != generation);
        drop(state);
        match host_response {
            Some(HostResponseState::Open) => HostResponseRevocation::RemovedOpen,
            Some(HostResponseState::DetachedQueued) => HostResponseRevocation::RemovedDetached,
            Some(HostResponseState::DirectClaimed) => HostResponseRevocation::RemovedDirectClaimed,
            None => HostResponseRevocation::Missing,
        }
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn host_response_slot_count_for_test(&self) -> usize {
        lock(&self.shared).host_responses.len()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn send_task_slot_count_for_test(&self) -> usize {
        lock(&self.shared).send_tasks.len()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn subscription_slot_count_for_test(&self) -> usize {
        lock(&self.shared).subscriptions.len()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn payload_count_for_test(&self) -> usize {
        lock(&self.shared).waiting.len()
    }
}

#[derive(Clone)]
pub(crate) struct CompletionSender {
    shared: Arc<Mutex<IngressState>>,
    wake: WakeHandle,
}

impl CompletionSender {
    fn submit_send_task(
        &self,
        payload: CompletionPayload,
    ) -> Result<(), CompletionSubmissionError> {
        let mut state = lock(&self.shared);
        if state.closed {
            return Err(CompletionSubmissionError::Closed(payload));
        }
        if !matches!(
            state.send_tasks.get(&payload.generation),
            Some(ProducerState::Starting | ProducerState::Running)
        ) {
            return Err(CompletionSubmissionError::Stale(payload));
        }
        if state.waiting.len() >= state.capacity {
            return Err(CompletionSubmissionError::Full(payload));
        }
        state.waiting.push_back(payload);
        drop(state);
        let _ = self.wake.request();
        Ok(())
    }

    fn submit_host_response(
        &self,
        payload: CompletionPayload,
    ) -> Result<(), CompletionSubmissionError> {
        let mut state = lock(&self.shared);
        if state.closed {
            return Err(CompletionSubmissionError::Closed(payload));
        }
        if state.host_responses.get(&payload.generation) != Some(&HostResponseState::Open) {
            return Err(CompletionSubmissionError::Stale(payload));
        }
        if state.waiting.len() >= state.capacity {
            return Err(CompletionSubmissionError::Full(payload));
        }
        state
            .host_responses
            .insert(payload.generation, HostResponseState::DetachedQueued);
        state.waiting.push_back(payload);
        drop(state);
        let _ = self.wake.request();
        Ok(())
    }

    pub(crate) fn submit_subscription(
        &self,
        generation: WorkGeneration,
        output: SendOutput,
        trace_identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<(), SendSubscriptionSinkError<SendOutput>> {
        let payload = CompletionPayload {
            generation,
            output,
            kind: CompletionKind::Subscription,
            trace_identity,
            causal_parent,
        };
        let mut state = lock(&self.shared);
        if state.closed {
            return Err(SendSubscriptionSinkError::Closed(payload.output));
        }
        match state.subscriptions.get(&generation) {
            Some(ProducerState::Starting) => {
                return Err(SendSubscriptionSinkError::NotStarted(payload.output));
            }
            Some(ProducerState::Running) => {}
            None => return Err(SendSubscriptionSinkError::Stale(payload.output)),
        }
        if state.waiting.len() >= state.capacity {
            return Err(SendSubscriptionSinkError::Full(payload.output));
        }
        state.waiting.push_back(payload);
        drop(state);
        let _ = self.wake.request();
        Ok(())
    }
}

fn promote(
    states: &mut HashMap<WorkGeneration, ProducerState>,
    generation: WorkGeneration,
) -> bool {
    let Some(state) = states.get_mut(&generation) else {
        return false;
    };
    if *state != ProducerState::Starting {
        return false;
    }
    *state = ProducerState::Running;
    true
}

fn lock(shared: &Mutex<IngressState>) -> std::sync::MutexGuard<'_, IngressState> {
    match shared.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Opaque send-capable job accepted or returned by an executor adapter.
#[must_use]
pub struct SendTaskJob {
    generation: WorkGeneration,
    future: SendFuture,
    sender: CompletionSender,
    trace_identity: TraceWorkIdentity,
    causal_parent: Option<TraceSequence>,
}

impl SendTaskJob {
    pub(crate) fn new(
        generation: WorkGeneration,
        future: SendFuture,
        sender: CompletionSender,
        trace_identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Self {
        Self {
            generation,
            future,
            sender,
            trace_identity,
            causal_parent,
        }
    }

    /// Runs the owned future and submits its exact output without blocking.
    ///
    /// # Errors
    ///
    /// Returns the exact completion on full or closed ingress.
    pub async fn run(self) -> Result<(), SendTaskCompletionError> {
        let output = self.future.await;
        let completion = SendTaskCompletion {
            payload: CompletionPayload {
                generation: self.generation,
                output,
                kind: CompletionKind::SendTask,
                trace_identity: self.trace_identity,
                causal_parent: self.causal_parent,
            },
            sender: self.sender,
        };
        completion.submit()
    }
}

/// Exact completion ownership returned when bounded ingress refuses it.
#[must_use]
pub struct SendTaskCompletion {
    payload: CompletionPayload,
    sender: CompletionSender,
}

impl SendTaskCompletion {
    /// Retries one non-blocking submission without changing completion identity.
    ///
    /// # Errors
    ///
    /// Returns `Full` or `Closed` with this exact completion still owned.
    pub fn submit(self) -> Result<(), SendTaskCompletionError> {
        let Self { payload, sender } = self;
        sender
            .submit_send_task(payload)
            .map_err(|error| match error {
                CompletionSubmissionError::Full(payload) => {
                    SendTaskCompletionError::Full(Self { payload, sender })
                }
                CompletionSubmissionError::Closed(payload) => {
                    SendTaskCompletionError::Closed(Self { payload, sender })
                }
                CompletionSubmissionError::Stale(payload) => {
                    SendTaskCompletionError::Stale(Self { payload, sender })
                }
            })
    }
}

pub enum SendTaskCompletionError {
    /// Bounded completion ingress returned the exact completion for retry.
    Full(SendTaskCompletion),
    /// Global scheduling authority is closed.
    Closed(SendTaskCompletion),
    /// The exact task generation is no longer live.
    Stale(SendTaskCompletion),
}

/// Exact send-capable host response completion for one opaque request token.
#[must_use]
pub struct HostResponseCompletion {
    payload: CompletionPayload,
    sender: CompletionSender,
}

impl HostResponseCompletion {
    pub(crate) fn new(
        generation: WorkGeneration,
        response: SendOutput,
        sender: CompletionSender,
        trace_identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Self {
        Self {
            payload: CompletionPayload {
                generation,
                output: response,
                kind: CompletionKind::HostResponse,
                trace_identity,
                causal_parent,
            },
            sender,
        }
    }

    /// Submits without blocking and returns exact ownership on saturation.
    ///
    /// # Errors
    ///
    /// Returns `Full`, `Closed`, or `Stale` with this exact completion still
    /// owned.
    pub fn submit(self) -> Result<(), HostResponseCompletionError> {
        let Self { payload, sender } = self;
        sender
            .submit_host_response(payload)
            .map_err(|error| match error {
                CompletionSubmissionError::Full(payload) => {
                    HostResponseCompletionError::Full(Self { payload, sender })
                }
                CompletionSubmissionError::Closed(payload) => {
                    HostResponseCompletionError::Closed(Self { payload, sender })
                }
                CompletionSubmissionError::Stale(payload) => {
                    HostResponseCompletionError::Stale(Self { payload, sender })
                }
            })
    }
}

pub enum HostResponseCompletionError {
    Full(HostResponseCompletion),
    Closed(HostResponseCompletion),
    Stale(HostResponseCompletion),
}

impl fmt::Debug for HostResponseCompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Full(_) => "HostResponseCompletionError::Full(..)",
            Self::Closed(_) => "HostResponseCompletionError::Closed(..)",
            Self::Stale(_) => "HostResponseCompletionError::Stale(..)",
        })
    }
}

impl fmt::Debug for SendTaskCompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Full(_) => "SendTaskCompletionError::Full(..)",
            Self::Closed(_) => "SendTaskCompletionError::Closed(..)",
            Self::Stale(_) => "SendTaskCompletionError::Stale(..)",
        })
    }
}

enum CompletionSubmissionError {
    Full(CompletionPayload),
    Closed(CompletionPayload),
    Stale(CompletionPayload),
}

/// Narrow executor adapter which never receives application state or actions.
pub trait SendTaskExecutor {
    /// Attempts to start exactly once and returns the owned job on refusal.
    ///
    /// # Errors
    ///
    /// Returns one structured refusal containing the unaccepted job.
    fn start(&mut self, job: SendTaskJob) -> Result<(), SendTaskStartError>;
}

pub enum SendTaskStartError {
    Unavailable(SendTaskJob),
    Full(SendTaskJob),
    Closed(SendTaskJob),
    Rejected(SendTaskJob),
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use runenui_core::{__runtime::SendOutput, NoHostProtocol};

    use super::{CompletionIngress, CompletionKind, CompletionPayload};
    use crate::{
        RuntimeLimits, TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner, wake::WakeState,
        work::WorkRegistry,
    };

    fn host_generation() -> crate::work::WorkGeneration {
        WorkRegistry::<(), NoHostProtocol>::new(RuntimeLimits::default())
            .preview_generations(1)
            .unwrap_or_else(|_| unreachable!("one test generation is available"))
            .0[0]
    }

    fn host_payload(generation: crate::work::WorkGeneration) -> CompletionPayload {
        CompletionPayload {
            generation,
            output: Box::new(7_u8) as SendOutput,
            kind: CompletionKind::HostResponse,
            trace_identity: TraceWorkIdentity::new(
                TraceWorkOwner::Application,
                TraceWorkFamily::HostRequest,
                generation.get(),
                None,
            ),
            causal_parent: None,
        }
    }

    #[test]
    fn direct_and_detached_host_response_race_has_exactly_one_winner() {
        let wake = WakeState::new();
        let ingress = CompletionIngress::new(1, wake.handle());
        let generation = host_generation();
        ingress.register_host_response(generation);
        let sender = ingress.sender();
        let barrier = Arc::new(Barrier::new(2));
        let producer_barrier = Arc::clone(&barrier);
        let producer = std::thread::spawn(move || {
            producer_barrier.wait();
            sender
                .submit_host_response(host_payload(generation))
                .is_ok()
        });

        barrier.wait();
        let direct_won = ingress.claim_direct_host_response(generation);
        let detached_won = producer.join().unwrap_or_else(|_| unreachable!());

        assert_ne!(direct_won, detached_won);
        assert_eq!(ingress.len(), usize::from(detached_won));
    }

    #[test]
    fn cancellation_and_detached_host_response_race_retains_no_payload() {
        let wake = WakeState::new();
        let ingress = CompletionIngress::new(1, wake.handle());
        let generation = host_generation();
        ingress.register_host_response(generation);
        let sender = ingress.sender();
        let barrier = Arc::new(Barrier::new(2));
        let producer_barrier = Arc::clone(&barrier);
        let producer = std::thread::spawn(move || {
            producer_barrier.wait();
            sender.submit_host_response(host_payload(generation))
        });

        barrier.wait();
        let _ = ingress.revoke_generation(generation);
        let submitted = producer.join().unwrap_or_else(|_| unreachable!());
        if submitted.is_ok() {
            let _ = ingress.revoke_generation(generation);
        }

        assert!(!ingress.host_response_is_open(generation));
        assert_eq!(ingress.len(), 0);
    }
}

impl fmt::Debug for SendTaskStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable(_) => "SendTaskStartError::Unavailable(..)",
            Self::Full(_) => "SendTaskStartError::Full(..)",
            Self::Closed(_) => "SendTaskStartError::Closed(..)",
            Self::Rejected(_) => "SendTaskStartError::Rejected(..)",
        })
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendTaskStartOutcome {
    Started,
    Unavailable,
    Full,
    Closed,
    Rejected,
}

pub(crate) struct UnavailableExecutor;

impl SendTaskExecutor for UnavailableExecutor {
    fn start(&mut self, job: SendTaskJob) -> Result<(), SendTaskStartError> {
        Err(SendTaskStartError::Unavailable(job))
    }
}
