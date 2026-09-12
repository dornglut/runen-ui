use core::{error::Error, fmt};
use std::{sync::Arc, time::Duration};

use runenui_core::Color;
use runenui_runtime::{PaintPublication, RasterScale};

use crate::{
    PublicationUpdatePlan, WgpuHasDisplayHandle,
    lineage::PublicationLineage,
    observation::PublicationObservation,
    scene_subset::{SceneValidationError, SupportedFillRect},
};

pub(crate) mod clipped;

const DEVICE_LABEL: &str = "runenui_render_wgpu device";
const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SUPPORTED_SURFACE_FORMATS: [wgpu::TextureFormat; 2] = [
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Bgra8UnormSrgb,
];
const READBACK_TIMEOUT: Duration = Duration::from_secs(30);

/// One native backend selection for renderer construction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BackendSelection {
    /// Let wgpu select among the reviewed native backends compiled into this crate.
    #[default]
    AllNative,
    /// Require Vulkan.
    Vulkan,
    /// Require Metal.
    Metal,
    /// Require Direct3D 12.
    Dx12,
    /// Require OpenGL or OpenGL ES.
    Gl,
}

impl BackendSelection {
    const fn wgpu_backends(self) -> wgpu::Backends {
        match self {
            Self::AllNative => wgpu::Backends::VULKAN
                .union(wgpu::Backends::METAL)
                .union(wgpu::Backends::DX12)
                .union(wgpu::Backends::GL),
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Metal => wgpu::Backends::METAL,
            Self::Dx12 => wgpu::Backends::DX12,
            Self::Gl => wgpu::Backends::GL,
        }
    }
}

/// Power preference supplied during renderer-owned adapter selection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AdapterPowerPreference {
    /// Do not bias adapter selection by power profile.
    #[default]
    None,
    /// Prefer a lower-power adapter.
    LowPower,
    /// Prefer a higher-performance adapter.
    HighPerformance,
}

impl AdapterPowerPreference {
    const fn wgpu_preference(self) -> wgpu::PowerPreference {
        match self {
            Self::None => wgpu::PowerPreference::None,
            Self::LowPower => wgpu::PowerPreference::LowPower,
            Self::HighPerformance => wgpu::PowerPreference::HighPerformance,
        }
    }
}

/// Explicit renderer-owned instance and adapter request policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RendererOptions {
    backend_selection: BackendSelection,
    power_preference: AdapterPowerPreference,
    force_fallback_adapter: bool,
}

impl RendererOptions {
    /// Uses the reviewed native backend set without forcing a fallback adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            backend_selection: BackendSelection::AllNative,
            power_preference: AdapterPowerPreference::None,
            force_fallback_adapter: false,
        }
    }

    /// Restricts adapter selection to one reviewed native backend policy.
    #[must_use]
    pub const fn with_backend_selection(mut self, selection: BackendSelection) -> Self {
        self.backend_selection = selection;
        self
    }

    /// Selects the adapter power preference.
    #[must_use]
    pub const fn with_power_preference(mut self, preference: AdapterPowerPreference) -> Self {
        self.power_preference = preference;
        self
    }

    /// Controls whether adapter selection requires a fallback adapter.
    #[must_use]
    pub const fn with_force_fallback_adapter(mut self, force: bool) -> Self {
        self.force_fallback_adapter = force;
        self
    }

    /// Returns the requested native backend policy.
    #[must_use]
    pub const fn backend_selection(self) -> BackendSelection {
        self.backend_selection
    }

    /// Returns the requested adapter power policy.
    #[must_use]
    pub const fn power_preference(self) -> AdapterPowerPreference {
        self.power_preference
    }

    /// Returns whether adapter selection requires a fallback adapter.
    #[must_use]
    pub const fn force_fallback_adapter(self) -> bool {
        self.force_fallback_adapter
    }
}

/// Structured renderer-construction failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RendererInitError {
    /// None of the requested backends is compiled for the current target.
    BackendUnavailable {
        requested: BackendSelection,
        compiled: wgpu::Backends,
    },
    /// wgpu could not select an adapter under the explicit policy.
    AdapterUnavailable {
        requested: BackendSelection,
        compatible_surface_required: bool,
        detail: Arc<str>,
    },
    /// wgpu could not create a surface from the caller-owned target.
    SurfaceCreation { detail: Arc<str> },
    /// The compatible surface exposes no sRGB format implemented by this renderer.
    SurfaceFormatUnavailable {
        advertised_formats: Arc<[wgpu::TextureFormat]>,
    },
    /// A noop or browser-only adapter reached the native reference path.
    DisallowedAdapterBackend { backend: wgpu::Backend },
    /// The selected adapter could not create the renderer-owned device and queue.
    DeviceUnavailable {
        adapter_name: Arc<str>,
        detail: Arc<str>,
    },
}

