use std::{ffi::c_void, fs, ptr};

use anyhow::{Result, anyhow};

use crate::hooks::FnSig;

/// Itanium ABI stores offset-to-top at [-2] and the RTTI pointer at [-1].
/// Copy them so dynamic_cast/typeid on the object keep working.
const PREFIX: usize = 2;
const MAX_ENTRIES: usize = 1024;

/// Replaces an object's vptr with a pointer to our own copy of its vtable.
/// The game's vtable is never written to.
pub struct VmtShadow {
    object: *mut *mut *mut c_void,
    original_vtable: *mut *mut c_void,
    shadow: Box<[*mut c_void]>,
    originals: Vec<FnSig>,
    installed: bool,
}

impl Default for VmtShadow {
    fn default() -> Self {
        VmtShadow {
            object: ptr::null_mut(),
            original_vtable: ptr::null_mut(),
            shadow: Box::new([]),
            originals: Vec::new(),
            installed: false,
        }
    }
}

impl VmtShadow {
    pub fn new(object: *mut c_void) -> Result<Self> {
        if object.is_null() {
            return Err(anyhow!("Object is null"));
        }

        let object = object as *mut *mut *mut c_void;

        unsafe {
            let original_vtable = *object;
            let len = vtable_len(original_vtable)?;

            let mut shadow = vec![ptr::null_mut(); PREFIX + len].into_boxed_slice();
            ptr::copy_nonoverlapping(
                original_vtable.sub(PREFIX),
                shadow.as_mut_ptr(),
                PREFIX + len,
            );

            log::debug!("Shadowing vtable {original_vtable:p} ({len} entries) of {object:p}");

            Ok(VmtShadow {
                object,
                original_vtable,
                shadow,
                originals: vec![FnSig::None; len],
                installed: false,
            })
        }
    }

    pub fn hook(&mut self, index: usize, hook: FnSig) -> Result<()> {
        let len = self.originals.len();

        if index >= len {
            return Err(anyhow!(
                "Index {index} out of bounds for vtable of {len} entries"
            ));
        }

        unsafe {
            self.originals[index] = FnSig::from_ptr(*self.original_vtable.add(index), hook);
        }
        self.shadow[PREFIX + index] = hook.as_ptr()?;

        log::debug!(
            "Vtable {:p} index {index}: original {:p}, hook {hook:p}",
            self.original_vtable,
            self.originals[index]
        );

        Ok(())
    }

    pub fn install(&mut self) -> Result<()> {
        if self.object.is_null() {
            return Err(anyhow!("VmtShadow not initialized"));
        }

        unsafe {
            // Single aligned 8 byte store, callers see either the old or new table
            self.object.write(self.shadow.as_mut_ptr().add(PREFIX));
        }
        self.installed = true;

        Ok(())
    }

    pub fn restore(&mut self) -> Result<()> {
        // Never hooked
        if !self.installed {
            return Ok(());
        }

        unsafe {
            if *self.object != self.shadow.as_mut_ptr().add(PREFIX) {
                return Err(anyhow!(
                    "vptr of {:p} was changed, not restoring",
                    self.object
                ));
            }

            self.object.write(self.original_vtable);
        }
        self.installed = false;

        Ok(())
    }

    pub fn object(&self) -> *mut c_void {
        self.object as *mut c_void
    }

    pub fn is_installed(&self) -> bool {
        self.installed
    }

    /// Installed and the object still points at our table, the object must still be alive
    pub fn is_active(&self) -> bool {
        self.installed && unsafe { *self.object == self.shadow.as_ptr().add(PREFIX) as *mut _ }
    }

    pub fn original(&self, index: usize) -> FnSig {
        self.originals.get(index).copied().unwrap_or(FnSig::None)
    }
}

/// Count entries until one doesn't point into executable memory. The next
/// vtable's offset-to-top (usually 0) or RTTI pointer ends the count.
fn vtable_len(vtable: *mut *mut c_void) -> Result<usize> {
    let exec = executable_ranges()?;
    let is_code = |addr: usize| exec.iter().any(|&(start, end)| addr >= start && addr < end);

    let len = (0..MAX_ENTRIES)
        .take_while(|&i| is_code(unsafe { *vtable.add(i) } as usize))
        .count();

    if len == 0 {
        return Err(anyhow!("No entries found in vtable {vtable:p}"));
    }

    Ok(len)
}

fn executable_ranges() -> Result<Vec<(usize, usize)>> {
    let maps = fs::read_to_string("/proc/self/maps")?;

    Ok(maps
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let range = fields.next()?;
            let perms = fields.next()?;

            if !perms.contains('x') {
                return None;
            }

            let (start, end) = range.split_once('-')?;
            Some((
                usize::from_str_radix(start, 16).ok()?,
                usize::from_str_radix(end, 16).ok()?,
            ))
        })
        .collect())
}
