#[path = "../framework.rs"]
mod framework;

use bytemuck::{Pod, Zeroable};

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    _pos: [f32; 2],
    _tex_coord: [f32; 2],
    _index: u32,
}

fn vertex(pos: [i8; 2], tc: [i8; 2], index: i8) -> Vertex {
    Vertex {
        _pos: [pos[0] as f32, pos[1] as f32],
        _tex_coord: [tc[0] as f32, tc[1] as f32],
        _index: index as u32,
    }
}

fn create_vertices() -> Vec<Vertex> {
    vec![
        // left rectangle
        vertex([-1, -1], [0, 1], 0),
        vertex([-1, 1], [0, 0], 0),
        vertex([0, 1], [1, 0], 0),
        vertex([0, -1], [1, 1], 0),
        // right rectangle
        vertex([0, -1], [0, 1], 1),
        vertex([0, 1], [0, 0], 1),
        vertex([1, 1], [1, 0], 1),
        vertex([1, -1], [1, 1], 1),
    ]
}

fn create_indices() -> Vec<u16> {
    vec![
        // Left rectangle
        0, 1, 2, // 1st
        2, 0, 3, // 2nd
        // Right rectangle
        4, 5, 6, // 1st
        6, 4, 7, // 2nd
    ]
}

#[derive(Copy, Clone)]
enum Color {
    RED,
    GREEN,
}

fn create_texture_data(color: Color) -> [u8; 8] {
    match color {
        Color::RED => [255, 0, 0, 255, 0, 255, 0, 255],
        Color::GREEN => [0, 255, 0, 255, 0, 255, 0, 255],
    }
}

struct Example {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_workaround: bool,
}

impl framework::Example for Example {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::UNSIZED_BINDING_ARRAY
            | wgpu::Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
            | wgpu::Features::SAMPLED_TEXTURE_ARRAY_DYNAMIC_INDEXING
            | wgpu::Features::PUSH_CONSTANTS
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::SAMPLED_TEXTURE_BINDING_ARRAY
    }
    fn required_limits() -> wgpu::Limits {
        wgpu::Limits {
            max_push_constant_size: 4,
            ..wgpu::Limits::default()
        }
    }
    fn init(
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let uniform_workaround = false;
        let vs_module = device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));
        let fs_source = // match device.features() {
            // f if f.contains(wgpu::Features::UNSIZED_BINDING_ARRAY) => {
            //     wgpu::include_spirv!("unsized-non-uniform.frag.spv")
            // }
            // f if f.contains(wgpu::Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING) => {
            //     wgpu::include_spirv!("non-uniform.frag.spv")
            // }
            // f if f.contains(wgpu::Features::SAMPLED_TEXTURE_ARRAY_DYNAMIC_INDEXING) => {
            //     uniform_workaround = true;
            //     wgpu::include_spirv!("uniform.frag.spv")
            // }
            // f if f.contains(wgpu::Features::SAMPLED_TEXTURE_BINDING_ARRAY) => {
                wgpu::include_spirv!("constant.frag.spv")
        //     }
        //     _ => unreachable!(),
        // };
        ;
        let fs_module = device.create_shader_module(fs_source);

        let vertex_size = std::mem::size_of::<Vertex>();
        let vertex_data = create_vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_data = create_indices();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsage::INDEX,
        });

        let red_texture_data = create_texture_data(Color::RED);
        let green_texture_data = create_texture_data(Color::GREEN);

        let size = wgpu::Extent3d {
            width: 1,
            height: 2,
            depth: 1,
        };

        let texture_descriptor = wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: None,
        };
        let red_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("red"),
            ..texture_descriptor
        });
        let green_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("green"),
            ..texture_descriptor
        });

        let red_texture_view = red_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let green_texture_view = green_texture.create_view(&wgpu::TextureViewDescriptor::default());

        queue.write_texture(
            wgpu::TextureCopyView {
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                texture: &red_texture,
            },
            &red_texture_data,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4,
                rows_per_image: 2,
            },
            size,
        );
        queue.write_texture(
            wgpu::TextureCopyView {
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                texture: &green_texture,
            },
            &green_texture_data,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4,
                rows_per_image: 2,
            },
            size,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: std::num::NonZeroU32::new(2),
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&[
                        red_texture_view,
                        green_texture_view,
                    ]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            layout: &bind_group_layout,
            label: Some("bind group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: if uniform_workaround {
                &[wgpu::PushConstantRange {
                    stages: wgpu::ShaderStage::FRAGMENT,
                    range: 0..4,
                }]
            } else {
                &[]
            },
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                ..Default::default()
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[sc_desc.format.into()],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: vertex_size as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float2, 1 => Float2, 2 => Int],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            bind_group,
            pipeline,
            uniform_workaround,
        }
    }
    fn resize(
        &mut self,
        _sc_desc: &wgpu::SwapChainDescriptor,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        // noop
    }
    fn update(&mut self, _event: winit::event::WindowEvent) {
        // noop
    }
    fn render(
        &mut self,
        frame: &wgpu::SwapChainTexture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _spawner: &impl futures::task::LocalSpawn,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("primary"),
        });

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..));
        if self.uniform_workaround {
            rpass.set_push_constants(wgpu::ShaderStage::FRAGMENT, 0, bytemuck::cast_slice(&[0]));
            rpass.draw_indexed(0..6, 0, 0..1);
            rpass.set_push_constants(wgpu::ShaderStage::FRAGMENT, 0, bytemuck::cast_slice(&[1]));
            rpass.draw_indexed(6..12, 0, 0..1);
        } else {
            rpass.draw_indexed(0..12, 0, 0..1);
        }

        drop(rpass);

        queue.submit(Some(encoder.finish()));
    }
}

fn main() {
    framework::run::<Example>("texture-arrays");
}
