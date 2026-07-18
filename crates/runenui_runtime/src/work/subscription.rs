//! Live declarative subscription sources and retained UI-thread mappers.

use core::{
    any::TypeId,
    task::{Context, Poll, Waker},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use runenui_core::{
    __runtime::{ErasedSendSubscriptionSource, SendOutput, Subscription, SubscriptionSource},
    SendSubscriptionSink, SendSubscriptionStartOutcome, WorkKey,
};

use crate::wake::WakeHandle;

use super::{WorkGeneration, WorkOwner};

pub(crate) struct LiveSubscription<Action> {
    pub(crate) generation: WorkGeneration,
    pub(crate) owner: WorkOwner,
    pub(crate) key: WorkKey,
    pub(crate) source_type: TypeId,
    pub(crate) revision: u64,
    pub(crate) source: LiveSubscriptionSource<Action>,
    pub(crate) started: bool,
}

pub(crate) enum LiveSubscriptionSource<Action> {
    Local {
        source: core::pin::Pin<Box<dyn runenui_core::LocalSubscriptionSource<Action>>>,
        readiness: Arc<SubscriptionReadiness>,
    },
    Send {
        source: Option<Box<dyn ErasedSendSubscriptionSource>>,
        map: Box<dyn FnMut(SendOutput) -> Action>,
    },
}

pub(crate) struct SubscriptionReadiness {
    eligible: AtomicBool,
    wake: WakeHandle,
}

impl std::task::Wake for SubscriptionReadiness {
    fn wake(self: Arc<Self>) {
        self.eligible.store(true, Ordering::Release);
        let _ = self.wake.request();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.eligible.store(true, Ordering::Release);
        let _ = self.wake.request();
    }
}

pub(crate) enum SubscriptionPoll<Action> {
    Pending,
    Item(Action),
    Closed,
    NotEligible,
}

impl<Action> LiveSubscription<Action> {
    pub(crate) fn new(
        generation: WorkGeneration,
        owner: WorkOwner,
        declaration: Subscription<Action>,
        wake: WakeHandle,
    ) -> Self {
        let source = match declaration.source {
            SubscriptionSource::Local(source) => LiveSubscriptionSource::Local {
                source,
                readiness: Arc::new(SubscriptionReadiness {
                    eligible: AtomicBool::new(true),
                    wake,
                }),
            },
            SubscriptionSource::Send { source, map } => LiveSubscriptionSource::Send {
                source: Some(source),
                map,
            },
        };
        Self {
            generation,
            owner,
            key: declaration.key,
            source_type: declaration.source_type,
            revision: declaration.revision,
            source,
            started: false,
        }
    }

    pub(crate) fn is_local_eligible(&self) -> bool {
        self.started
            && matches!(
                &self.source,
                LiveSubscriptionSource::Local { readiness, .. }
                    if readiness.eligible.load(Ordering::Acquire)
            )
    }

    pub(crate) fn poll_local_once(&mut self) -> SubscriptionPoll<Action> {
        if !self.started {
            return SubscriptionPoll::NotEligible;
        }
        let LiveSubscriptionSource::Local { source, readiness } = &mut self.source else {
            return SubscriptionPoll::NotEligible;
        };
        if !readiness.eligible.swap(false, Ordering::AcqRel) {
            return SubscriptionPoll::NotEligible;
        }
        let waker = Waker::from(Arc::clone(readiness));
        let mut context = Context::from_waker(&waker);
        match source.as_mut().poll_next(&mut context) {
            Poll::Pending => SubscriptionPoll::Pending,
            Poll::Ready(Some(action)) => {
                readiness.eligible.store(true, Ordering::Release);
                SubscriptionPoll::Item(action)
            }
            Poll::Ready(None) => SubscriptionPoll::Closed,
        }
    }

    pub(crate) fn start_send(
        &mut self,
        sink: SendSubscriptionSink<SendOutput>,
    ) -> Option<SendSubscriptionStartOutcome> {
        let LiveSubscriptionSource::Send { source, .. } = &mut self.source else {
            return None;
        };
        let source = source.take()?;
        Some(source.start(sink))
    }
}
