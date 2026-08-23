#![allow(refining_impl_trait)]

use runenui_core::{
    Element, HitContribution, HitContributionContext, HitRegion, LogicalLength, LogicalPoint,
    LogicalRect, LogicalTransform, NoHostProtocol, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, PointerPolicy, StyleTokens, UiApp,
    View, Widget, WidgetMeasure, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, SurfaceBuildContext,
    SurfacePublication, TraceRecordKind, WorkSequence,
};

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
        .unwrap_or_else(|_| unreachable!("test rectangle is valid"))
}

fn point() -> LogicalPoint {
    LogicalPoint::new(5.0, 5.0).unwrap_or_else(|_| unreachable!("test point is finite"))
}

#[derive(Debug)]
struct LowerTarget;

impl Widget<()> for LowerTarget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(20_u16),
        }
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        HitContribution::new(vec![HitRegion::rect(rect())])
    }
}

#[derive(Clone, Copy, Debug)]
enum UpperMode {
    Singular,
    Block,
}

#[derive(Debug)]
struct UpperRegion(UpperMode);

impl Widget<()> for UpperRegion {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(20_u16),
        }
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let region = match self.0 {
            UpperMode::Singular => {
                let singular_overlay = LogicalTransform::try_new(0.0, 0.0, 0.0, 1.0, 0.0, -20.0)
                    .unwrap_or_else(|_| unreachable!("test singular transform is finite"));
                HitRegion::rect(rect()).with_transform(singular_overlay)
            }
            UpperMode::Block => {
                let overlay = LogicalTransform::translation(0.0, -20.0)
                    .unwrap_or_else(|_| unreachable!("test translation is finite"));
                HitRegion::rect(rect())
                    .with_transform(overlay)
                    .with_pointer_policy(PointerPolicy::Block)
            }
        };
        HitContribution::new(vec![region])
    }
}

#[derive(Debug)]
struct State {
    mode: UpperMode,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        column(vec![
            Element::new(LowerTarget).id("lower").key("lower"),
            Element::new(UpperRegion(state.mode))
                .id("upper")
                .key("upper"),
        ])
        .key("root")
        .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn drain_mount(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );
}

fn publish(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let tokens = StyleTokens::new();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("pointer trace fixture publication is admitted"))
}

fn authored_target(publication: &SurfacePublication, authored: &str) -> MountedNodeId {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored fixture node is published"))
        .id()
        .clone()
}

fn submit_down(runtime: &mut AppRuntime<App>, publication: &SurfacePublication) -> WorkSequence {
    let pointer_id =
        PointerId::new(1).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point(),
        publication.input_context().clone(),
    )
    .with_changed_button(PointerButton::Primary)
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    let sequence = runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("displayed pointer event is admitted"))
        .sequence();
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    sequence
}

#[test]
fn singular_top_region_falls_through_to_next_region_and_pointer_trace_names_that_target() {
    let mut runtime = AppRuntime::<App>::mount(State {
        mode: UpperMode::Singular,
    });
    drain_mount(&mut runtime);
    let publication = publish(&mut runtime);
    let lower = authored_target(&publication, "lower");
    let upper = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "upper"))
        .unwrap_or_else(|| unreachable!("upper fixture node is published"));

    assert!(
        upper.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "runenui.scene.hit-transform-non-invertible"
        })
    );
    assert_eq!(
        publication.hit_test_scene().target_at(point()),
        Some(&lower)
    );

    let sequence = submit_down(&mut runtime, &publication);
    let resolved = runtime
        .trace()
        .records()
        .find(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerPhysicalTargetResolved
                )
        })
        .unwrap_or_else(|| unreachable!("pointer physical resolution is traced"));
    assert_eq!(
        resolved.target().map(|target| target.mounted_node_id()),
        Some(&lower)
    );
    assert!(runtime.trace().records().any(|record| {
        record.work_sequence() == Some(sequence)
            && matches!(record.kind(), TraceRecordKind::RoutedEventStarted)
    }));
    assert!(runtime.trace().records().any(|record| {
        record.work_sequence() == Some(sequence)
            && matches!(
                record.kind(),
                TraceRecordKind::PointerDefaultSuppressed {
                    phase: PointerPhase::Down,
                    ..
                }
            )
    }));
}

#[test]
fn block_region_occludes_lower_target_and_pointer_trace_proves_no_route() {
    let mut runtime = AppRuntime::<App>::mount(State {
        mode: UpperMode::Block,
    });
    drain_mount(&mut runtime);
    let publication = publish(&mut runtime);

    assert_eq!(publication.hit_test_scene().target_at(point()), None);
    assert!(publication.hit_test_scene().regions().iter().any(|region| {
        region.pointer_policy() == PointerPolicy::Block && region.contains_surface_point(point())
    }));

    let sequence = submit_down(&mut runtime, &publication);
    let resolved = runtime
        .trace()
        .records()
        .find(|record| {
            record.work_sequence() == Some(sequence)
                && matches!(
                    record.kind(),
                    TraceRecordKind::PointerPhysicalTargetResolved
                )
        })
        .unwrap_or_else(|| unreachable!("blocked physical resolution is traced"));
    assert!(resolved.target().is_none());
    assert!(!runtime.trace().records().any(|record| {
        record.work_sequence() == Some(sequence)
            && matches!(record.kind(), TraceRecordKind::RoutedEventStarted)
    }));
    assert!(runtime.trace().records().any(|record| {
        record.work_sequence() == Some(sequence)
            && matches!(
                record.kind(),
                TraceRecordKind::PointerInteractionCommitted { .. }
            )
    }));
}