impl fmt::Display for RendererInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable {
                requested,
                compiled,
            } => write!(
                formatter,
                "requested renderer backend {requested:?} is unavailable; compiled backends: {compiled:?}"
            ),
            Self::AdapterUnavailable {
                requested,
                compatible_surface_required,
                detail,
            } => write!(
                formatter,
                "no adapter satisfied renderer backend {requested:?} with compatible_surface_required={compatible_surface_required}: {detail}"
            ),
            Self::SurfaceCreation { detail } => {
                write!(
                    formatter,
                    "renderer could not create the native surface: {detail}"
                )
            }
            Self::SurfaceFormatUnavailable { advertised_formats } => write!(
                formatter,
                "native surface formats {advertised_formats:?} contain neither Rgba8UnormSrgb nor Bgra8UnormSrgb"
            ),
            Self::DisallowedAdapterBackend { backend } => {
                write!(
                    formatter,
                    "adapter backend {backend} is not a native pixel authority"
                )
            }
            Self::DeviceUnavailable {
                adapter_name,
                detail,
            } => write!(
                formatter,
                "adapter {adapter_name:?} could not create a renderer device: {detail}"
            ),
        }
    }
}

impl Error for RendererInitError {}

/// Immutable diagnostics for the renderer-owned instance, adapter, device, and target policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererDiagnostics {
    options: RendererOptions,
    compiled_backends: wgpu::Backends,
    adapter_info: wgpu::AdapterInfo,
    adapter_features: wgpu::Features,
    adapter_limits: wgpu::Limits,
    requested_device_features: wgpu::Features,
    requested_device_limits: wgpu::Limits,
    device_features: wgpu::Features,
    device_limits: wgpu::Limits,
    offscreen_format: wgpu::TextureFormat,
    surface_format: Option<wgpu::TextureFormat>,
}

impl RendererDiagnostics {
    /// Returns the exact construction policy.
    #[must_use]
    pub const fn options(&self) -> RendererOptions {
        self.options
    }

    /// Returns the backends compiled into wgpu for this target.
    #[must_use]
    pub const fn compiled_backends(&self) -> wgpu::Backends {
        self.compiled_backends
    }

    /// Returns wgpu's immutable selected-adapter diagnostics.
    #[must_use]
    pub const fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Returns all features exposed by the selected adapter before device policy is applied.
    #[must_use]
    pub const fn adapter_features(&self) -> wgpu::Features {
        self.adapter_features
    }

    /// Returns all limits exposed by the selected adapter before device policy is applied.
    #[must_use]
    pub const fn adapter_limits(&self) -> &wgpu::Limits {
        &self.adapter_limits
    }

    /// Returns the deliberately requested device feature policy.
    #[must_use]
    pub const fn requested_device_features(&self) -> wgpu::Features {
        self.requested_device_features
    }

    /// Returns the deliberately requested device limit policy.
    #[must_use]
    pub const fn requested_device_limits(&self) -> &wgpu::Limits {
        &self.requested_device_limits
    }

    /// Returns the actual feature set enabled on the renderer-owned device.
    #[must_use]
    pub const fn device_features(&self) -> wgpu::Features {
        self.device_features
    }

    /// Returns the actual limits exposed by the renderer-owned device.
    #[must_use]
    pub const fn device_limits(&self) -> &wgpu::Limits {
        &self.device_limits
    }

    /// Returns the controlled offscreen target format.
    #[must_use]
    pub const fn offscreen_format(&self) -> wgpu::TextureFormat {
        self.offscreen_format
    }

    /// Returns the selected presentable sRGB format for a renderer-owned surface.
    ///
    /// Headless and display-handle-only construction return `None` because those
    /// paths did not establish actual surface compatibility.
    #[must_use]
    pub const fn surface_format(&self) -> Option<wgpu::TextureFormat> {
        self.surface_format
    }
}

/// Validated physical pixel extent for one renderer-owned offscreen target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OffscreenExtent {
    width: u32,
    height: u32,
}

