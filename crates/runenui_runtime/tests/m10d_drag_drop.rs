#![allow(refining_impl_trait)]
#![allow(clippy::panic, clippy::too_many_lines)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    DragDropEvent, DragDropPayloadKind, DragDropPayloadMetadata, DragDropPhase, Element, ElementId,
    EventContext, FrameworkServiceRequest, FrameworkServiceResponse, HitContribution,
    HitContributionContext, LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    StyleEnvironment, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    WidgetMeasure, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext,
    TraceFrameworkServiceKind, TraceFrameworkServiceOutcome, TraceRecordKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct DropObservation {
    target: &'static str,
    phase: DragDropPhase,
    sequence: u64,
    current_target: MountedNodeId,
    physical_target: Option<MountedNodeId>,
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<DropObservation>>>,
    accept_drop: bool,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        row(children![
            Element::new(DropArea {
                name: "left",
                capture_on_down: true,
                accept_drop: state.accept_drop,
                observations: Rc::clone(&state.observations),
            })
            .id("left")
            .key("left"),
            Element::new(DropArea {
                name: "right",
                capture_on_down: false,
                accept_drop: state.accept_drop,
                observations: Rc::clone(&state.observations),
            })
            .id("right")
            .key("right"),
        ])
        .id("root")
        .key("root")
    }

    fn update(_state: &mut Self::State, (): Self::Action) {}
}

#[derive(Debug)]
struct DropArea {
    name: &'static str,
    capture_on_down: bool,
    accept_drop: bool,
    observations: Rc<RefCell<Vec<DropObservation>>>,
}

impl Widget<()> for DropArea {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::Pointer(pointer)
                if self.capture_on_down && pointer.phase() == PointerPhase::Down =>
            {
                context.capture_pointer();
            }
            UiEvent::DragDrop(drop) => {
                self.observations.borrow_mut().push(DropObservation {
                    target: self.name,
                    phase: drop.phase(),
                    sequence: context.sequence().get(),
                    current_target: context.current_target().clone(),
                    physical_target: context.physical_target().cloned(),
                });
                if self.accept_drop && drop.phase() == DragDropPhase::Drop {
                    context.accept_drag_drop();
                }
            }
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn measure(
        &self,
        _state: &Self::State,
        _input: runenui_core::WidgetMeasureInput,
    ) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::new(32.0).unwrap_or_default(),
            LogicalLength::new(32.0).unwrap_or_default(),
        )
    }

    fn hit_test(&self, _state: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let bounds = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("the fixture hit region is valid"));
        HitContribution::single_rect(bounds)
    }
}

struct Harness {
    runtime: AppRuntime<App>,
    surface: SurfaceInputContext,
    right: MountedNodeId,
    left_point: LogicalPoint,
    right_point: LogicalPoint,
    observations: Rc<RefCell<Vec<DropObservation>>>,
}

fn harness(accept_drop: bool) -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observations: Rc::clone(&observations),
        accept_drop,
    });
    let style = StyleEnvironment::default();
    let size = LogicalSize::try_new(96.0, 48.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&style, size))
        .unwrap_or_else(|error| panic!("surface publication is accepted: {error:?}"));
    let left_id = ElementId::new("left").unwrap_or_else(|_| unreachable!("valid element id"));
    let right_id = ElementId::new("right").unwrap_or_else(|_| unreachable!("valid element id"));
    let left = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&left_id))
        .unwrap_or_else(|| unreachable!("left target is mounted"));
    let right = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&right_id))
        .unwrap_or_else(|| unreachable!("right target is mounted"));
    let center = |bounds: LogicalRect| {
        LogicalPoint::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        )
        .unwrap_or_else(|_| unreachable!("published bounds are finite"))
    };
    runtime.pump(budget());
    Harness {
        runtime,
        surface: publication.input_context().clone(),
        right: right.id().clone(),
        left_point: center(left.bounds()),
        right_point: center(right.bounds()),
        observations,
    }
}

