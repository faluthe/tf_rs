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

#[macro_export]
macro_rules! struct_with_serialize {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $field_vis:vis $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $($field_vis $field : $ty),*
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                $(
                    let value_str = format!("{}", &self.$field);
                    let mut lines = value_str.lines();

                    if let Some(first) = lines.next() {
                        if lines.clone().next().is_none() {
                            writeln!(f, "{}: {}", stringify!($field), first)?;
                        } else {
                            writeln!(f, "{}:", stringify!($field))?;
                            writeln!(f, "    {}", first)?;
                            for line in lines {
                                writeln!(f, "    {}", line)?;
                            }
                        }
                    }
                )*
                Ok(())
            }
        }

        impl std::str::FromStr for $name
        where
            $name: Default,
            $(
                $ty: std::str::FromStr,
            )*
        {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let mut value = $name::default();

                let mut lines = s.lines().peekable();

                let indent = |l: &str| l.len() - l.trim_start().len();

                while let Some(raw) = lines.next() {
                    let line = raw.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let (key, rest) = match line.split_once(':') {
                        Some((k, r)) => (k.trim(), r),
                        None => continue, // Ignore malformed lines
                    };

                    match key {
                        $(
                            stringify!($field) => {
                                let rest_trimmed = rest.trim();

                                let field_text = if !rest_trimmed.is_empty() {
                                    rest_trimmed.to_string()
                                } else {
                                    // The lines indented deeper than this one, nested structs can
                                    // have fields named like ours
                                    let own_indent = indent(raw);
                                    let mut buf = String::new();
                                    let mut first = true;

                                    while let Some(&next_line) = lines.peek() {
                                        if !next_line.trim().is_empty()
                                            && indent(next_line) <= own_indent
                                        {
                                            break;
                                        }

                                        let consumed = lines.next().unwrap();
                                        if !first {
                                            buf.push('\n');
                                        }
                                        buf.push_str(consumed);
                                        first = false;
                                    }
                                    buf
                                };

                                value.$field = field_text
                                    .parse::<$ty>()
                                    .map_err(|e| format!(
                                        "Failed to parse field `{}` from `{}`: {:?}",
                                        stringify!($field),
                                        field_text,
                                        e
                                    ))?;

                            }
                        )*
                        // A field we no longer have, skip anything nested under it too
                        _ => {
                            let own_indent = indent(raw);
                            while lines.peek().is_some_and(|l| {
                                l.trim().is_empty() || indent(l) > own_indent
                            }) {
                                lines.next();
                            }
                        }
                    }
                }

                Ok(value)
            }
        }
    }
}