impl OffscreenExtent {
    /// Creates a non-zero physical target extent.
    ///
    /// # Errors
    ///
    /// Returns [`OffscreenRenderError::ZeroExtent`] when either dimension is zero.
    pub const fn new(width: u32, height: u32) -> Result<Self, OffscreenRenderError> {
        if width == 0 || height == 0 {
            Err(OffscreenRenderError::ZeroExtent { width, height })
        } else {
            Ok(Self { width, height })
        }
    }

    /// Returns the physical pixel width.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Returns the physical pixel height.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }
}

/// Exact continuous raster-space canvas bounds before integer texture rounding.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RasterCanvasExtent {
    width: f64,
    height: f64,
}

impl RasterCanvasExtent {
    const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    const fn width(self) -> f64 {
        self.width
    }

    const fn height(self) -> f64 {
        self.height
    }
}

/// Structured failure while clearing and reading a controlled offscreen target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OffscreenRenderError {
    ZeroExtent {
        width: u32,
        height: u32,
    },
    ExtentExceedsDeviceLimit {
        extent: OffscreenExtent,
        max_texture_dimension_2d: u32,
    },
    ReadbackLayoutOverflow {
        extent: OffscreenExtent,
    },
    ReadbackBufferExceedsDeviceLimit {
        required: u64,
        max_buffer_size: u64,
    },
    DevicePoll {
        detail: Arc<str>,
    },
    ReadbackCallbackClosed,
    BufferMap {
        detail: Arc<str>,
    },
    MappedRange {
        detail: Arc<str>,
    },
    PhysicalExtentOverflow,
    UnsupportedScene {
        item_index: Option<usize>,
        detail: Arc<str>,
    },
    UnsupportedTargetFormat {
        format: wgpu::TextureFormat,
    },
    TargetGenerationExhausted,
}

impl fmt::Display for OffscreenRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent { width, height } => {
                write!(
                    formatter,
                    "offscreen extent must be non-zero, got {width}x{height}"
                )
            }
            Self::ExtentExceedsDeviceLimit {
                extent,
                max_texture_dimension_2d,
            } => write!(
                formatter,
                "offscreen extent {}x{} exceeds device 2D texture limit {max_texture_dimension_2d}",
                extent.width(),
                extent.height()
            ),
            Self::ReadbackLayoutOverflow { extent } => write!(
                formatter,
                "offscreen readback layout overflows for {}x{}",
                extent.width(),
                extent.height()
            ),
            Self::ReadbackBufferExceedsDeviceLimit {
                required,
                max_buffer_size,
            } => write!(
                formatter,
                "offscreen readback buffer requires {required} bytes, exceeding device limit {max_buffer_size}"
            ),
            Self::DevicePoll { detail } => {
                write!(
                    formatter,
                    "device failed while waiting for offscreen readback: {detail}"
                )
            }
            Self::ReadbackCallbackClosed => {
                formatter.write_str("offscreen readback callback closed without a result")
            }
            Self::BufferMap { detail } => {
                write!(
                    formatter,
                    "offscreen readback buffer mapping failed: {detail}"
                )
            }
            Self::MappedRange { detail } => {
                write!(
                    formatter,
                    "offscreen readback mapped-range access failed: {detail}"
                )
            }
            Self::PhysicalExtentOverflow => formatter.write_str(
                "publication logical extent and raster scale exceed the renderer physical extent range",
            ),
            Self::UnsupportedScene { item_index, detail } => match item_index {
                Some(item_index) => write!(
                    formatter,
                    "renderer rejected scene item {item_index}: {detail}"
                ),
                None => write!(formatter, "renderer rejected scene: {detail}"),
            },
            Self::UnsupportedTargetFormat { format } => {
                write!(formatter, "renderer target format {format:?} is unsupported")
            }
            Self::TargetGenerationExhausted => {
                formatter.write_str("renderer exhausted its offscreen target generation space")
            }
        }
    }
}

impl Error for OffscreenRenderError {}

/// CPU-visible output copied from the actual renderer-owned wgpu texture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OffscreenReadback {
    extent: OffscreenExtent,
    format: wgpu::TextureFormat,
    rgba8_srgb: Vec<u8>,
}

/// Successful actual-GPU publication rendering and readback facts.
#[derive(Clone, Debug, PartialEq)]
pub struct OffscreenPublicationReadback {
    update_plan: PublicationUpdatePlan,
    target_generation: u64,
    readback: OffscreenReadback,
    observation: PublicationObservation,
}

