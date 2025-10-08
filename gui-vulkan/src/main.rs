use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use shared::grid::{CellState, Grid};
use wgpu::util::DeviceExt;
use wgpu::StoreOp;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

const GRID_WIDTH: usize = 200;
const GRID_HEIGHT: usize = GRID_WIDTH * 9 / 16;
const STEP_INTERVAL: Duration = Duration::from_millis(0);
const UI_HEIGHT: f32 = 90.0;
const BUTTON_WIDTH: f32 = 180.0;
const BUTTON_HEIGHT: f32 = 44.0;
const BUTTON_PADDING: f32 = 24.0;
const BUTTON_VERTICAL_OFFSET: f32 = 12.0;
const TEXT_SCALE_HEADING: f32 = 10.0;
const TEXT_SCALE_BUTTON: f32 = 8.0;
const GRID_BASE_VERTEX_COUNT: u32 = 6;
const FONT_WIDTH: usize = 5;
const FONT_HEIGHT: usize = 7;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CellInstance {
    min: [f32; 2],
    max: [f32; 2],
    color: [f32; 3],
    _pad: f32,
}

#[derive(Copy, Clone)]
struct Rect {
    min: [f32; 2],
    max: [f32; 2],
}

impl Rect {
    fn contains(&self, point: [f32; 2]) -> bool {
        point[0] >= self.min[0] && point[0] <= self.max[0] && point[1] >= self.min[1] && point[1] <= self.max[1]
    }
}

