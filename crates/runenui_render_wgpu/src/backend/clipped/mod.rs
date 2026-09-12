mod clip;
mod image;
#[allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::missing_errors_doc,
    clippy::too_many_lines,
    reason = "the private mixed-scene implementation is wrapped by the documented public facade; its long atomic preflight and explicit triangle chunk iteration remain implementation details"
)]
mod resource;
mod shaped;
mod shaped_outline;

pub use resource::{PublicationRenderError, UnsupportedShapedGlyphKind};

use runenui_core::Color;
use runenui_runtime::PaintPublication;

use crate::{ResourceProvider, WgpuHasDisplayHandle, observation::PublicationObservation};

const STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Stencil8;
const STENCIL_ALLOWED: u32 = 1;

/// Canonical production renderer over retained `RunenUI` paint publications.
///
/// The renderer owns only disposable GPU realization, target state, resource
/// caches, and presentation lineage. Runtime publication remains the semantic
/// authority; unsupported renderer breadth fails closed during preflight.
#[derive(Debug)]
pub struct ResourceRenderer {
    inner: resource::ResourceRenderer,
}

impl ResourceRenderer {
    /// Selects a native adapter and creates a renderer-owned wgpu device and queue.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request(
        options: super::RendererOptions,
    ) -> Result<Self, super::RendererInitError> {
        resource::ResourceRenderer::request(options)
            .await
            .map(|inner| Self { inner })
    }

    /// Selects a native adapter using a caller-owned display connection.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request_with_display_handle(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, super::RendererInitError> {
        resource::ResourceRenderer::request_with_display_handle(options, display)
            .await
            .map(|inner| Self { inner })
    }

    /// Creates and retains a native surface before selecting a compatible adapter.
    ///
    /// # Errors
    ///
    /// Returns structured surface-creation, compatible-adapter, target-format, or
    /// device diagnostics when construction fails.
    pub async fn request_with_surface_target(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, super::RendererInitError> {
        resource::ResourceRenderer::request_with_surface_target(options, display, window)
            .await
            .map(|inner| Self { inner })
    }

    /// Returns immutable instance, adapter, device, and target diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &super::RendererDiagnostics {
        self.inner.diagnostics()
    }

    /// Returns the immutable observation for the most recent publication attempt.
    #[must_use]
    pub const fn last_observation(&self) -> Option<&crate::PublicationObservation> {
        self.inner.last_observation()
    }

    /// Returns whether construction retained an actual native surface target.
    #[must_use]
    pub const fn has_surface(&self) -> bool {
        self.inner.has_surface()
    }

    /// Returns the exact configured native surface extent, when configured.
    #[must_use]
    pub const fn configured_surface_extent(&self) -> Option<super::OffscreenExtent> {
        self.inner.configured_surface_extent()
    }

    /// Returns the renderer-local generation of the current native surface configuration.
    #[must_use]
    pub const fn surface_target_generation(&self) -> u64 {
        self.inner.surface_target_generation()
    }

    /// Configures the retained native surface for one non-zero physical extent.
    ///
    /// Reconfiguration creates a new renderer-local target generation and forgets
    /// successful surface-publication lineage. Resource uploads remain disposable
    /// renderer state and may be reused across target recreation.
    ///
    /// # Errors
    ///
    /// Returns a structured error when no native surface exists, the extent is
    /// invalid for the selected device, or the renderer cannot allocate another
    /// target generation.
    pub fn configure_surface(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<super::OffscreenExtent, PublicationRenderError> {
        self.inner.configure_surface(width, height)
    }

    /// Drops the retained offscreen target and every publication realization tied to it.
    #[must_use]
    pub fn discard_offscreen_target(&mut self) -> bool {
        self.inner.discard_offscreen_target()
    }

    /// Drops renderer-owned uploaded resource realizations without changing logical refs.
    ///
    /// A real cache loss also invalidates successful publication lineage so the
    /// next complete publication is reconstructed with a full resync on every target.
    #[must_use]
    pub fn discard_resource_cache(&mut self) -> bool {
        self.inner.discard_resource_cache()
    }

    /// Renders one complete publication and reads actual GPU bytes.
    ///
    /// Generic fill/stroke geometry, images, and shaped text share one ordered target
    /// transaction. Scene validation, clip/solid geometry realization, device-limit checks,
    /// and resource preflight complete before retained-target mutation.
    ///
    /// # Errors
    ///
    /// Returns deterministic scene, geometry-realization, resource, target, device,
    /// or readback failures. Preflight failures do not mutate the retained target.
    pub fn render_offscreen_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
    ) -> Result<super::OffscreenPublicationReadback, PublicationRenderError> {
        self.inner
            .render_offscreen_publication(publication, provider)
    }

    /// Renders one complete publication directly into the configured native surface.
    ///
    /// The configured surface extent is the physical target authority. A newly
    /// acquired swapchain image is always completely rendered; publication lineage
    /// does not imply that the acquired image already contains current pixels.
    /// `before_present` is invoked exactly once after successful submission and
    /// immediately before presentation.
    ///
    /// # Errors
    ///
    /// Returns deterministic publication/resource/backend failures plus structured
    /// native-surface recovery states. Timeout and occlusion may be retried later;
    /// outdated/suboptimal targets should be reconfigured; a lost surface requires
    /// recreating the renderer. `before_present` is not invoked before successful
    /// GPU submission.
    pub fn render_surface_publication<P: ResourceProvider + ?Sized>(
        &mut self,
        publication: &PaintPublication,
        provider: &P,
        before_present: impl FnOnce(),
    ) -> Result<crate::PublicationObservation, PublicationRenderError> {
        self.inner
            .render_surface_publication(publication, provider, before_present)
    }

    /// Executes one real wgpu clear and returns actual texture bytes from GPU readback.
    ///
    /// This low-level diagnostic consumes no publication and never changes
    /// publication lineage.
    ///
    /// # Errors
    ///
    /// Returns structured extent, device-wait, buffer-map, or mapped-range failures.
    pub fn clear_offscreen(
        &self,
        extent: super::OffscreenExtent,
        color: Color,
    ) -> Result<super::OffscreenReadback, super::OffscreenRenderError> {
        self.inner.clear_offscreen(extent, color)
    }
}

