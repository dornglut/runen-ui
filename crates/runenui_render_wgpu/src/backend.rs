use core::{error::Error, fmt};
use std::{collections::HashMap, sync::Arc, time::Duration};

use runenui_core::Color;
use runenui_runtime::{PaintPublication, RasterScale};
use wgpu::util::DeviceExt;

use crate::{
    PublicationUpdateMode, PublicationUpdatePlan, WgpuHasDisplayHandle,
    lineage::PublicationLineage,
    scene_subset::{SceneValidationError, SupportedFillRect, validate_scene_subset},
};

const DEVICE_LABEL: &str = "runenui_render_wgpu device";
const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SUPPORTED_SURFACE_FORMATS: [wgpu::TextureFormat; 2] = [
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Bgra8UnormSrgb,
];
const READBACK_TIMEOUT: Duration = Duration::from_secs(30);
const FILL_RECT_SHADER: &str = r"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
";

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

    /// Controls whether wgpu must select a fallback adapter.
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
                    "offscreen readback mapped range failed: {detail}"
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

impl Error for OffscreenRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// CPU-visible output copied from the actual renderer-owned wgpu texture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OffscreenReadback {
    extent: OffscreenExtent,
    format: wgpu::TextureFormat,
    rgba8_srgb: Vec<u8>,
}

/// Successful actual-GPU publication rendering and readback facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OffscreenPublicationReadback {
    update_plan: PublicationUpdatePlan,
    target_generation: u64,
    readback: OffscreenReadback,
}

impl OffscreenPublicationReadback {
    /// Returns the classification captured before rendering began.
    #[must_use]
    pub const fn update_plan(&self) -> PublicationUpdatePlan {
        self.update_plan
    }

    /// Returns the renderer-local generation of the retained target realization.
    ///
    /// Equal generations identify the same still-live offscreen target only
    /// within this renderer instance.
    #[must_use]
    pub const fn target_generation(&self) -> u64 {
        self.target_generation
    }

    /// Returns actual tightly packed GPU-derived target pixels.
    #[must_use]
    pub const fn readback(&self) -> &OffscreenReadback {
        &self.readback
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

/// Disposable renderer-owned native wgpu state.
#[derive(Debug)]
pub struct Renderer {
    offscreen_target: Option<OffscreenTarget>,
    fill_rect_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    // Declared before the device/adapter/instance so it is dropped first. The
    // static lifetime is earned by moving an owned handle source into wgpu.
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
    /// Selects a native adapter and creates a renderer-owned wgpu device and queue.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request(options: RendererOptions) -> Result<Self, RendererInitError> {
        let (instance, compiled_backends) = Self::create_instance(
            options,
            wgpu::InstanceDescriptor::new_without_display_handle(),
        )?;
        Self::request_with_instance(options, compiled_backends, instance, None).await
    }

    /// Selects a native adapter using a caller-owned display connection.
    ///
    /// The handle is retained by wgpu's instance descriptor. This path is
    /// independent of winit and does not create a surface or own an event loop.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request_with_display_handle(
        options: RendererOptions,
        display: Box<dyn WgpuHasDisplayHandle>,
    ) -> Result<Self, RendererInitError> {
        let (instance, compiled_backends) = Self::create_instance(
            options,
            wgpu::InstanceDescriptor::new_with_display_handle(display),
        )?;
        Self::request_with_instance(options, compiled_backends, instance, None).await
    }

    /// Creates and retains a native surface before selecting a compatible adapter.
    ///
    /// The target must own its handle source for the renderer's full lifetime.
    /// wgpu stores that source inside the resulting `Surface<'static>`, so a later
    /// winit host can pass an `Arc<Window>` while retaining its own clone for native
    /// mechanics. This method creates no event loop, configures no swapchain, and
    /// performs no presentation. On macOS, wgpu requires surface creation to occur
    /// on the main thread.
    ///
    /// # Errors
    ///
    /// Returns structured surface-creation, compatible-adapter, target-format, or
    /// device diagnostics when construction fails.
    pub async fn request_with_surface_target(
        options: RendererOptions,
        target: impl wgpu::DisplayAndWindowHandle + 'static,
    ) -> Result<Self, RendererInitError> {
        let (instance, compiled_backends) = Self::create_instance(
            options,
            wgpu::InstanceDescriptor::new_without_display_handle(),
        )?;
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
        let mut fill_rect_pipelines = HashMap::new();
        if let Some(format) = surface_format {
            fill_rect_pipelines.insert(format, create_fill_rect_pipeline(&device, format));
        }
        Ok(Self {
            offscreen_target: None,
            fill_rect_pipelines,
            surface,
            queue,
            device,
            _adapter: adapter,
            _instance: instance,
            next_target_generation: 0,
            diagnostics,
        })
    }

