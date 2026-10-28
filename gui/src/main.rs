//! `nocodo-gui`: a window over the nocodo API. It starts a local `nocodo-api` unless told where
//! a running one is with `--server`, and only ever talks to it over HTTP.

mod client;
mod grid;
mod model;
mod server;
mod settings;
mod view;
mod words;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use akar_core::AkarCore;
use akar_winit::{process_window_event, set_presents_with_transaction};
use clap::Parser;
use wgpu::{
    CompositeAlphaMode, CurrentSurfaceTexture, InstanceDescriptor, PresentMode, TextureUsages,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

use client::ApiClient;
use server::ServerProcess;
use view::View;

#[derive(Parser)]
#[command(name = "nocodo-gui", version)]
struct Args {
    /// Use a server that is already running (e.g. http://127.0.0.1:8787) instead of starting one.
    #[arg(long, env = "NOCODO_SERVER")]
    server: Option<String>,
    /// Folder whose subfolders are the projects, for the server this GUI starts.
    #[arg(long, default_value = ".projects", env = "NOCODO_PROJECTS_DIR")]
    projects_dir: PathBuf,
}

struct Gfx {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    core: AkarCore,
}

struct App {
    gfx: Option<Gfx>,
    view: View,
    started: Instant,
    // Dropped with the app, which stops a server this GUI started.
    _server: Option<ServerProcess>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("nocodo")
            .with_inner_size(LogicalSize::new(760.0, 860.0))
            .with_min_inner_size(LogicalSize::new(420.0, 520.0));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("creating the window"),
        );

        let instance = wgpu::Instance::new(InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        )));
        let surface = instance
            .create_surface(window.clone())
            .expect("creating the drawing surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("finding a graphics adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("opening the graphics device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("configuring the drawing surface");
        config.usage = TextureUsages::RENDER_ATTACHMENT;
        config.present_mode = PresentMode::Fifo;
        config.alpha_mode = CompositeAlphaMode::Opaque;
        surface.configure(&device, &config);

        let core = AkarCore::new(
            &device,
            &queue,
            config.format,
            akar_core::TextPipelineConfig::default(),
        );
        self.gfx = Some(Gfx {
            window,
            device,
            queue,
            surface,
            config,
            core,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.gfx.is_none() {
            return;
        }

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            // The drawing surface takes the new size at once, so draw a frame for it now instead
            // of waiting for a later redraw: otherwise the compositor scales the last frame to
            // the new bounds and text appears to stretch and shake while the window is dragged.
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                let (width, height) = (s.width, s.height);
                if let Some(g) = &mut self.gfx {
                    g.config.width = width;
                    g.config.height = height;
                    g.surface.configure(&g.device, &g.config);
                    // On macOS this puts the frame in the resize's Core Animation transaction.
                    set_presents_with_transaction(&g.surface, true);
                }
                self.redraw();
                if let Some(g) = &self.gfx {
                    set_presents_with_transaction(&g.surface, false);
                }
            }
            // akar lays out in logical pixels, so pointer input must be logical too.
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(g) = &mut self.gfx {
                    let p: PhysicalPosition<f64> = *position;
                    let scale = g.window.scale_factor();
                    g.core
                        .input
                        .set_mouse_pos((p.x / scale) as f32, (p.y / scale) as f32);
                    g.window.request_redraw();
                }
                return;
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                if let Some(g) = &self.gfx {
                    g.window.request_redraw();
                }
                return;
            }
            _ => {}
        }

        if let Some(g) = &mut self.gfx {
            process_window_event(&mut g.core.input, &event);
            g.window.request_redraw();
        }
    }
}

impl App {
    /// Draws and presents one frame. A frame is drawn even when no redraw was requested, so a
    /// resize can present a correctly sized frame in the same transaction as the new bounds.
    fn redraw(&mut self) {
        let Some(g) = &mut self.gfx else { return };
        let size = g.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        // Resize events can lag behind the window's current bounds.
        if g.config.width != size.width || g.config.height != size.height {
            g.config.width = size.width;
            g.config.height = size.height;
            g.surface.configure(&g.device, &g.config);
        }

        let output = match g.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                g.surface.configure(&g.device, &g.config);
                g.window.request_redraw();
                return;
            }
            _ => {
                g.window.request_redraw();
                return;
            }
        };
        // Draw at the size of the texture we present, not the one the window last reported.
        let size = output.texture.size();
        let scale = g.window.scale_factor() as f32;
        let logical = [size.width as f32 / scale, size.height as f32 / scale];
        let cursor_visible = (self.started.elapsed().as_millis() / 530).is_multiple_of(2);

        g.core.begin_frame(size.width, size.height, scale);
        self.view.render(&mut g.core, logical, cursor_visible);

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = g
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let _ = g.core.end_frame(&g.device, &g.queue, &mut pass);
        }
        g.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        // One-frame events (clicks, typed characters) are consumed by this frame.
        g.core.input.begin_frame();
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (server, base_url) = match args.server {
        Some(url) => (None, url.trim_end_matches('/').to_string()),
        None => {
            let s = ServerProcess::spawn(&args.projects_dir)?;
            let url = s.base_url.clone();
            (Some(s), url)
        }
    };
    eprintln!("nocodo-gui: using the server at {base_url}");
    let view = View::new(ApiClient::new(base_url));

    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App {
        gfx: None,
        view,
        started: Instant::now(),
        _server: server,
    })?;
    Ok(())
}
