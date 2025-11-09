fn main() {
    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-arg=-Wl,-l:libGLEW.so.2.1");
}