    /// Returns immutable instance, adapter, device, and target diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &RendererDiagnostics {
        &self.diagnostics
    }

    /// Returns whether construction retained an actual native surface target.
    #[must_use]
    pub const fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    /// Drops the retained offscreen target and every publication realization tied to it.
    ///
    /// The next offscreen publication necessarily creates a fresh target and full-resyncs.
    #[must_use]
    pub fn discard_offscreen_target(&mut self) -> bool {
        self.offscreen_target.take().is_some()
    }

    /// Renders the exact supported scene subset to an offscreen target and reads actual GPU bytes.
    ///
    /// The retained target is initialized to transparent black independently of
    /// authored scene paint. Validation completes before target creation or GPU
    /// submission. Successful lineage is recorded only after submission, polling,
    /// mapping, padding removal, and readback all succeed. Target creation or
    /// rebuild forces a full resync; an already-current result is possible only
    /// while the exact target realization remains retained.
    ///
    /// # Errors
    ///
    /// Returns a deterministic scene, extent, target-format, device, or readback
    /// failure. Pre-submission failures retain the previous target realization;
    /// post-submission failures drop it conservatively so mutated GPU state cannot
    /// retain stale lineage.
    pub fn render_offscreen_publication(
        &mut self,
        publication: &PaintPublication,
    ) -> Result<OffscreenPublicationReadback, OffscreenRenderError> {
        let fill_rects = validate_scene_subset(publication).map_err(scene_validation_error)?;
        let extent = publication_extent(publication)?;
        self.validate_extent(extent)?;
        let layout = ReadbackLayout::new(extent)?;
        self.validate_readback_buffer(layout)?;
        self.ensure_fill_rect_pipeline(OFFSCREEN_FORMAT)?;

        let retained_target_matches = self
            .offscreen_target
            .as_ref()
            .is_some_and(|target| target.matches(extent, OFFSCREEN_FORMAT));
        let update_plan = if retained_target_matches {
            self.offscreen_target
                .as_ref()
                .map_or_else(PublicationUpdatePlan::full_resync, |target| {
                    target.lineage.plan(publication)
                })
        } else {
            PublicationUpdatePlan::full_resync()
        };

        if !retained_target_matches {
            let target = self.create_offscreen_target(extent)?;
            self.offscreen_target = Some(target);
        }

        let readback = self.create_readback_buffer(layout);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("runenui offscreen publication encoder"),
            });
        let target = self
            .offscreen_target
            .as_ref()
            .unwrap_or_else(|| unreachable!("matching or newly created target is retained"));
        let target_generation = target.generation;
        if update_plan.mode() != PublicationUpdateMode::AlreadyCurrent {
            let pipeline = self
                .fill_rect_pipelines
                .get(&target.format)
                .unwrap_or_else(|| unreachable!("target pipeline was realized before encoding"));
            encode_scene_to_target(
                &self.device,
                pipeline,
                &mut encoder,
                &target.view,
                extent,
                publication.raster_scale(),
                &fill_rects,
            );
        }
        encode_target_copy(&mut encoder, &target.texture, &readback, extent, layout);
        let submission = self.queue.submit([encoder.finish()]);
        let rgba8_srgb = match self.map_readback(&readback, layout, submission) {
            Ok(pixels) => pixels,
            Err(error) => {
                self.offscreen_target = None;
                return Err(error);
            }
        };
        let readback = OffscreenReadback {
            extent,
            format: OFFSCREEN_FORMAT,
            rgba8_srgb,
        };

        self.offscreen_target
            .as_mut()
            .unwrap_or_else(|| unreachable!("successful readback retains its target"))
            .lineage
            .record_success(publication);
        Ok(OffscreenPublicationReadback {
            update_plan,
            target_generation,
            readback,
        })
    }

    /// Executes one real wgpu render-pass clear and returns actual texture bytes from GPU readback.
    ///
    /// The supplied [`Color`] is unpremultiplied sRGB8. Color channels are
    /// linearized before wgpu clears the sRGB target; alpha remains linear.
    /// This is a low-level backend diagnostic: it consumes no [`PaintPublication`]
    /// and never mutates publication lineage.
    ///
    /// # Errors
    ///
    /// Returns structured extent, device-wait, buffer-map, or mapped-range failures.
    pub fn clear_offscreen(
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

    fn ensure_fill_rect_pipeline(
        &mut self,
        format: wgpu::TextureFormat,
    ) -> Result<(), OffscreenRenderError> {
        if !matches!(
            format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(OffscreenRenderError::UnsupportedTargetFormat { format });
        }
        self.fill_rect_pipelines
            .entry(format)
            .or_insert_with(|| create_fill_rect_pipeline(&self.device, format));
        Ok(())
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

/// Shared target drawing implementation. Target format/pipeline realization is
/// supplied by the renderer; target-specific completion and publication lineage
/// remain the caller's responsibility.
fn encode_scene_to_target(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    extent: OffscreenExtent,
    raster_scale: RasterScale,
    fill_rects: &[SupportedFillRect],
) {
    let vertex_bytes = fill_rect_vertex_bytes(fill_rects, extent, raster_scale);
    let vertex_buffer = (!vertex_bytes.is_empty()).then(|| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("runenui FillRect vertices"),
            contents: &vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        })
    });
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    });
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("runenui scene render pass"),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(vertex_buffer) = vertex_buffer.as_ref() {
        render_pass.set_pipeline(pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        let vertex_count = u32::try_from(fill_rects.len())
            .unwrap_or(u32::MAX)
            .saturating_mul(6);
        render_pass.draw(0..vertex_count, 0..1);
    }
}

