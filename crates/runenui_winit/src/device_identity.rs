use std::collections::BTreeMap;

use runenui_core::InputDeviceId;
use winit::event::DeviceId;

const FIRST_INPUT_DEVICE_ID: u64 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceIdentityError {
    Exhausted,
}

#[derive(Debug)]
pub struct DeviceIdentityMap {
    next_input_device_id: Option<u64>,
    identities: BTreeMap<DeviceId, InputDeviceId>,
}

impl Default for DeviceIdentityMap {
    fn default() -> Self {
        Self {
            next_input_device_id: Some(FIRST_INPUT_DEVICE_ID),
            identities: BTreeMap::new(),
        }
    }
}

impl DeviceIdentityMap {
    /// Resolves one native device to its stable host-session neutral identity.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceIdentityError::Exhausted`] when no further neutral device identity can be
    /// allocated.
    pub fn resolve(&mut self, native: DeviceId) -> Result<InputDeviceId, DeviceIdentityError> {
        if let Some(identity) = self.identities.get(&native).copied() {
            return Ok(identity);
        }

        let value = self
            .next_input_device_id
            .take()
            .ok_or(DeviceIdentityError::Exhausted)?;
        let identity = InputDeviceId::new(value).ok_or(DeviceIdentityError::Exhausted)?;
        self.next_input_device_id = value.checked_add(1);
        self.identities.insert(native, identity);
        Ok(identity)
    }

    /// Returns a previously allocated window-event device identity without
    /// creating another neutral source lifetime. A raw `DeviceEvent` ID is
    /// not guaranteed to match the window keyboard event's `DeviceId`.
    #[must_use]
    pub fn known(&self, native: DeviceId) -> Option<InputDeviceId> {
        self.identities.get(&native).copied()
    }

    /// Forgets only the expected live mapping after the host has separately
    /// retired its original keyboard, pointer, touch, and gameplay sources.
    /// An old removal cannot erase a newer incarnation with the same native ID.
    #[must_use]
    pub fn retire_exact(&mut self, native: DeviceId, expected: InputDeviceId) -> bool {
        if self.known(native) != Some(expected) {
            return false;
        }
        self.identities.remove(&native);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceId, DeviceIdentityMap};

    #[test]
    fn unknown_removal_does_not_allocate_a_neutral_identity() {
        let native = DeviceId::dummy();
        let mut map = DeviceIdentityMap::default();
        let unrelated = runenui_core::InputDeviceId::new(42)
            .unwrap_or_else(|| unreachable!("checked nonzero neutral ID"));
        assert_eq!(map.known(native), None);
        assert!(!map.retire_exact(native, unrelated));
        assert_eq!(
            map.resolve(native)
                .unwrap_or_else(|_| unreachable!("first allocation remains available"))
                .get(),
            1
        );
    }

    #[test]
    fn checked_retirement_blocks_stale_removal_after_reincarnation() {
        let native = DeviceId::dummy();
        let mut map = DeviceIdentityMap::default();
        let old = map
            .resolve(native)
            .unwrap_or_else(|_| unreachable!("first device"));
        assert_eq!(map.known(native), Some(old));
        assert!(map.retire_exact(native, old));
        assert_eq!(map.known(native), None);
        assert!(!map.retire_exact(native, old));

        let fresh = map
            .resolve(native)
            .unwrap_or_else(|_| unreachable!("new incarnation"));
        assert_ne!(fresh, old);
        assert_eq!(fresh.get(), 2);
        assert!(!map.retire_exact(native, old));
        assert_eq!(map.known(native), Some(fresh));
        assert_eq!(
            map.resolve(native)
                .unwrap_or_else(|_| unreachable!("stable present incarnation")),
            fresh
        );
        assert!(map.retire_exact(native, fresh));
        assert_eq!(
            map.resolve(native)
                .unwrap_or_else(|_| unreachable!("third incarnation"))
                .get(),
            3
        );
    }

    #[test]
    fn repeated_native_identity_resolves_to_one_host_session_identity() {
        let native = DeviceId::dummy();
        let mut identities = DeviceIdentityMap::default();

        let first = identities
            .resolve(native)
            .unwrap_or_else(|_| unreachable!("first neutral device identity is available"));
        let repeated = identities
            .resolve(native)
            .unwrap_or_else(|_| unreachable!("known native device identity remains available"));

        assert_eq!(first.get(), 1);
        assert_eq!(repeated, first);
    }
}
