//! Utilities to cast between integers.

/// A trait for safe and infallible casts.
/// 
/// You can easily cast between integers using the `as` keywords, but these
/// casts aren't always says, e.g. using `as` you can cast a `u64` to a `u32`
/// but you'll lose precision.
/// 
/// This trait facilitates casts that are always known to be safe.
pub trait CastFrom<T> {
    fn cast_from(from: T) -> Self;
}


macro_rules! cast_from {
    ($from:ty, $to:ty) => {
        paste::paste! {
            impl crate::cast::CastFrom<$from> for $to {
                #[allow(clippy::as_conversions)]
                fn cast_from(from: $from) -> $to {
                    from as $to
                }
            }

            /// Casts [`$from`] to [`$to`].
            ///
            /// This is equivalent to the [`crate::cast::CastFrom`] implementation but is
            /// available as a `const fn`.
            #[allow(clippy::as_conversions)]
            pub const fn [< $from _to_ $to >](from: $from) -> $to {
                from as $to
            }
        }
    };
}


#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
mod target32 {
    cast_from!(u8, u8);
    cast_from!(u8, u16);
    cast_from!(u8, u32);
    cast_from!(u8, u64);
    cast_from!(u8, usize);
    cast_from!(i8, i8);
    cast_from!(i8, i16);
    cast_from!(i8, i32);
    cast_from!(i8, i64);
    cast_from!(i8, isize);

    cast_from!(u16, u16);
    cast_from!(u16, u32);
    cast_from!(u16, u64);
    cast_from!(u16, usize);
    cast_from!(i16, i16);
    cast_from!(i16, i32);
    cast_from!(i16, i64);
    cast_from!(i16, isize);

    cast_from!(u32, u32);
    cast_from!(u32, u64);
    cast_from!(u32, usize);
    cast_from!(i32, i32);
    cast_from!(i32, i64);
    cast_from!(i32, isize);

    cast_from!(u64, u64);
    cast_from!(i64, i64);
}
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub use target32::*;


// Casts that are safe on 64-bit architectures.
#[cfg(target_pointer_width = "64")]
mod target64 {
    cast_from!(u64, usize);
    cast_from!(i64, isize);
}
#[cfg(target_pointer_width = "64")]
pub use target64::*;