impl OffscreenPublicationReadback {
    /// Returns the classification captured before rendering began.
    #[must_use]
    pub const fn update_plan(&self) -> PublicationUpdatePlan {
        self.update_plan
    }

    /// Returns the renderer-local generation of the retained target realization.
    #[must_use]
    pub const fn target_generation(&self) -> u64 {
        self.target_generation
    }

    /// Returns actual tightly packed GPU-derived target pixels.
    #[must_use]
    pub const fn readback(&self) -> &OffscreenReadback {
        &self.readback
    }

    /// Returns the immutable publication/backend/resource correlation record.
    #[must_use]
    pub const fn observation(&self) -> &PublicationObservation {
        &self.observation
    }
}

impl OffscreenReadback {
    /// Returns the physical target extent.
    #[must_use]
    pub const fn extent(&self) -> OffscreenExtent {
        self.extent
    }

    /// Returns the texture format copied to CPU memory.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Returns tightly packed row-major RGBA8 sRGB texels with copy padding removed.
    #[must_use]
    pub fn rgba8_srgb(&self) -> &[u8] {
        &self.rgba8_srgb
    }
}

#[derive(Debug)]
struct Renderer {
    offscreen_target: Option<OffscreenTarget>,
    last_observation: Option<PublicationObservation>,
    surface: Option<wgpu::Surface<'static>>,
    queue: wgpu::Queue,
    device: wgpu::Device,
    _adapter: wgpu::Adapter,
    _instance: wgpu::Instance,
    next_target_generation: u64,
    diagnostics: RendererDiagnostics,
}

#[derive(Debug)]
struct OffscreenTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    extent: OffscreenExtent,
    format: wgpu::TextureFormat,
    generation: u64,
    lineage: PublicationLineage,
}

impl OffscreenTarget {
    fn matches(&self, extent: OffscreenExtent, format: wgpu::TextureFormat) -> bool {
        self.extent == extent && self.format == format
    }
}

impl Renderer {
    async fn request(options: RendererOptions) -> Result<Self, RendererInitError> {
        let (instance, compiled_backends) = Self::create_instance(
            options,
            wgpu::InstanceDescriptor::new_without_display_handle(),
        )?;
        Self::request_with_instance(options, compiled_backends, instance, None).await
    }

