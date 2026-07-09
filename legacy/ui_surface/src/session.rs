//! File: domain/ui/ui_surface/src/session.rs
//! Purpose: Session-scope handle and retention-class contracts.

use crate::SurfaceInstanceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRetentionClass {
    Ephemeral,
    Restorable,
    Persistent,
    Shareable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionScopeHandle {
    pub surface_instance_id: SurfaceInstanceId,
    pub scope_id: u64,
    pub retention_class: SessionRetentionClass,
}

impl SessionScopeHandle {
    pub const fn new(
        surface_instance_id: SurfaceInstanceId,
        scope_id: u64,
        retention_class: SessionRetentionClass,
    ) -> Self {
        Self {
            surface_instance_id,
            scope_id,
            retention_class,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_scope_handle_keeps_surface_and_retention_class_explicit() {
        let handle = SessionScopeHandle::new(
            SurfaceInstanceId::new(7),
            21,
            SessionRetentionClass::Restorable,
        );

        assert_eq!(handle.surface_instance_id, SurfaceInstanceId::new(7));
        assert_eq!(handle.scope_id, 21);
        assert_eq!(handle.retention_class, SessionRetentionClass::Restorable);
    }
}
