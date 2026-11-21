use std::ffi::{CStr, c_int, c_void};

use libc::{PF_X, PT_LOAD, dl_iterate_phdr, dl_phdr_info, size_t};

/// A loaded segment of a shared library
#[derive(Clone, Copy)]
pub struct Segment {
    pub start: usize,
    pub len: usize,
    pub exec: bool,
}

impl Segment {
    fn bytes(&self) -> &'static [u8] {
        unsafe { std::slice::from_raw_parts(self.start as *const u8, self.len) }
    }
}

/// Loaded segments of the library whose path ends with `lib`
pub fn segments(lib: &str) -> Vec<Segment> {
    struct Search<'a> {
        lib: &'a str,
        segments: Vec<Segment>,
    }

    unsafe extern "C" fn callback(info: *mut dl_phdr_info, _: size_t, data: *mut c_void) -> c_int {
        unsafe {
            let search = &mut *(data as *mut Search);
            let info = &*info;
            if info.dlpi_name.is_null() {
                return 0;
            }

            let name = CStr::from_ptr(info.dlpi_name).to_string_lossy();
            if !name.ends_with(search.lib) {
                return 0;
            }

            for i in 0..info.dlpi_phnum as usize {
                let phdr = &*info.dlpi_phdr.add(i);
                if phdr.p_type == PT_LOAD {
                    search.segments.push(Segment {
                        start: info.dlpi_addr as usize + phdr.p_vaddr as usize,
                        len: phdr.p_memsz as usize,
                        exec: phdr.p_flags & PF_X != 0,
                    });
                }
            }
            1
        }
    }

    let mut search = Search {
        lib,
        segments: Vec::new(),
    };
    unsafe { dl_iterate_phdr(Some(callback), &mut search as *mut Search as *mut c_void) };
    search.segments
}

/// Address of a null terminated string in any of the segments
pub fn find_string(segments: &[Segment], s: &CStr) -> Option<usize> {
    let needle = s.to_bytes_with_nul();
    segments.iter().find_map(|seg| {
        seg.bytes()
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|i| seg.start + i)
    })
}

/// Address of the first RIP relative `lea reg, [rip+disp32]` that points at `target`
pub fn find_lea_to(segments: &[Segment], target: usize) -> Option<usize> {
    segments.iter().filter(|seg| seg.exec).find_map(|seg| {
        let bytes = seg.bytes();
        (0..bytes.len().saturating_sub(7)).find_map(|i| {
            let rex = bytes[i];
            let is_lea =
                (rex == 0x48 || rex == 0x4C) && bytes[i + 1] == 0x8D && bytes[i + 2] & 0xC7 == 0x05;
            if !is_lea {
                return None;
            }
            let disp = i32::from_le_bytes(bytes[i + 3..i + 7].try_into().unwrap());
            let next = seg.start + i + 7;
            (next.wrapping_add_signed(disp as isize) == target).then_some(seg.start + i)
        })
    })
}

/// Start of the function containing `addr`, found by walking back to the closest
/// 16 byte aligned `push rbp; mov rbp, rsp` prologue
pub fn function_start(segments: &[Segment], addr: usize) -> Option<usize> {
    const PROLOGUE: [u8; 4] = [0x55, 0x48, 0x89, 0xE5];
    const MAX_DISTANCE: usize = 0x1000;

    let seg = segments
        .iter()
        .find(|s| s.exec && (s.start..s.start + s.len).contains(&addr))?;
    let lowest = addr.saturating_sub(MAX_DISTANCE).max(seg.start);

    let mut candidate = addr & !0xF;
    while candidate >= lowest {
        let bytes = unsafe { std::slice::from_raw_parts(candidate as *const u8, 4) };
        if bytes == PROLOGUE {
            return Some(candidate);
        }
        candidate = candidate.checked_sub(0x10)?;
    }
    None
}
