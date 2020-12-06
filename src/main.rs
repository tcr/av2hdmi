#[path = "../framework.rs"]
mod framework;

use bytemuck::{Pod, Zeroable};

use wgpu::util::DeviceExt;
use std::fs::File;
use std::path::Path;
use std::io::BufWriter;
use png::HasParameters;
use byteorder::{ReadBytesExt, NativeEndian, LittleEndian, BigEndian};
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::FFTplanner;

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

        let chunk_width = 12;
        let dim_height = 180;
        let dim_width = 222;
        let dim_full_width = 2654;
        let bytes_per_row = 2;

        let mut file = File::open("./captures/2").unwrap();
        let mut frame_out: Vec<i16> = vec![0; dim_full_width * dim_height]; //file.metadata().unwrap().len() as usize/2];
        file.read_i16_into::<NativeEndian>(&mut frame_out[0..(dim_full_width * dim_height)]).unwrap();

        let shift = std::env::var("SHIFT").unwrap_or("7.0".to_string()).parse::<f32>().unwrap();

        let carrier_freq = |x: usize| -> f32 {
            (x as f32 + shift) * (2.0 * std::f32::consts::PI) * (3.58/41.66)
        };

        fn vec_avg(v: &std::collections::VecDeque<f32>) -> f32 {
            v.into_iter().sum::<f32>() / v.len() as f32
        }

        // Do some charts with a sample the first five scanlines.
        if true {
            use std::io::Write;

            let dim_offset = 80;
            let dim_range = dim_offset*dim_full_width..(dim_offset + 5)*dim_full_width;

            // Create out.csv, which is the low-pass filtered signal.
            let mut file = File::create("./out/out.csv").unwrap();
            let mut bandpass = vec![];
            let win_len = 32;
            for samp in frame_out[dim_range.clone()]
                .windows(win_len)
                .enumerate()
                .map(|(w_i, w)| {

                    // compute the RFFT of the samples
                    // println!("samps> {:?}", samples);
                    // let spectrum = microfft::real::rfft_16(&mut samples);

                    // // the spectrum has a spike at index `signal_freq`
                    // let amplitudes: Vec<_> = spectrum.iter().map(|c| c.norm() as u32).collect();
                    // // assert_eq!(&amplitudes, &[0, 0, 0, 8, 0, 0, 0, 0]);
                    // println!("amps> {:?}", amplitudes);
                    // let args: Vec<_> = spectrum.iter().map(|c| c.arg() as u32).collect();
                    // // assert_eq!(&amplitudes, &[0, 0, 0, 8, 0, 0, 0, 0]);
                    // println!("args> {:?}", args);
                    // println!();


                    let mut samples_real = w.iter().map(|y| {
                        Complex::new(volt_decode(*y as u16) as f64 + 100.0, 0.0)
                    }).collect::<Vec<_>>();
                    let mut samples = samples_real.clone();

                    // For fake data
                    // let mut samples_real = w.iter().map(|y| {
                    //     let a = 20.0 * (((w_i as f64) + 4.8)/(41.66/(3.58 * 2.0 * std::f64::consts::PI))).sin();
                    //     Complex::new(a, 0.0)
                    // }).collect::<Vec<_>>();
                    // let mut samples = samples_real.clone();

                    // let mut indata = vec![0.0f64; 256];
                    let mut spectrum: Vec<Complex<f64>> = vec![Complex::zero(); win_len];
                    let mut outdata: Vec<Complex<f64>> = vec![Complex::zero(); win_len];

                    let mut planner = FFTplanner::new(false);
                    let fft = planner.plan_fft(win_len);
                    fft.process(&mut samples, &mut spectrum);

                    // spectrum[0] = Complex::zero();
                    // spectrum[1] = Complex::zero();
                    for l in 5..win_len {
                        spectrum[l] = Complex::zero();
                    }
                    //create an FFT and forward transform the input data
                    // let mut r2c = RealToComplex::<f64>::new(16).unwrap();
                    // r2c.process(&mut samples, &mut spectrum[..]).unwrap();

                    // println!("spectrum; {:?}", spectrum);
                    // println!("samps> {:?}", samples_real.iter().map(|x| x.norm()).collect::<Vec<_>>());
                    // println!("spec> {:?}", spectrum.iter().map(|x| x.norm()).collect::<Vec<_>>()[4]);
                    // println!("args> {:?}", spectrum.iter().map(|x| x.arg()).collect::<Vec<_>>()[4]);


                    // create an iFFT and inverse transform the spectum
                    let mut planner_i = FFTplanner::<f64>::new(true);
                    let fft_i = planner.plan_fft(win_len);
                    fft_i.process(&mut spectrum, &mut outdata);
                    // println!("samp2> {:?}", outdata.iter().map(|x| x.norm()/(win_len as f64)).collect::<Vec<_>>());
                    // println!();

                    // let spectrum = microfft::real::rfft_16(&mut samps[..]);
                    // let amplitudes: Vec<_> = spectrum.iter().map(|c| c.norm()).collect();
                    // println!("--[a]> {:?}", amplitudes);
                    // let phases: Vec<_> = spectrum.iter().map(|c| c.arg()).collect();
                    // println!("-----> {:?}", phases);
                    // println!();
                    // }

                    let segments = false;
                    if segments {
                        if w_i % win_len == (win_len / 2) {
                            outdata.iter().map(|x| (x.norm() / (win_len as f64)) as f32).collect::<Vec<_>>()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![
                            ((outdata[win_len / 2].norm() / (win_len as f64)) as f32) - 100.0,
                        ]
                    }
                }).flatten() {
                bandpass.push(samp);
                writeln!(file, "{}", samp);
            }


            let sample_subset = frame_out[dim_range].iter()
                .map(|x| volt_decode(*x as u16))
                .collect::<Vec<_>>();

            // Create out-sine.csv, which is a sine wave at the carrier frequency.
            let mut file = File::create("./out/out-sine.csv").unwrap();
            for (i, samp) in sample_subset.iter().enumerate() {
                writeln!(file, "{}", 20.0 * carrier_freq(i).sin()).ok();
            }

            // Create out-Y.csv
            let mut avg = std::collections::VecDeque::new();
            let mut y_out = vec![];
            for (i, samp) in sample_subset.iter().enumerate() {
                avg.push_front(*samp);
                if avg.len() > 12 {
                    avg.pop_back();
                }
                let mut res = vec_avg(&avg);
                if res < -100. {
                    res = -100.
                }
                y_out.push(res);
            }
            // Write out file.
            let mut file = File::create("./out/out-Y.csv").unwrap();
            for samp in &y_out {
                writeln!(file, "{}", samp).ok();
            }

            // Create out-I.csv
            let mut avg = std::collections::VecDeque::new();
            let mut i_out = vec![];
            for (i, samp) in sample_subset.iter().enumerate() {
                avg.push_front(samp * carrier_freq(i).sin() * 4.);
                if avg.len() > 12 {
                    avg.pop_back();
                }
                let mut res = vec_avg(&avg);
                if res < -100. {
                    res = -100.
                }
                i_out.push(res);
            }
            // Write out file.
            let mut file = File::create("./out/out-I.csv").unwrap();
            for samp in &i_out {
                writeln!(file, "{}", samp).ok();
            }

            // Create out-Q.csv
            let mut avg = std::collections::VecDeque::<f32>::new();
            let mut q_out = vec![];
            for (i, samp) in bandpass.iter().enumerate() {
                avg.push_front(samp * carrier_freq(i).cos() * 4.);
                if avg.len() > 12 {
                    avg.pop_back();
                }
                let mut res = vec_avg(&avg);
                if res < -100. {
                    res = -100.
                }
                q_out.push(res);
            }
            // Write out file.
            let mut file = File::create("./out/out-Q.csv").unwrap();
            for samp in &q_out {
                writeln!(file, "{}", samp).ok();
            }

            // let _ = bandpass
            //     .windows(win_len)
            //     .enumerate()
            //     .map(|(w_i, w)| {
            //         let mut samples_real = w.iter().map(|y| {
            //             // TODO don't volt_decode wtf
            //             Complex::new(*y as f64, 0.0)
            //         }).collect::<Vec<_>>();
            //         let mut samples = samples_real.clone();

            //         // let mut indata = vec![0.0f64; 256];
            //         let mut spectrum: Vec<Complex<f64>> = vec![Complex::zero(); win_len];
            //         let mut outdata: Vec<Complex<f64>> = vec![Complex::zero(); win_len];

            //         let mut planner = FFTplanner::new(false);
            //         let fft = planner.plan_fft(win_len);
            //         fft.process(&mut samples, &mut spectrum);

            //         println!("spectrum; {:?}", &spectrum[2..5]);


            //         // For fake data
            //         let mut samples_real = w.iter().map(|y| {
            //             let a = 20.0 * (((w_i as f64) + *y as f64 + 4.8)/(41.66/(3.58 * 2.0 * std::f64::consts::PI))).sin();
            //             Complex::new(a, 0.0)
            //         }).collect::<Vec<_>>();
            //         let mut samples = samples_real.clone();

            //         // let mut indata = vec![0.0f64; 256];
            //         let mut spectrum: Vec<Complex<f64>> = vec![Complex::zero(); win_len];
            //         let mut outdata: Vec<Complex<f64>> = vec![Complex::zero(); win_len];

            //         let mut planner = FFTplanner::new(false);
            //         let fft = planner.plan_fft(win_len);
            //         fft.process(&mut samples, &mut spectrum);

            //         println!("comparison; {:?}", &spectrum[2..5]);

            //         println!();
            //     })
            //     .collect::<Vec<_>>();

            // for i in 0..dim_full_width*5 {
            //     // value += ((frame_out[i] as f32) - value) / smoothing;
            //     writeln!(file, "{}", volt_decode(frame_out[i] as u16));
            // }
        }

        if true {
            let path = Path::new(r"out/frame.png");
            let file = File::create(path).unwrap();
            let ref mut w = BufWriter::new(file);

            let mut encoder = png::Encoder::new(w, dim_width as u32, dim_height as u32); // Width is 2 pixels and height is 1.
            encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();

            // Enumerate over frame chunks.
            let data = {
                frame_out
                    .chunks(dim_full_width)
                    .enumerate()
                    .map(|(line_i, x)| {
                        let mut input = x.iter()
                            .map(|x| *x as u16)
                            .collect::<Vec<_>>();

                        // Find the inline carrier signal.
                        let i_fall = input.windows(chunk_width).enumerate().rev().find_map(|(i, samples)| {
                            let sample = samples.iter()
                                .map(|x| volt_decode(*x))
                                .sum::<f32>() / (chunk_width as f32);
                            if sample < -40. && sample > -100. {
                                Some(i)
                            } else {
                                None
                            }
                        }).unwrap_or(0);
                        let mut i_rise = input.windows(chunk_width).enumerate().find_map(|(i, samples)| {
                            if i < i_fall {
                                return None;
                            }
                            let sample = samples.iter()
                                .map(|x| volt_decode(*x))
                                .sum::<f32>() / (chunk_width as f32);
                            if sample > -5. {
                                Some(i)
                            } else {
                                None
                            }
                        }).unwrap_or(0);

                        // TODO this should rotate on x, not samples_vec, for precise targeting
                        input.rotate_left(i_rise);

                        let mut samples_vec = input
                            .chunks(chunk_width)
                            .map(move |y| {
                                y.iter().map(|s| {
                                    volt_decode(*s as u16)
                                }).collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();

                        // Convert to RGB values.
                        return samples_vec.clone().into_iter()
                            .enumerate()
                            .map(move |(chunk_i, samples)| {
                            let mut chunk_index = chunk_i * chunk_width;

                            // Calculate YIQ against carrier frequency.
                            let y_sample = samples.iter().sum::<f32>() / (chunk_width as f32);
                            let i_sample = samples.iter()
                                .enumerate()
                                .map(|(i, x)| x * carrier_freq(i + chunk_index).sin() * 4.)
                                .sum::<f32>() / (chunk_width as f32);
                            let q_sample = samples.iter()
                                .enumerate()
                                .map(|(i, x)| x * carrier_freq(i + chunk_index).cos() * 4.)
                                .sum::<f32>() / (chunk_width as f32);

                            let i_amps = samples.iter()
                                .enumerate()
                                .map(|(i, x)| x * carrier_freq(i + chunk_index).sin() * 4.)
                                .map(|y| (y * 100.) as u32) // two digit precision
                                .collect::<Vec<_>>();
                            let i_amp_input = (i_amps.iter().max().unwrap() - i_amps.iter().min().unwrap()) as f32;
                            let i_amp = num::clamp(i_amp_input / 80000., 0., 1.0);
                            // let i_amp = 1.0;
                            let q_amps = samples.iter()
                                .enumerate()
                                .map(|(i, x)| x * carrier_freq(i + chunk_index).cos() * 4.)
                                .map(|y| (y * 100.) as u32) // two digit precision
                                .collect::<Vec<_>>();
                            let q_amp_input = (q_amps.iter().max().unwrap() - q_amps.iter().min().unwrap()) as f32;
                            let q_amp = num::clamp(q_amp_input / 80000., 0., 1.0);
                            // let q_amp = 1.0;

                            // println!("i_amp {:?} q_amp {:?}", i_amp_input, q_amp_input);

                            let y_clamped = (num::clamp(y_sample, 0., 140.) / 140.);
                            let mut i_clamped = (num::clamp(i_sample, -60., 60.) / 60.) * i_amp;
                            let mut q_clamped = (num::clamp(q_sample, -60., 60.) / 60.) * q_amp;
                            // println!("{:?}", (y_clamped, i_clamped, q_clamped));

                            // Uncomment for monochrome
                            // i_clamped = 0.;
                            // q_clamped = 0.;

                            // let matrix = ndarray::arr1(&[y_sample, i_clamped, q_clamped]);
                            // let con_matrix = ndarray::arr2(&[
                            //     [1., 0.956, 0.619],
                            //     [1., -0.272, -0.647],
                            //     [1., -1.106, 1.703]
                            // ]);
                            // let rgb_matrix = matrix.dot(&con_matrix);
                            // println!("{:?}", rgb_matrix);

                            let r = y_clamped + (2.4563 * i_clamped) + (1.6190 * q_clamped);
                            let g = y_clamped - (0.2721 * i_clamped) - (0.6474 * q_clamped);
                            let b = y_clamped - (1.1070 * i_clamped) + (1.7046 * q_clamped);
                            // println!("    rgb -----> {:?}", (r, g, b));
                            let r_ = r * 255.;
                            let g_ = g * 255.;
                            let b_ = b * 255.;

                            // if avg.len() > 12 {
                            //     avg.pop_back();
                            // }
                            // let mut res = vec_avg(&avg);
                            // if res < -60. {
                            //     res = -60.
                            // } else if res > 60. {
                            // res = 60
                            // }

                            let mut color = y_clamped * 255.;
                            if color > 255. {
                                color = 0.;
                            }

                            return vec![r_ as u8, g_ as u8, b_ as u8, 255];
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

        panic!("done");

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