    async fn request_with_display_handle(
        options: RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, RendererInitError> {
        let (instance, compiled_backends) = Self::create_instance(
            options,
            wgpu::InstanceDescriptor::new_with_display_handle(display),
        )?;
        Self::request_with_instance(options, compiled_backends, instance, None).await
    }

    async fn request_with_surface_target(
        options: RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
        window: impl wgpu::WindowHandle + 'static,
    ) -> Result<Self, RendererInitError> {
        let (instance, compiled_backends) = Self::create_instance(
            options,
            wgpu::InstanceDescriptor::new_with_display_handle(display),
        )?;
        let target = wgpu::SurfaceTarget::from_window_without_display(window);
        let surface = instance.create_surface(target).map_err(|error| {
            RendererInitError::SurfaceCreation {
                detail: error.to_string().into(),
            }
        })?;
        Self::request_with_instance(options, compiled_backends, instance, Some(surface)).await
    }

    fn create_instance(
        options: RendererOptions,
        mut instance_descriptor: wgpu::InstanceDescriptor,
    ) -> Result<(wgpu::Instance, wgpu::Backends), RendererInitError> {
        let compiled_backends = wgpu::Instance::enabled_backend_features();
        let requested_backends = options.backend_selection().wgpu_backends();
        if !compiled_backends.intersects(requested_backends) {
            return Err(RendererInitError::BackendUnavailable {
                requested: options.backend_selection(),
                compiled: compiled_backends,
            });
        }

        instance_descriptor.backends = requested_backends;
        Ok((wgpu::Instance::new(instance_descriptor), compiled_backends))
    }

    async fn request_with_instance(
        options: RendererOptions,
        compiled_backends: wgpu::Backends,
        instance: wgpu::Instance,
        surface: Option<wgpu::Surface<'static>>,
    ) -> Result<Self, RendererInitError> {
        let compatible_surface_required = surface.is_some();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference().wgpu_preference(),
                force_fallback_adapter: options.force_fallback_adapter(),
                compatible_surface: surface.as_ref(),
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| RendererInitError::AdapterUnavailable {
                requested: options.backend_selection(),
                compatible_surface_required,
                detail: error.to_string().into(),
            })?;
        let adapter_info = adapter.get_info();
        if matches!(
            adapter_info.backend,
            wgpu::Backend::Noop | wgpu::Backend::BrowserWebGpu
        ) {
            return Err(RendererInitError::DisallowedAdapterBackend {
                backend: adapter_info.backend,
            });
        }

        let surface_format = surface
            .as_ref()
            .map(|surface| select_surface_format(&surface.get_capabilities(&adapter).formats))
            .transpose()?;
        let adapter_features = adapter.features();
        let adapter_limits = adapter.limits();
        let requested_device_features = wgpu::Features::empty();
        let requested_device_limits =
            wgpu::Limits::downlevel_defaults().using_resolution(adapter_limits.clone());
        let device_descriptor = wgpu::DeviceDescriptor {
            label: Some(DEVICE_LABEL),
            required_features: requested_device_features,
            required_limits: requested_device_limits.clone(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        };
        let (device, queue) =
            adapter
                .request_device(&device_descriptor)
                .await
                .map_err(|error| RendererInitError::DeviceUnavailable {
                    adapter_name: adapter_info.name.clone().into(),
                    detail: error.to_string().into(),
                })?;
        let diagnostics = RendererDiagnostics {
            options,
            compiled_backends,
            adapter_info,
            adapter_features,
            adapter_limits,
            requested_device_features,
            requested_device_limits,
            device_features: device.features(),
            device_limits: device.limits(),
            offscreen_format: OFFSCREEN_FORMAT,
            surface_format,
        };
        Ok(Self {
            offscreen_target: None,
            last_observation: None,
            surface,
            queue,
            device,
            _adapter: adapter,
            _instance: instance,
            next_target_generation: 0,
            diagnostics,
        })
    }

    const fn diagnostics(&self) -> &RendererDiagnostics {
        &self.diagnostics
    }

    const fn last_observation(&self) -> Option<&PublicationObservation> {
        self.last_observation.as_ref()
    }

    fn record_observation(&mut self, observation: PublicationObservation) {
        self.last_observation = Some(observation);
    }

    const fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    fn discard_offscreen_target(&mut self) -> bool {
        self.offscreen_target.take().is_some()
    }

    fn clear_offscreen(
        &self,
        extent: OffscreenExtent,
        color: Color,
    ) -> Result<OffscreenReadback, OffscreenRenderError> {
        self.validate_extent(extent)?;
        let layout = ReadbackLayout::new(extent)?;
        self.validate_readback_buffer(layout)?;
        let (texture, view) = self.create_texture_target(extent, OFFSCREEN_FORMAT);
        let readback = self.create_readback_buffer(layout);
        let commands =
            self.encode_clear_and_copy(&texture, &view, &readback, extent, layout, color);
        let submission = self.queue.submit([commands]);
        let rgba8_srgb = self.map_readback(&readback, layout, submission)?;
        Ok(OffscreenReadback {
            extent,
            format: OFFSCREEN_FORMAT,
            rgba8_srgb,
        })
    }

    fn create_offscreen_target(
        &mut self,
        extent: OffscreenExtent,
    ) -> Result<OffscreenTarget, OffscreenRenderError> {
        let generation = self
            .next_target_generation
            .checked_add(1)
            .ok_or(OffscreenRenderError::TargetGenerationExhausted)?;
        let (texture, view) = self.create_texture_target(extent, OFFSCREEN_FORMAT);
        self.next_target_generation = generation;
        Ok(OffscreenTarget {
            texture,
            view,
            extent,
            format: OFFSCREEN_FORMAT,
            generation,
            lineage: PublicationLineage::new(),
        })
    }

    fn create_texture_target(
        &self,
        extent: OffscreenExtent,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("runenui offscreen target"),
            size: texture_extent(extent),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_readback_buffer(&self, layout: ReadbackLayout) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("runenui offscreen readback"),
            size: layout.buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }

    fn encode_clear_and_copy(
        &self,
        texture: &wgpu::Texture,
        view: &wgpu::TextureView,
        readback: &wgpu::Buffer,
        extent: OffscreenExtent,
        layout: ReadbackLayout,
        color: Color,
    ) -> wgpu::CommandBuffer {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("runenui offscreen clear encoder"),
            });
        {
            let color_attachment = Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu_clear_color(color)),
                    store: wgpu::StoreOp::Store,
                },
            });
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("runenui offscreen clear pass"),
                color_attachments: &[color_attachment],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        encode_target_copy(&mut encoder, texture, readback, extent, layout);
        encoder.finish()
    }

    fn map_readback(
        &self,
        readback: &wgpu::Buffer,
        layout: ReadbackLayout,
        submission: wgpu::SubmissionIndex,
    ) -> Result<Vec<u8>, OffscreenRenderError> {
        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            drop(sender.send(result));
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(READBACK_TIMEOUT),
            })
            .map_err(|error| OffscreenRenderError::DevicePoll {
                detail: error.to_string().into(),
            })?;
        receiver
            .recv()
            .map_err(|_| OffscreenRenderError::ReadbackCallbackClosed)?
            .map_err(|error| OffscreenRenderError::BufferMap {
                detail: error.to_string().into(),
            })?;
        let mapped =
            slice
                .get_mapped_range()
                .map_err(|error| OffscreenRenderError::MappedRange {
                    detail: error.to_string().into(),
                })?;
        let mut rgba8_srgb = Vec::with_capacity(layout.tight_buffer_size);
        for row in mapped.chunks_exact(layout.padded_bytes_per_row as usize) {
            rgba8_srgb.extend_from_slice(&row[..layout.tight_bytes_per_row]);
        }
        drop(mapped);
        readback.unmap();
        Ok(rgba8_srgb)
    }

    const fn validate_extent(&self, extent: OffscreenExtent) -> Result<(), OffscreenRenderError> {
        let max_texture_dimension_2d = self.diagnostics.device_limits.max_texture_dimension_2d;
        if extent.width() > max_texture_dimension_2d || extent.height() > max_texture_dimension_2d {
            Err(OffscreenRenderError::ExtentExceedsDeviceLimit {
                extent,
                max_texture_dimension_2d,
            })
        } else {
            Ok(())
        }
    }

    const fn validate_readback_buffer(
        &self,
        layout: ReadbackLayout,
    ) -> Result<(), OffscreenRenderError> {
        let max_buffer_size = self.diagnostics.device_limits.max_buffer_size;
        if layout.buffer_size > max_buffer_size {
            Err(OffscreenRenderError::ReadbackBufferExceedsDeviceLimit {
                required: layout.buffer_size,
                max_buffer_size,
            })
        } else {
            Ok(())
        }
    }
}

