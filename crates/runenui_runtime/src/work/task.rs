//! Deterministic UI-thread local task storage.

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{wake::WakeHandle, work::WorkGeneration};

pub(crate) struct LocalTask<Action> {
    pub(crate) generation: WorkGeneration,
    future: Pin<Box<dyn Future<Output = Option<Action>>>>,
    readiness: TaskReadiness<Action>,
    wake_readiness: Arc<TaskWakeReadiness>,
}

struct TaskWakeReadiness {
    eligible: AtomicBool,
    wake: WakeHandle,
}

impl std::task::Wake for TaskWakeReadiness {
    fn wake(self: Arc<Self>) {
        self.eligible.store(true, Ordering::Release);
        let _ = self.wake.request();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.eligible.store(true, Ordering::Release);
        let _ = self.wake.request();
    }
}

enum TaskReadiness<Action> {
    Pending,
    Ready(Option<Action>),
}

pub(crate) enum TaskReady<Action> {
    NotReady,
    Complete(Option<Action>),
}

impl<Action> LocalTask<Action> {
    pub(crate) fn new(
        generation: WorkGeneration,
        future: Pin<Box<dyn Future<Output = Option<Action>>>>,
        wake: WakeHandle,
    ) -> Self {
        Self {
            generation,
            future,
            readiness: TaskReadiness::Pending,
            wake_readiness: Arc::new(TaskWakeReadiness {
                eligible: AtomicBool::new(true),
                wake,
            }),
        }
    }

    pub(crate) fn poll_once(&mut self) -> bool {
        if matches!(self.readiness, TaskReadiness::Ready(_)) {
            return false;
        }
        if !self.wake_readiness.eligible.swap(false, Ordering::AcqRel) {
            return false;
        }
        let waker = Waker::from(Arc::clone(&self.wake_readiness));
        let mut context = Context::from_waker(&waker);
        if let Poll::Ready(action) = self.future.as_mut().poll(&mut context) {
            self.readiness = TaskReadiness::Ready(action);
        }
        true
    }

    pub(crate) fn take_ready(&mut self) -> TaskReady<Action> {
        match core::mem::replace(&mut self.readiness, TaskReadiness::Pending) {
            TaskReadiness::Pending => TaskReady::NotReady,
            TaskReadiness::Ready(action) => TaskReady::Complete(action),
        }
    }

    pub(crate) const fn is_ready(&self) -> bool {
        matches!(self.readiness, TaskReadiness::Ready(_))
    }

    pub(crate) fn is_eligible(&self) -> bool {
        !self.is_ready() && self.wake_readiness.eligible.load(Ordering::Acquire)
    }
}
