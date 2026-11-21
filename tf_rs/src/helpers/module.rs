use std::ffi::{CStr, c_int, c_void};

use libc::{PF_W, PF_X, PT_LOAD, dl_iterate_phdr, dl_phdr_info, size_t};

/// A loaded segment of a shared library
#[derive(Clone, Copy)]
pub struct Segment {
    pub start: usize,
    pub len: usize,
    pub exec: bool,
    pub write: bool,
}

impl Segment {
    fn bytes(&self) -> &'static [u8] {
        unsafe { std::slice::from_raw_parts(self.start as *const u8, self.len) }
    }
}

/// Loaded segments of the library whose file name is `lib`
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
            // A whole file name, so client.so doesn't match steamclient.so
            let file = name.rsplit('/').next().unwrap_or(&name);
            if file != search.lib {
                return 0;
            }

            for i in 0..info.dlpi_phnum as usize {
                let phdr = &*info.dlpi_phdr.add(i);
                if phdr.p_type == PT_LOAD {
                    search.segments.push(Segment {
                        start: info.dlpi_addr as usize + phdr.p_vaddr as usize,
                        len: phdr.p_memsz as usize,
                        exec: phdr.p_flags & PF_X != 0,
                        write: phdr.p_flags & PF_W != 0,
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

/// Target of the `call rel32` at `addr`, if that's what is there
pub fn call_target(addr: usize) -> Option<usize> {
    let bytes = unsafe { std::slice::from_raw_parts(addr as *const u8, 5) };
    if bytes[0] != 0xE8 {
        return None;
    }
    let rel = i32::from_le_bytes(bytes[1..5].try_into().unwrap());
    Some((addr + 5).wrapping_add_signed(rel as isize))
}

/// Addresses of every 8 byte aligned pointer sized `value` in the segments
pub fn find_pointers(segments: &[Segment], value: usize, writable: bool) -> Vec<usize> {
    let needle = value.to_ne_bytes();
    segments
        .iter()
        .filter(|seg| !seg.exec && (!writable || seg.write))
        .flat_map(|seg| {
            let start = (seg.start + 7) & !7;
            let len = seg.len.saturating_sub(start - seg.start);
            let bytes = unsafe { std::slice::from_raw_parts(start as *const u8, len) };
            bytes
                .chunks_exact(8)
                .enumerate()
                .filter(move |(_, c)| *c == needle)
                .map(move |(i, _)| start + i * 8)
        })
        .collect()
}

/// Address of the first occurrence of `needle` in the writable segments
pub fn find_bytes_writable(segments: &[Segment], needle: &[u8]) -> Option<usize> {
    segments.iter().filter(|seg| seg.write).find_map(|seg| {
        seg.bytes()
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|i| seg.start + i)
    })
}

/// The vtable (address of its first function) of the class with the mangled RTTI name `name`,
/// e.g. "17CMoveHelperClient"
pub fn vtable_of(segments: &[Segment], name: &CStr) -> Option<usize> {
    // The name string, standing on its own rather than the end of a longer one
    let needle = name.to_bytes_with_nul();
    let name_addr = segments.iter().filter(|seg| !seg.exec).find_map(|seg| {
        let bytes = seg.bytes();
        bytes
            .windows(needle.len())
            .enumerate()
            .find(|(i, w)| *w == needle && (*i == 0 || bytes[i - 1] == 0))
            .map(|(i, _)| seg.start + i)
    })?;
    // type_info is { vtable, name }, and a vtable is { offset to top, type_info, functions... }
    find_pointers(segments, name_addr, false)
        .into_iter()
        .map(|p| p - 8)
        .flat_map(|type_info| find_pointers(segments, type_info, false))
        .find(|&slot| unsafe { *((slot - 8) as *const isize) } == 0)
        .map(|slot| slot + 8)
}

/// A static object of the class whose vtable is `vtable`
pub fn find_object(segments: &[Segment], vtable: usize) -> Option<usize> {
    find_pointers(segments, vtable, true).into_iter().next()
}
