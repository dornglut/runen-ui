use std::sync::Arc;

use crate::{
    RedrawAcknowledgeError, RedrawRequest, SurfaceBuildContext, SurfacePhase, SurfacePhaseReport,
    SurfacePublication,
    mounted::MountedTree,
    surface::{SurfaceCache, publish_mounted_surface_cached},
};

/// Sole runtime-owned state for current surface publication and redraw revision.
///
/// This authority intentionally retains only the current publication cache. M4C2
/// may extend this boundary with bounded displayed-generation history, but this
/// prerequisite does not create that history or any public surface identity.
pub(crate) struct SurfacePublicationState {
    cache: Option<SurfaceCache>,
    phase_report: SurfacePhaseReport,
    redraw_namespace: Arc<()>,
    redraw_revision: u64,
    redraw_acknowledged: u64,
}

impl SurfacePublicationState {
    pub(crate) fn new() -> Self {
        Self {
            cache: None,
            phase_report: SurfacePhaseReport::default(),
            redraw_namespace: Arc::new(()),
            redraw_revision: 1,
            redraw_acknowledged: 0,
        }
    }

    pub(crate) fn publish<Action>(
        &mut self,
        tree: &mut MountedTree<Action>,
        context: &SurfaceBuildContext<'_>,
    ) -> SurfacePublication {
        let (publication, report) = publish_mounted_surface_cached(tree, context, &mut self.cache);
        self.phase_report = report;
        publication
    }

    pub(crate) fn note_focus_validation(&mut self) {
        self.phase_report = SurfacePhaseReport::one(SurfacePhase::FocusValidation);
    }

    pub(crate) const fn phase_report(&self) -> &SurfacePhaseReport {
        &self.phase_report
    }

    pub(crate) fn clear_cache(&mut self) {
        self.cache = None;
    }

    pub(crate) fn request_redraw(&mut self) -> Option<u64> {
        let next = self.redraw_revision.checked_add(1)?;
        self.redraw_revision = next;
        Some(next)
    }

    pub(crate) fn take_redraw_request(&self) -> Option<RedrawRequest> {
        (self.redraw_revision > self.redraw_acknowledged).then(|| RedrawRequest {
            namespace: Arc::clone(&self.redraw_namespace),
            revision: self.redraw_revision,
        })
    }

    pub(crate) fn acknowledge_redraw(
        &mut self,
        request: &RedrawRequest,
    ) -> Result<(), RedrawAcknowledgeError> {
        if !Arc::ptr_eq(&self.redraw_namespace, &request.namespace) {
            return Err(RedrawAcknowledgeError::ForeignRuntime);
        }
        if request.revision > self.redraw_revision {
            return Err(RedrawAcknowledgeError::FutureRevision);
        }
        self.redraw_acknowledged = self.redraw_acknowledged.max(request.revision);
        Ok(())
    }

    pub(crate) const fn is_dirty(&self) -> bool {
        self.redraw_revision > self.redraw_acknowledged
    }
}