fn select_surface_format(
    advertised_formats: &[wgpu::TextureFormat],
) -> Result<wgpu::TextureFormat, RendererInitError> {
    advertised_formats
        .iter()
        .copied()
        .find(|format| SUPPORTED_SURFACE_FORMATS.contains(format))
        .ok_or_else(|| RendererInitError::SurfaceFormatUnavailable {
            advertised_formats: advertised_formats.into(),
        })
}

fn scene_validation_error(error: SceneValidationError) -> OffscreenRenderError {
    match error {
        SceneValidationError::UnsupportedResourceKind { resource_kind } => {
            OffscreenRenderError::UnsupportedScene {
                item_index: None,
                detail: format!("unsupported resource kind {resource_kind:?}").into(),
            }
        }
        SceneValidationError::UnsupportedItem {
            item_index,
            semantic,
        } => OffscreenRenderError::UnsupportedScene {
            item_index: Some(item_index),
            detail: format!("unsupported paint semantics: {semantic:?}").into(),
        },
    }
}

fn publication_extents(
    publication: &PaintPublication,
) -> Result<(RasterCanvasExtent, OffscreenExtent), OffscreenRenderError> {
    let logical_size = publication.logical_size();
    let scale = f64::from(publication.raster_scale().get());
    let canvas_extent = RasterCanvasExtent::new(
        f64::from(logical_size.width()) * scale,
        f64::from(logical_size.height()) * scale,
    );
    let width = texture_dimension(canvas_extent.width())?;
    let height = texture_dimension(canvas_extent.height())?;
    Ok((canvas_extent, OffscreenExtent::new(width, height)?))
}

fn texture_dimension(physical_canvas_dimension: f64) -> Result<u32, OffscreenRenderError> {
    let rounded = physical_canvas_dimension.ceil();
    if !rounded.is_finite() || rounded > f64::from(u32::MAX) {
        Err(OffscreenRenderError::PhysicalExtentOverflow)
    } else {
        Ok(physical_to_u32(rounded))
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the caller proves the finite ceil result is within the complete u32 range"
)]
const fn physical_to_u32(physical: f64) -> u32 {
    physical as u32
}

