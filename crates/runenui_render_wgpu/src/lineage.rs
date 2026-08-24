use runenui_core::SurfaceId;
use runenui_runtime::{PaintDamage, PaintPublication, PaintRevision};

/// How one supplied publication relates to the renderer's last successful realization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationUpdateMode {
    /// No exact predecessor is available; rebuild from the complete publication.
    FullResync,
    /// The publication's declared base is exactly the last successfully realized revision.
    ExactBaseMatch,
    /// The exact surface/revision is already realized.
    AlreadyCurrent,
}

/// Renderer-owned plan for consuming one complete paint publication.
///
/// Relative damage is exposed only for [`PublicationUpdateMode::ExactBaseMatch`].
/// A full resync must reconstruct from the complete publication without consuming
/// base-relative damage, while an already-current publication requires no update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PublicationUpdatePlan {
    mode: PublicationUpdateMode,
    incremental_damage: Option<PaintDamage>,
}

impl PublicationUpdatePlan {
    #[must_use]
    const fn new(mode: PublicationUpdateMode, incremental_damage: Option<PaintDamage>) -> Self {
        Self {
            mode,
            incremental_damage,
        }
    }

    pub(crate) const fn full_resync() -> Self {
        Self::new(PublicationUpdateMode::FullResync, None)
    }

    /// Returns how the publication relates to successfully realized renderer state.
    #[must_use]
    pub const fn mode(self) -> PublicationUpdateMode {
        self.mode
    }

    /// Returns base-relative damage only when the exact realized predecessor matches.
    #[must_use]
    pub const fn incremental_damage(self) -> Option<PaintDamage> {
        self.incremental_damage
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RealizedPublication {
    surface_id: SurfaceId,
    revision: PaintRevision,
}

/// Renderer-owned successful-realization lineage.
///
/// Classification and planning never mutate state. Call [`Self::record_success`]
/// only after the renderer has successfully realized the supplied publication.
/// Failed work therefore cannot accidentally become predecessor authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PublicationLineage {
    realized: Option<RealizedPublication>,
}

impl PublicationLineage {
    /// Creates empty renderer lineage.
    #[must_use]
    pub const fn new() -> Self {
        Self { realized: None }
    }

    /// Classifies one complete public paint publication against successful renderer state.
    #[must_use]
    pub fn classify(&self, publication: &PaintPublication) -> PublicationUpdateMode {
        let Some(realized) = self.realized.as_ref() else {
            return PublicationUpdateMode::FullResync;
        };

        if &realized.surface_id == publication.surface_id()
            && realized.revision == publication.revision()
        {
            PublicationUpdateMode::AlreadyCurrent
        } else if &realized.surface_id == publication.surface_id()
            && publication.base_revision() == Some(realized.revision)
        {
            PublicationUpdateMode::ExactBaseMatch
        } else {
            PublicationUpdateMode::FullResync
        }
    }

    /// Plans one renderer update while gating base-relative damage behind an exact predecessor.
    #[must_use]
    pub fn plan(&self, publication: &PaintPublication) -> PublicationUpdatePlan {
        let mode = self.classify(publication);
        let incremental_damage =
            (mode == PublicationUpdateMode::ExactBaseMatch).then_some(publication.damage());
        PublicationUpdatePlan::new(mode, incremental_damage)
    }

