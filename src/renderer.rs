use wgpu::{
    Device, Queue, Surface, SurfaceConfiguration, TextureUsages,
    CommandEncoderDescriptor, LoadOp, Operations, RenderPassDescriptor,
    Color, TextureViewDescriptor, InstanceDescriptor, Backends, InstanceFlags,
    MemoryBudgetThresholds, BackendOptions, SurfaceColorSpace,
};
use app_surface::AppSurface;
use raw_window_handle::HasWindowHandle;
use jni::objects::JObject;
use jni::Env;

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("AppSurface error: {0}")]
    AppSurface(#[from] anyhow::Error),
    #[error("No suitable adapter")]
    NoAdapter,
    #[error("Surface lost or outdated")]
    SurfaceLost,
    #[error("Surface timeout")]
    SurfaceTimeout,
    #[error("Surface occluded")]
    SurfaceOccluded,
    #[error("wgpu request device error: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("wgpu surface error: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
}

pub struct Renderer {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    width: u32,
    height: u32,
}

impl Renderer {
    pub async fn new(surface_obj: JObject<'_>, env: &Env<'_>, width: u32, height: u32) -> Result<Self, RendererError> {
        let raw_env = env.get_raw();
        let raw_surface = surface_obj.as_raw();
        
        let app_surface = unsafe { 
            AppSurface::new(raw_env as *mut jni::sys::JNIEnv, raw_surface) 
        };

        let window_handle = app_surface
            .native_window
            .window_handle()
            .map_err(|e| RendererError::AppSurface(anyhow::anyhow!(e)))?;

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            flags: InstanceFlags::empty(),
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
            backend_options: BackendOptions::default(),
            display: None,
        });

        let surface = unsafe { instance.create_surface(&window_handle)? };

        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
        {
            Some(adapter) => adapter,
            None => return Err(RendererError::NoAdapter),
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        let config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            color_space: SurfaceColorSpace::Srgb,
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            width,
            height,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self) -> Result<(), RendererError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                return Err(RendererError::SurfaceTimeout);
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                return Err(RendererError::SurfaceOccluded);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let _ = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.12,
                            g: 0.12,
                            b: 0.14,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}
