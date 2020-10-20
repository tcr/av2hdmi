extern "C" {
    fn fn_setup() -> u32;
    fn fn_collect() -> *const u16;
}

fn main() {
    // Instead of hardcoding the path, you could also use `Framebuffer::list()`
    // to find paths to available devices.
    let fb = linuxfb::Framebuffer::new("/dev/fb0").unwrap();

    println!("Size in pixels: {:?}", fb.get_size());

    println!("Bytes per pixel: {:?}", fb.get_bytes_per_pixel());

    println!("Physical size in mm: {:?}", fb.get_physical_size());

    // Map the framebuffer into memory, so we can write to it:
    let mut data = fb.map().unwrap();

    // Make everything black:
    for i in 0..data.len() {
        data[i] = 0;
    }

    // Make everything white:
    for i in 0..data.len() {
        if i % 4 == 0 {
            data[i] = 0x00;
        } else if i % 4 == 2 {
            data[i] = 0x94;
        } else {
            data[i] = 0x00;
        }
    }

    unsafe {
        fn_setup();

        let sample_count = 10000;

        loop {
            let mut frame = vec![];
            for i in 0..80 {
                let buf = fn_collect();
                // println!("collecting...");
                let line = std::slice::from_raw_parts(buf, sample_count).iter().map(|x| (2080.0 - (*x as f32)) / 410.0).collect::<Vec<_>>();
                // println!("{:?}", line);
                // println!("done...\n\n");
                frame.extend(&line);
            }

            // let mut file = File::create("out.csv").unwrap();

            // TODO draw directly to framebuffer with no raquote. try to capture full frame

            // let mut dt = DrawTarget::new(800, 600);
            let size = fb.get_size();


            let (prefix, pixels, suffix) = unsafe { data.align_to_mut::<u32>() };
            assert_eq!(prefix.len(), 0);
            assert_eq!(suffix.len(), 0);

            // Smooth it!
            let WIN_LENGTH = 16;
            let mut x = 0;
            let mut y = 0;
            let mut last = 0.0;
            // println!("----");
            for (i, w) in frame.chunks(WIN_LENGTH).take(200*40).enumerate() {
                let a: f32 = w.iter().sum::<f32>() / (WIN_LENGTH as f32);
                if last < -0.1 && a > -0.1 {
                    // if y < 10 {
                    //     println!("{:?}", x);
                    // }
                    x = 0;
                    y += 1;
                }
                if y > 100 {
                    break;
                }
                last = a;
                // println!("{:?}", a);
                // if a < 0.0 {
                //     if x > 20 {
                //         y += 1;
                //     }
                //     x = 0;
                //     continue;
                // }

                draw_pixel(pixels, x, y, a);

                if x < 165 {
                    x += 1;
                } else {
                    // x = 0;
                    // y += 1;
                }
                // if x > 165 {
                //     y += 1;
                //     x = 0;
                // }
            }
        }
    }
}

fn draw_pixel(pixels: &mut [u32], x: usize, y: usize, value: f32) {
    let MUL = 6;
    let mut lum = (a * (0xff as f32)) as u32;
    if lum > 0xFF {
        lum = 0xFF;
    }
    for nx in x*MUL..(x+1)*MUL {
        for ny in y*5..(y+1)*5 {
            pixels[ ((12 + ny) * (size.0 as usize)) + (60 + nx)] = (lum << 16) + (lum << 8) + lum;
        }
    }
}

fn color_loop(fb: &linuxfb::Framebuffer) {
    // Map the framebuffer into memory, so we can write to it:
    let mut data = fb.map().unwrap();

    loop {
        for temp_val in 0..0xff {
            let mut dt = DrawTarget::new(800, 600);
            let mut pb = PathBuilder::new();
            pb.move_to(100., 10.);
            pb.cubic_to(150., 40., 175., 0., 200., 10.);
            pb.quad_to(120., 100., 80., 200.);
            pb.quad_to(150., 180., 300., 300.);
            pb.close();
            let path = pb.finish();

            let gradient = Source::new_radial_gradient(
                Gradient {
                    stops: vec![
                        GradientStop {
                            position: 0.2,
                            color: Color::new(0xff, 0, 0xff, 0),
                        },
                        GradientStop {
                            position: 0.8,
                            color: Color::new(0xff, 0xff, 0xff, 0xff),
                        },
                        GradientStop {
                            position: 1.,
                            color: Color::new(temp_val, 0xff, 0, 0xff),
                        },
                    ],
                },
                Point::new(150., 150.),
                128.,
                Spread::Pad,
            );
            dt.fill(&path, &gradient, &DrawOptions::new());

            let mut pb = PathBuilder::new();
            pb.move_to(100., 100.);
            pb.line_to(300., 300.);
            pb.line_to(200., 300.);
            let path = pb.finish();

            dt.stroke(
                &path,
                &Source::Solid(SolidSource {
                    r: 0x0,
                    g: 0x0,
                    b: temp_val,
                    a: 0x80,
                }),
                &StrokeStyle {
                    cap: LineCap::Round,
                    join: LineJoin::Round,
                    width: 10.,
                    miter_limit: 2.,
                    dash_array: vec![10., 18.],
                    dash_offset: 16.,
                },
                &DrawOptions::new()
            );

            let (prefix, pixels, suffix) = unsafe { data.align_to_mut::<u32>() };
            assert_eq!(prefix.len(), 0);
            assert_eq!(suffix.len(), 0);

            let dt_data = dt.get_data();
            let size = fb.get_size();
            for y in 0..600 {
                for x in 0..800 {
                    pixels[ ((12 + y) * (size.0 as usize)) + (60 + x)] = dt_data[(y*800) + x];
                }
            }

            std::thread::sleep_ms(1);
        }
    }
}
