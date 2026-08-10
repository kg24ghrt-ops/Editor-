// src/renderer.rs
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, TextureView};
use app_surface::AppSurface;
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

pub struct Renderer {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    // Text rendering state will go here (glyph atlas, shaders, etc.)
}

impl Renderer {
    /// Create a wgpu renderer from an Android Surface JNI object.
    /// `surface_obj` is a JNI reference to android.view.Surface.
    pub async fn new(surface_obj: jni::objects::JObject<'_>, env: &jni::JNIEnv<'_>) -> Result<Self, RendererError> {
        // 1. Wrap the Java Surface into an AppSurface
        let app_surface = AppSurface::from_surface(surface_obj, env)?;
        
        // 2. Get the raw window handle (implements HasRawWindowHandle)
        let raw_handle = app_surface.raw_window_handle();
        
        // 3. Create wgpu instance and surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        
        let surface = unsafe { instance.create_surface(raw_handle)? };
        
        // 4. Pick adapter and device
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
        
        // 5. Configure the surface
        let size = app_surface.size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        
        Ok(Self { surface, device, queue, config })
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
    
    pub fn render(&mut self) -> Result<(), RendererError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // ... rendering commands go here (clear, draw text, draw cursor, etc.)
        
        self.queue.submit(None);
        frame.present();
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("AppSurface error: {0}")]
    AppSurface(#[from] app_surface::Error),
    #[error("wgpu error: {0}")]
    Wgpu(#[from] wgpu::Error),
    #[error("No suitable GPU adapter found")]
    NoAdapter,
}