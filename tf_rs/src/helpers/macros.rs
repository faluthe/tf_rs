#[macro_export]
macro_rules! vfunc {
    // With explicit ABI: vfunc!(vtable_expr, idx, extern "C" fn(args...) -> ret)
    ($vtable:expr, $idx:expr, extern $abi:literal fn($($arg:ty),*) -> $ret:ty) => {{
        // $vtable: *const *const _
        let slot = unsafe { *($vtable).add($idx) };
        let f: extern $abi fn($($arg),*) -> $ret = unsafe { core::mem::transmute(slot) };
        f
    }};
    // Shorthand that defaults to extern "C": vfunc!(vtable_expr, idx, (args...) -> ret)
    ($vtable:expr, $idx:expr, ($($arg:ty),*) -> $ret:ty) => {{
        vfunc!($vtable, $idx, extern "C" fn($($arg),*) -> $ret)
    }};
}
