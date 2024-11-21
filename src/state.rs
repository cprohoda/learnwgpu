use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent}, keyboard::{Key, KeyCode, PhysicalKey}, window::Window
};

pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    // Window need to be declared after surface
    // because it contains unsafe reference, so
    // it has to get created and dropped afterwards
    // (according to tutorial -- TODO double check)
    window: &'a Window,
    clear: wgpu::Color,
    render_state: RenderPipelineState,
}

impl<'a> State<'a> {
    pub async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch="wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch="wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch="wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
                memory_hints: Default::default(),
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // assumme sRGB surface texture from here out
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let clear = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        };
        let standard_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Standard Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("standard_shader.wgsl").into()),
        });
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let standard_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Standard Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &standard_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &standard_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        let position_color_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Position Color Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("position_color_shader.wgsl").into()),
        });
        let position_color_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Position Color Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &position_color_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &position_color_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        let render_state = RenderPipelineState::new(standard_pipeline, position_color_pipeline);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            clear,
            render_state,
        }
    }

    pub fn alter_clear(&mut self) {
        self.clear.r += 0.15;
        self.clear.b += 0.2;
        self.clear.g -= -0.1;
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { device_id: _, state: ElementState::Pressed, button: MouseButton::Left } => {
                self.alter_clear();
            },
            WindowEvent::KeyboardInput { device_id: _, event: KeyEvent {physical_key: PhysicalKey::Code(KeyCode::Space), state: ElementState::Pressed, repeat: false, ..}, is_synthetic: _} => {
                self.render_state.state = self.render_state.next();
                println!("{:?}", event); // TODO: Fix double eventing for keyboard spacebar input
                println!("Space pressed. New state: {:?}", self.render_state.state);
            }
            _ => {},
        };

        false
    }

    pub fn update(&mut self) {}

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(match self.render_state.state {
                RenderState::Standard => {
                    &self.render_state.standard
                },
                RenderState::PositionColor => {
                    &self.render_state.position_color
                },
            });
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Debug)]
enum RenderState {
    Standard,
    PositionColor,
}

struct RenderPipelineState {
    state: RenderState,
    standard: wgpu::RenderPipeline,
    position_color: wgpu::RenderPipeline,
}

impl RenderPipelineState {
    fn new(standard: wgpu::RenderPipeline, position_color: wgpu::RenderPipeline) -> Self {
        Self {
            state: RenderState::Standard,
            standard,
            position_color,
        }
    }

    fn next(&self) -> RenderState {
        match self.state {
            RenderState::Standard => RenderState::PositionColor,
            RenderState::PositionColor => RenderState::Standard,
        }
    }
}