struct State {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    grid_pipeline: wgpu::RenderPipeline,
    ui_pipeline: wgpu::RenderPipeline,
    grid_vertex_buffer: wgpu::Buffer,
    grid_instance_buffer: wgpu::Buffer,
    grid_instance_capacity: usize,
    ui_vertex_buffer: wgpu::Buffer,
    ui_vertex_capacity: usize,
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: wgpu::InstanceFlags::from_env_or_default(),
            backend_options: wgpu::BackendOptions::default(),
        });

        let surface = instance.create_surface(window.clone()).context("create surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("request adapter")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::default(),
            })
            .await
            .context("request device")?;

        let capabilities = surface.get_capabilities(&adapter);
        let surface_format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| matches!(mode, wgpu::PresentMode::Mailbox))
            .or_else(|| {
                capabilities
                    .present_modes
                    .iter()
                    .copied()
                    .find(|mode| matches!(mode, wgpu::PresentMode::Immediate))
            })
            .unwrap_or(wgpu::PresentMode::Fifo);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: capabilities.alpha_modes[0],
            desired_maximum_frame_latency: 1,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let ui_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let grid_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid_vertex_buffer"),
            contents: bytemuck::cast_slice(&[[0.0_f32, 0.0], [1.0, 0.0], [0.0, 1.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let grid_instance_capacity = GRID_WIDTH * GRID_HEIGHT;
        let grid_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid_instance_buffer"),
            size: (grid_instance_capacity * std::mem::size_of::<CellInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ui_vertex_capacity = 4096;
        let ui_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_vertex_buffer"),
            size: (ui_vertex_capacity * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid_pipeline"),
            layout: Some(&grid_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_grid"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<CellInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 8,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 3,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui_pipeline"),
            layout: Some(&ui_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_ui"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            instance,
            surface,
            device,
            queue,
            config,
            size,
            grid_pipeline,
            ui_pipeline,
            grid_vertex_buffer,
            grid_instance_buffer,
            grid_instance_capacity,
            ui_vertex_buffer,
            ui_vertex_capacity,
        })
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn ensure_grid_instance_capacity(&mut self, required_instances: usize) {
        if required_instances <= self.grid_instance_capacity {
            return;
        }
        self.grid_instance_capacity = required_instances.next_power_of_two();
        self.grid_instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid_instance_buffer"),
            size: (self.grid_instance_capacity * std::mem::size_of::<CellInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    fn ensure_ui_vertex_capacity(&mut self, required_vertices: usize) {
        if required_vertices <= self.ui_vertex_capacity {
            return;
        }
        self.ui_vertex_capacity = required_vertices.next_power_of_two();
        self.ui_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_vertex_buffer"),
            size: (self.ui_vertex_capacity * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    fn render(&mut self, instances: &[CellInstance], ui_vertices: &[Vertex]) -> std::result::Result<(), wgpu::SurfaceError> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                match err {
                    wgpu::SurfaceError::Lost => {
                        self.surface.configure(&self.device, &self.config);
                    }
                    wgpu::SurfaceError::OutOfMemory => return Err(err),
                    _ => {}
                }
                self.surface.get_current_texture()?
            }
        };

        if !instances.is_empty() {
            self.ensure_grid_instance_capacity(instances.len());
            let bytes = bytemuck::cast_slice(instances);
            self.queue.write_buffer(&self.grid_instance_buffer, 0, bytes);
        }

        if !ui_vertices.is_empty() {
            self.ensure_ui_vertex_capacity(ui_vertices.len());
            let bytes = bytemuck::cast_slice(ui_vertices);
            self.queue.write_buffer(&self.ui_vertex_buffer, 0, bytes);
        }

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("encoder") });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.07,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if !instances.is_empty() {
                render_pass.set_pipeline(&self.grid_pipeline);
                render_pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
                let instance_bytes = std::mem::size_of_val(instances) as u64;
                render_pass.set_vertex_buffer(1, self.grid_instance_buffer.slice(0..instance_bytes));
                render_pass.draw(0..GRID_BASE_VERTEX_COUNT, 0..instances.len() as u32);
            }

            if !ui_vertices.is_empty() {
                render_pass.set_pipeline(&self.ui_pipeline);
                let vertex_bytes = std::mem::size_of_val(ui_vertices) as u64;
                render_pass.set_vertex_buffer(0, self.ui_vertex_buffer.slice(0..vertex_bytes));
                render_pass.draw(0..ui_vertices.len() as u32, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

struct GameOfLifeApp {
    grid: Grid,
    last_step: Instant,
    window_size: PhysicalSize<u32>,
    cursor_position: Option<[f32; 2]>,
    instances: Vec<CellInstance>,
    ui_vertices: Vec<Vertex>,
}

impl GameOfLifeApp {
    fn new(window_size: PhysicalSize<u32>) -> Self {
        Self {
            grid: Grid::new(GRID_WIDTH, GRID_HEIGHT),
            last_step: Instant::now(),
            window_size,
            cursor_position: None,
            instances: Vec::with_capacity(GRID_WIDTH * GRID_HEIGHT),
            ui_vertices: Vec::with_capacity(2048),
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.window_size = size;
    }

    fn update(&mut self) {
        if self.last_step.elapsed() >= STEP_INTERVAL {
            self.grid.advance();
            self.last_step = Instant::now();
        }
    }

    fn randomize(&mut self) {
        self.grid.randomize();
        self.last_step = Instant::now();
    }

    fn handle_click(&mut self, position: [f32; 2]) {
        if self.button_rect().contains(position) {
            self.randomize();
        }
    }

    fn button_rect(&self) -> Rect {
        let width = self.window_size.width.max(1) as f32;
        Rect {
            min: [width - BUTTON_PADDING - BUTTON_WIDTH, BUTTON_PADDING + BUTTON_VERTICAL_OFFSET],
            max: [width - BUTTON_PADDING, BUTTON_PADDING + BUTTON_VERTICAL_OFFSET + BUTTON_HEIGHT],
        }
    }

    fn build_frame(&mut self) -> (&[CellInstance], &[Vertex]) {
        self.instances.clear();
        self.ui_vertices.clear();

        let width = self.window_size.width.max(1) as f32;
        let height = self.window_size.height.max(1) as f32;

        let usable_height = (height - UI_HEIGHT).max(1.0);
        let cell_size = ((width / GRID_WIDTH as f32).min(usable_height / GRID_HEIGHT as f32)).max(1.0);
        let grid_pixel_width = cell_size * GRID_WIDTH as f32;
        let grid_pixel_height = cell_size * GRID_HEIGHT as f32;
        let grid_offset_x = (width - grid_pixel_width) * 0.5;
        let grid_offset_y = UI_HEIGHT + (usable_height - grid_pixel_height) * 0.5;

        for (row_index, row) in self.grid.cells.iter().enumerate() {
            for (col_index, cell) in row.iter().enumerate() {
                let x = grid_offset_x + col_index as f32 * cell_size;
                let y = grid_offset_y + row_index as f32 * cell_size;
                let min = [to_ndc(x, width), to_ndc_y(y, height)];
                let max = [to_ndc(x + cell_size, width), to_ndc_y(y + cell_size, height)];
                let color = match cell {
                    CellState::Alive => [0.95, 0.95, 0.95],
                    CellState::Dead => [0.18, 0.18, 0.22],
                };
                self.instances.push(CellInstance { min, max, color, _pad: 0.0 });
            }
        }

        let header_line = Rect {
            min: [0.0, UI_HEIGHT - 4.0],
            max: [width, UI_HEIGHT],
        };
        push_rect(&mut self.ui_vertices, header_line, [0.15, 0.15, 0.2], [width, height]);

        let button_rect = self.button_rect();
        let hovered = self.cursor_position.map(|pos| button_rect.contains(pos)).unwrap_or(false);
        let button_color = if hovered { [0.35, 0.45, 0.75] } else { [0.25, 0.33, 0.55] };
        push_rect(&mut self.ui_vertices, button_rect, button_color, [width, height]);

        draw_text(
            &mut self.ui_vertices,
            "Game of Life",
            [BUTTON_PADDING, BUTTON_PADDING],
            TEXT_SCALE_HEADING,
            [0.9, 0.9, 0.95],
            [width, height],
        );

        let button_text = "Randomize";
        let text_width = text_pixel_width(button_text) * TEXT_SCALE_BUTTON;
        let text_height = FONT_HEIGHT as f32 * TEXT_SCALE_BUTTON;
        let origin_x = button_rect.min[0] + (button_rect.max[0] - button_rect.min[0] - text_width) * 0.5;
        let origin_y = button_rect.min[1] + (button_rect.max[1] - button_rect.min[1] - text_height) * 0.5;
        draw_text(
            &mut self.ui_vertices,
            button_text,
            [origin_x, origin_y],
            TEXT_SCALE_BUTTON,
            [0.95, 0.95, 0.98],
            [width, height],
        );

        (&self.instances, &self.ui_vertices)
    }
}

fn push_rect(vertices: &mut Vec<Vertex>, rect: Rect, color: [f32; 3], window_size: [f32; 2]) {
    let [width, height] = window_size;
    let x0 = to_ndc(rect.min[0], width);
    let y0 = to_ndc_y(rect.min[1], height);
    let x1 = to_ndc(rect.max[0], width);
    let y1 = to_ndc_y(rect.max[1], height);

    vertices.push(Vertex { position: [x0, y1], color });
    vertices.push(Vertex { position: [x1, y1], color });
    vertices.push(Vertex { position: [x0, y0], color });
    vertices.push(Vertex { position: [x0, y0], color });
    vertices.push(Vertex { position: [x1, y1], color });
    vertices.push(Vertex { position: [x1, y0], color });
}

fn to_ndc(x: f32, width: f32) -> f32 {
    (x / width) * 2.0 - 1.0
}

fn to_ndc_y(y: f32, height: f32) -> f32 {
    1.0 - (y / height) * 2.0
}

fn text_pixel_width(text: &str) -> f32 {
    let mut units = 0.0;
    for ch in text.chars() {
        if ch == ' ' || glyph_bits(ch).is_some() {
            units += (FONT_WIDTH as f32) + 1.0;
        }
    }
    (units - 1.0).max(0.0)
}

fn draw_text(vertices: &mut Vec<Vertex>, text: &str, origin: [f32; 2], scale: f32, color: [f32; 3], window_size: [f32; 2]) {
    let mut cursor_x = origin[0];
    for ch in text.to_uppercase().chars() {
        if ch == ' ' {
            cursor_x += (FONT_WIDTH as f32 + 1.0) * scale;
            continue;
        }
        if let Some(rows) = glyph_bits(ch) {
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..FONT_WIDTH {
                    if (bits >> (FONT_WIDTH - 1 - col)) & 1 == 1 {
                        let rect = Rect {
                            min: [cursor_x + col as f32 * scale, origin[1] + row as f32 * scale],
                            max: [cursor_x + (col as f32 + 1.0) * scale, origin[1] + (row as f32 + 1.0) * scale],
                        };
                        push_rect(vertices, rect, color, window_size);
                    }
                }
            }
        }
        cursor_x += (FONT_WIDTH as f32 + 1.0) * scale;
    }
}

fn glyph_bits(ch: char) -> Option<[u8; FONT_HEIGHT]> {
    match ch {
        'A' => Some([0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'D' => Some([0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
        'E' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        'F' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        'G' => Some([0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111]),
        'I' => Some([0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111]),
        'L' => Some([0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        'M' => Some([0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
        'N' => Some([0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        'O' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'R' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        'Z' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
        _ => None,
    }
}

fn key_matches(event: &KeyEvent, target: &str) -> bool {
    match &event.logical_key {
        Key::Named(NamedKey::Space) => target.eq_ignore_ascii_case("SPACE"),
        Key::Character(text) => text.eq_ignore_ascii_case(target),
        _ => false,
    }
}

struct VulkanApp {
    window_attrs: WindowAttributes,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    state: Option<State>,
    app: Option<GameOfLifeApp>,
    last_cursor: [f32; 2],
    frame_count: u32,
    last_fps_log: Instant,
}

impl VulkanApp {
    fn new() -> Self {
        let attrs = Window::default_attributes()
            .with_title("Game of Life - Vulkan")
            .with_inner_size(PhysicalSize::new(1280, 720));
        Self {
            window_attrs: attrs,
            window: None,
            window_id: None,
            state: None,
            app: None,
            last_cursor: [0.0, 0.0],
            frame_count: 0,
            last_fps_log: Instant::now(),
        }
    }
}

impl ApplicationHandler<()> for VulkanApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = event_loop.create_window(self.window_attrs.clone()).expect("failed to create window");
        let window = Arc::new(window);
        let window_id = window.id();

        let state = pollster::block_on(State::new(window.clone())).expect("failed to create GPU state");
        let app = GameOfLifeApp::new(state.size);
        window.request_redraw();

        self.window = Some(window);
        self.window_id = Some(window_id);
        self.state = Some(state);
        self.app = Some(app);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        if Some(window_id) != self.window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(state) = self.state.as_mut() {
                    state.resize(size);
                }
                if let Some(app) = self.app.as_mut() {
                    app.resize(size);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { mut inner_size_writer, .. } => {
                if let Some(state) = self.state.as_ref() {
                    let size = PhysicalSize::new(state.config.width, state.config.height);
                    let _ = inner_size_writer.request_inner_size(size);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor = [position.x as f32, position.y as f32];
                if let Some(app) = self.app.as_mut() {
                    app.cursor_position = Some(self.last_cursor);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left && state == ElementState::Released {
                    if let Some(app) = self.app.as_mut() {
                        app.handle_click(self.last_cursor);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let Some(app) = self.app.as_mut() {
                        if key_matches(&event, "R") || key_matches(&event, "SPACE") {
                            app.randomize();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(state), Some(app)) = (self.state.as_mut(), self.app.as_mut()) {
                    app.update();
                    let (instances, ui_vertices) = app.build_frame();
                    if let Err(err) = state.render(instances, ui_vertices) {
                        match err {
                            wgpu::SurfaceError::Lost => state.resize(state.size),
                            wgpu::SurfaceError::OutOfMemory => event_loop.exit(),
                            _ => {}
                        }
                    } else {
                        self.frame_count += 1;
                        let elapsed = self.last_fps_log.elapsed();
                        if elapsed >= Duration::from_secs(1) {
                            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
                            log::info!("fps: {:.1}", fps);
                            self.frame_count = 0;
                            self.last_fps_log = Instant::now();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    let mut app = VulkanApp::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}
