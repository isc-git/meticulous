use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::{application::ApplicationHandler, window::Window};

static APP_NAME: &str = "Meticulous";

#[derive(Debug)]
struct Application {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    gpu_instance: wgpu::Instance,
    gpu_adapter: wgpu::Adapter,
    gpu_device: Arc<wgpu::Device>,
    gpu_queue: wgpu::Queue,
}

impl Application {
    async fn default() -> Self {
        let gpu_instance = wgpu::Instance::default();
        let gpu_adapter = gpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("found no valid adaptor");

        let (gpu_device, gpu_queue) = gpu_adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default().using_resolution(gpu_adapter.limits()),
                },
                None,
            )
            .await
            .expect("found no valid device");

        Application {
            window: None,
            surface: None,
            surface_config: None,
            render_pipeline: None,
            gpu_instance,
            gpu_adapter,
            gpu_device: Arc::new(gpu_device),
            gpu_queue,
        }
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes().with_title(APP_NAME);

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            attributes = attributes.with_append(true);
        }

        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        tracing::info!("created window");

        let mut size = window.inner_size();
        tracing::info!("size: {size:?}");
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        tracing::info!("size: {size:?}");

        #[cfg(target_arch = "wasm32")]
        {
            use winit::dpi::{LogicalSize, PhysicalSize};
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().unwrap();
            let (width, height) = (canvas.client_width(), canvas.client_height());
            tracing::debug!("canvas width: {width:?}, height: {height:?}");

            let factor = window.scale_factor();
            tracing::debug!("window scale factor: {factor:?}");
            let logical = LogicalSize { width, height };
            tracing::debug!("logical size: {logical:?}");
            let PhysicalSize { width, height }: PhysicalSize<u32> = logical.to_physical(factor);

            canvas.set_width(width as u32);
            canvas.set_height(height as u32);
            size.width = width;
            size.height = height;
        }

        tracing::debug!("size: {size:?}");

        let surface = self.gpu_instance.create_surface(window.clone()).unwrap();
        tracing::debug!("created surface");

        let config = surface
            .get_default_config(&self.gpu_adapter, size.width, size.height)
            .expect("no default config");
        surface.configure(&self.gpu_device, &config);

        let shader = self
            .gpu_device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                    "../shader.wgsl"
                ))),
            });

        let pipeline_layout =
            self.gpu_device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let swapchain_capabilities = surface.get_capabilities(&self.gpu_adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline =
            self.gpu_device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        compilation_options: Default::default(),
                        targets: &[Some(swapchain_format.into())],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });

        self.window = Some(window);
        self.surface = Some(surface);
        self.surface_config = Some(config);
        self.render_pipeline = Some(render_pipeline);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                // avoid warning when window is dropped in `windowWillClose`
                let _ = self.window.take();
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(new_size) => {
                tracing::info!("resizing too: {new_size:?}");

                let conf = self.surface_config.as_mut().expect("valid config");
                conf.width = new_size.width.max(1);
                conf.height = new_size.height.max(1);
                self.surface
                    .as_ref()
                    .expect("valid config")
                    .configure(&self.gpu_device, conf);

                //#[cfg(target_arch = "wasm32")]
                //{
                //    use winit::dpi::{LogicalSize, PhysicalSize};
                //    use winit::platform::web::WindowExtWebSys;
                //    let canvas = self.window.as_ref().unwrap().canvas().unwrap();
                //    let (width, height) = (canvas.client_width(), canvas.client_height());

                //    let factor = self.window.as_ref().unwrap().scale_factor();
                //    let logical = LogicalSize { width, height };
                //    let PhysicalSize { width, height }: PhysicalSize<u32> =
                //        logical.to_physical(factor);

                //    canvas.set_width(width as u32);
                //    canvas.set_height(height as u32);
                //}

                if let Some(win) = self.window.as_ref() {
                    win.request_redraw();
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                // prevents a double borrow error?
                let _ = (&self.gpu_instance, &self.gpu_adapter);

                let Some(window) = self.window.as_ref() else {
                    tracing::warn!("redraw requested on closed window");
                    event_loop.exit();
                    return;
                };

                let Some(surface) = self.surface.as_ref() else {
                    tracing::warn!("redraw requested on no surface");
                    event_loop.exit();
                    return;
                };

                let Some(render_pipeline) = self.render_pipeline.as_ref() else {
                    tracing::warn!("redraw requested with no pipeline");
                    event_loop.exit();
                    return;
                };

                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    self.gpu_device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            // `None` value causes "already borrowed: BorrowMutError"
                            label: Some("render triangle"),
                        });

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rpass.set_pipeline(render_pipeline);
                    rpass.draw(0..3, 0..1);
                }

                let commands = encoder.finish();
                self.gpu_queue.submit([commands]);
                frame.present();
                //window.request_redraw()
            }
            _ => (),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let event_loop = EventLoop::new().expect("failed to create event loop");
    // respond only to user input to reduce CPU time
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = Application::default().await;
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    console_error_panic_hook::set_once();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .without_time()
                .with_writer(tracing_web::MakeWebConsoleWriter::new()),
        )
        .init();

    tracing::info!("hello!!");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    // respond only to user input to reduce CPU time
    event_loop.set_control_flow(ControlFlow::Poll);

    wasm_bindgen_futures::spawn_local(async {
        let mut app = Application::default().await;
        tracing::debug!("starting app");
        event_loop.run_app(&mut app).unwrap()
    });
}
