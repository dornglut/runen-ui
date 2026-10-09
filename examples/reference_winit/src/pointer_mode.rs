//! Host-private game pointer-mode policy. No RunenUI routing or camera semantics live here.
use runenui_core::{CursorShape, SurfaceId};
use winit::{
    error::ExternalError,
    window::{CursorGrabMode, Window},
};

use crate::framework_services::cursor_icon;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Absolute,
    ConfinedAbsolute,
    LockedRelative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Unsupported,
    Ignored,
    Native,
    Inactive,
    Exhausted,
    ReleaseFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope {
    pub window_epoch: u64,
    pub surface: Option<SurfaceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lease {
    pub generation: u64,
    pub scope: Scope,
    pub mode: Mode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum State {
    Absolute,
    AwaitingMotion(Lease),
    Active(Lease),
    ReleaseFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    Released,
    WaitingForMotion(Lease),
    Realized(Lease),
}

pub trait Platform {
    fn grab(&mut self, mode: Mode) -> Result<(), Failure>;
    fn cursor(&mut self, shape: CursorShape, visible: bool);
}

pub struct WinitPointer<'a>(pub &'a Window);

impl Platform for WinitPointer<'_> {
    fn grab(&mut self, mode: Mode) -> Result<(), Failure> {
        let grab = match mode {
            Mode::Absolute => CursorGrabMode::None,
            Mode::ConfinedAbsolute => CursorGrabMode::Confined,
            Mode::LockedRelative => CursorGrabMode::Locked,
        };
        self.0.set_cursor_grab(grab).map_err(|error| match error {
            ExternalError::NotSupported(_) => Failure::Unsupported,
            ExternalError::Ignored => Failure::Ignored,
            ExternalError::Os(_) => Failure::Native,
        })
    }

    fn cursor(&mut self, shape: CursorShape, visible: bool) {
        self.0.set_cursor(cursor_icon(shape));
        self.0.set_cursor_visible(visible);
    }
}

/// One native-window controller. Its host owns native window/seat association.
/// State is not a substitute for an OS query, and native failures remain explicit.
pub struct PointerModes {
    state: State,
    next_generation: Option<u64>,
    focused: bool,
    scope: Option<Scope>,
    ui_shape: CursorShape,
    ui_visible: bool,
}

impl Default for PointerModes {
    fn default() -> Self {
        Self {
            state: State::Absolute,
            next_generation: Some(1),
            focused: false,
            scope: None,
            ui_shape: CursorShape::Default,
            ui_visible: true,
        }
    }
}

impl PointerModes {
    pub const fn state(&self) -> &State {
        &self.state
    }

    pub fn ui_pointer_allowed(&self) -> bool {
        self.focused && matches!(
            self.state,
            State::Absolute | State::Active(Lease { mode: Mode::ConfinedAbsolute, .. })
        )
    }

    pub fn gameplay_motion_allowed(&self) -> bool {
        self.focused && matches!(
            self.state,
            State::Active(Lease { mode: Mode::LockedRelative, .. })
        )
    }

    pub fn set_ui_cursor(&mut self, platform: &mut impl Platform, shape: CursorShape, visible: bool) {
        self.ui_shape = shape;
        self.ui_visible = visible;
        self.apply_cursor(platform);
    }

    fn apply_cursor(&self, platform: &mut impl Platform) {
        if !self.focused {
            platform.cursor(CursorShape::Default, true);
            return;
        }
        let locked = matches!(
            self.state,
            State::AwaitingMotion(_) | State::Active(Lease { mode: Mode::LockedRelative, .. })
        );
        if locked {
            platform.cursor(CursorShape::Default, false);
        } else if matches!(self.state, State::ReleaseFailed) {
            platform.cursor(CursorShape::Default, true);
        } else {
            platform.cursor(self.ui_shape, self.ui_visible);
        }
    }

