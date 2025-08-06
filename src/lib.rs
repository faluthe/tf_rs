#[used]
#[unsafe(link_section = ".init_array")]
static INIT: unsafe extern "C" fn() = {
    unsafe extern "C" fn init() {
        println!("tf_rs initializing...");
    }
    init
};

#[used]
#[unsafe(link_section = ".fini_array")]
static FINI: unsafe extern "C" fn() = {
    unsafe extern "C" fn fini() {
        println!("tf_rs restoring...")
    }
    fini
};
