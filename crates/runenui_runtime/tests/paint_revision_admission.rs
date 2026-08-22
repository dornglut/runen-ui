#![allow(refining_impl_trait)]
#![cfg(feature = "internal-test-seams")]

use runenui_core::{LogicalLength, NoHostProtocol, StyleTokens, UiApp, View, text};
use runenui_runtime::{
    AppRuntime, LogicalSize, PublishSurfaceError, RuntimeStatus, RuntimeTerminalReason,
    SurfaceBuildContext, SurfacePublicationCounter, TraceRecordKind,
};

struct App;

impl UiApp for App {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("probe")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn size(width: u16, height: u16) -> LogicalSize {
    LogicalSize::new(LogicalLength::from(width), LogicalLength::from(height))
}

fn published_count(runtime: &AppRuntime<App>) -> usize {
    runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::SurfacePublished))
        .count()
}

#[test]
fn exhausted_paint_revision_does_not_block_unchanged_renderer_publication() {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::tight(&tokens, size(16, 16));
    let mut runtime = AppRuntime::<App>::mount(());
    let first = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("initial paint publication is admitted"));
    let published_before = published_count(&runtime);

    runtime.__seed_next_paint_revision_for_test(None);
    let second = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("unchanged paint reuses its accepted publication"));

    assert_eq!(second.paint_publication(), first.paint_publication());
    assert_eq!(
        second.paint_publication().revision(),
        first.paint_publication().revision()
    );
    assert_eq!(published_count(&runtime), published_before + 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn paint_revision_max_is_issued_once_then_changed_publication_fails_before_commit() {
    let tokens = StyleTokens::new();
    let mut runtime = AppRuntime::<App>::mount(());
    runtime
        .publish_surface(&SurfaceBuildContext::tight(&tokens, size(16, 16)))
        .unwrap_or_else(|_| unreachable!("initial paint publication is admitted"));

    runtime.__seed_next_paint_revision_for_test(Some(u64::MAX));
    let max_revision = runtime
        .publish_surface(&SurfaceBuildContext::tight(&tokens, size(32, 16)))
        .unwrap_or_else(|_| unreachable!("the final non-wrapping paint revision is admitted"));
    assert_eq!(max_revision.paint_publication().revision().get(), u64::MAX);
    assert_eq!(
        max_revision.paint_publication().logical_size(),
        size(32, 16)
    );

    let phase_before_failure = runtime.last_surface_phase_report().clone();
    let published_before_failure = published_count(&runtime);
    let result = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size(48, 16)));
    let reason = RuntimeTerminalReason::SurfacePublicationCounterExhausted(
        SurfacePublicationCounter::PaintRevision,
    );

    assert_eq!(
        result.as_ref().err(),
        Some(&PublishSurfaceError::Terminal(reason))
    );
    assert_eq!(runtime.status(), RuntimeStatus::Terminal(reason));
    assert_eq!(published_count(&runtime), published_before_failure);
    assert_eq!(runtime.last_surface_phase_report(), &phase_before_failure);
}