    pub fn focus_changed(&mut self, focused: bool, platform: &mut impl Platform) -> Result<(), Failure> {
        self.focused = focused;
        if !focused {
            // Loss of activation invalidates the native lease before runtime work.
            self.release(platform)?;
        }
        self.apply_cursor(platform);
        Ok(())
    }

    pub fn request(
        &mut self,
        scope: Scope,
        desired: Mode,
        platform: &mut impl Platform,
    ) -> Result<Outcome, Failure> {
        if desired == Mode::Absolute {
            self.release(platform)?;
            self.scope = Some(scope);
            return Ok(Outcome::Released);
        }
        if !self.focused {
            return Err(Failure::Inactive);
        }
        if matches!(self.state, State::ReleaseFailed) {
            return Err(Failure::ReleaseFailed);
        }
        match &self.state {
            State::Active(lease) if lease.scope == scope && lease.mode == desired => {
                return Ok(Outcome::Realized(lease.clone()));
            }
            State::AwaitingMotion(lease) if lease.scope == scope && lease.mode == desired => {
                return Ok(Outcome::WaitingForMotion(lease.clone()));
            }
            _ => {}
        }
        self.release(platform)?;
        let generation = self.next_generation.ok_or(Failure::Exhausted)?;
        self.next_generation = generation.checked_add(1);
        let lease = Lease { generation, scope: scope.clone(), mode: desired };
        // A failed native grab can still have partially changed native state.
        if let Err(error) = platform.grab(desired) {
            self.state = State::ReleaseFailed;
            platform.cursor(CursorShape::Default, true);
            if platform.grab(Mode::Absolute).is_ok() {
                self.state = State::Absolute;
                self.apply_cursor(platform);
                return Err(error);
            }
            return Err(Failure::ReleaseFailed);
        }
        self.scope = Some(scope);
        if desired == Mode::LockedRelative {
            self.state = State::AwaitingMotion(lease.clone());
            self.apply_cursor(platform);
            Ok(Outcome::WaitingForMotion(lease))
        } else {
            self.state = State::Active(lease.clone());
            self.apply_cursor(platform);
            Ok(Outcome::Realized(lease))
        }
    }

    /// A genuine finite host raw-device observation certifies motion availability,
    /// without claiming a window association from a device ID.
    pub fn observe_motion(&mut self, scope: &Scope, delta: (f64, f64)) -> bool {
        if !self.focused || self.scope.as_ref() != Some(scope) || !delta.0.is_finite() || !delta.1.is_finite() {
            return false;
        }
        match &self.state {
            State::AwaitingMotion(lease)
                if &lease.scope == scope && lease.mode == Mode::LockedRelative =>
            {
                self.state = State::Active(lease.clone());
                true
            }
            State::Active(lease)
                if &lease.scope == scope && lease.mode == Mode::LockedRelative => true,
            _ => false,
        }
    }

    /// Invalidate the lease before native I/O. Failure leaves all input gated.
    pub fn release(&mut self, platform: &mut impl Platform) -> Result<(), Failure> {
        if matches!(self.state, State::Absolute) {
            self.apply_cursor(platform);
            return Ok(());
        }
        self.state = State::ReleaseFailed;
        platform.cursor(CursorShape::Default, true);
        match platform.grab(Mode::Absolute) {
            Ok(()) => {
                self.state = State::Absolute;
                self.apply_cursor(platform);
                Ok(())
            }
            Err(_) => Err(Failure::ReleaseFailed),
        }
    }

