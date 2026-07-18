//! Race-free shared wake request handshake.

#![allow(clippy::redundant_pub_crate)]

use std::sync::{Arc, Mutex};

/// Narrow host adapter invoked once for each claimed wake request epoch.
pub trait WakeTransport: Send + Sync {
    fn request_wake(&self);
}

impl<F> WakeTransport for F
where
    F: Fn() + Send + Sync,
{
    fn request_wake(&self) {
        self();
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WakeRequestOutcome {
    Requested,
    AlreadyRequested,
    Closed,
}

pub(crate) struct WakeState {
    shared: Arc<WakeShared>,
}

struct WakeShared {
    inner: Mutex<WakeInner>,
}

struct WakeInner {
    phase: WakePhase,
    transport: Option<Arc<dyn WakeTransport>>,
    callback_in_flight: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WakePhase {
    Idle,
    Requested { delivered: bool },
    Closed,
}

struct DeliveryClaim {
    transport: Arc<dyn WakeTransport>,
}

#[derive(Clone)]
pub(crate) struct WakeHandle {
    shared: Arc<WakeShared>,
}

impl WakeState {
    pub(crate) fn new() -> Self {
        Self {
            shared: Arc::new(WakeShared {
                inner: Mutex::new(WakeInner {
                    phase: WakePhase::Idle,
                    transport: None,
                    callback_in_flight: false,
                }),
            }),
        }
    }

    pub(crate) fn handle(&self) -> WakeHandle {
        WakeHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    pub(crate) fn set_transport(&self, transport: impl WakeTransport + 'static) {
        let claim = {
            let mut inner = lock(&self.shared.inner);
            if matches!(inner.phase, WakePhase::Closed) {
                return;
            }
            inner.transport = Some(Arc::new(transport));
            let claim = claim_delivery(&mut inner);
            drop(inner);
            claim
        };
        deliver_claims(&self.shared, claim);
    }

    pub(crate) fn acknowledge(&self) {
        let mut inner = lock(&self.shared.inner);
        if matches!(inner.phase, WakePhase::Requested { .. }) {
            inner.phase = WakePhase::Idle;
        }
    }

    pub(crate) fn close(&self) {
        let mut inner = lock(&self.shared.inner);
        inner.phase = WakePhase::Closed;
        inner.transport = None;
    }
}

impl WakeHandle {
    pub(crate) fn request(&self) -> WakeRequestOutcome {
        let (outcome, claim) = {
            let mut inner = lock(&self.shared.inner);
            let result = match inner.phase {
                WakePhase::Idle => {
                    inner.phase = WakePhase::Requested { delivered: false };
                    (WakeRequestOutcome::Requested, claim_delivery(&mut inner))
                }
                WakePhase::Requested { .. } => (WakeRequestOutcome::AlreadyRequested, None),
                WakePhase::Closed => (WakeRequestOutcome::Closed, None),
            };
            drop(inner);
            result
        };
        deliver_claims(&self.shared, claim);
        outcome
    }
}

/// Claims one callback under wake-state authority. The returned transport is
/// invoked only after this function's mutex guard has been released.
fn claim_delivery(inner: &mut WakeInner) -> Option<DeliveryClaim> {
    if inner.callback_in_flight {
        return None;
    }
    let WakePhase::Requested { delivered } = &mut inner.phase else {
        return None;
    };
    if *delivered {
        return None;
    }
    let transport = inner.transport.as_ref()?.clone();
    *delivered = true;
    inner.callback_in_flight = true;
    Some(DeliveryClaim { transport })
}

/// Runs every owned claim without a `RunenUI` mutex guard and serially claims a
/// later pending epoch after the preceding callback returns normally.
fn deliver_claims(shared: &WakeShared, mut claim: Option<DeliveryClaim>) {
    while let Some(current) = claim {
        current.transport.request_wake();
        claim = {
            let mut inner = lock(&shared.inner);
            inner.callback_in_flight = false;
            let claim = claim_delivery(&mut inner);
            drop(inner);
            claim
        };
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
            mpsc,
        },
        time::Duration,
    };

    use super::{WakePhase, WakeRequestOutcome, WakeState, lock};

    const TIMEOUT: Duration = Duration::from_secs(2);
    const CLOSE_RETURN_TIMEOUT: Duration = Duration::from_secs(1);
    const NO_CALLBACK_WINDOW: Duration = Duration::from_millis(50);

    fn snapshot(state: &WakeState) -> (WakePhase, bool, bool) {
        let inner = lock(&state.shared.inner);
        (
            inner.phase,
            inner.callback_in_flight,
            inner.transport.is_some(),
        )
    }

    fn blocking_release() -> (mpsc::Sender<()>, Arc<Mutex<mpsc::Receiver<()>>>) {
        let (sender, receiver) = mpsc::channel();
        (sender, Arc::new(Mutex::new(receiver)))
    }

    #[test]
    fn pending_request_is_claimed_once_after_transport_installation() {
        let state = WakeState::new();
        assert_eq!(state.handle().request(), WakeRequestOutcome::Requested);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: false }, false, false)
        );

        let first_calls = Arc::new(AtomicUsize::new(0));
        let first = Arc::clone(&first_calls);
        state.set_transport(move || {
            first.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: true }, false, true)
        );

        let replacement_calls = Arc::new(AtomicUsize::new(0));
        let replacement = Arc::clone(&replacement_calls);
        state.set_transport(move || {
            replacement.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(replacement_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            state.handle().request(),
            WakeRequestOutcome::AlreadyRequested
        );
    }

    #[test]
    fn transport_replacement_does_not_reclaim_delivered_request() {
        let state = Arc::new(WakeState::new());
        let original_calls = Arc::new(AtomicUsize::new(0));
        let original = Arc::clone(&original_calls);
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = blocking_release();
        let release = Arc::clone(&release_rx);
        state.set_transport(move || {
            original.fetch_add(1, Ordering::SeqCst);
            entered_tx.send(()).unwrap_or_else(|_| unreachable!());
            release
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .recv_timeout(TIMEOUT)
                .unwrap_or_else(|_| unreachable!("test releases the callback"));
        });
        let requester = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.handle().request())
        };
        entered_rx
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| unreachable!("claimed callback enters"));
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: true }, true, true)
        );

        let replacement_calls = Arc::new(AtomicUsize::new(0));
        let replacement = Arc::clone(&replacement_calls);
        state.set_transport(move || {
            replacement.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(replacement_calls.load(Ordering::SeqCst), 0);
        release_tx.send(()).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            requester
                .join()
                .unwrap_or_else(|_| unreachable!("request thread completes")),
            WakeRequestOutcome::Requested
        );
        assert_eq!(original_calls.load(Ordering::SeqCst), 1);
        assert_eq!(replacement_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: true }, false, true)
        );
    }

    #[test]
    fn wake_callback_can_close_same_state_without_deadlock() {
        let state = Arc::new(WakeState::new());
        let callback_calls = Arc::new(AtomicUsize::new(0));
        let callback = Arc::clone(&callback_calls);
        let (closed_tx, closed_rx) = mpsc::channel();
        let callback_state = Arc::clone(&state);
        state.set_transport(move || {
            callback.fetch_add(1, Ordering::SeqCst);
            callback_state.close();
            closed_tx.send(()).unwrap_or_else(|_| unreachable!());
        });
        let requester = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.handle().request())
        };
        closed_rx
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| unreachable!("reentrant close must not deadlock"));
        requester
            .join()
            .unwrap_or_else(|_| unreachable!("request thread completes"));
        assert_eq!(callback_calls.load(Ordering::SeqCst), 1);
        assert_eq!(snapshot(&state), (WakePhase::Closed, false, false));
        assert_eq!(state.handle().request(), WakeRequestOutcome::Closed);
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        state.set_transport(move || {
            probe.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn wake_callback_can_reenter_request_and_transport_setup() {
        let state = Arc::new(WakeState::new());
        let initial_calls = Arc::new(AtomicUsize::new(0));
        let initial = Arc::clone(&initial_calls);
        let replacement_calls = Arc::new(AtomicUsize::new(0));
        let replacement = Arc::clone(&replacement_calls);
        let (reentered_tx, reentered_rx) = mpsc::channel();
        let callback_state = Arc::clone(&state);
        state.set_transport(move || {
            initial.fetch_add(1, Ordering::SeqCst);
            let replacement = Arc::clone(&replacement);
            callback_state.set_transport(move || {
                replacement.fetch_add(1, Ordering::SeqCst);
            });
            callback_state.acknowledge();
            assert_eq!(
                callback_state.handle().request(),
                WakeRequestOutcome::Requested
            );
            reentered_tx.send(()).unwrap_or_else(|_| unreachable!());
        });

        let requester = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.handle().request())
        };
        reentered_rx
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| unreachable!("callback re-entry must not deadlock"));
        assert_eq!(
            requester
                .join()
                .unwrap_or_else(|_| unreachable!("request thread completes")),
            WakeRequestOutcome::Requested
        );
        assert_eq!(initial_calls.load(Ordering::SeqCst), 1);
        assert_eq!(replacement_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: true }, false, true)
        );
    }

    #[test]
    fn blocking_wake_callback_does_not_block_close() {
        let state = Arc::new(WakeState::new());
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = blocking_release();
        let release = Arc::clone(&release_rx);
        let callback_calls = Arc::new(AtomicUsize::new(0));
        let callback_finished = Arc::new(AtomicBool::new(false));
        let calls = Arc::clone(&callback_calls);
        let finished = Arc::clone(&callback_finished);
        state.set_transport(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            entered_tx.send(()).unwrap_or_else(|_| unreachable!());
            release
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .recv_timeout(TIMEOUT)
                .unwrap_or_else(|_| unreachable!("test releases the callback"));
            finished.store(true, Ordering::SeqCst);
        });
        let requester = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.handle().request())
        };
        entered_rx
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| unreachable!("callback enters"));
        let (closed_tx, closed_rx) = mpsc::channel();
        let closer = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                state.close();
                closed_tx.send(()).unwrap_or_else(|_| unreachable!());
            })
        };
        let close_returned_before_release = closed_rx.recv_timeout(CLOSE_RETURN_TIMEOUT).is_ok();
        assert_eq!(callback_calls.load(Ordering::SeqCst), 1);
        assert!(!callback_finished.load(Ordering::SeqCst));
        release_tx.send(()).unwrap_or_else(|_| unreachable!());
        requester
            .join()
            .unwrap_or_else(|_| unreachable!("request thread completes"));
        closer
            .join()
            .unwrap_or_else(|_| unreachable!("close thread completes"));
        assert!(close_returned_before_release);
        assert!(callback_finished.load(Ordering::SeqCst));
        assert_eq!(callback_calls.load(Ordering::SeqCst), 1);
        assert_eq!(snapshot(&state), (WakePhase::Closed, false, false));
    }

    #[test]
    fn wake_callbacks_are_serialized_without_lock_held_invocation() {
        let state = Arc::new(WakeState::new());
        let current = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = blocking_release();
        let release = Arc::clone(&release_rx);
        let callback_current = Arc::clone(&current);
        let callback_maximum = Arc::clone(&maximum);
        let callback_calls = Arc::clone(&calls);
        state.set_transport(move || {
            let concurrent = callback_current.fetch_add(1, Ordering::SeqCst) + 1;
            callback_maximum.fetch_max(concurrent, Ordering::SeqCst);
            let call = callback_calls.fetch_add(1, Ordering::SeqCst) + 1;
            entered_tx.send(call).unwrap_or_else(|_| unreachable!());
            if call == 1 {
                release
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .recv_timeout(TIMEOUT)
                    .unwrap_or_else(|_| unreachable!("test releases first callback"));
            }
            callback_current.fetch_sub(1, Ordering::SeqCst);
        });

        let requester = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.handle().request())
        };
        assert_eq!(
            entered_rx
                .recv_timeout(TIMEOUT)
                .unwrap_or_else(|_| unreachable!("first callback enters")),
            1
        );
        state.acknowledge();
        assert_eq!(state.handle().request(), WakeRequestOutcome::Requested);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: false }, true, true)
        );
        assert!(entered_rx.recv_timeout(NO_CALLBACK_WINDOW).is_err());
        release_tx.send(()).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            entered_rx
                .recv_timeout(TIMEOUT)
                .unwrap_or_else(|_| unreachable!("second callback enters")),
            2
        );
        requester
            .join()
            .unwrap_or_else(|_| unreachable!("serialized callback loop completes"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(maximum.load(Ordering::SeqCst), 1);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: true }, false, true)
        );
    }

    #[test]
    fn request_during_in_flight_callback_is_delivered_after_callback_returns() {
        let state = Arc::new(WakeState::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = blocking_release();
        let release = Arc::clone(&release_rx);
        let callback_calls = Arc::clone(&calls);
        state.set_transport(move || {
            let call = callback_calls.fetch_add(1, Ordering::SeqCst) + 1;
            entered_tx.send(call).unwrap_or_else(|_| unreachable!());
            if call == 1 {
                release
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .recv_timeout(TIMEOUT)
                    .unwrap_or_else(|_| unreachable!("test releases first callback"));
            }
        });
        let requester = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.handle().request())
        };
        assert_eq!(
            entered_rx
                .recv_timeout(TIMEOUT)
                .unwrap_or_else(|_| unreachable!("first callback enters")),
            1
        );
        state.acknowledge();
        assert_eq!(state.handle().request(), WakeRequestOutcome::Requested);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: false }, true, true)
        );
        release_tx.send(()).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            entered_rx
                .recv_timeout(TIMEOUT)
                .unwrap_or_else(|_| unreachable!("second callback enters")),
            2
        );
        requester
            .join()
            .unwrap_or_else(|_| unreachable!("request thread completes"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: true }, false, true)
        );
    }

    #[test]
    fn close_prevents_new_delivery_claims() {
        let state = WakeState::new();
        assert_eq!(state.handle().request(), WakeRequestOutcome::Requested);
        assert_eq!(
            snapshot(&state),
            (WakePhase::Requested { delivered: false }, false, false)
        );
        state.close();
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        state.set_transport(move || {
            probe.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(state.handle().request(), WakeRequestOutcome::Closed);
        assert_eq!(snapshot(&state), (WakePhase::Closed, false, false));
    }

    #[test]
    fn claimed_callback_may_finish_after_close_without_rearming() {
        let state = Arc::new(WakeState::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = blocking_release();
        let release = Arc::clone(&release_rx);
        let callback_calls = Arc::clone(&calls);
        state.set_transport(move || {
            entered_tx.send(()).unwrap_or_else(|_| unreachable!());
            release
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .recv_timeout(TIMEOUT)
                .unwrap_or_else(|_| unreachable!("test releases callback"));
            callback_calls.fetch_add(1, Ordering::SeqCst);
        });
        let requester = {
            let state = Arc::clone(&state);
            std::thread::spawn(move || state.handle().request())
        };
        entered_rx
            .recv_timeout(TIMEOUT)
            .unwrap_or_else(|_| unreachable!("claimed callback enters"));
        state.close();
        assert_eq!(state.handle().request(), WakeRequestOutcome::Closed);
        assert_eq!(snapshot(&state), (WakePhase::Closed, true, false));
        release_tx.send(()).unwrap_or_else(|_| unreachable!());
        requester
            .join()
            .unwrap_or_else(|_| unreachable!("prior claim finishes"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(snapshot(&state), (WakePhase::Closed, false, false));
    }

    #[test]
    fn closed_wake_state_cannot_be_reopened() {
        let state = WakeState::new();
        state.close();
        state.acknowledge();
        assert_eq!(state.handle().request(), WakeRequestOutcome::Closed);
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Arc::clone(&calls);
        state.set_transport(move || {
            probe.fetch_add(1, Ordering::SeqCst);
        });
        state.acknowledge();
        assert_eq!(state.handle().request(), WakeRequestOutcome::Closed);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(snapshot(&state), (WakePhase::Closed, false, false));
    }

    #[test]
    fn request_and_transport_install_race_delivers_once_stress() {
        for _ in 0..1_000 {
            let state = Arc::new(WakeState::new());
            let calls = Arc::new(AtomicUsize::new(0));
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let requester = {
                let state = Arc::clone(&state);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    assert_eq!(state.handle().request(), WakeRequestOutcome::Requested);
                })
            };
            barrier.wait();
            let probe = Arc::clone(&calls);
            state.set_transport(move || {
                probe.fetch_add(1, Ordering::SeqCst);
            });
            requester
                .join()
                .unwrap_or_else(|_| unreachable!("request thread remains deterministic"));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn request_and_transport_replacement_race_delivers_once_stress() {
        for _ in 0..1_000 {
            let state = Arc::new(WakeState::new());
            let calls = Arc::new(AtomicUsize::new(0));
            let first = Arc::clone(&calls);
            state.set_transport(move || {
                first.fetch_add(1, Ordering::SeqCst);
            });
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let requester = {
                let state = Arc::clone(&state);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    assert_eq!(state.handle().request(), WakeRequestOutcome::Requested);
                })
            };
            barrier.wait();
            let replacement = Arc::clone(&calls);
            state.set_transport(move || {
                replacement.fetch_add(1, Ordering::SeqCst);
            });
            requester
                .join()
                .unwrap_or_else(|_| unreachable!("request thread remains deterministic"));
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }
}
