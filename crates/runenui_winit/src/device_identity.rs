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
}

#[cfg(test)]
mod tests {
    use super::{DeviceId, DeviceIdentityMap};

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
