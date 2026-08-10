use wgpu::{
    Device, Queue, Surface, SurfaceConfiguration, TextureUsages,
    CommandEncoderDescriptor, LoadOp, Operations, RenderPassDescriptor,
    Color, TextureViewDescriptor,
};
use app_surface::AppSurface;
use raw_window_handle::HasWindowHandle;
use jni::objects::JObject;
use jni::Env;

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("AppSurface error: {0}")]
    AppSurface(#[from] anyhow::Error),
    #[error("wgpu surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),
    #[error("wgpu creation error: {0}")]
    Wgpu(#[from] wgpu::Error),
    #[error("No suitable adapter")]
    NoAdapter,
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
    pub async fn new(surface_obj: JObject<'_>, env: &Env<'_>) -> Result<Self, RendererError> {
        let app_surface = AppSurface::from_surface(surface_obj, env)
            .map_err(|e| RendererError::AppSurface(anyhow::anyhow!(e)))?;

        let window_handle = app_surface
            .window_handle()
            .map_err(|e| RendererError::AppSurface(anyhow::anyhow!(e)))?;
        let size = app_surface.size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window_handle)? };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or(RendererError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        let config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            width: size.width,
            height: size.height,
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
        // In wgpu v30, get_current_texture() returns CurrentSurfaceTexture enum
        // which can be a Success or an Error variant.
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Error(err) => {
                return Err(RendererError::Surface(err));
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