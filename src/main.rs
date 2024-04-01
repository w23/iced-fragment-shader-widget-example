use glam::Vec2;
use iced::advanced::Shell;
use iced::event::Status;
use iced::mouse;
use iced::mouse::Cursor;
use iced::widget::shader::wgpu;
use iced::widget::shader::Event;
use iced::widget::{column, row, shader, slider, text};
use iced::{Alignment, Element, Length, Rectangle, Sandbox, Settings, Size};

const ZOOM_MIN: f32 = 1.0;
const ZOOM_DEFAULT: f32 = 2.0;
const ZOOM_MAX: f32 = 17.0;

const ZOOM_PIXELS_FACTOR: f32 = 200.0;
const ZOOM_WHEEL_SCALE: f32 = 0.2;

const ITERS_MIN: u32 = 20;
const ITERS_DEFAULT: u32 = 20;
const ITERS_MAX: u32 = 200;

const CENTER_DEFAULT: Vec2 = Vec2::new(-1.5, 0.0);

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Uniforms {
    resolution: Vec2,
    center: Vec2,
    scale: f32,
    max_iter: u32,
}

struct FragmentShaderPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl FragmentShaderPipeline {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FragmentShaderPipeline shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shader.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FragmentShaderPipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_quad uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = pipeline.get_bind_group_layout(0);
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shader_quad uniform bind group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    fn render(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: Rectangle<u32>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fill color test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_viewport(
            viewport.x as f32,
            viewport.y as f32,
            viewport.width as f32,
            viewport.height as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        pass.draw(0..3, 0..1);
    }
}

#[derive(Debug, Clone, Copy)]
struct Controls {
    max_iter: u32,
    zoom: f32,
    center: Vec2,
}

impl Controls {
    fn scale(&self) -> f32 {
        1.0 / 2.0_f32.powf(self.zoom) / ZOOM_PIXELS_FACTOR
    }
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            max_iter: ITERS_DEFAULT,
            zoom: ZOOM_DEFAULT,
            center: CENTER_DEFAULT,
        }
    }
}

#[derive(Debug)]
struct FragmentShaderPrimitive {
    controls: Controls,
}

impl FragmentShaderPrimitive {
    fn new(controls: Controls) -> Self {
        Self { controls }
    }
}

impl shader::Primitive for FragmentShaderPrimitive {
    fn prepare(
        &self,
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: Rectangle,
        target_size: Size<u32>,
        _scale_factor: f32,
        storage: &mut shader::Storage,
    ) {
        if !storage.has::<FragmentShaderPipeline>() {
            storage.store(FragmentShaderPipeline::new(device, format));
        }

        let pipeline = storage.get_mut::<FragmentShaderPipeline>().unwrap();

        pipeline.update(
            queue,
            &Uniforms {
                resolution: Vec2::new(target_size.width as f32, target_size.height as f32),
                center: self.controls.center,
                scale: self.controls.scale(),
                max_iter: self.controls.max_iter,
            },
        );
    }

    fn render(
        &self,
        storage: &shader::Storage,
        target: &wgpu::TextureView,
        _target_size: Size<u32>,
        viewport: Rectangle<u32>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let pipeline = storage.get::<FragmentShaderPipeline>().unwrap();
        pipeline.render(target, encoder, viewport);
    }
}

#[derive(Debug, Clone)]
enum Message {
    UpdateMaxIterations(u32),
    UpdateZoom(f32),
    PanningDelta(Vec2),
    ZoomDelta(Vec2, Rectangle, f32),
}

enum MouseInteraction {
    Idle,
    Panning(Vec2),
}

impl Default for MouseInteraction {
    fn default() -> Self {
        MouseInteraction::Idle
    }
}

struct FragmentShaderProgram {
    controls: Controls,
}

impl FragmentShaderProgram {
    fn new() -> Self {
        Self {
            controls: Controls::default(),
        }
    }
}

impl shader::Program<Message> for FragmentShaderProgram {
    type State = MouseInteraction;
    type Primitive = FragmentShaderPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        FragmentShaderPrimitive::new(self.controls)
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
        _shell: &mut Shell<'_, Message>,
    ) -> (Status, Option<Message>) {
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            if let Some(pos) = cursor.position_in(bounds) {
                let pos = Vec2::new(pos.x, pos.y);
                let delta = match delta {
                    mouse::ScrollDelta::Lines { x: _, y } => y,
                    mouse::ScrollDelta::Pixels { x: _, y } => y,
                };
                return (
                    Status::Captured,
                    Some(Message::ZoomDelta(pos, bounds, delta)),
                );
            }
        }

        match state {
            MouseInteraction::Idle => match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    if let Some(pos) = cursor.position_over(bounds) {
                        *state = MouseInteraction::Panning(Vec2::new(pos.x, pos.y));
                        return (Status::Captured, None);
                    }
                }
                _ => {}
            },
            MouseInteraction::Panning(prev_pos) => match event {
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    *state = MouseInteraction::Idle;
                }
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    let pos = Vec2::new(position.x, position.y);
                    let delta = pos - *prev_pos;
                    *state = MouseInteraction::Panning(pos);
                    return (Status::Captured, Some(Message::PanningDelta(delta)));
                }
                _ => {}
            },
        };

        (Status::Ignored, None)
    }
}

struct FragmentShaderApp {
    program: FragmentShaderProgram,
}

fn control<'a>(
    label: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![text(label), control.into()].spacing(10).into()
}

impl Sandbox for FragmentShaderApp {
    type Message = Message;

    fn new() -> Self {
        Self {
            program: FragmentShaderProgram::new(),
        }
    }

    fn title(&self) -> String {
        String::from("Fragment Shader Widget - Iced")
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            control(
                "Max iterations",
                slider(
                    ITERS_MIN..=ITERS_MAX,
                    self.program.controls.max_iter,
                    move |iter| { Message::UpdateMaxIterations(iter) }
                )
                .width(Length::Fill)
            ),
            control(
                "Zoom",
                slider(
                    ZOOM_MIN..=ZOOM_MAX,
                    self.program.controls.zoom,
                    move |zoom| { Message::UpdateZoom(zoom) }
                )
                .step(0.01)
                .width(Length::Fill)
            ),
        ];

        let shader = shader(&self.program)
            .width(Length::Fill)
            .height(Length::Fill);

        column![shader, controls]
            .align_items(Alignment::Center)
            .padding(10)
            .spacing(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::UpdateMaxIterations(max_iter) => {
                self.program.controls.max_iter = max_iter;
            }
            Message::UpdateZoom(zoom) => {
                self.program.controls.zoom = zoom;
            }
            Message::PanningDelta(delta) => {
                self.program.controls.center -= 2.0 * delta * self.program.controls.scale();
            }
            Message::ZoomDelta(pos, bounds, delta) => {
                let delta = delta * ZOOM_WHEEL_SCALE;
                let prev_scale = self.program.controls.scale();
                let prev_zoom = self.program.controls.zoom;
                self.program.controls.zoom = (prev_zoom + delta).max(ZOOM_MIN).min(ZOOM_MAX);

                let vec = pos - Vec2::new(bounds.width, bounds.height) * 0.5;
                let new_scale = self.program.controls.scale();
                self.program.controls.center += vec * (prev_scale - new_scale) * 2.0;
            }
        }
    }
}

fn main() -> iced::Result {
    FragmentShaderApp::run(Settings::default())
}
