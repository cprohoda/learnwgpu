use std::ops::Range;

use wgpu::{util::DeviceExt, BufferSlice};
use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent}, keyboard::{KeyCode, PhysicalKey}, window::Window
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
    shape_state: ShapeState,
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
                buffers: &[Vertex::desc()],
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
        let shape_state = ShapeState::new(&device);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            clear,
            render_state,
            shape_state,
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
                println!("{:?}", event);
                self.alter_clear();
            },
            WindowEvent::KeyboardInput { event: KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::Space),
                state: ElementState::Pressed,
                repeat: false,
                ..
            }, ..} => {
                self.render_state.state = self.render_state.next();
            },
            WindowEvent::KeyboardInput { event: KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyZ),
                state: ElementState::Pressed,
                repeat: false,
                ..
            }, ..} => {
                self.shape_state.swap();
            },
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

            render_pass.set_pipeline(self.render_state.pipeline());
            render_pass.set_vertex_buffer(0, self.shape_state.vertex_buffer_slice());
            render_pass.set_index_buffer(self.shape_state.index_buffer_slice(), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(self.shape_state.index_buffer_indices(), self.shape_state.base_vertex(), 0..1);
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

    fn pipeline(&self) -> &wgpu::RenderPipeline {
        match self.state {
            RenderState::Standard => {
                &self.standard
            },
            RenderState::PositionColor => {
                &self.position_color
            },
        }
    }

    fn next(&self) -> RenderState {
        match self.state {
            RenderState::Standard => RenderState::PositionColor,
            RenderState::PositionColor => RenderState::Standard,
        }
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout::<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] },
    Vertex { position: [0.0, 0.0, 5.0], color: [0.0, 0.0, 1.0] },
    Vertex { position: [-1.5, -1.5, 2.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [1.5, -1.5, 2.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.0, 2.0, 2.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.0, 0.0, 2.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [-0.7, -0.7, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.7, 0.7, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [0.0, 1.0, 0.0], color: [1.0, 0.0, 0.0] },
];

const INDICES: &[u16] = &[
    // Pentagon, 3 triangles
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
    // arrow, 8 triangles
    5, 6, 7,
    7, 8, 5,
    5, 8, 6,
    6, 7, 8,
    9, 10, 11,
    11, 12, 9,
    12, 9, 10,
    10, 11, 12,
];

#[derive(Debug)]
enum Shapes {
    Pentagon,
    Arrow,
}

struct ShapeState {
    state: Shapes,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl ShapeState {
    fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Pentagon Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Pentagon Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            state: Shapes::Pentagon,
            vertex_buffer,
            index_buffer,
        }
    }

    fn vertex_buffer_slice(&self) -> BufferSlice {
        self.vertex_buffer.slice(..)
    }

    fn index_buffer_indices(&self) -> Range<u32> {
        match self.state {
            Shapes::Pentagon => {
                0..9
            },
            Shapes::Arrow => {
                9..(9 + 24)
            },
        }
    }

    fn index_buffer_slice(&self) -> BufferSlice {
        self.index_buffer.slice(..)
    }

    fn base_vertex(&self) -> i32 {
        0
    }

    fn swap(&mut self) {
        match self.state {
            Shapes::Pentagon => {
                self.state = Shapes::Arrow;
            },
            Shapes::Arrow => {
                self.state = Shapes::Pentagon;
            }
        }
    }
}
