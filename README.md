# av2hdmi

Raspi code for reading an uncompressed NTSC signal and rendering it with the GPU, aka an AV to HDMI converter.

```
cargo run; ./plot
```

![Capture decoded](https://user-images.githubusercontent.com/80639/101274128-8822ae00-3769-11eb-8237-7439e8969320.png)

Decoded partial capture of AV video from Sonic 3 for Genesis.

# Status

* SMI code doesn't capture a whole NTSC frame yet + occasional digital artifacts
* Decoding only implemented for CPU, not yet working on GPU shader
* Color decoding is incorrect
* Voltage normalization is incorrect

# Sources

* [Raspberry Pi Secondary Memory Interface (SMI)](https://iosoft.blog/2020/07/16/raspberry-pi-smi/), iosoft.blog