fn encode_target_copy(
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    readback: &wgpu::Buffer,
    extent: OffscreenExtent,
    layout: ReadbackLayout,
) {
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(layout.padded_bytes_per_row),
                rows_per_image: Some(extent.height()),
            },
        },
        texture_extent(extent),
    );
}

fn transformed_fill_polygon(
    fill: &SupportedFillRect,
    canvas_extent: RasterCanvasExtent,
    raster_scale: RasterScale,
) -> Vec<[f64; 2]> {
    if fill.local_to_surface.inverse().is_none() {
        return Vec::new();
    }
    let [m11, m12, m21, m22, tx, ty] = fill.local_to_surface.components().map(f64::from);
    let scale = f64::from(raster_scale.get());
    let local_corners = [
        [f64::from(fill.rect.x()), f64::from(fill.rect.y())],
        [f64::from(fill.rect.x()), f64::from(fill.rect.max_y())],
        [f64::from(fill.rect.max_x()), f64::from(fill.rect.max_y())],
        [f64::from(fill.rect.max_x()), f64::from(fill.rect.y())],
    ];
    let polygon = local_corners
        .into_iter()
        .map(|[x, y]| {
            [
                m11.mul_add(x, m21.mul_add(y, tx)) * scale,
                m12.mul_add(x, m22.mul_add(y, ty)) * scale,
            ]
        })
        .collect::<Vec<_>>();
    clip_polygon_to_canvas(polygon, canvas_extent)
}

#[derive(Clone, Copy, Debug)]
enum CanvasEdge {
    Left,
    Right,
    Top,
    Bottom,
}

fn clip_polygon_to_canvas(
    mut polygon: Vec<[f64; 2]>,
    canvas_extent: RasterCanvasExtent,
) -> Vec<[f64; 2]> {
    for edge in [
        CanvasEdge::Left,
        CanvasEdge::Right,
        CanvasEdge::Top,
        CanvasEdge::Bottom,
    ] {
        polygon = clip_polygon_against_edge(&polygon, edge, canvas_extent);
        if polygon.is_empty() {
            break;
        }
    }
    polygon
}

fn clip_polygon_against_edge(
    polygon: &[[f64; 2]],
    edge: CanvasEdge,
    canvas_extent: RasterCanvasExtent,
) -> Vec<[f64; 2]> {
    let Some(&last) = polygon.last() else {
        return Vec::new();
    };
    let mut output = Vec::with_capacity(polygon.len().saturating_add(1));
    let mut previous = last;
    let mut previous_inside = canvas_edge_contains(edge, previous, canvas_extent);
    for &current in polygon {
        let current_inside = canvas_edge_contains(edge, current, canvas_extent);
        match (previous_inside, current_inside) {
            (true, true) => output.push(current),
            (true, false) => output.push(canvas_edge_intersection(
                edge,
                previous,
                current,
                canvas_extent,
            )),
            (false, true) => {
                output.push(canvas_edge_intersection(
                    edge,
                    previous,
                    current,
                    canvas_extent,
                ));
                output.push(current);
            }
            (false, false) => {}
        }
        previous = current;
        previous_inside = current_inside;
    }
    output
}

fn canvas_edge_contains(
    edge: CanvasEdge,
    point: [f64; 2],
    canvas_extent: RasterCanvasExtent,
) -> bool {
    match edge {
        CanvasEdge::Left => point[0] >= 0.0,
        CanvasEdge::Right => point[0] <= canvas_extent.width(),
        CanvasEdge::Top => point[1] >= 0.0,
        CanvasEdge::Bottom => point[1] <= canvas_extent.height(),
    }
}

fn canvas_edge_intersection(
    edge: CanvasEdge,
    from: [f64; 2],
    to: [f64; 2],
    canvas_extent: RasterCanvasExtent,
) -> [f64; 2] {
    match edge {
        CanvasEdge::Left => vertical_edge_intersection(0.0, from, to),
        CanvasEdge::Right => vertical_edge_intersection(canvas_extent.width(), from, to),
        CanvasEdge::Top => horizontal_edge_intersection(0.0, from, to),
        CanvasEdge::Bottom => horizontal_edge_intersection(canvas_extent.height(), from, to),
    }
}

fn vertical_edge_intersection(x: f64, from: [f64; 2], to: [f64; 2]) -> [f64; 2] {
    let denominator = to[0] - from[0];
    let t = if denominator == 0.0 {
        0.0
    } else {
        ((x - from[0]) / denominator).clamp(0.0, 1.0)
    };
    [x, (to[1] - from[1]).mul_add(t, from[1])]
}