    /// A replaced native window never inherits the old lease or its generation.
    pub fn retire_window(&mut self, platform: &mut impl Platform) -> Result<(), Failure> {
        let result = self.release(platform);
        self.focused = false;
        self.scope = None;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        mode: Mode,
        visible: bool,
        failures: Vec<(Mode, Failure)>,
        calls: Vec<(Mode, Option<bool>)>,
    }
    impl Default for Mode {
        fn default() -> Self { Self::Absolute }
    }
    impl Platform for Fake {
        fn grab(&mut self, mode: Mode) -> Result<(), Failure> {
            self.calls.push((mode, None));
            if let Some(at) = self.failures.iter().position(|(when, _)| *when == mode) {
                return Err(self.failures.remove(at).1);
            }
            self.mode = mode;
            Ok(())
        }
        fn cursor(&mut self, _shape: CursorShape, visible: bool) {
            self.visible = visible;
            self.calls.push((self.mode, Some(visible)));
        }
    }
    fn scope(epoch: u64) -> Scope {
        Scope { window_epoch: epoch, surface: None }
    }
    fn focused() -> (PointerModes, Fake) {
        let mut controller = PointerModes::default();
        let mut host = Fake::default();
        controller.focus_changed(true, &mut host).unwrap();
        (controller, host)
    }

    #[test]
    fn ordinary_confined_and_game_modes_are_distinct() {
        let (mut modes, mut host) = focused();
        assert!(modes.ui_pointer_allowed());
        assert!(matches!(
            modes.request(scope(1), Mode::ConfinedAbsolute, &mut host),
            Ok(Outcome::Realized(_))
        ));
        assert!(modes.ui_pointer_allowed());
        assert!(!modes.gameplay_motion_allowed());
        assert!(matches!(
            modes.request(scope(1), Mode::LockedRelative, &mut host),
            Ok(Outcome::WaitingForMotion(_))
        ));
        assert!(!modes.ui_pointer_allowed());
        assert!(!modes.gameplay_motion_allowed());
        assert!(!host.visible);
        assert!(!modes.observe_motion(&scope(2), (2.0, 1.0)));
        assert!(!modes.observe_motion(&scope(1), (f64::NAN, 1.0)));
        assert!(modes.observe_motion(&scope(1), (2.0, 1.0)));
        assert!(modes.gameplay_motion_allowed());
        modes.release(&mut host).unwrap();
        assert!(modes.ui_pointer_allowed());
        assert!(!modes.gameplay_motion_allowed());
        assert!(host.visible);
        assert_eq!(host.mode, Mode::Absolute);
    }

    #[test]
    fn lock_failure_does_not_fallback_to_confinement() {
        let (mut modes, mut host) = focused();
        host.failures.push((Mode::LockedRelative, Failure::Unsupported));
        assert_eq!(
            modes.request(scope(1), Mode::LockedRelative, &mut host),
            Err(Failure::Unsupported)
        );
        assert_eq!(host.mode, Mode::Absolute);
        assert!(host.visible);
        assert!(modes.ui_pointer_allowed());
    }

    #[test]
    fn release_failure_never_reenables_input_or_accepts_new_lease() {
        let (mut modes, mut host) = focused();
        modes.request(scope(1), Mode::LockedRelative, &mut host).unwrap();
        host.failures.push((Mode::Absolute, Failure::Native));
        assert_eq!(modes.release(&mut host), Err(Failure::ReleaseFailed));
        assert!(!modes.ui_pointer_allowed());
        assert!(!modes.gameplay_motion_allowed());
        assert!(host.visible);
        assert_eq!(
            modes.request(scope(2), Mode::LockedRelative, &mut host),
            Err(Failure::ReleaseFailed)
        );
        modes.release(&mut host).unwrap();
        assert!(modes.ui_pointer_allowed());
    }

    #[test]
    fn failure_after_partial_acquire_requires_rollback() {
        let (mut modes, mut host) = focused();
        host.failures.push((Mode::ConfinedAbsolute, Failure::Ignored));
        host.failures.push((Mode::Absolute, Failure::Native));
        assert_eq!(
            modes.request(scope(1), Mode::ConfinedAbsolute, &mut host),
            Err(Failure::ReleaseFailed)
        );
        assert!(matches!(modes.state(), State::ReleaseFailed));
        assert!(!modes.ui_pointer_allowed());
        modes.release(&mut host).unwrap();
    }

