use realfft::{ComplexToReal, RealToComplex};
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

fn main() {
    use std::f32::consts::PI;

// generate 16 samples of a sine wave at frequency 3
let sample_count = 32;
let signal_freq = 3.8;
let sample_interval = 1. / sample_count as f32;
let mut samples: Vec<_> = (0..sample_count)
    .map(|i| (2. * PI * signal_freq * sample_interval * (i as f32 + 2.95) as f32).sin())
    .collect();
let mut samples2 = samples.clone();

// compute the RFFT of the samples
println!("samps> {:?}", samples);
let spectrum = microfft::real::rfft_32(&mut samples);

// the spectrum has a spike at index `signal_freq`
let amplitudes: Vec<_> = spectrum.iter().map(|c| c.norm() as u32).collect();
// assert_eq!(&amplitudes, &[0, 0, 0, 8, 0, 0, 0, 0]);
println!("amps> {:?}", amplitudes);
let args: Vec<_> = spectrum.iter().map(|c| c.arg() as u32).collect();
// assert_eq!(&amplitudes, &[0, 0, 0, 8, 0, 0, 0, 0]);
println!("args> {:?}", args);
println!();

// let mut indata = vec![0.0f64; 256];
let mut indata = samples2.iter().map(|x| *x as f64).collect::<Vec<_>>();
let mut spectrum: Vec<Complex<f64>> = vec![Complex::zero(); 17];
let mut outdata: Vec<f64> = vec![0.0; 256];

//create an FFT and forward transform the input data
let mut r2c = RealToComplex::<f64>::new(32).unwrap();
r2c.process(&mut indata, &mut spectrum[..]).unwrap();

println!("samps> {:?}", samples2);
println!("spec> {:?}", spectrum.iter().map(|x| x.norm()).collect::<Vec<_>>()[4]);
println!("args> {:?}", spectrum.iter().map(|x| x.arg()).collect::<Vec<_>>()[4]);

}
