use std::{env, path::PathBuf};

fn main() {
    for path in ["build.rs", "nuklear", "src"] {
        println!("cargo:rerun-if-changed={path}");
    }

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let nuklear = bindgen::Builder::default()
        .header("src/nuklear_decl.h")
        .generate_comments(false)
        .blocklist_item("^FP_.*")
        .generate()
        .expect("Unable to generate nuklear bindings");
    nuklear
        .write_to_file(out_path.join("nuklear_bindings.rs"))
        .expect("Couldn't write nuklear bindings!");

    cc::Build::new()
        .file("src/nuklear_impl.c")
        .compile("nuklear_impl.a");
}
