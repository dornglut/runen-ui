//! Single runtime-owned authority for active pointer streams.

mod processing;

use core::num::NonZeroU64;
use std::collections::BTreeMap;

use runenui_core::{
    InputDeviceId, LogicalPoint, MountedNodeId, PointerButtons, PointerDeviceKind, PointerId,
    SurfaceId, SurfaceInputContext,
};

/// Exact state retained for one active pointer stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct PointerStreamState {
    registration_sequence: NonZeroU64,
    surface: SurfaceId,
    device_id: Option<InputDeviceId>,
    device_kind: PointerDeviceKind,
    position: LogicalPoint,
    physical_path: Vec<MountedNodeId>,
    buttons: PointerButtons,
    pressed_owner: Option<MountedNodeId>,
    pressed_inside: bool,
    capture_owner: Option<MountedNodeId>,
    surface_context: Option<SurfaceInputContext>,
}

impl PointerStreamState {
    pub(in crate::runtime) const fn registration_sequence(&self) -> NonZeroU64 {
        self.registration_sequence
    }

    pub(in crate::runtime) const fn surface(&self) -> &SurfaceId {
        &self.surface
    }

    pub(in crate::runtime) const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }

    pub(in crate::runtime) const fn device_kind(&self) -> PointerDeviceKind {
        self.device_kind
    }

    #[cfg(test)]
    pub(in crate::runtime) const fn position(&self) -> LogicalPoint {
        self.position
    }

    pub(in crate::runtime) fn physical_path(&self) -> &[MountedNodeId] {
        &self.physical_path
    }

    pub(in crate::runtime) const fn buttons(&self) -> &PointerButtons {
        &self.buttons
    }

    pub(in crate::runtime) const fn pressed_owner(&self) -> Option<&MountedNodeId> {
        self.pressed_owner.as_ref()
    }

    #[cfg(test)]
    pub(in crate::runtime) const fn pressed_inside(&self) -> bool {
        self.pressed_inside
    }

    pub(in crate::runtime) const fn capture_owner(&self) -> Option<&MountedNodeId> {
        self.capture_owner.as_ref()
    }

    pub(in crate::runtime) const fn surface_context(&self) -> Option<&SurfaceInputContext> {
        self.surface_context.as_ref()
    }

    pub(in crate::runtime) fn update_observation(
        &mut self,
        position: LogicalPoint,
        physical_path: Vec<MountedNodeId>,
        buttons: PointerButtons,
    ) {
        self.position = position;
        self.physical_path = physical_path;
        self.buttons = buttons;
    }

    pub(in crate::runtime) fn set_buttons(&mut self, buttons: PointerButtons) {
        self.buttons = buttons;
    }

    pub(in crate::runtime) fn set_pressed_owner(&mut self, owner: Option<MountedNodeId>) {
        self.pressed_owner = owner;
        self.pressed_inside = self.pressed_owner.is_some();
    }

    pub(in crate::runtime) const fn set_pressed_inside(&mut self, inside: bool) {
        self.pressed_inside = inside && self.pressed_owner.is_some();
    }

    pub(in crate::runtime) fn set_capture_owner(&mut self, owner: Option<MountedNodeId>) {
        self.capture_owner = owner;
    }

    pub(in crate::runtime) fn set_surface_context(&mut self, context: SurfaceInputContext) {
        self.surface_context = Some(context);
    }

    #[cfg(test)]
    fn clear_target(&mut self, target: &MountedNodeId) -> PointerTargetCleanup {
        let mut cleanup = PointerTargetCleanup::default();
        if self.pressed_owner.as_ref() == Some(target) {
            self.pressed_owner = None;
            self.pressed_inside = false;
            cleanup.pressed = true;
        }
        if self.capture_owner.as_ref() == Some(target) {
            self.capture_owner = None;
            cleanup.capture = true;
        }
        if self.physical_path.iter().any(|node| node == target) {
            self.physical_path.clear();
            cleanup.physical_path = true;
        }
        cleanup
    }
}

