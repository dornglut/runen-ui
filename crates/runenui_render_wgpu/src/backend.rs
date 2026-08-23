use core::{error::Error, fmt};
use std::{sync::Arc, time::Duration};

use runenui_core::Color;

const DEVICE_LABEL: &str = "runenui_render_wgpu device";
const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
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
        detail: Arc<str>,
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
            Self::AdapterUnavailable { requested, detail } => write!(
                formatter,
                "no adapter satisfied renderer backend {requested:?}: {detail}"
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
    device_features: wgpu::Features,
    device_limits: wgpu::Limits,
    offscreen_format: wgpu::TextureFormat,
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

    /// Returns the exact feature set enabled on the renderer-owned device.
    #[must_use]
    pub const fn device_features(&self) -> wgpu::Features {
        self.device_features
    }

    /// Returns the exact limits requested for the renderer-owned device.
    #[must_use]
    pub const fn device_limits(&self) -> &wgpu::Limits {
        &self.device_limits
    }

    /// Returns the controlled offscreen target format.
    #[must_use]
    pub const fn offscreen_format(&self) -> wgpu::TextureFormat {
        self.offscreen_format
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
    // Retained as renderer-owned state for later host-supplied surface targets.
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    diagnostics: RendererDiagnostics,
}

impl Renderer {
    /// Selects a native adapter and creates a renderer-owned wgpu device and queue.
    ///
    /// # Errors
    ///
    /// Returns structured backend, adapter, or device diagnostics when construction fails.
    pub async fn request(options: RendererOptions) -> Result<Self, RendererInitError> {
        let compiled_backends = wgpu::Instance::enabled_backend_features();
        let requested_backends = options.backend_selection().wgpu_backends();
        if !compiled_backends.intersects(requested_backends) {
            return Err(RendererInitError::BackendUnavailable {
                requested: options.backend_selection(),
                compiled: compiled_backends,
            });
        }

        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = requested_backends;
        let instance = wgpu::Instance::new(instance_descriptor);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference().wgpu_preference(),
                force_fallback_adapter: options.force_fallback_adapter(),
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| RendererInitError::AdapterUnavailable {
                requested: options.backend_selection(),
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

        let device_descriptor = wgpu::DeviceDescriptor {
            label: Some(DEVICE_LABEL),
            ..Default::default()
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
            device_features: device.features(),
            device_limits: device.limits(),
            offscreen_format: OFFSCREEN_FORMAT,
        };

        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            diagnostics,
        })
    }

    /// Returns immutable instance, adapter, device, and target diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &RendererDiagnostics {
        &self.diagnostics
    }

    /// Executes one real wgpu render-pass clear and returns actual texture bytes from GPU readback.
    ///
    /// The supplied [`Color`] is unpremultiplied sRGB8. Color channels are
    /// linearized before wgpu clears the sRGB target; alpha remains linear.
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
        let (texture, view) = self.create_offscreen_target(extent);
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
        &self,
        extent: OffscreenExtent,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("runenui offscreen clear target"),
            size: texture_extent(extent),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_readback_buffer(&self, layout: ReadbackLayout) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("runenui offscreen clear readback"),
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
    use core::{error::Error, future::Future, pin::pin, task::Poll};
    use std::{
        sync::Arc,
        task::{Context, Wake, Waker},
        thread,
    };

    use runenui_core::Color;

    use super::{
        BackendSelection, OffscreenExtent, OffscreenRenderError, ReadbackLayout, Renderer,
        RendererInitError, RendererOptions,
    };

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
        let renderer = match block_on(Renderer::request(RendererOptions::new())) {
            Ok(renderer) => renderer,
            Err(RendererInitError::AdapterUnavailable { requested, detail }) => {
                eprintln!(
                    "native wgpu clear unavailable under {requested:?}; structured adapter failure: {detail}"
                );
                assert_eq!(requested, BackendSelection::AllNative);
                assert!(!detail.is_empty());
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        let diagnostics = renderer.diagnostics();
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
        assert!(
            readback
                .rgba8_srgb()
                .chunks_exact(4)
                .all(|pixel| pixel == [0, 255, 0, 255])
        );
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
