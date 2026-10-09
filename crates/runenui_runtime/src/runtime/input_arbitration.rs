//! Read-only projection of the **existing** live focus/input/presentation authority.

use runenui_core::{HostProtocol, WidgetTextInput, __runtime::RuntimeNamespace};

use crate::{
    InputArbitrationScope, InputObservationError, InputOwnershipRevision, InputOwnershipSnapshot,
    KeyboardInputOwnership, PointerInputOwnership, SurfaceInputOwnership,
};

use super::Runtime;

/// Derived cached projection for change detection; never a writable focus/input authority.
pub(super) struct InputObservationState {
    scope: InputArbitrationScope,
    revision: InputOwnershipRevision,
    last: Option<InputOwnershipSnapshot>,
}

impl InputObservationState {
    pub(super) fn new(namespace: RuntimeNamespace) -> Self {
        Self {
            scope: InputArbitrationScope::new(namespace),
            revision: InputOwnershipRevision::new(1),
            last: None,
        }
    }
}

/// Fallible copy of immutable observations; no unbounded or unchecked Vec clone.
fn copy_snapshot(
    source: &InputOwnershipSnapshot,
) -> Result<InputOwnershipSnapshot, InputObservationError> {
    let mut pointers = Vec::new();
    pointers
        .try_reserve_exact(source.pointers.len())
        .map_err(|_| InputObservationError::Capacity)?;
    pointers.extend_from_slice(&source.pointers);
    let mut surfaces = Vec::new();
    surfaces
        .try_reserve_exact(source.surfaces.len())
        .map_err(|_| InputObservationError::Capacity)?;
    surfaces.extend_from_slice(&source.surfaces);
    Ok(InputOwnershipSnapshot {
        scope: source.scope.clone(),
        revision: source.revision,
        status: source.status,
        keyboard: source.keyboard.clone(),
        surfaces,
        pointers,
    })
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    /// Returns a new immutable projection without changing any live authority.
    fn project_input_ownership(
        &mut self,
    ) -> Result<InputOwnershipSnapshot, InputObservationError> {
        let focused_node = self.focus.focused_node().cloned();
        let text_input_capability = focused_node
            .as_ref()
            .and_then(|target| self.tree.text_input_probe(target).ok())
            .unwrap_or(WidgetTextInput::NONE);
        let keyboard = KeyboardInputOwnership {
            focused_node,
            text_input_capability,
            composition_generation: self.composition.generation().cloned(),
            space_activation_owner: self.space_ownership.as_ref().map(|x| x.target.clone()),
        };
        let ids = self.pointer_registry.ordered_pointer_ids();
        let mut pointers = Vec::new();
        pointers
            .try_reserve_exact(ids.len())
            .map_err(|_| InputObservationError::Capacity)?;
        for id in ids {
            let stream = self
                .pointer_registry
                .stream(id)
                .unwrap_or_else(|| unreachable!("ordered pointer is active"));
            pointers.push(PointerInputOwnership {
                pointer_id: id,
                device_id: stream.device_id(),
                surface_id: stream.surface().clone(),
                pressed_owner: stream.pressed_owner().cloned(),
                capture_owner: stream.capture_owner().cloned(),
            });
        }
        // This is only the retained logical RunenUI scene: never native Present.
        let modal_blocker = self
            .surface_publication
            .current_presentation_interaction_roots()
            .into_iter()
            .find(|entry| entry.presentation.is_modal())
            .map(|entry| entry.root);
        let mut surfaces = Vec::new();
        surfaces
            .try_reserve_exact(1)
            .map_err(|_| InputObservationError::Capacity)?;
        surfaces.push(SurfaceInputOwnership {
            surface_id: self.surface_publication.surface_id().clone(),
            latest_retained_context: self.surface_publication.current_surface_input_context(),
            modal_blocker,
        });
        Ok(InputOwnershipSnapshot {
            scope: self.input_observation.scope.clone(),
            revision: self.input_observation.revision,
            status: self.status,
            keyboard,
            surfaces,
            pointers,
        })
    }

    pub(crate) fn input_ownership(
        &mut self,
    ) -> Result<InputOwnershipSnapshot, InputObservationError> {
        let mut current = self.project_input_ownership()?;
        if let Some(previous) = &self.input_observation.last {
            if !previous.same_ownership_facts(&current) {
                let revision = self
                    .input_observation
                    .revision
                    .get()
                    .checked_add(1)
                    .ok_or(InputObservationError::RevisionExhausted)?;
                current.revision = InputOwnershipRevision::new(revision);
            }
        }
        // Construct all caller/retained projections **before** committing a
        // new revision. A failed capacity reservation does not partially publish.
        let retained = copy_snapshot(&current)?;
        let result = copy_snapshot(&current)?;
        self.input_observation.revision = current.revision;
        self.input_observation.last = Some(retained);
        Ok(result)
    }
}