/// One exact lifecycle cleanup outcome for a mounted target.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::runtime) struct PointerTargetCleanup {
    pub(in crate::runtime) pressed: bool,
    pub(in crate::runtime) capture: bool,
    pub(in crate::runtime) physical_path: bool,
}

#[cfg(test)]
impl PointerTargetCleanup {
    pub(in crate::runtime) const fn any(self) -> bool {
        self.pressed || self.capture || self.physical_path
    }

    fn merge(&mut self, other: Self) {
        self.pressed |= other.pressed;
        self.capture |= other.capture;
        self.physical_path |= other.physical_path;
    }
}

/// Failure to register a new pointer stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum PointerRegistrationError {
    Duplicate,
    Full,
    RegistrationSequenceExhausted,
}

/// Failure to address an active pointer stream consistently.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum PointerStreamError {
    Missing,
    ForeignSurface,
    DeviceMismatch,
    DeviceKindMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum PointerCommitError {
    Duplicate,
    Full,
    Missing,
    RegistrationSequenceMismatch,
}

/// Bounded exact-owner registry for active pointer streams.
pub(in crate::runtime) struct PointerRegistry {
    capacity: usize,
    next_registration_sequence: Option<NonZeroU64>,
    streams: BTreeMap<PointerId, PointerStreamState>,
}