/// Renderer-local device/target infrastructure retained underneath the mixed public facade.
#[derive(Debug)]
struct Renderer {
    base: super::Renderer,
}

impl Renderer {
    async fn request(options: super::RendererOptions) -> Result<Self, super::RendererInitError> {
        super::Renderer::request(options).await.map(Self::from_base)
    }

    async fn request_with_display_handle(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request_with_display_handle(options, display)
            .await
            .map(Self::from_base)
    }

    async fn request_with_surface_target(
        options: super::RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, super::RendererInitError> {
        super::Renderer::request_with_surface_target(options, display, window)
            .await
            .map(Self::from_base)
    }

    const fn from_base(base: super::Renderer) -> Self {
        Self { base }
    }

    const fn diagnostics(&self) -> &super::RendererDiagnostics {
        self.base.diagnostics()
    }

    const fn last_observation(&self) -> Option<&PublicationObservation> {
        self.base.last_observation()
    }

    const fn has_surface(&self) -> bool {
        self.base.has_surface()
    }

    fn discard_offscreen_target(&mut self) -> bool {
        self.base.discard_offscreen_target()
    }

    fn clear_offscreen(
        &self,
        extent: super::OffscreenExtent,
        color: Color,
    ) -> Result<super::OffscreenReadback, super::OffscreenRenderError> {
        self.base.clear_offscreen(extent, color)
    }
}

const fn clipped_fill_stencil_face() -> wgpu::StencilFaceState {
    wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: wgpu::StencilOperation::Keep,
    }
}

fn clipped_fill_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState::stencil(
        STENCIL_FORMAT,
        wgpu::StencilState {
            front: clipped_fill_stencil_face(),
            back: clipped_fill_stencil_face(),
            read_mask: STENCIL_ALLOWED,
            write_mask: 0,
        },
    )
}

fn create_stencil_target(
    device: &wgpu::Device,
    extent: super::OffscreenExtent,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("runenui scene stencil target"),
        size: super::texture_extent(extent),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: STENCIL_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn clear_color_target(encoder: &mut wgpu::CommandEncoder, color_view: &wgpu::TextureView) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: color_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    });
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui scene clear pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

fn clear_stencil_mask(encoder: &mut wgpu::CommandEncoder, stencil_view: &wgpu::TextureView) {
    let stencil_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: stencil_view,
        depth_ops: None,
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(STENCIL_ALLOWED),
            store: wgpu::StoreOp::Store,
        }),
    };
    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui clip stencil reset pass"),
        color_attachments: &[],
        depth_stencil_attachment: Some(stencil_attachment),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}
