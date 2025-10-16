use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../nuklear/nuklear.h");
    println!("cargo:rerun-if-changed=../nuklear/demo/sdl_opengl3/nuklear_sdl_gl3.h");
    println!("cargo:rerun-if-changed=src/nuklear/mod.rs");
    println!("cargo:rerun-if-changed=src/nuklear/nuklear_decl.h");
    println!("cargo:rerun-if-changed=src/nuklear/nuklear_impl.c");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let nuklear = bindgen::Builder::default()
        .header("src/nuklear/nuklear_decl.h")
        .generate_comments(false)
        .blocklist_item("^FP_.*")
        .generate()
        .expect("Unable to generate nuklear bindings");
    nuklear
        .write_to_file(out_path.join("nuklear_bindings.rs"))
        .expect("Couldn't write nuklear bindings!");

    cc::Build::new()
        .file("src/nuklear/nuklear_impl.c")
        .compile("nuklear_impl.a");
}
