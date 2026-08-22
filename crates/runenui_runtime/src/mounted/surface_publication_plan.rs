use runenui_core::{
    ChildLayout, HitContribution, HitContributionContext, PaintContribution,
    PaintContributionContext, WidgetDiagnostic, WidgetMeasure,
};

use super::{CachedCapability, DirtyPhases, MountedNodeId, MountedTree, node::state_is_corrupted};

pub(crate) struct SurfaceCapabilityPlan {
    owners: Vec<PlannedSurfaceCapabilities>,
    needs_paint: bool,
    needs_hit_test: bool,
}

struct PlannedSurfaceCapabilities {
    owner: MountedNodeId,
    measurement: Option<CachedCapability<WidgetMeasure>>,
    child_layout: Option<CachedCapability<Option<ChildLayout>>>,
    paint: Option<CachedCapability<PaintContribution>>,
    paint_context: Option<PaintContributionContext>,
    hit_test: Option<CachedCapability<HitContribution>>,
    hit_test_context: Option<HitContributionContext>,
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
    ) -> Option<PaintContribution> {
        self.owner_at(position, owner)
            .paint
            .as_ref()
            .and_then(CachedCapability::ready)
    }

    pub(crate) fn hit_test_at(
        &self,
        position: usize,
        owner: &MountedNodeId,
    ) -> Option<HitContribution> {
        self.owner_at(position, owner)
            .hit_test
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
    /// Stages capabilities that do not depend on final publication-local layout.
    /// Paint and hit callbacks are deferred until their final contexts exist so
    /// a ready cached contribution can be reused only when that exact context is
    /// unchanged.
    pub(crate) fn plan_surface_publication_capabilities(
        &self,
        phases: DirtyPhases,
    ) -> SurfaceCapabilityPlan {
        let needs_layout = phases.contains(DirtyPhases::LAYOUT);
        let needs_paint = phases.contains(DirtyPhases::PAINT);
        let needs_hit_test = phases.contains(DirtyPhases::HIT_TEST);
        let needs_diagnostics = phases.contains(DirtyPhases::DIAGNOSTICS);
        if !needs_layout && !needs_paint && !needs_hit_test && !needs_diagnostics {
            return SurfaceCapabilityPlan {
                owners: Vec::new(),
                needs_paint: false,
                needs_hit_test: false,
            };
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
                    paint_context: None,
                    hit_test: None,
                    hit_test_context: None,
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
                    if needs_hit_test {
                        planned.hit_test = Some(CachedCapability::StatePayloadMismatch);
                    }
                    if needs_diagnostics {
                        planned.diagnostics = Some(CachedCapability::StatePayloadMismatch);
                    }
                    planned.mark_integrity_failed =
                        needs_layout || needs_paint || needs_hit_test || needs_diagnostics;
                    return planned;
                }

                if needs_layout {
                    planned.measurement = Some(stage_cached_capability(
                        &node.caches.measurement,
                        || node.widget.measure(&node.state),
                        &mut planned.mark_integrity_failed,
                    ));
                    planned.child_layout = Some(stage_cached_capability(
                        &node.caches.child_layout,
                        || node.widget.child_layout(&node.state),
                        &mut planned.mark_integrity_failed,
                    ));
                }
                if needs_diagnostics {
                    planned.diagnostics = Some(stage_cached_capability(
                        &node.caches.diagnostics,
                        || node.widget.diagnostics(&node.state),
                        &mut planned.mark_integrity_failed,
                    ));
                }
                planned
            })
            .collect();
        SurfaceCapabilityPlan {
            owners,
            needs_paint,
            needs_hit_test,
        }
    }

    /// Stages contextual scene contributions after final layout/style facts exist.
    ///
    /// A ready contribution is reused when its complete owner-visible context is
    /// unchanged, even when the composed scene phase executes for another reason
    /// such as interaction invalidation. A changed local size/resolved style or
    /// an explicitly invalidated capability re-evaluates only that callback.
    pub(crate) fn plan_surface_publication_contributions(
        &self,
        plan: &mut SurfaceCapabilityPlan,
        paint_contexts: &[PaintContributionContext],
        hit_contexts: &[HitContributionContext],
    ) {
        if !plan.needs_paint && !plan.needs_hit_test {
            return;
        }
        debug_assert_eq!(plan.owners.len(), paint_contexts.len());
        debug_assert_eq!(plan.owners.len(), hit_contexts.len());
        for (position, planned) in plan.owners.iter_mut().enumerate() {
            if planned.paint.is_some() && planned.hit_test.is_some() {
                continue;
            }
            let node = self
                .node(&planned.owner)
                .unwrap_or_else(|| unreachable!("surface capability owner remains live"));
            if state_is_corrupted(node) {
                if plan.needs_paint && planned.paint.is_none() {
                    planned.paint = Some(CachedCapability::StatePayloadMismatch);
                }
                if plan.needs_hit_test && planned.hit_test.is_none() {
                    planned.hit_test = Some(CachedCapability::StatePayloadMismatch);
                }
                planned.mark_integrity_failed = true;
                continue;
            }
            if plan.needs_paint && planned.paint.is_none() {
                let context = paint_contexts[position];
                planned.paint = Some(stage_contextual_capability(
                    &node.caches.paint,
                    node.caches.paint_context,
                    context,
                    || node.widget.paint(&node.state, context),
                    &mut planned.mark_integrity_failed,
                ));
                planned.paint_context = Some(context);
            }
            if plan.needs_hit_test && planned.hit_test.is_none() {
                let context = hit_contexts[position];
                planned.hit_test = Some(stage_contextual_capability(
                    &node.caches.hit_test,
                    node.caches.hit_test_context,
                    context,
                    || node.widget.hit_test(&node.state, context),
                    &mut planned.mark_integrity_failed,
                ));
                planned.hit_test_context = Some(context);
            }
        }
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
                node.caches.paint_context = planned.paint_context;
            }
            if let Some(hit_test) = planned.hit_test {
                node.caches.hit_test = hit_test;
                node.caches.hit_test_context = planned.hit_test_context;
            }
            if let Some(diagnostics) = planned.diagnostics {
                node.caches.diagnostics = diagnostics;
            }
            node.integrity_failed |= planned.mark_integrity_failed;
        }
    }
}

