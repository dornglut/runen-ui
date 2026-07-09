//! File: domain/ui/ui_surface/src/capability.rs
//! Purpose: Surface capability/trust classes for mount and intent boundaries.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SurfaceCapability {
    Observe,
    Interact,
    RequestMutation,
    Ratify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceCapabilitySet {
    observe: bool,
    interact: bool,
    request_mutation: bool,
    ratify: bool,
}

impl SurfaceCapabilitySet {
    pub const fn new(observe: bool, interact: bool, request_mutation: bool, ratify: bool) -> Self {
        Self {
            observe,
            interact,
            request_mutation,
            ratify,
        }
    }

    pub const fn allows(self, capability: SurfaceCapability) -> bool {
        match capability {
            SurfaceCapability::Observe => self.observe,
            SurfaceCapability::Interact => self.interact,
            SurfaceCapability::RequestMutation => self.request_mutation,
            SurfaceCapability::Ratify => self.ratify,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_set_checks_each_capability_class_explicitly() {
        let caps = SurfaceCapabilitySet::new(true, true, false, false);

        assert!(caps.allows(SurfaceCapability::Observe));
        assert!(caps.allows(SurfaceCapability::Interact));
        assert!(!caps.allows(SurfaceCapability::RequestMutation));
        assert!(!caps.allows(SurfaceCapability::Ratify));
    }
}
