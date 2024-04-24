extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {

    cc::Build::new()
        .file("rpi_ws281x/ws2811.c")
        .file("rpi_ws281x/rpihw.c")
        .file("rpi_ws281x/pwm.c")
        .file("rpi_ws281x/dma.c")
        .file("rpi_ws281x/mailbox.c")
        .file("rpi_ws281x/pcm.c")
        .compile("wrapper");
    println!("cargo::rerun-if-changed=rpi_ws281x/ws2811.c");
    println!("cargo::rerun-if-changed=rpi_ws281x/rpihw.c");
    println!("cargo::rerun-if-changed=rpi_ws281x/pwm.c");
    println!("cargo::rerun-if-changed=rpi_ws281x/dma.c");
    println!("cargo::rerun-if-changed=rpi_ws281x/mailbox.c");
    println!("cargo::rerun-if-changed=rpi_ws281x/pcm.c");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

