#[macro_export]
macro_rules! vfunc {
    // With explicit ABI: vfunc!(vtable_expr, idx, extern "C" fn(args...) -> ret)
    ($vtable:expr, $idx:expr, extern $abi:literal fn($($arg:ty),*) -> $ret:ty) => {{
        // $vtable: *const *const _
        #[allow(clippy::macro_metavars_in_unsafe)]
        let slot = unsafe { *($vtable).add($idx) };
        let f: extern $abi fn($($arg),*) -> $ret = unsafe { core::mem::transmute(slot) };
        f
    }};
    // Shorthand that defaults to extern "C": vfunc!(vtable_expr, idx, (args...) -> ret)
    ($vtable:expr, $idx:expr, ($($arg:ty),*) -> $ret:ty) => {{
        vfunc!($vtable, $idx, extern "C" fn($($arg),*) -> $ret)
    }};
}

#[macro_export]
macro_rules! offset_get {
    // Usage: offset_get!(pub fn name: Type, offset);
    ($vis:vis fn $name:ident : $ty:ty, $off:expr) => {
        #[inline(always)]
        $vis fn $name(&self) -> $ty {
            #[allow(clippy::macro_metavars_in_unsafe)]
            unsafe {
                let p = (self.this as *const u8).add($off as usize) as *const $ty;
                core::ptr::read(p)
            }
        }
    };
}
