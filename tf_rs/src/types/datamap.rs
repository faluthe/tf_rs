//! Prediction datamaps: the fields of an entity prediction saves and restores between commands.

use std::{collections::HashMap, ffi::c_void, sync::Mutex};

use once_cell::sync::Lazy;

const FIELD_EMBEDDED: i32 = 10;
const FIELD_FUNCTION: i32 = 21;
const TYPEDESCRIPTION_SIZE: usize = 96;

#[repr(C)]
struct DataMap {
    desc: *const u8,
    num_fields: i32,
    class_name: *const i8,
    base: *const DataMap,
}

/// Byte ranges of every field in the map, its base classes' and its embedded structs'
fn collect(map: *const DataMap, base_offset: usize, out: &mut Vec<(usize, usize)>, depth: u32) {
    let mut map = map;
    while !map.is_null() && depth < 8 {
        let m = unsafe { &*map };
        for i in 0..m.num_fields.max(0) as usize {
            unsafe {
                let td = m.desc.add(i * TYPEDESCRIPTION_SIZE);
                let field_type = *(td as *const i32);
                let offset = *(td.add(16) as *const i32);
                let embedded = *(td.add(64) as *const *const DataMap);
                let size = *(td.add(72) as *const i32);
                if offset < 0 {
                    continue;
                }
                let offset = base_offset + offset as usize;
                match field_type {
                    FIELD_EMBEDDED if !embedded.is_null() => {
                        collect(embedded, offset, out, depth + 1)
                    }
                    FIELD_FUNCTION | 0 => {}
                    _ if size > 0 => out.push((offset, size as usize)),
                    _ => {}
                }
            }
        }
        map = m.base;
    }
}

/// Merged, sorted byte ranges of a map, cached per map
fn ranges(map: *const c_void) -> Vec<(usize, usize)> {
    static CACHE: Lazy<Mutex<HashMap<usize, Vec<(usize, usize)>>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));
    let mut cache = CACHE.lock().unwrap();
    cache
        .entry(map as usize)
        .or_insert_with(|| {
            let mut fields = Vec::new();
            collect(map as *const DataMap, 0, &mut fields, 0);
            fields.sort();
            let mut merged: Vec<(usize, usize)> = Vec::new();
            for (offset, size) in fields {
                match merged.last_mut() {
                    Some((o, s)) if offset <= *o + *s => *s = (*s).max(offset + size - *o),
                    _ => merged.push((offset, size)),
                }
            }
            merged
        })
        .clone()
}

/// A copy of an entity's predicted fields, written back with `restore`
pub struct Snapshot {
    object: *mut u8,
    ranges: Vec<(usize, usize)>,
    bytes: Vec<u8>,
}

impl Snapshot {
    pub fn take(object: *mut c_void, map: *const c_void) -> Option<Self> {
        if object.is_null() || map.is_null() {
            return None;
        }
        let ranges = ranges(map);
        let object = object as *mut u8;
        let mut bytes = Vec::with_capacity(ranges.iter().map(|r| r.1).sum());
        for &(offset, size) in &ranges {
            bytes
                .extend_from_slice(unsafe { std::slice::from_raw_parts(object.add(offset), size) });
        }
        Some(Snapshot {
            object,
            ranges,
            bytes,
        })
    }

    pub fn restore(&self) {
        let mut at = 0;
        for &(offset, size) in &self.ranges {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.bytes.as_ptr().add(at),
                    self.object.add(offset),
                    size,
                )
            };
            at += size;
        }
    }
}