const fn budget() -> PumpBudget {
    PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

fn pointer(harness: &Harness, phase: PointerPhase, point: LogicalPoint) -> PointerEvent {
    let pointer = PointerEvent::new(
        PointerId::new(51).unwrap_or_else(|| unreachable!("non-zero pointer id")),
        PointerDeviceKind::Mouse,
        phase,
        point,
        harness.surface.clone(),
    );
    match phase {
        PointerPhase::Down => pointer
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        PointerPhase::Move => pointer.with_buttons(PointerButtons::new([PointerButton::Primary])),
        _ => pointer,
    }
}

fn drop_event(harness: &Harness, phase: DragDropPhase, point: LogicalPoint) -> PointerEvent {
    pointer(harness, PointerPhase::Move, point).with_drag_drop(DragDropEvent::new(
        phase,
        DragDropPayloadMetadata::new(DragDropPayloadKind::Files, core::num::NonZeroU32::MIN, None),
    ))
}

#[test]
fn accepted_drop_uses_the_physical_hit_target_not_the_captured_pointer_owner() {
    let mut harness = harness(true);
    harness
        .runtime
        .submit_pointer(pointer(&harness, PointerPhase::Down, harness.left_point))
        .unwrap_or_else(|error| panic!("pointer down is admitted: {error}"));
    harness.runtime.pump(budget());

    let submission = harness
        .runtime
        .submit_pointer(drop_event(
            &harness,
            DragDropPhase::Drop,
            harness.right_point,
        ))
        .unwrap_or_else(|error| panic!("drop event is admitted: {error}"));
    let drop_sequence = submission.sequence();
    harness.runtime.pump(budget());

    let observations = harness.observations.borrow();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].target, "right");
    assert_eq!(observations[0].phase, DragDropPhase::Drop);
    assert_eq!(observations[0].sequence, drop_sequence.get());
    assert_eq!(observations[0].current_target, harness.right);
    assert_eq!(observations[0].physical_target, Some(harness.right.clone()));
    drop(observations);

    let (token, request_debug) = {
        let service = harness
            .runtime
            .pending_framework_services()
            .into_iter()
            .find(|service| {
                matches!(
                    service.request(),
                    FrameworkServiceRequest::DragDrop {
                        source,
                        phase: DragDropPhase::Drop,
                        accepted: true,
                        ..
                    } if *source == drop_sequence
                )
            })
            .unwrap_or_else(|| unreachable!("accepted physical drop stages a service"));
        assert_eq!(service.binding().owner(), &harness.right);
        assert_eq!(service.binding().surface_context(), Some(&harness.surface));
        (service.token(), format!("{:?}", service.request()))
    };
    assert!(!request_debug.contains("private/native/path"));
    harness
        .runtime
        .complete_framework_service(&token, FrameworkServiceResponse::DragDrop(Ok(())))
        .unwrap_or_else(|error| panic!("fake host completes accepted drop: {error:?}"));
    harness.runtime.pump(budget());
    assert!(
        harness
            .runtime
            .complete_framework_service(&token, FrameworkServiceResponse::DragDrop(Ok(())))
            .is_err()
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FrameworkServiceResponseOutcome {
            service: TraceFrameworkServiceKind::DragDrop,
            outcome: TraceFrameworkServiceOutcome::DragDrop {
                phase: DragDropPhase::Drop,
                payload: DragDropPayloadKind::Files,
                items: 1,
                accepted: true,
            },
        }
    )));
    assert!(
        !harness
            .runtime
            .trace()
            .export_jsonl()
            .contains("private/native/path")
    );
}

#[test]
fn hover_is_provisional_until_a_separate_drop_admission() {
    let mut harness = harness(false);
    let submission = harness
        .runtime
        .submit_pointer(drop_event(
            &harness,
            DragDropPhase::Hover,
            harness.right_point,
        ))
        .unwrap_or_else(|error| panic!("hover is admitted: {error}"));
    harness.runtime.pump(budget());
    let token = harness
        .runtime
        .pending_framework_services()
        .into_iter()
        .find(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::DragDrop {
                    source,
                    phase: DragDropPhase::Hover,
                    accepted: false,
                    ..
                } if *source == submission.sequence()
            )
        })
        .unwrap_or_else(|| unreachable!("hover is provisional service work"))
        .token();
    harness
        .runtime
        .complete_framework_service(&token, FrameworkServiceResponse::DragDrop(Ok(())))
        .unwrap_or_else(|error| panic!("fake host completes hover: {error:?}"));
    harness.runtime.pump(budget());
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FrameworkServiceResponseOutcome {
            service: TraceFrameworkServiceKind::DragDrop,
            outcome: TraceFrameworkServiceOutcome::DragDrop {
                phase: DragDropPhase::Hover,
                accepted: false,
                ..
            },
        }
    )));
    assert!(harness.runtime.pending_framework_services().iter().all(|service| {
        !matches!(
            service.request(),
            FrameworkServiceRequest::DragDrop { source, .. } if *source == submission.sequence()
        )
    }));
}

