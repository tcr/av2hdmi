// Example custom build script.
fn main() {
  // Tell Cargo that if the given file changes, to rerun this build script.
  println!("cargo:rerun-if-changed=iosoft-rpi/rpi_smi_adc_test.c");
  // Use the `cc` crate to build a C file and statically link it.
  cc::Build::new()
      .file("iosoft-rpi/rpi_dma_utils.c")
      .file("iosoft-rpi/rpi_smi_adc_test.c")
      .compile("rpi_smi_adc_test");
}
