//! Winit-free downstream host-ownership proof for M7D.
//!
//! The executable proof lives in this non-publishable downstream crate so Cargo
//! enforces that host sequencing uses ordinary public runtime and renderer APIs.

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use core::{error::Error, future::Future, pin::pin, task::Poll};
    use std::{
        cell::Cell,
        io,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        task::{Context, Wake, Waker},
        thread,
    };

    use runenui_core::{
        Brush, Color, CommandOrigin, Element, ElementId, EventContext, EventPhase, ImageDescriptor,
        ImageIntrinsicSize, ImageMapping, ImagePaintDescriptor, KeyLocation, KeyModifiers,
        KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, LogicalLength,
        LogicalRect, LogicalSize, NoHostProtocol, PaintContribution, PaintContributionContext,
        PaintContributionItem, PhysicalKey, ResourceKind, ResourceRef, SceneShape, SemanticAction,
        SemanticActionRequest, SemanticCommand, SemanticContribution, SemanticContributionContext,
        SemanticNodeContribution, SemanticRole, StyleEnvironment, UiApp, UiEvent, View, Widget,
        WidgetActivation, WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput,
        WidgetMeasure,
    };
    use runenui_render_wgpu::{
        BackendSelection, ImagePayload, OffscreenPublicationReadback, OffscreenReadback,
        PayloadValidationError, PublicationRenderError, Renderer, RendererInitError,
        RendererOptions, ResourcePayload, ResourceProvider, ResourceProviderError,
        ResourceProviderErrorKind, ResourceRequest, ResourceResolveError,
    };
    use runenui_runtime::{
        AppRuntime, InputArbitrationRecord, InputScopeRetirementReason, PumpBudget,
        SurfaceBuildContext, SurfacePublication, UiInputClaimReason, UiInputConflict,
        UiInputFamily, UiInputFinality,
    };

    const SURFACE_EXTENT: u16 = 8;
    const IMAGE_EXTENT: f32 = 4.0;
    const ACTIVE_BACKGROUND: Color = Color::rgb(0x18, 0x58, 0xA8);
    const INACTIVE_BACKGROUND: Color = Color::rgb(0x88, 0x28, 0x18);
    const IMAGE_PIXEL: [u8; 4] = [0xE8, 0xB8, 0x28, 0xFF];
    const HOST_PUMP_BUDGET: PumpBudget = PumpBudget::new(64, 64, 64, 64);

    #[derive(Debug)]
    struct HostState {
        image: ResourceRef,
        active: bool,
    }

    #[derive(Debug)]
    enum HostAction {
        SetActive(bool),
        Toggle,
    }

    #[derive(Debug)]
    struct ExternalHostWidget {
        image: ResourceRef,
        active: bool,
    }

    impl Widget<HostAction> for ExternalHostWidget {
        type State = ();

        fn create_state(&self) -> Self::State {}

        fn activation(&self, (): &Self::State) -> WidgetActivation {
            WidgetActivation::actionable(true)
        }

        fn activate(
            &mut self,
            (): &mut Self::State,
            _: &mut WidgetActivationContext<HostAction>,
        ) -> WidgetActivationOutput<HostAction> {
            WidgetActivationOutput::action(HostAction::Toggle)
        }

        fn measure(
            &self,
            (): &Self::State,
            _input: runenui_core::WidgetMeasureInput,
        ) -> WidgetMeasure {
            WidgetMeasure::measured(
                LogicalLength::from(SURFACE_EXTENT),
                LogicalLength::from(SURFACE_EXTENT),
            )
        }

        fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
            let background = if self.active {
                ACTIVE_BACKGROUND
            } else {
                INACTIVE_BACKGROUND
            };
            let descriptor = ImageDescriptor::new(
                self.image.clone(),
                ImageIntrinsicSize::new(1, 1)
                    .unwrap_or_else(|| unreachable!("fixture image extent is non-zero")),
            )
            .unwrap_or_else(|_| unreachable!("fixture image reference has image kind"));
            let image = PaintContributionItem::image(
                ImagePaintDescriptor::new(
                    descriptor,
                    rect(0.0, 0.0, IMAGE_EXTENT, IMAGE_EXTENT),
                    ImageMapping::default(),
                )
                .unwrap_or_else(|_| unreachable!("fixture image mapping is valid")),
            );
            PaintContribution::new(vec![
                PaintContributionItem::fill(
                    SceneShape::rect(rect(
                        0.0,
                        0.0,
                        f32::from(SURFACE_EXTENT),
                        f32::from(SURFACE_EXTENT),
                    )),
                    Brush::solid(background),
                ),
                image,
            ])
        }

        fn semantics(
            &self,
            (): &Self::State,
            _: SemanticContributionContext,
        ) -> SemanticContribution {
            SemanticContribution::single(
                SemanticNodeContribution::primary(SemanticRole::Button)
                    .with_name("External host action")
                    .with_action(SemanticAction::Activate),
            )
        }
    }

    struct ExternalHostApp;

    impl UiApp for ExternalHostApp {
        type State = HostState;
        type Action = HostAction;
        type HostProtocol = NoHostProtocol;

        fn root(state: &Self::State) -> impl View<Self::Action> {
            Element::new(ExternalHostWidget {
                image: state.image.clone(),
                active: state.active,
            })
        }

        fn update(
            state: &mut Self::State,
            action: Self::Action,
        ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
            match action {
                HostAction::SetActive(active) => state.active = active,
                HostAction::Toggle => state.active = !state.active,
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum FrameStep {
        SubmitAction,
        SubmitSemanticAction,
        Pump,
        TakeRedraw,
        Publish,
        Acknowledge,
        Render,
        RenderFailed,
        RenderRetrySamePublication,
        Present,
    }

    struct FailingImageProvider {
        expected: ResourceRef,
        loads: Cell<usize>,
    }

    impl FailingImageProvider {
        fn new(expected: ResourceRef) -> Self {
            Self {
                expected,
                loads: Cell::new(0),
            }
        }

        const fn loads(&self) -> usize {
            self.loads.get()
        }
    }

    impl ResourceProvider for FailingImageProvider {
        fn load(
            &self,
            resource: &ResourceRef,
            request: ResourceRequest,
        ) -> Result<ResourcePayload, ResourceProviderError> {
            self.loads.set(self.loads.get() + 1);
            if resource != &self.expected || request != ResourceRequest::Image {
                return Err(ResourceProviderError::new(
                    ResourceProviderErrorKind::Malformed,
                    "external-host renderer requested a different resource identity",
                ));
            }
            Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Unavailable,
                "intentional external-host retry proof",
            ))
        }
    }

    struct ImageProvider {
        expected: ResourceRef,
        payload: ImagePayload,
        loads: Cell<usize>,
    }

    impl ImageProvider {
        fn new(expected: ResourceRef) -> Result<Self, PayloadValidationError> {
            Ok(Self {
                expected,
                payload: ImagePayload::new(1, 1, IMAGE_PIXEL.to_vec())?,
                loads: Cell::new(0),
            })
        }

        const fn loads(&self) -> usize {
            self.loads.get()
        }
    }

    impl ResourceProvider for ImageProvider {
        fn load(
            &self,
            resource: &ResourceRef,
            request: ResourceRequest,
        ) -> Result<ResourcePayload, ResourceProviderError> {
            self.loads.set(self.loads.get() + 1);
            if resource != &self.expected || request != ResourceRequest::Image {
                return Err(ResourceProviderError::new(
                    ResourceProviderErrorKind::Malformed,
                    "external-host renderer requested a different resource identity",
                ));
            }
            Ok(ResourcePayload::Image(self.payload.clone()))
        }
    }

    // Downstream public-only authoring proof: no runtime-internal types,
    // event outcome inspectors, extra input FIFO, or gameplay manager.
    #[derive(Debug)]
    struct ExternalHostClaimProbe {
        reached: Arc<AtomicBool>,
    }

    impl Widget<()> for ExternalHostClaimProbe {
        type State = ();

        fn create_state(&self) -> Self::State {}

        fn event(
            &mut self,
            (): &mut Self::State,
            event: &UiEvent,
            context: &mut EventContext<'_, ()>,
        ) -> WidgetEventOutput {
            if matches!(event, UiEvent::Keyboard(_)) && context.phase() == EventPhase::Target {
                context.claim_host_input();
                assert!(context.host_input_is_claimed());
                self.reached.store(true, Ordering::Relaxed);
            }
            WidgetEventOutput::none()
        }
    }

    struct ExternalHostClaimApp;

    impl UiApp for ExternalHostClaimApp {
        type State = Arc<AtomicBool>;
        type Action = ();
        type HostProtocol = NoHostProtocol;

        fn root(reached: &Self::State) -> impl View<Self::Action> {
            Element::new(ExternalHostClaimProbe {
                reached: Arc::clone(reached),
            })
            .id("external-claim-probe")
            .key("external-claim-probe")
            .focusable(true)
        }

        fn update(
            _: &mut Self::State,
            (): Self::Action,
        ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        }
    }

    #[test]
    fn downstream_widget_can_claim_keyboard_input_using_only_public_framework_api() {
        let reached = Arc::new(AtomicBool::new(false));
        let mut runtime = AppRuntime::<ExternalHostClaimApp>::mount(Arc::clone(&reached));
        let _ = runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("pump observation"));
        let id = ElementId::new("external-claim-probe")
            .unwrap_or_else(|_| unreachable!("static authored id"));
        let target = runtime
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&id))
            .unwrap_or_else(|| unreachable!("fixture node mounted"))
            .id()
            .clone();
        runtime
            .submit_command(
                target,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("public focus request accepted"));
        let _ = runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("pump observation"));
        let receipt = runtime
            .submit_keyboard(KeyboardEvent::new(
                KeyboardPhase::Down,
                PhysicalKey::Code(String::from("KeyW")),
                LogicalKey::Character(String::from("w")),
                KeyModifiers::NONE,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            ))
            .unwrap_or_else(|_| unreachable!("native keyboard input accepted"));

        // The queue receipt is Pending until this exact FIFO boundary commits.
        let pending = runtime
            .pump(PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("bounded empty checkpoint"));
        assert_eq!(pending.processed_through(), None);
        assert!(
            !pending
                .ordered_records()
                .iter()
                .any(|record| { matches!(record, InputArbitrationRecord::InputSettled(_)) })
        );
        assert!(!reached.load(Ordering::Relaxed));

        let committed = runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("host input settlement returned"));
        let settlement = committed
            .ordered_records()
            .iter()
            .filter_map(|record| match record {
                InputArbitrationRecord::InputSettled(settlement) => Some(settlement),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            settlement.len(),
            1,
            "exactly one reached host input settles"
        );
        let settled = settlement[0];
        assert_eq!(settled.sequence(), receipt.sequence());
        assert_eq!(settled.scope(), committed.final_ownership().scope());
        assert_eq!(settled.family(), UiInputFamily::Keyboard);
        assert!(matches!(
            settled.finality(),
            UiInputFinality::Committed(facts)
                if facts.conflict() == UiInputConflict::ExclusiveUi
                    && facts.reasons().contains(&UiInputClaimReason::ExplicitWidgetClaim)
        ));
        assert!(reached.load(Ordering::Relaxed));

        let closed = runtime
            .shutdown()
            .unwrap_or_else(|_| unreachable!("observed final shutdown"));
        assert_eq!(
            closed.final_ownership().status(),
            runenui_runtime::RuntimeStatus::Closed
        );
        assert_eq!(
            closed
                .ordered_records()
                .iter()
                .filter(|record| {
                    matches!(record, InputArbitrationRecord::ScopeRetired(retirement)
                if retirement.reason() == InputScopeRetirementReason::Shutdown)
                })
                .count(),
            1
        );
        let again = runtime
            .shutdown()
            .unwrap_or_else(|_| unreachable!("idempotent final observation"));
        assert!(
            !again
                .ordered_records()
                .iter()
                .any(|record| { matches!(record, InputArbitrationRecord::ScopeRetired(_)) })
        );
        // A host that retains native receipt associations must settle the
        // scope before consuming the runtime; no second implicit retirement
        // is synthesized by into_state.
        let returned = runtime.into_state();
        assert!(returned.load(Ordering::Relaxed));
    }

    #[test]
    fn independent_public_host_slots_have_disjoint_runtime_scopes() {
        let mut first = AppRuntime::<ExternalHostApp>::mount(HostState {
            image: ResourceRef::new(ResourceKind::Image),
            active: false,
        });
        let mut second = AppRuntime::<ExternalHostApp>::mount(HostState {
            image: ResourceRef::new(ResourceKind::Image),
            active: false,
        });
        let first_snapshot = first.input_ownership().unwrap_or_else(|_| {
            unreachable!("first runtime can project its initial input ownership")
        });
        let second_snapshot = second.input_ownership().unwrap_or_else(|_| {
            unreachable!("second runtime can project its initial input ownership")
        });
        assert_ne!(first_snapshot.scope(), second_snapshot.scope());
        assert_eq!(first_snapshot.revision().get(), 1);
        assert_eq!(second_snapshot.revision().get(), 1);
        assert_eq!(
            first_snapshot.revision(),
            first
                .input_ownership()
                .unwrap_or_else(|_| unreachable!("unchanged state projects"))
                .revision()
        );
        let _ = first
            .shutdown()
            .unwrap_or_else(|_| unreachable!("shutdown observation"));
        let closed = first
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("terminal ownership is observable without a pump"));
        assert_ne!(closed.revision(), first_snapshot.revision());
        assert_eq!(closed.status(), runenui_runtime::RuntimeStatus::Closed);
        assert_eq!(
            second
                .input_ownership()
                .unwrap_or_else(|_| unreachable!("other runtime remains valid"))
                .status(),
            runenui_runtime::RuntimeStatus::Running
        );
    }

    #[test]
    fn downstream_host_owns_publication_acknowledgement_renderer_retry_and_semantic_next_frame()
    -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };

        let image = ResourceRef::new(ResourceKind::Image);
        let mut runtime = AppRuntime::<ExternalHostApp>::mount(HostState {
            image: image.clone(),
            active: false,
        });
        let style_environment = StyleEnvironment::default();
        let logical_size =
            LogicalSize::try_new(f32::from(SURFACE_EXTENT), f32::from(SURFACE_EXTENT))?;
        let build_context = SurfaceBuildContext::tight(&style_environment, logical_size);
        let failing_provider = FailingImageProvider::new(image.clone());
        let provider = ImageProvider::new(image)?;
        let mut steps = Vec::new();
        let mut publication_count = 0_u8;

        steps.push(FrameStep::SubmitAction);
        runtime
            .submit_action(HostAction::SetActive(true))
            .map_err(|error| io::Error::other(error.to_string()))?;

        steps.push(FrameStep::Pump);
        let _ = runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("pump observation"));
        assert!(runtime.state().active);

        steps.push(FrameStep::TakeRedraw);
        let first_redraw = runtime
            .take_redraw_request()
            .ok_or_else(|| io::Error::other("first host frame had no redraw request"))?;

        steps.push(FrameStep::Publish);
        let first_publication = runtime
            .publish_surface(&build_context)
            .map_err(|error| debug_error("first publication failed", &error))?;
        publication_count += 1;
        let first_revision = first_publication.paint_publication().revision();
        let semantic_request = semantic_activate_request(&first_publication)?;

        steps.push(FrameStep::Acknowledge);
        runtime
            .acknowledge_redraw(&first_redraw)
            .map_err(|error| debug_error("first redraw acknowledgement failed", &error))?;

        steps.push(FrameStep::Render);
        let render_failure = renderer
            .render_offscreen_publication(first_publication.paint_publication(), &failing_provider);
        assert_expected_resource_failure(render_failure, &failing_provider)?;
        steps.push(FrameStep::RenderFailed);
        assert_eq!(publication_count, 1);

        steps.push(FrameStep::RenderRetrySamePublication);
        let first_render = renderer
            .render_offscreen_publication(first_publication.paint_publication(), &provider)?;
        assert_eq!(
            first_publication.paint_publication().revision(),
            first_revision
        );
        assert_ne!(provider.loads(), 0);

        steps.push(FrameStep::Present);
        let first_presented = present_and_assert(first_render.readback(), ACTIVE_BACKGROUND);
        assert!(runtime.take_redraw_request().is_none());

        steps.push(FrameStep::SubmitSemanticAction);
        runtime
            .submit_semantic_action(semantic_request)
            .map_err(|error| debug_error("semantic action submission failed", &error))?;

        steps.push(FrameStep::Pump);
        let _ = runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("pump observation"));
        assert!(!runtime.state().active);

        steps.push(FrameStep::TakeRedraw);
        let second_redraw = runtime
            .take_redraw_request()
            .ok_or_else(|| io::Error::other("semantic action produced no redraw request"))?;

        steps.push(FrameStep::Publish);
        let second_publication = runtime
            .publish_surface(&build_context)
            .map_err(|error| debug_error("second publication failed", &error))?;
        publication_count += 1;
        assert_ne!(
            second_publication.paint_publication().revision(),
            first_revision
        );

        steps.push(FrameStep::Acknowledge);
        runtime
            .acknowledge_redraw(&second_redraw)
            .map_err(|error| debug_error("second redraw acknowledgement failed", &error))?;

        steps.push(FrameStep::Render);
        let second_render = renderer
            .render_offscreen_publication(second_publication.paint_publication(), &provider)?;

        steps.push(FrameStep::Present);
        let second_presented = present_and_assert(second_render.readback(), INACTIVE_BACKGROUND);
        assert_ne!(first_presented, second_presented);
        assert_eq!(publication_count, 2);
        assert_eq!(steps, expected_steps());

        let _ = runtime
            .shutdown()
            .unwrap_or_else(|_| unreachable!("shutdown observation"))
            .report()
            .to_owned();
        eprintln!(
            "M7D EXTERNAL HOST PROOF: retained-publication retry and two host-owned frames succeeded; adapter={:?} backend={}",
            renderer.diagnostics().adapter_info().name,
            renderer.diagnostics().adapter_info().backend,
        );
        Ok(())
    }

    fn semantic_activate_request(
        publication: &SurfacePublication,
    ) -> Result<SemanticActionRequest, io::Error> {
        let snapshot = publication.semantic_publication().snapshot();
        let target = snapshot
            .nodes()
            .iter()
            .find(|node| node.supported_actions().contains(&SemanticAction::Activate))
            .ok_or_else(|| io::Error::other("published fixture has no actionable semantic node"))?
            .id()
            .clone();
        Ok(SemanticActionRequest::new(
            snapshot.surface_id().clone(),
            target,
            SemanticAction::Activate,
        ))
    }

    fn assert_expected_resource_failure(
        result: Result<OffscreenPublicationReadback, PublicationRenderError>,
        provider: &FailingImageProvider,
    ) -> Result<(), Box<dyn Error>> {
        let Err(error) = result else {
            return Err(
                io::Error::other("intentional provider failure rendered successfully").into(),
            );
        };
        assert!(matches!(
            error,
            PublicationRenderError::Resource {
                error: ResourceResolveError::Provider(ref provider_error),
                ..
            } if provider_error.kind() == ResourceProviderErrorKind::Unavailable
        ));
        assert_eq!(provider.loads(), 1);
        Ok(())
    }

    fn present_and_assert(readback: &OffscreenReadback, background: Color) -> Vec<u8> {
        let presented = present(readback);
        assert_eq!(
            presented_pixel(&presented, readback.extent().width(), 1, 1),
            IMAGE_PIXEL
        );
        assert_eq!(
            presented_pixel(&presented, readback.extent().width(), 6, 6),
            color_pixel(background)
        );
        presented
    }

    fn expected_steps() -> Vec<FrameStep> {
        vec![
            FrameStep::SubmitAction,
            FrameStep::Pump,
            FrameStep::TakeRedraw,
            FrameStep::Publish,
            FrameStep::Acknowledge,
            FrameStep::Render,
            FrameStep::RenderFailed,
            FrameStep::RenderRetrySamePublication,
            FrameStep::Present,
            FrameStep::SubmitSemanticAction,
            FrameStep::Pump,
            FrameStep::TakeRedraw,
            FrameStep::Publish,
            FrameStep::Acknowledge,
            FrameStep::Render,
            FrameStep::Present,
        ]
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
    }

    fn color_pixel(color: Color) -> [u8; 4] {
        [color.red(), color.green(), color.blue(), color.alpha()]
    }

    fn present(readback: &OffscreenReadback) -> Vec<u8> {
        readback.rgba8_srgb().to_vec()
    }

    fn presented_pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let index = (y as usize * width as usize + x as usize) * 4;
        pixels[index..index + 4]
            .try_into()
            .unwrap_or_else(|_| unreachable!("fixture pixel is inside the presented frame"))
    }

    fn debug_error(context: &str, error: &impl core::fmt::Debug) -> io::Error {
        io::Error::other(format!("{context}: {error:?}"))
    }

    #[allow(clippy::assert_is_empty)]
    fn renderer_or_adapterless() -> Result<Option<Renderer>, Box<dyn Error>> {
        match block_on(Renderer::request(RendererOptions::new())) {
            Ok(renderer) => Ok(Some(renderer)),
            Err(RendererInitError::AdapterUnavailable {
                requested,
                compatible_surface_required,
                detail,
            }) => {
                eprintln!("M7D external-host GPU proof unavailable under {requested:?}: {detail}");
                assert_eq!(requested, BackendSelection::AllNative);
                assert!(!compatible_surface_required);
                assert!(!detail.is_empty());
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    struct ThreadWake(thread::Thread);

    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => thread::park(),
            }
        }
    }
}