#[test]
fn later_drop_does_not_supersede_a_pending_hover_service() {
    let mut harness = harness(true);
    let hover = harness
        .runtime
        .submit_pointer(drop_event(
            &harness,
            DragDropPhase::Hover,
            harness.right_point,
        ))
        .unwrap_or_else(|error| panic!("hover is admitted: {error}"));
    harness.runtime.pump(budget());
    let drop = harness
        .runtime
        .submit_pointer(drop_event(
            &harness,
            DragDropPhase::Drop,
            harness.right_point,
        ))
        .unwrap_or_else(|error| panic!("drop is admitted: {error}"));
    harness.runtime.pump(budget());

    let (hover_token, drop_token) = {
        let services = harness.runtime.pending_framework_services();
        let token_for = |source, phase| {
            services
                .iter()
                .find(|service| {
                    matches!(
                        service.request(),
                        FrameworkServiceRequest::DragDrop {
                            source: current_source,
                            phase: current_phase,
                            ..
                        } if *current_source == source && *current_phase == phase
                    )
                })
                .unwrap_or_else(|| unreachable!("each drag/drop phase remains pending"))
                .token()
        };
        (
            token_for(hover.sequence(), DragDropPhase::Hover),
            token_for(drop.sequence(), DragDropPhase::Drop),
        )
    };
    harness
        .runtime
        .complete_framework_service(&hover_token, FrameworkServiceResponse::DragDrop(Ok(())))
        .unwrap_or_else(|error| panic!("hover completion remains live: {error:?}"));
    harness
        .runtime
        .complete_framework_service(&drop_token, FrameworkServiceResponse::DragDrop(Ok(())))
        .unwrap_or_else(|error| panic!("drop completion remains live: {error:?}"));
    harness.runtime.pump(budget());

    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FrameworkServiceResponseOutcome {
            service: TraceFrameworkServiceKind::DragDrop,
            outcome: TraceFrameworkServiceOutcome::DragDrop {
                phase: DragDropPhase::Hover,
                ..
            },
        }
    )));
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FrameworkServiceResponseOutcome {
            service: TraceFrameworkServiceKind::DragDrop,
            outcome: TraceFrameworkServiceOutcome::DragDrop {
                phase: DragDropPhase::Drop,
                accepted: true,
                ..
            },
        }
    )));
}

#[test]
fn accepted_drop_failure_is_typed_and_does_not_change_runtime_lifetime() {
    let mut harness = harness(true);
    let submission = harness
        .runtime
        .submit_pointer(drop_event(
            &harness,
            DragDropPhase::Drop,
            harness.right_point,
        ))
        .unwrap_or_else(|error| panic!("drop event is admitted: {error}"));
    harness.runtime.pump(budget());
    let token = harness
        .runtime
        .pending_framework_services()
        .into_iter()
        .find(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::DragDrop { source, .. }
                    if *source == submission.sequence()
            )
        })
        .unwrap_or_else(|| unreachable!("explicit application admission stages request"))
        .token();
    harness
        .runtime
        .complete_framework_service(
            &token,
            FrameworkServiceResponse::DragDrop(Err(
                runenui_core::FrameworkServiceFailure::PermissionDenied,
            )),
        )
        .unwrap_or_else(|error| panic!("typed drop failure is admitted: {error:?}"));
    harness.runtime.pump(budget());
    assert_eq!(
        harness.runtime.status(),
        runenui_runtime::RuntimeStatus::Running
    );
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FrameworkServiceResponseOutcome {
            service: TraceFrameworkServiceKind::DragDrop,
            outcome: TraceFrameworkServiceOutcome::Failed(
                runenui_core::FrameworkServiceFailure::PermissionDenied
            ),
        }
    )));
}

#[test]
fn rejected_drop_does_not_stage_admission_but_cancel_is_delivered_without_acceptance() {
    let mut harness = harness(false);
    let drop_submission = harness
        .runtime
        .submit_pointer(drop_event(
            &harness,
            DragDropPhase::Drop,
            harness.right_point,
        ))
        .unwrap_or_else(|error| panic!("drop event is admitted: {error}"));
    harness.runtime.pump(budget());
    assert!(harness.runtime.pending_framework_services().iter().all(|service| {
        !matches!(
            service.request(),
            FrameworkServiceRequest::DragDrop { source, .. } if *source == drop_submission.sequence()
        )
    }));

    let cancel_submission = harness
        .runtime
        .submit_pointer(drop_event(
            &harness,
            DragDropPhase::Cancel,
            harness.right_point,
        ))
        .unwrap_or_else(|error| panic!("cancel event is admitted: {error}"));
    harness.runtime.pump(budget());
    let cancel_token = harness
        .runtime
        .pending_framework_services()
        .into_iter()
        .find(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::DragDrop {
                    source,
                    phase: DragDropPhase::Cancel,
                    accepted: false,
                    ..
                } if *source == cancel_submission.sequence()
            )
        })
        .unwrap_or_else(|| unreachable!("cancel is a typed service request"))
        .token();
    harness
        .runtime
        .complete_framework_service(&cancel_token, FrameworkServiceResponse::DragDrop(Ok(())))
        .unwrap_or_else(|error| panic!("fake host completes cancellation: {error:?}"));
    harness.runtime.pump(budget());
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FrameworkServiceResponseOutcome {
            service: TraceFrameworkServiceKind::DragDrop,
            outcome: TraceFrameworkServiceOutcome::DragDrop {
                phase: DragDropPhase::Cancel,
                accepted: false,
                ..
            },
        }
    )));
    assert!(
        harness
            .observations
            .borrow()
            .iter()
            .all(|observation| observation.phase != DragDropPhase::Cancel
                || observation.target == "right")
    );
}
