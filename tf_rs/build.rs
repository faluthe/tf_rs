use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../nuklear/nuklear.h");
    println!("cargo:rerun-if-changed=src/nuklear/mod.rs");

    let builder = bindgen::Builder::default()
        .header("../nuklear/nuklear.h")
        .generate_comments(false);

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("nuklear_bindings.rs"))
        .expect("Couldn't write bindings!");
}
