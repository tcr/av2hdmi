#[path = "../framework.rs"]
mod framework;

use bytemuck::{Pod, Zeroable};

use wgpu::util::DeviceExt;
use std::fs::File;
use std::path::Path;
use std::io::BufWriter;
use png::HasParameters;
use byteorder::{ReadBytesExt, NativeEndian, LittleEndian, BigEndian};

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
        vertex([1, 1], [1, 0], 0),
        vertex([1, -1], [1, 1], 0),
    ]
}

const FRAGMENT_COUNT: u32 = 1;
fn create_indices() -> Vec<u16> {
    vec![
        // Left rectangle
        0, 1, 2, // 1st
        2, 0, 3, // 2nd
    ]
}

fn volt_decode(input: u16) -> f32 {
    ((((2080 - ((input >> 4) as i16)) as f32) / 410.0) * 300.0) as f32
}

struct Example {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
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
        let vs_module = device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));
        let fs_source = // match device.features() {
            // f if f.contains(wgpu::Features::UNSIZED_BINDING_ARRAY) => {
            //     wgpu::include_spirv!("unsized-non-uniform.frag.spv")
            // }
            // f if f.contains(wgpu::Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING) => {
            //     wgpu::include_spirv!("non-uniform.frag.spv")
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

        // let red_texture_data = [255, 0, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255];
        // let bytes_per_row = 4;
        // let size = wgpu::Extent3d {
        //     width: 2,
        //     height: 2,
        //     depth: 1,
        // };

        let dim_height = 180;
        let dim_width = 166;
        let dim_full_width = 2654;
        let bytes_per_row = 2;

        let mut file = File::open("./frame-out").unwrap();
        let mut frame_out: Vec<i16> = vec![0; dim_full_width * dim_height]; //file.metadata().unwrap().len() as usize/2];
        file.read_i16_into::<NativeEndian>(&mut frame_out[0..(dim_full_width * dim_height)]).unwrap();

        {
            use std::io::Write;
            let mut file = File::create("./out.csv").unwrap();

            let smoothing = 16.0;
            let mut value = frame_out[0] as f32;
            for i in 0..dim_full_width*5 {
                // value += ((frame_out[i] as f32) - value) / smoothing;
                writeln!(file, "{}", volt_decode(frame_out[i] as u16));
            }
        }

        if true {
            let png_line_height = 8;
            let path = Path::new(r"frame.png");
            let file = File::create(path).unwrap();
            let ref mut w = BufWriter::new(file);

            let mut encoder = png::Encoder::new(w, dim_width as u32, dim_height as u32); // Width is 2 pixels and height is 1.
            encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();

            let data = {
                frame_out.chunks(dim_full_width).enumerate().map(|(x_i, x)| {
                    let mut y_i = 0;
                    return x
                        .chunks(16)
                        .map(move |y| {
                            let sample = y.iter().map(|s| {
                                volt_decode(*s as u16)
                            }).collect::<Vec<f32>>();

                            let mut color = sample.iter().sum::<f32>() / 16.0;
                            if color > 255.0 {
                                color = 0.0;
                            }

                            let mut fft_sample = sample.clone();
                            if fft_sample.len() == 16 && x_i == 1 && y_i < 24 {
                                println!("--[x]> {:?} == {:?}", y_i, color);
                                y_i += 1;
                                println!("--[s]> {:?}", sample);
                                let spectrum = microfft::real::rfft_16(&mut fft_sample[..]);
                                let amplitudes: Vec<_> = spectrum.iter().map(|c| c.norm()).collect();
                                println!("--[a]> {:?}", amplitudes);
                                let phases: Vec<_> = spectrum.iter().map(|c| c.arg()).collect();
                                println!("-----> {:?}", phases);
                                println!();
                            }

                            // println!("---> {} from {}", color, 2080 - (*y >> 4));
                            return vec![color as u8, color as u8, color as u8, 255];
                        })
                        .flatten();
                }).flatten().collect::<Vec<_>>()
            };
            writer.write_image_data(&data).unwrap(); // Save
        }

        // let mut fake_frame: Vec<u8> = vec![];
        // for y in 0..182 {
        //     for x in 0..165 {
        //         for i in 0..16 {
        //             fake_frame.extend(&[y]);
        //         }
        //     }
        // }

        // panic!("file: {:?}", frame_out.len());
        let red_texture_data = unsafe {
            &std::slice::from_raw_parts(frame_out.as_ptr() as *const u8, frame_out.len() * (bytes_per_row as usize))
        };

        // let frame_out_trunc = frame_out[0..(165 * 16 * 182)].to_vec();
        // println!("----> len {:?}", frame_out.len() as f32 / (165.0 * 16.0));

        // TODO correct this extent data
        let size = wgpu::Extent3d {
            width: dim_full_width as u32,
            height: dim_height as u32,
            depth: 1,
        };

        let texture_descriptor = wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Uint,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: None,
        };

        let red_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("red"),
            ..texture_descriptor
        });

        let red_texture_view = red_texture.create_view(&wgpu::TextureViewDescriptor::default());

        queue.write_texture(
            wgpu::TextureCopyView {
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                texture: &red_texture,
            },
            &red_texture_data,
            // &frame_out,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: bytes_per_row*size.width,
                rows_per_image: size.height,
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
                        component_type: wgpu::TextureComponentType::Uint,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: std::num::NonZeroU32::new(FRAGMENT_COUNT),
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
            push_constant_ranges: &[],
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
        rpass.draw_indexed(0..(6*FRAGMENT_COUNT), 0, 0..1);

        drop(rpass);

        queue.submit(Some(encoder.finish()));
    }
}

fn main() {
    framework::run::<Example>("texture-arrays");
}