fn horizontal_edge_intersection(y: f64, from: [f64; 2], to: [f64; 2]) -> [f64; 2] {
    let denominator = to[1] - from[1];
    let t = if denominator == 0.0 {
        0.0
    } else {
        ((y - from[1]) / denominator).clamp(0.0, 1.0)
    };
    [(to[0] - from[0]).mul_add(t, from[0]), y]
}

fn physical_point_to_ndc(point: [f64; 2], extent: OffscreenExtent) -> [f32; 2] {
    let width = f64::from(extent.width());
    let height = f64::from(extent.height());
    let x = point[0].clamp(0.0, width);
    let y = point[1].clamp(0.0, height);
    [
        normalized_position((x / width).mul_add(2.0, -1.0)),
        normalized_position((y / height).mul_add(-2.0, 1.0)),
    ]
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "canvas clipping bounds normalized coordinates to the finite [-1, 1] GPU range"
)]
const fn normalized_position(value: f64) -> f32 {
    value.clamp(-1.0, 1.0) as f32
}

fn srgb8_to_linear_f32(value: u8) -> f32 {
    let encoded = f32::from(value) / 255.0;
    if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

const fn texture_extent(extent: OffscreenExtent) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: extent.width(),
        height: extent.height(),
        depth_or_array_layers: 1,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReadbackLayout {
    tight_bytes_per_row: usize,
    padded_bytes_per_row: u32,
    tight_buffer_size: usize,
    buffer_size: u64,
}

impl ReadbackLayout {
    fn new(extent: OffscreenExtent) -> Result<Self, OffscreenRenderError> {
        let tight_bytes_per_row_u32 = extent
            .width()
            .checked_mul(4)
            .ok_or(OffscreenRenderError::ReadbackLayoutOverflow { extent })?;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = tight_bytes_per_row_u32
            .checked_add(alignment - 1)
            .map(|value| value / alignment * alignment)
            .ok_or(OffscreenRenderError::ReadbackLayoutOverflow { extent })?;
        let tight_buffer_size_u64 = u64::from(tight_bytes_per_row_u32)
            .checked_mul(u64::from(extent.height()))
            .ok_or(OffscreenRenderError::ReadbackLayoutOverflow { extent })?;
        let buffer_size = u64::from(padded_bytes_per_row)
            .checked_mul(u64::from(extent.height()))
            .ok_or(OffscreenRenderError::ReadbackLayoutOverflow { extent })?;
        let tight_bytes_per_row = usize::try_from(tight_bytes_per_row_u32)
            .map_err(|_| OffscreenRenderError::ReadbackLayoutOverflow { extent })?;
        let tight_buffer_size = usize::try_from(tight_buffer_size_u64)
            .map_err(|_| OffscreenRenderError::ReadbackLayoutOverflow { extent })?;
        Ok(Self {
            tight_bytes_per_row,
            padded_bytes_per_row,
            tight_buffer_size,
            buffer_size,
        })
    }
}

fn wgpu_clear_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: srgb8_to_linear(color.red()),
        g: srgb8_to_linear(color.green()),
        b: srgb8_to_linear(color.blue()),
        a: f64::from(color.alpha()) / 255.0,
    }
}

fn srgb8_to_linear(value: u8) -> f64 {
    let encoded = f64::from(value) / 255.0;
    if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::{OffscreenExtent, ReadbackLayout, RendererInitError, select_surface_format};

    #[test]
    fn readback_layout_aligns_rows_without_changing_tight_size() {
        let extent = OffscreenExtent::new(17, 3)
            .unwrap_or_else(|_| unreachable!("controlled extent is valid"));
        let layout = ReadbackLayout::new(extent)
            .unwrap_or_else(|_| unreachable!("controlled layout is valid"));
        assert_eq!(layout.tight_bytes_per_row, 68);
        assert_eq!(layout.tight_buffer_size, 204);
        assert_eq!(
            layout.padded_bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
            0
        );
    }

    #[test]
    fn surface_format_selection_is_explicit_and_fail_closed() {
        assert_eq!(
            select_surface_format(&[
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            ]),
            Ok(wgpu::TextureFormat::Bgra8UnormSrgb)
        );
        assert!(matches!(
            select_surface_format(&[wgpu::TextureFormat::Rgba16Float]),
            Err(RendererInitError::SurfaceFormatUnavailable { .. })
        ));
    }
}