impl PointerRegistry {
    pub(in crate::runtime) const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            next_registration_sequence: NonZeroU64::new(1),
            streams: BTreeMap::new(),
        }
    }

    pub(in crate::runtime) fn plan_registration(
        &self,
        pointer_id: PointerId,
        surface: SurfaceId,
        device_id: Option<InputDeviceId>,
        device_kind: PointerDeviceKind,
        position: LogicalPoint,
        buttons: PointerButtons,
    ) -> Result<PointerStreamState, PointerRegistrationError> {
        if self.streams.contains_key(&pointer_id) {
            return Err(PointerRegistrationError::Duplicate);
        }
        if self.streams.len() == self.capacity {
            return Err(PointerRegistrationError::Full);
        }
        let registration_sequence = self
            .next_registration_sequence
            .ok_or(PointerRegistrationError::RegistrationSequenceExhausted)?;
        Ok(PointerStreamState {
            registration_sequence,
            surface,
            device_id,
            device_kind,
            position,
            physical_path: Vec::new(),
            buttons,
            pressed_owner: None,
            pressed_inside: false,
            capture_owner: None,
            surface_context: None,
        })
    }

    pub(in crate::runtime) fn commit_registration(
        &mut self,
        pointer_id: PointerId,
        state: PointerStreamState,
    ) -> Result<(), PointerCommitError> {
        if self.streams.contains_key(&pointer_id) {
            return Err(PointerCommitError::Duplicate);
        }
        if self.streams.len() == self.capacity {
            return Err(PointerCommitError::Full);
        }
        if self.next_registration_sequence != Some(state.registration_sequence()) {
            return Err(PointerCommitError::RegistrationSequenceMismatch);
        }
        self.next_registration_sequence = state
            .registration_sequence()
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new);
        self.streams.insert(pointer_id, state);
        Ok(())
    }

    pub(in crate::runtime) fn replace(
        &mut self,
        pointer_id: PointerId,
        state: PointerStreamState,
    ) -> Result<(), PointerCommitError> {
        if !self.streams.contains_key(&pointer_id) {
            return Err(PointerCommitError::Missing);
        }
        self.streams.insert(pointer_id, state);
        Ok(())
    }

    #[cfg(test)]
    pub(in crate::runtime) fn register(
        &mut self,
        pointer_id: PointerId,
        surface: SurfaceId,
        device_id: Option<InputDeviceId>,
        device_kind: PointerDeviceKind,
        position: LogicalPoint,
        buttons: PointerButtons,
    ) -> Result<&mut PointerStreamState, PointerRegistrationError> {
        let state = self.plan_registration(
            pointer_id,
            surface,
            device_id,
            device_kind,
            position,
            buttons,
        )?;
        self.commit_registration(pointer_id, state)
            .unwrap_or_else(|_| {
                unreachable!("registration plan remains valid until immediate commit")
            });
        Ok(self
            .streams
            .get_mut(&pointer_id)
            .unwrap_or_else(|| unreachable!("new pointer stream was inserted")))
    }

    pub(in crate::runtime) fn validate(
        &self,
        pointer_id: PointerId,
        surface: &SurfaceId,
        device_id: Option<InputDeviceId>,
        device_kind: PointerDeviceKind,
    ) -> Result<&PointerStreamState, PointerStreamError> {
        let stream = self
            .streams
            .get(&pointer_id)
            .ok_or(PointerStreamError::Missing)?;
        if stream.surface() != surface {
            return Err(PointerStreamError::ForeignSurface);
        }
        if stream.device_id() != device_id {
            return Err(PointerStreamError::DeviceMismatch);
        }
        if stream.device_kind() != device_kind {
            return Err(PointerStreamError::DeviceKindMismatch);
        }
        Ok(stream)
    }

    #[cfg(test)]
    pub(in crate::runtime) fn stream_mut(
        &mut self,
        pointer_id: PointerId,
    ) -> Option<&mut PointerStreamState> {
        self.streams.get_mut(&pointer_id)
    }

    pub(in crate::runtime) fn close(
        &mut self,
        pointer_id: PointerId,
    ) -> Option<PointerStreamState> {
        self.streams.remove(&pointer_id)
    }

    pub(in crate::runtime) fn stream(&self, pointer_id: PointerId) -> Option<&PointerStreamState> {
        self.streams.get(&pointer_id)
    }

    #[cfg(test)]
    pub(in crate::runtime) fn clear_target(
        &mut self,
        target: &MountedNodeId,
    ) -> PointerTargetCleanup {
        let mut cleanup = PointerTargetCleanup::default();
        for stream in self.streams.values_mut() {
            cleanup.merge(stream.clear_target(target));
        }
        cleanup
    }

    pub(in crate::runtime) fn clear(&mut self) -> usize {
        let count = self.streams.len();
        self.streams.clear();
        count
    }

    #[cfg(test)]
    pub(in crate::runtime) fn ordered_pointer_ids(&self) -> Vec<PointerId> {
        let mut registered = self
            .streams
            .iter()
            .map(|(pointer_id, stream)| (*pointer_id, stream.registration_sequence()))
            .collect::<Vec<_>>();
        registered.sort_unstable_by_key(|(_, sequence)| *sequence);
        registered
            .into_iter()
            .map(|(pointer_id, _)| pointer_id)
            .collect()
    }

    #[cfg(test)]
    pub(in crate::runtime) fn len(&self) -> usize {
        self.streams.len()
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, LogicalPoint, PointerButtons, PointerDeviceKind, PointerId,
    };

    use super::{PointerRegistrationError, PointerRegistry, PointerStreamError};

    fn pointer(value: u64) -> PointerId {
        PointerId::new(value)
            .unwrap_or_else(|| unreachable!("test pointer identities are non-zero"))
    }

    fn point(value: f32) -> LogicalPoint {
        LogicalPoint::new(value, value).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    #[test]
    fn registry_is_bounded_and_rejects_duplicate_streams() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut registry = PointerRegistry::new(1);
        registry
            .register(
                pointer(1),
                surface.clone(),
                None,
                PointerDeviceKind::Mouse,
                point(1.0),
                PointerButtons::default(),
            )
            .unwrap_or_else(|_| unreachable!("first stream fits"));
        assert_eq!(
            registry.register(
                pointer(1),
                surface.clone(),
                None,
                PointerDeviceKind::Mouse,
                point(2.0),
                PointerButtons::default(),
            ),
            Err(PointerRegistrationError::Duplicate)
        );
        assert_eq!(
            registry.register(
                pointer(2),
                surface,
                None,
                PointerDeviceKind::Mouse,
                point(2.0),
                PointerButtons::default(),
            ),
            Err(PointerRegistrationError::Full)
        );
    }

    #[test]
    fn registration_plan_consumes_no_sequence_before_commit() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut registry = PointerRegistry::new(2);
        let first = registry
            .plan_registration(
                pointer(1),
                surface.clone(),
                None,
                PointerDeviceKind::Mouse,
                point(1.0),
                PointerButtons::default(),
            )
            .unwrap_or_else(|_| unreachable!("first stream fits"));
        let repeated = registry
            .plan_registration(
                pointer(1),
                surface,
                None,
                PointerDeviceKind::Mouse,
                point(2.0),
                PointerButtons::default(),
            )
            .unwrap_or_else(|_| unreachable!("uncommitted plan consumes no identity"));
        assert_eq!(
            first.registration_sequence(),
            repeated.registration_sequence()
        );
        registry
            .commit_registration(pointer(1), first)
            .unwrap_or_else(|_| unreachable!("unchanged plan commits"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn validation_preserves_surface_and_device_consistency() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let foreign_surface = namespace.__runtime_surface_id(1, 1);
        let mut registry = PointerRegistry::new(2);
        registry
            .register(
                pointer(1),
                surface.clone(),
                None,
                PointerDeviceKind::Touch,
                point(1.0),
                PointerButtons::default(),
            )
            .unwrap_or_else(|_| unreachable!("stream fits"));
        assert!(
            registry
                .validate(pointer(1), &surface, None, PointerDeviceKind::Touch)
                .is_ok()
        );
        assert_eq!(
            registry.validate(pointer(1), &foreign_surface, None, PointerDeviceKind::Touch,),
            Err(PointerStreamError::ForeignSurface)
        );
        assert_eq!(
            registry.validate(pointer(1), &surface, None, PointerDeviceKind::Pen,),
            Err(PointerStreamError::DeviceKindMismatch)
        );
    }

    #[test]
    fn registration_order_is_independent_of_pointer_identity_order() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut registry = PointerRegistry::new(3);
        for pointer_id in [pointer(9), pointer(2), pointer(5)] {
            registry
                .register(
                    pointer_id,
                    surface.clone(),
                    None,
                    PointerDeviceKind::Mouse,
                    point(1.0),
                    PointerButtons::default(),
                )
                .unwrap_or_else(|_| unreachable!("stream fits"));
        }
        assert_eq!(
            registry.ordered_pointer_ids(),
            [pointer(9), pointer(2), pointer(5)]
        );
    }

    #[test]
    fn target_cleanup_is_exact_per_stream() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let target = namespace.__runtime_mounted_id(3, 4);
        let other = namespace.__runtime_mounted_id(5, 6);
        let mut registry = PointerRegistry::new(2);
        let stream = registry
            .register(
                pointer(1),
                surface,
                None,
                PointerDeviceKind::Mouse,
                point(1.0),
                PointerButtons::default(),
            )
            .unwrap_or_else(|_| unreachable!("stream fits"));
        stream.update_observation(
            point(2.0),
            vec![other, target.clone()],
            PointerButtons::default(),
        );
        stream.set_pressed_owner(Some(target.clone()));
        stream.set_capture_owner(Some(target.clone()));
        assert_eq!(stream.position(), point(2.0));
        assert_eq!(stream.buttons(), &PointerButtons::default());
        assert!(stream.pressed_inside());

        let cleanup = registry.clear_target(&target);

        assert!(cleanup.any());
        let stream = registry
            .stream_mut(pointer(1))
            .unwrap_or_else(|| unreachable!("stream remains active"));
        assert_eq!(stream.pressed_owner(), None);
        assert_eq!(stream.capture_owner(), None);
        assert!(stream.physical_path().is_empty());
        assert_eq!(registry.len(), 1);
    }
}