    #[test]
    fn ui_cursor_updates_cannot_unhide_locked_cursor() {
        let (mut modes, mut host) = focused();
        modes.request(scope(1), Mode::LockedRelative, &mut host).unwrap();
        modes.set_ui_cursor(&mut host, CursorShape::Pointer, true);
        assert!(!host.visible);
        modes.focus_changed(false, &mut host).unwrap();
        assert!(host.visible);
        assert!(!modes.gameplay_motion_allowed());
        assert!(!modes.ui_pointer_allowed());
        modes.focus_changed(true, &mut host).unwrap();
        assert!(modes.ui_pointer_allowed());
        assert!(!modes.gameplay_motion_allowed());
    }

    #[test]
    fn focus_loss_revokes_pending_motion_and_requires_new_acquisition() {
        let (mut modes, mut host) = focused();
        let pending = modes.request(scope(1), Mode::LockedRelative, &mut host).unwrap();
        modes.focus_changed(false, &mut host).unwrap();
        assert!(!modes.observe_motion(&scope(1), (4.0, 6.0)));
        assert!(!modes.gameplay_motion_allowed());
        assert!(host.visible);
        modes.focus_changed(true, &mut host).unwrap();
        let fresh = modes.request(scope(1), Mode::LockedRelative, &mut host).unwrap();
        assert_ne!(pending, fresh);
    }

    #[test]
    fn window_handoff_releases_old_lease_without_input_retargeting() {
        let (mut modes, mut host) = focused();
        modes.request(scope(1), Mode::LockedRelative, &mut host).unwrap();
        assert!(modes.observe_motion(&scope(1), (1.0, 2.0)));
        let next = modes.request(scope(2), Mode::LockedRelative, &mut host).unwrap();
        assert!(matches!(next, Outcome::WaitingForMotion(_)));
        assert!(!modes.observe_motion(&scope(1), (8.0, 9.0)));
        assert!(modes.observe_motion(&scope(2), (8.0, 9.0)));
        assert!(host.calls.iter().any(|(mode, _)| *mode == Mode::Absolute));
    }

    #[test]
    fn unavailable_generation_fails_without_native_grab() {
        let (mut modes, mut host) = focused();
        modes.next_generation = None;
        let before = host.calls.len();
        assert_eq!(modes.request(scope(1), Mode::LockedRelative, &mut host), Err(Failure::Exhausted));
        assert!(modes.ui_pointer_allowed());
        assert_eq!(host.calls.len(), before + 1); // harmless baseline restoration
        assert_eq!(host.mode, Mode::Absolute);
    }

    #[test]
    fn unfocused_cursor_service_cannot_hide_system_cursor() {
        let (mut modes, mut host) = focused();
        modes.focus_changed(false, &mut host).unwrap();
        modes.set_ui_cursor(&mut host, CursorShape::Grabbing, false);
        assert!(host.visible);
        assert!(!modes.ui_pointer_allowed());
        modes.focus_changed(true, &mut host).unwrap();
        assert!(!host.visible); // UI baseline resumes only when the host is focused
    }

    #[test]
    fn repeated_request_is_idempotent_and_retirement_invalidates_lease() {
        let (mut modes, mut host) = focused();
        let first = modes.request(scope(1), Mode::LockedRelative, &mut host).unwrap();
        let count = host.calls.len();
        assert_eq!(modes.request(scope(1), Mode::LockedRelative, &mut host), Ok(first));
        assert_eq!(host.calls.len(), count);
        assert!(modes.observe_motion(&scope(1), (1.0, -1.0)));
        modes.retire_window(&mut host).unwrap();
        assert!(!modes.observe_motion(&scope(1), (1.0, -1.0)));
        assert_eq!(modes.request(scope(2), Mode::LockedRelative, &mut host), Err(Failure::Inactive));
    }
}
