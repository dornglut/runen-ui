use runenui_core::{ChildLayout, WidgetDiagnostic, WidgetMeasure, WidgetPaintProof};

use super::{CachedCapability, DirtyPhases, MountedNodeId, MountedTree, node::state_is_corrupted};

pub(crate) struct SurfaceCapabilityPlan {
    owners: Vec<PlannedSurfaceCapabilities>,
}

struct PlannedSurfaceCapabilities {
    owner: MountedNodeId,
    measurement: Option<CachedCapability<WidgetMeasure>>,
    child_layout: Option<CachedCapability<Option<ChildLayout>>>,
    paint: Option<CachedCapability<WidgetPaintProof>>,
    diagnostics: Option<CachedCapability<Vec<WidgetDiagnostic>>>,
    mark_integrity_failed: bool,
}

impl SurfaceCapabilityPlan {
    pub(crate) fn measurement_at(
        &self,
        position: usize,
        owner: &MountedNodeId,
    ) -> Option<WidgetMeasure> {
        self.owner_at(position, owner)
            .measurement
            .as_ref()
            .and_then(CachedCapability::ready)
    }

    pub(crate) fn child_layout_at_or_else(
        &self,
        position: usize,
        owner: &MountedNodeId,
        fallback: impl FnOnce() -> Option<ChildLayout>,
    ) -> Option<ChildLayout> {
        self.owner_at(position, owner)
            .child_layout
            .as_ref()
            .and_then(CachedCapability::ready)
            .unwrap_or_else(fallback)
    }

    pub(crate) fn paint_at(
        &self,
        position: usize,
        owner: &MountedNodeId,
    ) -> Option<WidgetPaintProof> {
        self.owner_at(position, owner)
            .paint
            .as_ref()
            .and_then(CachedCapability::ready)
    }

    pub(crate) fn diagnostics_at(
        &self,
        position: usize,
        owner: &MountedNodeId,
    ) -> Option<Vec<WidgetDiagnostic>> {
        self.owner_at(position, owner)
            .diagnostics
            .as_ref()
            .and_then(CachedCapability::ready)
    }

    fn owner_at(&self, position: usize, owner: &MountedNodeId) -> &PlannedSurfaceCapabilities {
        let planned = self
            .owners
            .get(position)
            .unwrap_or_else(|| unreachable!("surface capability plan stays topology-aligned"));
        debug_assert_eq!(
            &planned.owner, owner,
            "surface capability plan owner stays topology-aligned"
        );
        planned
    }
}

impl<Action> MountedTree<Action> {
    pub(crate) fn plan_surface_publication_capabilities(
        &self,
        phases: DirtyPhases,
    ) -> SurfaceCapabilityPlan {
        let needs_layout = phases.contains(DirtyPhases::LAYOUT);
        let needs_paint = phases.contains(DirtyPhases::PAINT);
        let needs_diagnostics = phases.contains(DirtyPhases::DIAGNOSTICS);
        if !needs_layout && !needs_paint && !needs_diagnostics {
            return SurfaceCapabilityPlan { owners: Vec::new() };
        }
        let owners = self
            .publication_preorder_ids()
            .into_iter()
            .map(|owner| {
                let node = self
                    .node(&owner)
                    .unwrap_or_else(|| unreachable!("surface capability owner remains live"));
                let mut planned = PlannedSurfaceCapabilities {
                    owner,
                    measurement: None,
                    child_layout: None,
                    paint: None,
                    diagnostics: None,
                    mark_integrity_failed: false,
                };
                if state_is_corrupted(node) {
                    if needs_layout {
                        planned.measurement = Some(CachedCapability::StatePayloadMismatch);
                        planned.child_layout = Some(CachedCapability::StatePayloadMismatch);
                    }
                    if needs_paint {
                        planned.paint = Some(CachedCapability::StatePayloadMismatch);
                    }
                    if needs_diagnostics {
                        planned.diagnostics = Some(CachedCapability::StatePayloadMismatch);
                    }
                    planned.mark_integrity_failed =
                        needs_layout || needs_paint || needs_diagnostics;
                    return planned;
                }

                if needs_layout {
                    planned.measurement = Some(stage_capability(
                        &node.caches.measurement,
                        || node.widget.measure(&node.state),
                        &mut planned.mark_integrity_failed,
                    ));
                    planned.child_layout = Some(stage_capability(
                        &node.caches.child_layout,
                        || node.widget.child_layout(&node.state),
                        &mut planned.mark_integrity_failed,
                    ));
                }
                if needs_paint {
                    planned.paint = Some(stage_capability(
                        &node.caches.paint,
                        || node.widget.paint(&node.state),
                        &mut planned.mark_integrity_failed,
                    ));
                }
                if needs_diagnostics {
                    planned.diagnostics = Some(stage_capability(
                        &node.caches.diagnostics,
                        || node.widget.diagnostics(&node.state),
                        &mut planned.mark_integrity_failed,
                    ));
                }
                planned
            })
            .collect();
        SurfaceCapabilityPlan { owners }
    }

    pub(crate) fn commit_surface_publication_capabilities(&mut self, plan: SurfaceCapabilityPlan) {
        for planned in plan.owners {
            let node = self
                .node_mut(&planned.owner)
                .unwrap_or_else(|| unreachable!("planned surface capability owner remains live"));
            if let Some(measurement) = planned.measurement {
                node.caches.measurement = measurement;
            }
            if let Some(child_layout) = planned.child_layout {
                node.caches.child_layout = child_layout;
            }
            if let Some(paint) = planned.paint {
                node.caches.paint = paint;
            }
            if let Some(diagnostics) = planned.diagnostics {
                node.caches.diagnostics = diagnostics;
            }
            node.integrity_failed |= planned.mark_integrity_failed;
        }
    }
}

fn stage_capability<T: Clone, E>(
    cached: &CachedCapability<T>,
    resolve: impl FnOnce() -> Result<T, E>,
    mark_integrity_failed: &mut bool,
) -> CachedCapability<T> {
    match cached {
        CachedCapability::Unresolved => resolve().map_or_else(
            |_| {
                *mark_integrity_failed = true;
                CachedCapability::StatePayloadMismatch
            },
            CachedCapability::Ready,
        ),
        cached => cached.clone(),
    }
}