const FILL_RECT_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 8,
        shader_location: 1,
    },
];

fn create_fill_rect_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui FillRect shader"),
        source: wgpu::ShaderSource::Wgsl(FILL_RECT_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui FillRect pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("runenui FillRect pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &FILL_RECT_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
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

fn publication_extent(
    publication: &PaintPublication,
) -> Result<OffscreenExtent, OffscreenRenderError> {
    let logical_size = publication.logical_size();
    let scale = publication.raster_scale();
    let width = scaled_dimension(logical_size.width(), scale)?;
    let height = scaled_dimension(logical_size.height(), scale)?;
    OffscreenExtent::new(width, height)
}

fn scaled_dimension(logical: f32, scale: RasterScale) -> Result<u32, OffscreenRenderError> {
    let physical = (f64::from(logical) * f64::from(scale.get())).ceil();
    if !physical.is_finite() || physical > f64::from(u32::MAX) {
        Err(OffscreenRenderError::PhysicalExtentOverflow)
    } else {
        Ok(physical_to_u32(physical))
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

fn fill_rect_vertex_bytes(
    fill_rects: &[SupportedFillRect],
    extent: OffscreenExtent,
    raster_scale: RasterScale,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(fill_rects.len().saturating_mul(6 * 24));
    for fill in fill_rects {
        let scale_x = f64::from(extent.width());
        let scale_y = f64::from(extent.height());
        let logical_to_physical = |value: f32, dimension: f64| {
            (f64::from(value) * f64::from(raster_scale.get())).clamp(0.0, dimension)
        };
        let left = logical_to_physical(fill.rect.x(), scale_x);
        let top = logical_to_physical(fill.rect.y(), scale_y);
        let right = logical_to_physical(fill.rect.max_x(), scale_x);
        let bottom = logical_to_physical(fill.rect.max_y(), scale_y);
        let left = normalized_position((left / scale_x).mul_add(2.0, -1.0));
        let right = normalized_position((right / scale_x).mul_add(2.0, -1.0));
        let top = normalized_position((top / scale_y).mul_add(-2.0, 1.0));
        let bottom = normalized_position((bottom / scale_y).mul_add(-2.0, 1.0));
        let color = [
            srgb8_to_linear_f32(fill.color.red()),
            srgb8_to_linear_f32(fill.color.green()),
            srgb8_to_linear_f32(fill.color.blue()),
            1.0,
        ];
        for [x, y] in [
            [left, top],
            [left, bottom],
            [right, top],
            [right, top],
            [left, bottom],
            [right, bottom],
        ] {
            push_vertex(&mut bytes, [x, y], color);
        }
    }
    bytes
}

fn push_vertex(bytes: &mut Vec<u8>, position: [f32; 2], color: [f32; 4]) {
    for component in position.into_iter().chain(color) {
        bytes.extend_from_slice(&component.to_ne_bytes());
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the value is clamped and normalized to [-1, 1] before conversion to shader f32"
)]
const fn normalized_position(value: f64) -> f32 {
    value as f32
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
    #![allow(refining_impl_trait)]

    use core::{error::Error, future::Future, pin::pin, task::Poll};
    use std::{
        fs,
        sync::Arc,
        task::{Context, Wake, Waker},
        thread,
    };

    use image::{ImageEncoder, codecs::png::PngEncoder};
    use runenui_core::{
        Color, ContributionClip, Element, LogicalLength, LogicalPoint, LogicalRect, LogicalSize,
        LogicalTransform, NoHostProtocol, PaintContribution, PaintContributionContext,
        PaintContributionItem, PaintPrimitive, Radius, ResourceKind, ResourceRef, SceneOpacity,
        SceneShape, StyleTokens, UiApp, Widget, WidgetInvalidation, WidgetMeasure,
        WidgetUpdateContext,
    };
    use runenui_runtime::{
        AppRuntime, LayoutConstraints, PaintPublication, PumpBudget, RasterScale,
        SurfaceBuildContext,
    };

    use super::{
        BackendSelection, OffscreenExtent, OffscreenRenderError, ReadbackLayout, Renderer,
        RendererInitError, RendererOptions, select_surface_format,
    };
    use crate::{
        PublicationUpdateMode,
        scene_subset::{SceneValidationError, UnsupportedSceneSemantic},
    };

    const SURFACE_WIDTH: u16 = 64;
    const SURFACE_HEIGHT: u16 = 48;

    #[derive(Clone, Debug)]
    struct SceneFixture {
        items: Vec<PaintContributionItem>,
    }

    impl Widget<Vec<PaintContributionItem>> for SceneFixture {
        type State = Vec<PaintContributionItem>;

        fn create_state(&self) -> Self::State {
            self.items.clone()
        }

        fn update(
            &self,
            state: &mut Self::State,
            context: &mut WidgetUpdateContext<Vec<PaintContributionItem>>,
        ) {
            *state = self.items.clone();
            context.invalidate(WidgetInvalidation::PAINT);
        }

        fn measure(&self, _: &Self::State) -> WidgetMeasure {
            WidgetMeasure::Fixed {
                width: LogicalLength::from(SURFACE_WIDTH),
                height: LogicalLength::from(SURFACE_HEIGHT),
            }
        }

        fn paint(&self, items: &Self::State, _: PaintContributionContext) -> PaintContribution {
            PaintContribution::new(items.clone())
        }
    }

    struct FixtureApp;

    impl UiApp for FixtureApp {
        type State = Vec<PaintContributionItem>;
        type Action = Vec<PaintContributionItem>;
        type HostProtocol = NoHostProtocol;

        fn root(items: &Self::State) -> Element<Self::Action> {
            Element::new(SceneFixture {
                items: items.clone(),
            })
        }

        fn update(items: &mut Self::State, replacement: Self::Action) {
            *items = replacement;
        }
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
    }

    fn publication(items: Vec<PaintContributionItem>, scale: f32) -> PaintPublication {
        publication_sequence(items, &[scale])
            .pop()
            .unwrap_or_else(|| unreachable!("one fixture publication was requested"))
    }

    fn publication_sequence(
        items: Vec<PaintContributionItem>,
        scales: &[f32],
    ) -> Vec<PaintPublication> {
        let mut runtime = AppRuntime::<FixtureApp>::mount(items);
        let tokens = StyleTokens::new();
        let logical_size =
            LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
                .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
        scales
            .iter()
            .map(|scale| {
                let raster_scale = RasterScale::new(*scale)
                    .unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
                let context =
                    SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(logical_size))
                        .with_raster_scale(raster_scale);
                runtime
                    .publish_surface(&context)
                    .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
                    .paint_publication()
                    .clone()
            })
            .collect()
    }

    fn publication_update_and_scale(
        initial_items: Vec<PaintContributionItem>,
        updated_items: Vec<PaintContributionItem>,
        initial_scale: f32,
        rebuilt_scale: f32,
    ) -> (PaintPublication, PaintPublication, PaintPublication) {
        let mut runtime = AppRuntime::<FixtureApp>::mount(initial_items);
        let tokens = StyleTokens::new();
        let logical_size =
            LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
                .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
        let raster_scale = RasterScale::new(initial_scale)
            .unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(logical_size))
            .with_raster_scale(raster_scale);
        let first = runtime
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("initial fixture publication is admitted"))
            .paint_publication()
            .clone();
        runtime
            .submit_action(updated_items)
            .unwrap_or_else(|_| unreachable!("fixture update is admitted"));
        let report = runtime.pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ));
        assert!(report.is_quiescent());
        let second = runtime
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("updated fixture publication is admitted"))
            .paint_publication()
            .clone();
        let rebuilt_raster_scale = RasterScale::new(rebuilt_scale)
            .unwrap_or_else(|_| unreachable!("rebuilt fixture raster scale is valid"));
        let rebuilt_context =
            SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(logical_size))
                .with_raster_scale(rebuilt_raster_scale);
        let rebuilt = runtime
            .publish_surface(&rebuilt_context)
            .unwrap_or_else(|_| unreachable!("rebuilt fixture publication is admitted"))
            .paint_publication()
            .clone();
        (first, second, rebuilt)
    }

    fn pixel(readback: &super::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
        let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
        readback.rgba8_srgb()[index..index + 4]
            .try_into()
            .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
    }

    fn assert_order_probes(readback: &super::OffscreenReadback, scale: u32) {
        let scaled = |logical: u32| logical * scale;
        assert_eq!(pixel(readback, scaled(2), scaled(2)), [0, 0, 0, 0]);
        assert_eq!(
            pixel(readback, scaled(10), scaled(8)),
            [0xC3, 0x4A, 0x42, 0xFF]
        );
        assert_eq!(
            pixel(readback, scaled(35), scaled(20)),
            [0x37, 0x86, 0xC8, 0xFF]
        );
        assert_eq!(
            pixel(readback, scaled(22), scaled(14)),
            [0x37, 0x86, 0xC8, 0xFF],
            "the later FillRect must win in the overlap"
        );
    }

    fn prove_png_round_trip(readback: &super::OffscreenReadback) -> Result<(), Box<dyn Error>> {
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(
            readback.rgba8_srgb(),
            readback.extent().width(),
            readback.extent().height(),
            image::ExtendedColorType::Rgba8,
        )?;
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let decoded =
            image::load_from_memory_with_format(&png, image::ImageFormat::Png)?.into_rgba8();
        assert_eq!(decoded.as_raw(), readback.rgba8_srgb());
        let artifact_directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/runenui-render-wgpu-proof");
        fs::create_dir_all(&artifact_directory)?;
        let artifact = artifact_directory.join("fill-rect-order.png");
        fs::write(&artifact, &png)?;
        eprintln!(
            "PNG ENCODING PROOF: encoded and exact-decoded {} GPU bytes to {} PNG bytes; artifact={}",
            readback.rgba8_srgb().len(),
            png.len(),
            artifact.display(),
        );
        eprintln!(
            "GOLDEN COMPARISON PROOF: not claimed by this checkpoint; no expected golden was read or overwritten"
        );
        Ok(())
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
                    "native wgpu proof unavailable under {requested:?}; structured adapter failure: {detail}"
                );
                assert_eq!(requested, BackendSelection::AllNative);
                assert!(!compatible_surface_required);
                assert!(!detail.is_empty());
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    #[test]
    fn backend_policy_excludes_webgpu_and_noop() {
        let native = BackendSelection::AllNative.wgpu_backends();
        assert!(!native.contains(wgpu::Backends::BROWSER_WEBGPU));
        assert!(!native.contains(wgpu::Backends::NOOP));
        assert!(native.contains(wgpu::Backends::METAL));
        assert!(native.contains(wgpu::Backends::VULKAN));
        assert!(native.contains(wgpu::Backends::DX12));
        assert!(native.contains(wgpu::Backends::GL));
    }

    #[test]
    fn surface_format_policy_honors_supported_advertised_order_and_rejects_other_formats() {
        assert_eq!(
            select_surface_format(&[
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ]),
            Ok(wgpu::TextureFormat::Bgra8UnormSrgb),
            "the first renderer-supported format honors wgpu's surface preference order"
        );
        assert_eq!(
            select_surface_format(&[wgpu::TextureFormat::Rgba8UnormSrgb]),
            Ok(wgpu::TextureFormat::Rgba8UnormSrgb),
            "the controlled RGBA8 sRGB format remains supported"
        );
        assert_eq!(
            select_surface_format(&[
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba16Float,
            ]),
            Err(RendererInitError::SurfaceFormatUnavailable {
                advertised_formats: Arc::from([
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureFormat::Rgba16Float,
                ]),
            }),
            "a non-sRGB surface cannot silently change the renderer color contract"
        );
        assert_eq!(
            select_surface_format(&[]),
            Err(RendererInitError::SurfaceFormatUnavailable {
                advertised_formats: Arc::from([]),
            }),
            "an adapter-incompatible empty capability list fails structurally"
        );
    }

    #[test]
    fn offscreen_extent_and_readback_layout_fail_deterministically() {
        assert_eq!(
            OffscreenExtent::new(0, 1),
            Err(OffscreenRenderError::ZeroExtent {
                width: 0,
                height: 1,
            })
        );
        let overflowing = OffscreenExtent::new(u32::MAX, 1)
            .unwrap_or_else(|_| unreachable!("fixture extent is non-zero"));
        assert!(matches!(
            ReadbackLayout::new(overflowing),
            Err(OffscreenRenderError::ReadbackLayoutOverflow { extent }) if extent == overflowing
        ));
    }

    #[test]
    fn real_native_wgpu_clear_is_cpu_visible_without_row_padding() -> Result<(), Box<dyn Error>> {
        let Some(renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let diagnostics = renderer.diagnostics();
        assert!(!renderer.has_surface());
        assert_eq!(diagnostics.surface_format(), None);
        eprintln!(
            "real wgpu adapter: name={:?} backend={} device_type={:?} driver={:?} driver_info={:?} format={:?}",
            diagnostics.adapter_info().name,
            diagnostics.adapter_info().backend,
            diagnostics.adapter_info().device_type,
            diagnostics.adapter_info().driver,
            diagnostics.adapter_info().driver_info,
            diagnostics.offscreen_format(),
        );
        assert!(!matches!(
            diagnostics.adapter_info().backend,
            wgpu::Backend::Noop | wgpu::Backend::BrowserWebGpu
        ));

        if let Some(unsupported_width) = diagnostics
            .device_limits()
            .max_texture_dimension_2d
            .checked_add(1)
        {
            let unsupported_extent = OffscreenExtent::new(unsupported_width, 1)?;
            assert!(matches!(
                renderer.clear_offscreen(unsupported_extent, Color::BLACK),
                Err(OffscreenRenderError::ExtentExceedsDeviceLimit { extent, .. })
                    if extent == unsupported_extent
            ));
        }

        let extent = OffscreenExtent::new(3, 2)?;
        let readback = renderer.clear_offscreen(extent, Color::rgba(0, 255, 0, 255))?;
        assert_eq!(readback.extent(), extent);
        assert_eq!(readback.format(), wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(readback.rgba8_srgb().len(), 3 * 2 * 4);
        let (pixels, remainder) = readback.rgba8_srgb().as_chunks::<4>();
        assert!(remainder.is_empty());
        assert!(pixels.iter().all(|pixel| pixel == &[0, 255, 0, 255]));
        Ok(())
    }

    #[test]
    fn device_policy_is_explicit_and_distinguishes_adapter_request_and_device()
    -> Result<(), Box<dyn Error>> {
        let Some(renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let diagnostics = renderer.diagnostics();
        let expected_limits = wgpu::Limits::downlevel_defaults()
            .using_resolution(diagnostics.adapter_limits().clone());

        assert_eq!(
            diagnostics.requested_device_features(),
            wgpu::Features::empty()
        );
        assert_eq!(diagnostics.requested_device_limits(), &expected_limits);
        assert_eq!(diagnostics.device_features(), wgpu::Features::empty());
        assert_eq!(diagnostics.device_limits(), &expected_limits);
        assert!(
            diagnostics
                .adapter_features()
                .contains(diagnostics.requested_device_features())
        );
        assert!(
            diagnostics.adapter_limits().max_texture_dimension_2d
                >= diagnostics
                    .requested_device_limits()
                    .max_texture_dimension_2d
        );
        Ok(())
    }

    #[test]
    fn fill_rect_pipeline_cache_is_target_format_aware() -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        renderer.ensure_fill_rect_pipeline(wgpu::TextureFormat::Rgba8UnormSrgb)?;
        renderer.ensure_fill_rect_pipeline(wgpu::TextureFormat::Bgra8UnormSrgb)?;
        renderer.ensure_fill_rect_pipeline(wgpu::TextureFormat::Bgra8UnormSrgb)?;

        assert_eq!(renderer.fill_rect_pipelines.len(), 2);
        assert!(
            renderer
                .fill_rect_pipelines
                .contains_key(&wgpu::TextureFormat::Rgba8UnormSrgb)
        );
        assert!(
            renderer
                .fill_rect_pipelines
                .contains_key(&wgpu::TextureFormat::Bgra8UnormSrgb)
        );
        assert!(matches!(
            renderer.ensure_fill_rect_pipeline(wgpu::TextureFormat::Rgba16Float),
            Err(OffscreenRenderError::UnsupportedTargetFormat {
                format: wgpu::TextureFormat::Rgba16Float,
            })
        ));
        Ok(())
    }

    #[test]
    fn shared_scene_encoder_renders_bgra_srgb_target_with_matching_pipeline()
    -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let publication = publication(
            vec![PaintContributionItem::fill_rect(
                rect(8.0, 6.0, 20.0, 12.0),
                Color::rgb(0xC3, 0x4A, 0x42),
            )],
            1.0,
        );
        let fill_rects = super::validate_scene_subset(&publication)?;
        let extent = super::publication_extent(&publication)?;
        let layout = ReadbackLayout::new(extent)?;
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        renderer.ensure_fill_rect_pipeline(format)?;
        let (texture, view) = renderer.create_texture_target(extent, format);
        let readback = renderer.create_readback_buffer(layout);
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("runenui BGRA target format proof encoder"),
            });
        super::encode_scene_to_target(
            &renderer.device,
            renderer
                .fill_rect_pipelines
                .get(&format)
                .unwrap_or_else(|| unreachable!("BGRA pipeline is cached")),
            &mut encoder,
            &view,
            extent,
            publication.raster_scale(),
            &fill_rects,
        );
        super::encode_target_copy(&mut encoder, &texture, &readback, extent, layout);
        let submission = renderer.queue.submit([encoder.finish()]);
        let bgra8_srgb = renderer.map_readback(&readback, layout, submission)?;
        let pixel = |x: usize, y: usize| {
            let index = (y * extent.width() as usize + x) * 4;
            &bgra8_srgb[index..index + 4]
        };
        assert_eq!(pixel(2, 2), &[0, 0, 0, 0]);
        assert_eq!(pixel(10, 8), &[0x42, 0x4A, 0xC3, 0xFF]);
        Ok(())
    }

    #[test]
    fn retained_target_lifetime_gates_publication_update_modes() -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let (first, successor, rebuilt) = publication_update_and_scale(
            vec![PaintContributionItem::fill_rect(
                rect(4.0, 4.0, 12.0, 8.0),
                Color::rgb(0x74, 0xB2, 0x5A),
            )],
            vec![PaintContributionItem::fill_rect(
                rect(4.0, 4.0, 12.0, 8.0),
                Color::rgb(0x58, 0x8C, 0xD1),
            )],
            1.0,
            2.0,
        );
        assert_eq!(successor.base_revision(), Some(first.revision()));
        assert_eq!(rebuilt.base_revision(), Some(successor.revision()));

        let first_output = renderer.render_offscreen_publication(&first)?;
        assert_eq!(
            first_output.update_plan().mode(),
            PublicationUpdateMode::FullResync
        );
        let first_generation = first_output.target_generation();

        let retained_current = renderer.render_offscreen_publication(&first)?;
        assert_eq!(
            retained_current.update_plan().mode(),
            PublicationUpdateMode::AlreadyCurrent
        );
        assert_eq!(retained_current.target_generation(), first_generation);

        let exact_successor = renderer.render_offscreen_publication(&successor)?;
        assert_eq!(
            exact_successor.update_plan().mode(),
            PublicationUpdateMode::ExactBaseMatch
        );
        assert_eq!(exact_successor.target_generation(), first_generation);

        assert!(renderer.discard_offscreen_target());
        assert!(!renderer.discard_offscreen_target());
        let fresh_target = renderer.render_offscreen_publication(&successor)?;
        assert_eq!(
            fresh_target.update_plan().mode(),
            PublicationUpdateMode::FullResync
        );
        assert_ne!(fresh_target.target_generation(), first_generation);

        let fresh_generation = fresh_target.target_generation();
        let rebuilt_target = renderer.render_offscreen_publication(&rebuilt)?;
        assert_eq!(
            rebuilt_target.update_plan().mode(),
            PublicationUpdateMode::FullResync,
            "a changed physical extent rebuilds the target and cannot consume exact-base damage"
        );
        assert_ne!(rebuilt_target.target_generation(), fresh_generation);

        let rebuilt_current = renderer.render_offscreen_publication(&rebuilt)?;
        assert_eq!(
            rebuilt_current.update_plan().mode(),
            PublicationUpdateMode::AlreadyCurrent
        );
        assert_eq!(
            rebuilt_current.target_generation(),
            rebuilt_target.target_generation()
        );
        Ok(())
    }

    #[test]
    fn partial_renderer_rejects_every_unsupported_scene_semantic() {
        let stroke = PaintContributionItem::stroke_rect(
            rect(1.0, 1.0, 2.0, 2.0),
            Color::WHITE,
            LogicalLength::from(1_u16),
        );
        let image = PaintContributionItem::image(
            ResourceRef::new(ResourceKind::Image),
            rect(1.0, 1.0, 2.0, 2.0),
        )
        .unwrap_or_else(|_| unreachable!("fixture resource kind matches"));
        let shaped_text = PaintContributionItem::shaped_text_run(
            ResourceRef::new(ResourceKind::ShapedTextRun),
            LogicalPoint::new(1.0, 1.0).unwrap_or_else(|_| unreachable!("fixture point is finite")),
            Color::WHITE,
        )
        .unwrap_or_else(|_| unreachable!("fixture resource kind matches"));
        let translated = PaintContributionItem::fill_rect(rect(1.0, 1.0, 2.0, 2.0), Color::WHITE)
            .with_transform(
                LogicalTransform::translation(1.0, 0.0)
                    .unwrap_or_else(|_| unreachable!("fixture transform is finite")),
            );
        let clipped = PaintContributionItem::fill_rect(rect(1.0, 1.0, 2.0, 2.0), Color::WHITE)
            .with_clip(ContributionClip::new(
                SceneShape::rounded_rect(rect(0.0, 0.0, 3.0, 3.0), Radius::ZERO),
                LogicalTransform::IDENTITY,
            ));
        let translucent_item =
            PaintContributionItem::fill_rect(rect(1.0, 1.0, 2.0, 2.0), Color::WHITE).with_opacity(
                SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("fixture opacity is valid")),
            );
        let translucent_color =
            PaintContributionItem::fill_rect(rect(1.0, 1.0, 2.0, 2.0), Color::rgba(1, 2, 3, 128));

        let cases = [
            (
                stroke,
                SceneValidationError::UnsupportedItem {
                    item_index: 0,
                    semantic: UnsupportedSceneSemantic::StrokeRect,
                },
            ),
            (
                image,
                SceneValidationError::UnsupportedItem {
                    item_index: 0,
                    semantic: UnsupportedSceneSemantic::Image,
                },
            ),
            (
                shaped_text,
                SceneValidationError::UnsupportedItem {
                    item_index: 0,
                    semantic: UnsupportedSceneSemantic::ShapedTextRun,
                },
            ),
            (
                translated,
                SceneValidationError::UnsupportedItem {
                    item_index: 0,
                    semantic: UnsupportedSceneSemantic::NonIdentityTransform,
                },
            ),
            (
                clipped,
                SceneValidationError::UnsupportedItem {
                    item_index: 0,
                    semantic: UnsupportedSceneSemantic::NonEmptyClips,
                },
            ),
            (
                translucent_item,
                SceneValidationError::UnsupportedItem {
                    item_index: 0,
                    semantic: UnsupportedSceneSemantic::NonUnitItemOpacity,
                },
            ),
            (
                translucent_color,
                SceneValidationError::UnsupportedItem {
                    item_index: 0,
                    semantic: UnsupportedSceneSemantic::NonOpaqueFillColor,
                },
            ),
        ];

        for (item, expected) in cases {
            let publication = publication(vec![item], 1.0);
            assert_eq!(
                super::validate_scene_subset(&publication),
                Err(expected),
                "unsupported content must be rejected before it can be partially rendered"
            );
        }
    }

    #[test]
    fn real_gpu_two_fill_rect_order_scale_readback_and_png_proof() -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let color_a = Color::rgb(0xC3, 0x4A, 0x42);
        let color_b = Color::rgb(0x37, 0x86, 0xC8);
        let logical_items = vec![
            PaintContributionItem::fill_rect(rect(8.0, 6.0, 20.0, 12.0), color_a),
            PaintContributionItem::fill_rect(rect(20.0, 12.0, 24.0, 18.0), color_b),
        ];
        let publication_1x = publication(logical_items.clone(), 1.0);
        let publication_2x = publication(logical_items, 2.0);
        let logical_geometry = |publication: &PaintPublication| {
            publication
                .scene()
                .items()
                .iter()
                .map(|item| match item.primitive() {
                    PaintPrimitive::FillRect { rect, .. } => *rect,
                    _ => unreachable!("the proof corpus contains only FillRects"),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            logical_geometry(&publication_1x),
            logical_geometry(&publication_2x),
            "RasterScale must not mutate logical scene geometry"
        );

        let output_1x = renderer.render_offscreen_publication(&publication_1x)?;
        assert_eq!(
            output_1x.update_plan().mode(),
            PublicationUpdateMode::FullResync
        );
        assert_eq!(output_1x.readback().extent(), OffscreenExtent::new(64, 48)?);
        assert_order_probes(output_1x.readback(), 1);

        let output = renderer.render_offscreen_publication(&publication_2x)?;
        assert_eq!(
            output.update_plan().mode(),
            PublicationUpdateMode::FullResync
        );
        assert_ne!(
            output.target_generation(),
            output_1x.target_generation(),
            "the changed physical extent must rebuild the target"
        );
        let readback = output.readback();
        assert_eq!(readback.extent(), OffscreenExtent::new(128, 96)?);
        assert_eq!(readback.format(), wgpu::TextureFormat::Rgba8UnormSrgb);

        assert_order_probes(readback, 2);
        eprintln!(
            "REAL GPU PIXEL PROOF: same logical geometry at scale=1 extent=64x48 and scale=2 extent=128x96; corresponding background/A-only/B-only/overlap probes exact; adapter={:?} backend={}",
            renderer.diagnostics().adapter_info().name,
            renderer.diagnostics().adapter_info().backend,
        );

        prove_png_round_trip(readback)
    }

    #[test]
    fn failed_scene_validation_does_not_advance_retained_target_realization()
    -> Result<(), Box<dyn Error>> {
        let Some(mut renderer) = renderer_or_adapterless()? else {
            return Ok(());
        };
        let accepted = publication(
            vec![PaintContributionItem::fill_rect(
                rect(1.0, 1.0, 4.0, 4.0),
                Color::WHITE,
            )],
            1.0,
        );
        let first = renderer.render_offscreen_publication(&accepted)?;
        assert_eq!(
            first.update_plan().mode(),
            PublicationUpdateMode::FullResync
        );
        let retained_generation = first.target_generation();

        let rejected = publication(
            vec![PaintContributionItem::stroke_rect(
                rect(1.0, 1.0, 4.0, 4.0),
                Color::WHITE,
                LogicalLength::from(1_u16),
            )],
            1.0,
        );
        let Err(error) = renderer.render_offscreen_publication(&rejected) else {
            return Err(std::io::Error::other(
                "unsupported scene rendered instead of failing before target mutation",
            )
            .into());
        };
        assert!(matches!(
            &error,
            OffscreenRenderError::UnsupportedScene {
                item_index: Some(0),
                detail,
            } if detail.contains("StrokeRect")
        ));

        let retained = renderer.render_offscreen_publication(&accepted)?;
        assert_eq!(
            retained.update_plan().mode(),
            PublicationUpdateMode::AlreadyCurrent
        );
        assert_eq!(retained.target_generation(), retained_generation);
        Ok(())
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