fn stage_cached_capability<T: Clone, E>(
    cached: &CachedCapability<T>,
    resolve: impl FnOnce() -> Result<T, E>,
    mark_integrity_failed: &mut bool,
) -> CachedCapability<T> {
    match cached {
        CachedCapability::Unresolved => stage_fresh_capability(resolve, mark_integrity_failed),
        cached => cached.clone(),
    }
}

fn stage_contextual_capability<T: Clone, Context: Copy + PartialEq, E>(
    cached: &CachedCapability<T>,
    cached_context: Option<Context>,
    context: Context,
    resolve: impl FnOnce() -> Result<T, E>,
    mark_integrity_failed: &mut bool,
) -> CachedCapability<T> {
    match cached {
        CachedCapability::Ready(value) if cached_context == Some(context) => {
            CachedCapability::Ready(value.clone())
        }
        CachedCapability::StatePayloadMismatch => CachedCapability::StatePayloadMismatch,
        CachedCapability::Unresolved | CachedCapability::Ready(_) => {
            stage_fresh_capability(resolve, mark_integrity_failed)
        }
    }
}

fn stage_fresh_capability<T, E>(
    resolve: impl FnOnce() -> Result<T, E>,
    mark_integrity_failed: &mut bool,
) -> CachedCapability<T> {
    resolve().map_or_else(
        |_| {
            *mark_integrity_failed = true;
            CachedCapability::StatePayloadMismatch
        },
        CachedCapability::Ready,
    )
}
