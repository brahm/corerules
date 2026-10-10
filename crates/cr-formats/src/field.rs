//! The [`Field`] trait: one value as `Serialize` reads and writes it, and
//! the [`record!`](crate::record) macro that builds a class from its fields
//! in archive order.

use crate::{ArchiveReader, ArchiveWriter, Result};

/// A value with a fixed archive encoding.
///
/// * Integers are little-endian; `u8` is a `BYTE`, `bool` a `BYTE` that the
///   program reads with `!= 0` (written back as stored, so non-0/1 values
///   would not survive; the original never writes them).
/// * `String` is a `CString`.
/// * `Vec<T>` is an MFC collection: `WriteCount`, then each element. This
///   covers `CStringArray`, `CDWordArray`, `CByteArray`, `CObArray` (with
///   `T` = [`Object`](crate::Object)) and `CList`s serialized element by
///   element.
/// * `[T; N]` is `N` values with no count, the way a C array member is
///   written in a loop.
pub trait Field: Sized {
    fn read(r: &mut ArchiveReader) -> Result<Self>;
    fn write(&self, w: &mut ArchiveWriter) -> Result<()>;
}

impl Field for u8 {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        r.read_u8()
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_u8(*self);
        Ok(())
    }
}

impl Field for u32 {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        r.read_u32()
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_u32(*self);
        Ok(())
    }
}

impl Field for i32 {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        r.read_i32()
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_i32(*self);
        Ok(())
    }
}

impl Field for String {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        r.read_string()
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_string(self)
    }
}

impl<T: Field> Field for Vec<T> {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let count = r.read_count()?;
        // Cap the preallocation: a corrupt count must fail on EOF, not abort
        // on allocation.
        let mut items = Vec::with_capacity(count.min(1024) as usize);
        for _ in 0..count {
            items.push(T::read(r)?);
        }
        Ok(items)
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        w.write_count(self.len() as u32);
        self.iter().try_for_each(|item| item.write(w))
    }
}

impl<T: Field, const N: usize> Field for [T; N] {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        let mut items = Vec::with_capacity(N);
        for _ in 0..N {
            items.push(T::read(r)?);
        }
        match items.try_into() {
            Ok(array) => Ok(array),
            Err(_) => unreachable!("exactly N items were read"),
        }
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        self.iter().try_for_each(|item| item.write(w))
    }
}

impl<T: Field> Field for Box<T> {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        T::read(r).map(Box::new)
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        (**self).write(w)
    }
}

impl<A: Field, B: Field> Field for (A, B) {
    fn read(r: &mut ArchiveReader) -> Result<Self> {
        Ok((A::read(r)?, B::read(r)?))
    }
    fn write(&self, w: &mut ArchiveWriter) -> Result<()> {
        self.0.write(w)?;
        self.1.write(w)
    }
}

/// A field that exists only in some file versions, declared as
/// `Option<T>`. A value present in the struct but absent from the version's
/// layout (or the reverse) is an error on write rather than silently
/// dropped.
pub trait Gated: Sized {
    fn read_if(r: &mut ArchiveReader, present: bool) -> Result<Self>;
    fn write_if(&self, w: &mut ArchiveWriter, present: bool, field: &'static str) -> Result<()>;
}

impl<T: Field> Gated for Option<T> {
    fn read_if(r: &mut ArchiveReader, present: bool) -> Result<Self> {
        present.then(|| T::read(r)).transpose()
    }
    fn write_if(&self, w: &mut ArchiveWriter, present: bool, field: &'static str) -> Result<()> {
        match (present, self) {
            (true, Some(v)) => v.write(w),
            (false, None) => Ok(()),
            (expected, _) => Err(crate::Error::FieldMismatch { field, expected }),
        }
    }
}

/// Declares a struct whose fields are read and written in declaration
/// order.
///
/// Fields that exist only in some versions of a file are declared as
/// `name: Option<T> = if condition`; the condition may use the version
/// variable named after the struct (`(v)`), which `read_fields` and
/// `write_fields` receive from the caller.
#[macro_export]
macro_rules! record {
    (
        $(#[$meta:meta])*
        pub struct $name:ident ($v:ident) {
            $(
                $(#[$fmeta:meta])*
                $field:ident : $ty:ty $(= if $cond:expr)?
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $( $(#[$fmeta])* pub $field: $ty, )*
        }

        impl $name {
            #[allow(unused_variables)]
            pub fn read_fields(r: &mut $crate::ArchiveReader, $v: i32) -> $crate::Result<Self> {
                $( let $field = $crate::record!(@read r, $ty $(, $cond)?); )*
                Ok(Self { $($field),* })
            }

            #[allow(unused_variables)]
            pub fn write_fields(&self, w: &mut $crate::ArchiveWriter, $v: i32) -> $crate::Result<()> {
                $( $crate::record!(@write w, self.$field, stringify!($field) $(, $cond)?); )*
                Ok(())
            }
        }
    };
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$fmeta:meta])*
                $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $( $(#[$fmeta])* pub $field: $ty, )*
        }

        impl $crate::Field for $name {
            fn read(r: &mut $crate::ArchiveReader) -> $crate::Result<Self> {
                $( let $field = <$ty as $crate::Field>::read(r)?; )*
                Ok(Self { $($field),* })
            }

            fn write(&self, w: &mut $crate::ArchiveWriter) -> $crate::Result<()> {
                $( $crate::Field::write(&self.$field, w)?; )*
                Ok(())
            }
        }
    };
    (@read $r:ident, $ty:ty) => { <$ty as $crate::Field>::read($r)? };
    (@read $r:ident, $ty:ty, $cond:expr) => {
        <$ty as $crate::field::Gated>::read_if($r, $cond)?
    };
    (@write $w:ident, $value:expr, $fname:expr) => { $crate::Field::write(&$value, $w)? };
    (@write $w:ident, $value:expr, $fname:expr, $cond:expr) => {
        $crate::field::Gated::write_if(&$value, $w, $cond, $fname)?
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    record! {
        pub struct Sample {
            name: String,
            pair: [i32; 2],
            tags: Vec<String>,
        }
    }

    record! {
        pub struct Versioned(v) {
            always: u32,
            late: Option<u8> = if v > 3,
        }
    }

    #[test]
    fn plain_record_round_trips() {
        let s = Sample {
            name: "Fighter".into(),
            pair: [3, -1],
            tags: vec!["a".into(), "b".into()],
        };
        let mut w = ArchiveWriter::new();
        s.write(&mut w).unwrap();
        let bytes = w.into_bytes();
        assert_eq!(bytes.len(), 8 + 8 + 2 + 4);
        let mut r = ArchiveReader::new(&bytes);
        assert_eq!(Sample::read(&mut r).unwrap(), s);
        assert!(r.is_at_end());
    }

    #[test]
    fn gated_fields_follow_the_version() {
        let bytes = [1, 0, 0, 0, 9];
        let old = Versioned::read_fields(&mut ArchiveReader::new(&bytes), 3).unwrap();
        assert_eq!(old.late, None);
        let new = Versioned::read_fields(&mut ArchiveReader::new(&bytes), 4).unwrap();
        assert_eq!(new.late, Some(9));

        let mut w = ArchiveWriter::new();
        assert!(new.write_fields(&mut w, 3).is_err());
    }
}
