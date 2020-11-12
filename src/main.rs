extern "C" {
    fn fn_setup() -> u32;
    fn fn_collect(buf: *mut u16) -> *const u16;
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
        let sample_loop = 40;

        let mut section = 0;
        let SECTION_HEIGHT = 8;
        loop {
            let mut raw_frame: Vec<u16> = vec![0; sample_count];
            // for i in 0..sample_loop {
                fn_collect(&mut raw_frame[0] as *mut u16);
                // println!("collecting...");
                // let line = std::slice::from_raw_parts(buf, sample_count)();
                // println!("{:?}", line);
                // println!("done...\n\n");
                // frame.extend(&line);
                std::thread::sleep_ms(1);
            // }
            let frame = raw_frame.iter().map(|x| (2080.0 - (*x as f32)) / 410.0).collect::<Vec<_>>();

            // let mut file = File::create("out.csv").unwrap();

            // let mut dt = DrawTarget::new(800, 600);
            let size = fb.get_size();

            let (prefix, pixels, suffix) = unsafe { data.align_to_mut::<u32>() };
            assert_eq!(prefix.len(), 0);
            assert_eq!(suffix.len(), 0);

            // Smooth it!
            let WIN_LENGTH = 16;
            let mut x = 0;
            let mut y = section * SECTION_HEIGHT;

            // Make everything white:
            for i in 0..SECTION_HEIGHT {
                for x2 in 0..166 {
                    draw_pixel(pixels, x2, y + i, 0x940000, size);
                }
            }

            let mut last = 0;
            let mut active = false;
            for (i, w) in frame.chunks(WIN_LENGTH).take(200*40).enumerate() {
                let a: f32 = w.iter().sum::<f32>() / (WIN_LENGTH as f32);

                // if a < -0.1 {
                //     last += 1;
                // } else {
                //     if last > 5 {
                //         active = true;
                        // if x > 40 {
                        //     y += 1;
                        // }
                        // x = 0;

                        // for xfill in 0..165 {
                        //     draw_pixel(pixels, xfill, y, 0x221111, size);
                        // }
                //     }
                //     last = 0;
                // }

                // if !active {
                //     continue;
                // }

                // if y < 10 {
                //     println!("{:?}", x);
                // }
                // }
                if y > 100 {
                    break;
                }
                // last = a;

                draw_pixel(pixels, x, y, volt_to_color(a), size);

                // if i % 625 == 0 {
                //     draw_pixel(pixels, x, y, 0x00ff00, size);
                // }

                if x < 165 {
                    x += 1;
                } else {
                    x = 0;
                    y += 1;
                }
                // if x > 165 {
                //     y += 1;
                //     x = 0;
                // }
            }

            section += 1;
            if section > 12 {
                section = 0;
            }
        }
    }
}

fn volt_to_color(value: f32) -> u32 {
    let mut lum = (value * (0xff as f32)) as u32;
    if lum > 0xFF {
        lum = 0xFF;
    }
    return (lum << 16) + (lum << 8) + lum;
}

fn draw_pixel(pixels: &mut [u32], x: usize, y: usize, color: u32, size: (u32, u32)) {
    let MUL = 6;
    for nx in x*MUL..(x+1)*MUL {
        for ny in y*5..(y+1)*5 {
            pixels[ ((12 + ny) * (size.0 as usize)) + (60 + nx)] = color;
        }
    }
}