    /// Records one publication as successfully realized by the renderer.
    pub fn record_success(&mut self, publication: &PaintPublication) {
        self.realized = Some(RealizedPublication {
            surface_id: publication.surface_id().clone(),
            revision: publication.revision(),
        });
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{IntoEffects, LogicalSize, NoHostProtocol, StyleTokens, UiApp, View, text};
    use runenui_runtime::{
        AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SurfaceBuildContext,
    };

    use super::{PublicationLineage, PublicationUpdateMode};

    struct App;

    impl UiApp for App {
        type State = ();
        type Action = ();
        type HostProtocol = NoHostProtocol;

        fn root((): &Self::State) -> impl View<Self::Action> {
            text("renderer lineage")
        }

        fn update(
            (): &mut Self::State,
            (): Self::Action,
        ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        }
    }

    fn publication(
        runtime: &mut AppRuntime<App>,
        tokens: &StyleTokens,
        scale: f32,
    ) -> PaintPublication {
        let size = LogicalSize::try_new(32.0, 24.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is valid"));
        let raster_scale = RasterScale::new(scale)
            .unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
        let context = SurfaceBuildContext::new(tokens, LayoutConstraints::tight(size))
            .with_raster_scale(raster_scale);
        runtime
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
            .paint_publication()
            .clone()
    }

    #[test]
    fn lineage_uses_only_successfully_recorded_exact_predecessor_state() {
        let tokens = StyleTokens::new();
        let mut runtime = AppRuntime::<App>::mount(());
        let first = publication(&mut runtime, &tokens, 1.0);
        let second = publication(&mut runtime, &tokens, 2.0);
        let third = publication(&mut runtime, &tokens, 3.0);

        let mut lineage = PublicationLineage::new();
        assert_eq!(lineage.classify(&first), PublicationUpdateMode::FullResync);

        lineage.record_success(&first);
        assert_eq!(
            lineage.classify(&first),
            PublicationUpdateMode::AlreadyCurrent
        );
        assert_eq!(
            lineage.classify(&second),
            PublicationUpdateMode::ExactBaseMatch
        );

        assert_eq!(
            lineage.classify(&third),
            PublicationUpdateMode::FullResync,
            "an unrecorded intermediate publication cannot become renderer predecessor authority"
        );

        lineage.record_success(&third);
        assert_eq!(
            lineage.classify(&third),
            PublicationUpdateMode::AlreadyCurrent
        );
    }

    #[test]
    fn update_plan_exposes_damage_only_for_exact_predecessor() {
        let tokens = StyleTokens::new();
        let mut runtime = AppRuntime::<App>::mount(());
        let first = publication(&mut runtime, &tokens, 1.0);
        let second = publication(&mut runtime, &tokens, 2.0);
        let third = publication(&mut runtime, &tokens, 3.0);

        let mut lineage = PublicationLineage::new();
        let first_plan = lineage.plan(&first);
        assert_eq!(first_plan.mode(), PublicationUpdateMode::FullResync);
        assert_eq!(first_plan.incremental_damage(), None);

        lineage.record_success(&first);
        let current_plan = lineage.plan(&first);
        assert_eq!(current_plan.mode(), PublicationUpdateMode::AlreadyCurrent);
        assert_eq!(current_plan.incremental_damage(), None);

        let contiguous_plan = lineage.plan(&second);
        assert_eq!(
            contiguous_plan.mode(),
            PublicationUpdateMode::ExactBaseMatch
        );
        assert_eq!(contiguous_plan.incremental_damage(), Some(second.damage()));

        let skipped_plan = lineage.plan(&third);
        assert_eq!(skipped_plan.mode(), PublicationUpdateMode::FullResync);
        assert_eq!(skipped_plan.incremental_damage(), None);
    }

    #[test]
    fn foreign_surface_never_matches_realized_lineage() {
        let tokens = StyleTokens::new();
        let mut first_runtime = AppRuntime::<App>::mount(());
        let mut second_runtime = AppRuntime::<App>::mount(());
        let first = publication(&mut first_runtime, &tokens, 1.0);
        let foreign = publication(&mut second_runtime, &tokens, 1.0);

        let mut lineage = PublicationLineage::new();
        lineage.record_success(&first);

        assert_ne!(first.surface_id(), foreign.surface_id());
        assert_eq!(
            lineage.classify(&foreign),
            PublicationUpdateMode::FullResync
        );
        let foreign_plan = lineage.plan(&foreign);
        assert_eq!(foreign_plan.mode(), PublicationUpdateMode::FullResync);
        assert_eq!(foreign_plan.incremental_damage(), None);
    }
}
