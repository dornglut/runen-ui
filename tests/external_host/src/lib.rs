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
        sync::Arc,
        task::{Context, Wake, Waker},
        thread,
    };

    use runenui_core::{
        Color, Element, IntoEffects, LogicalLength, LogicalRect, LogicalSize, NoHostProtocol,
        PaintContribution, PaintContributionContext, PaintContributionItem, ResourceKind,
        ResourceRef, SemanticAction, SemanticActionRequest, SemanticContribution,
        SemanticContributionContext, SemanticNodeContribution, SemanticRole, UiApp, View, Widget,
        WidgetActivation, WidgetActivationContext, WidgetActivationOutput, WidgetMeasure,
    };
    use runenui_render_wgpu::{
        BackendSelection, ImagePayload, PayloadValidationError, PublicationRenderError, Renderer,
        RendererInitError, RendererOptions, ResourcePayload, ResourceProvider, ResourceProviderError,
        ResourceProviderErrorKind, ResourceRequest, ResourceResolveError,
    };
    use runenui_runtime::{AppRuntime, PumpBudget, StyleTokens, SurfaceBuildContext};

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

        fn measure(&self, (): &Self::State) -> WidgetMeasure {
            WidgetMeasure::Fixed {
                width: LogicalLength::from(SURFACE_EXTENT),
                height: LogicalLength::from(SURFACE_EXTENT),
            }
        }

        fn paint(
            &self,
            (): &Self::State,
            _: PaintContributionContext,
        ) -> PaintContribution {
            let background = if self.active {
                ACTIVE_BACKGROUND
            } else {
                INACTIVE_BACKGROUND
            };
            let image = PaintContributionItem::image(
                self.image.clone(),
                rect(0.0, 0.0, IMAGE_EXTENT, IMAGE_EXTENT),
            )
            .unwrap_or_else(|_| unreachable!("fixture image reference has image kind"));
            PaintContribution::new(vec![
                PaintContributionItem::fill_rect(
                    rect(
                        0.0,
                        0.0,
                        f32::from(SURFACE_EXTENT),
                        f32::from(SURFACE_EXTENT),
                    ),
                    background,
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
        ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
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
        const fn new(expected: ResourceRef) -> Self {
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
        let style_tokens = StyleTokens::new();
        let logical_size = LogicalSize::try_new(
            f32::from(SURFACE_EXTENT),
            f32::from(SURFACE_EXTENT),
        )?;
        let build_context = SurfaceBuildContext::tight(&style_tokens, logical_size);
        let failing_provider = FailingImageProvider::new(image.clone());
        let provider = ImageProvider::new(image)?;
        let mut steps = Vec::new();
        let mut publication_count = 0_u8;

        steps.push(FrameStep::SubmitAction);
        runtime
            .submit_action(HostAction::SetActive(true))
            .map_err(|error| io::Error::other(error.to_string()))?;

        steps.push(FrameStep::Pump);
        let _ = runtime.pump(HOST_PUMP_BUDGET);
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
        let semantic_snapshot = first_publication.semantic_publication().snapshot();
        let semantic_target = semantic_snapshot
            .nodes()
            .iter()
            .find(|node| node.supported_actions().contains(&SemanticAction::Activate))
            .ok_or_else(|| io::Error::other("published fixture has no actionable semantic node"))?
            .id()
            .clone();
        let semantic_surface = semantic_snapshot.surface_id().clone();

        steps.push(FrameStep::Acknowledge);
        runtime
            .acknowledge_redraw(&first_redraw)
            .map_err(|error| debug_error("first redraw acknowledgement failed", &error))?;

        steps.push(FrameStep::Render);
        let render_failure = renderer.render_offscreen_publication(
            first_publication.paint_publication(),
            &failing_provider,
        );
        let Err(render_failure) = render_failure else {
            return Err(io::Error::other("intentional provider failure rendered successfully").into());
        };
        assert!(matches!(
            render_failure,
            PublicationRenderError::Resource {
                error: ResourceResolveError::Provider(ref provider_error),
                ..
            } if provider_error.kind() == ResourceProviderErrorKind::Unavailable
        ));
        assert_eq!(failing_provider.loads(), 1);
        steps.push(FrameStep::RenderFailed);
        assert_eq!(publication_count, 1);

        steps.push(FrameStep::RenderRetrySamePublication);
        let first_render = renderer.render_offscreen_publication(
            first_publication.paint_publication(),
            &provider,
        )?;
        assert_eq!(first_publication.paint_publication().revision(), first_revision);
        assert!(provider.loads() >= 1);

        steps.push(FrameStep::Present);
        let first_presented = present(first_render.readback());
        assert_eq!(
            presented_pixel(
                &first_presented,
                first_render.readback().extent().width(),
                1,
                1,
            ),
            IMAGE_PIXEL
        );
        assert_eq!(
            presented_pixel(
                &first_presented,
                first_render.readback().extent().width(),
                6,
                6,
            ),
            color_pixel(ACTIVE_BACKGROUND)
        );
        assert!(runtime.take_redraw_request().is_none());

        steps.push(FrameStep::SubmitSemanticAction);
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                semantic_surface,
                semantic_target,
                SemanticAction::Activate,
            ))
            .map_err(|error| debug_error("semantic action submission failed", &error))?;

        steps.push(FrameStep::Pump);
        let _ = runtime.pump(HOST_PUMP_BUDGET);
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
        assert_ne!(second_publication.paint_publication().revision(), first_revision);

        steps.push(FrameStep::Acknowledge);
        runtime
            .acknowledge_redraw(&second_redraw)
            .map_err(|error| debug_error("second redraw acknowledgement failed", &error))?;

        steps.push(FrameStep::Render);
        let second_render = renderer.render_offscreen_publication(
            second_publication.paint_publication(),
            &provider,
        )?;

        steps.push(FrameStep::Present);
        let second_presented = present(second_render.readback());
        assert_eq!(
            presented_pixel(
                &second_presented,
                second_render.readback().extent().width(),
                6,
                6,
            ),
            color_pixel(INACTIVE_BACKGROUND)
        );
        assert_ne!(first_presented, second_presented);
        assert_eq!(publication_count, 2);

        assert_eq!(
            steps,
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
        );

        let _ = runtime.shutdown();
        eprintln!(
            "M7D EXTERNAL HOST PROOF: retained-publication retry and two host-owned frames succeeded; adapter={:?} backend={}",
            renderer.diagnostics().adapter_info().name,
            renderer.diagnostics().adapter_info().backend,
        );
        Ok(())
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
    }

    fn color_pixel(color: Color) -> [u8; 4] {
        [color.red(), color.green(), color.blue(), color.alpha()]
    }

    fn present(readback: &runenui_render_wgpu::OffscreenReadback) -> Vec<u8> {
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

    fn renderer_or_adapterless() -> Result<Option<Renderer>, Box<dyn Error>> {
        match block_on(Renderer::request(RendererOptions::new())) {
            Ok(renderer) => Ok(Some(renderer)),
            Err(RendererInitError::AdapterUnavailable {
                requested,
                compatible_surface_required,
                detail,
            }) => {
                eprintln!(
                    "M7D external-host GPU proof unavailable under {requested:?}: {detail}"
                );
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
