use crossbeam::channel::bounded;
use itertools::Itertools;

extern "C" {
    fn fn_setup(nsamp: u32) -> u32;
    fn fn_collect(nsamp: u32, dest: *mut u16);
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

    // Make everything red:
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

        let sample_count: u32 = 30000;
        let sample_max = sample_count + 40;
        let sample_loop = 20;

        let PIXEL_LEN = 165;

        let (tx, rx) = bounded(128);

        // Spawn a thread for taking several ADC samples and sending them over
        // a crossbeam channel.
        std::thread::spawn(move || {
            fn_setup(sample_count);

            let mut raw_frame: Vec<u16> = vec![2080 << 4; (sample_max as usize)*sample_loop];
            loop {
                // Sample DMA several times.
                for i in 0..sample_loop {
                    fn_collect(sample_count, &mut raw_frame[i * (sample_max as usize)] as *mut u16);
                }

                tx.send(raw_frame.clone());
            }
        });

        while let Ok(raw_frame) = rx.recv() {
            if rx.is_full() {
                panic!("Not moving fast enough");
            }

            // let mut file = File::create("out.csv").unwrap();

            let size = fb.get_size();

            let (prefix, pixels, suffix) = unsafe { data.align_to_mut::<u32>() };
            assert_eq!(prefix.len(), 0);
            assert_eq!(suffix.len(), 0);

            // Smooth it!
            let WIN_LENGTH = 16;
            let mut x = 0;
            let mut y = 0;

            let mut last = 0;
            let mut active = false;
            for value in raw_frame.iter()
                // Convert raw values into voltages
                .map(|x| (2080.0 - ((*x >> 4) as f32)) / 410.0)
                .chunks(WIN_LENGTH)
                .into_iter()
                .map(|value| value.sum::<f32>()) {
                // Average this window into a color value.
                let color: u32 = volt_to_color(value / WIN_LENGTH as f32);

                // Failsafe for not SEGFAULTing when we run out of VRAM
                if y > 100 {
                    break;
                }

                // Draw the pixel.
                draw_pixel(pixels, x, y, color, size);

                // We roll over at pixel PIXEL_LEN
                if x < PIXEL_LEN {
                    x += 1;
                } else {
                    x = 0;
                    y += 1;
                }
            }
        }
    }
}

/**
 * Converts a voltage value into a ARGB color.
 */
fn volt_to_color(value: f32) -> u32 {
    let mut lum = (value * (0xff as f32)) as u32;
    if lum > 0xFF {
        lum = 0xFF;
    }
    return (lum << 16) + (lum << 8) + lum;
}

fn draw_pixel(pixels: &mut [u32], x: usize, y: usize, color: u32, size: (u32, u32)) {
    let XMUL = 6;
    let YMUL  = 4;
    for nx in x*XMUL..(x+1)*XMUL {
        for ny in y*YMUL..(y+1)*YMUL {
            pixels[ ((12 + ny) * (size.0 as usize)) + (60 + nx)] = color;
        }
    }
}
