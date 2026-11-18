//! `Domain` impls for std types. Integer words match Frost's built-in
//! `IntKey` words (sign-flip for signed integers, `u32` for narrow types).

use crate::domain::{
    CoverPreserving, Dense, Discrete, DiscreteDomain, Domain, OrderedDomain, Ranked,
};
use alloc::string::String;
use core::net::Ipv4Addr;
use core::num::NonZero;

/// `u32` and `u64` are their own words.
macro_rules! word_domain {
    ($($t:ty),*) => {$(
        impl Domain for $t {}
        impl OrderedDomain for $t {
            type Density = Discrete;
            const MIN: Option<Self> = Some(<$t>::MIN);
            const MAX: Option<Self> = Some(<$t>::MAX);
            crate::identity_encoding!();
        }
        impl DiscreteDomain for $t {
            #[inline] fn succ(&self) -> Option<Self> { self.checked_add(1) }
            #[inline] fn pred(&self) -> Option<Self> { self.checked_sub(1) }
        }
        impl CoverPreserving for $t {}
    )*};
}
word_domain!(u32, u64);

/// Integers that encode into a word.
macro_rules! int_domain {
    ($t:ty => $w:ty, |$e:ident| $encode:expr, |$d:ident| $pre:expr) => {
        impl Domain for $t {}
        impl OrderedDomain for $t {
            type Density = Discrete;
            type Word = $w;
            const MIN: Option<Self> = Some(<$t>::MIN);
            const MAX: Option<Self> = Some(<$t>::MAX);
            #[inline]
            #[allow(
                clippy::cast_sign_loss,
                clippy::cast_possible_wrap,
                clippy::cast_lossless
            )]
            fn encode(&self) -> $w {
                let $e = *self;
                $encode
            }
            #[inline]
            #[allow(
                clippy::cast_sign_loss,
                clippy::cast_possible_wrap,
                clippy::cast_possible_truncation
            )]
            fn decode(word: &$w) -> Option<Self> {
                let $d = *word;
                $pre
            }
        }
        impl DiscreteDomain for $t {
            #[inline]
            fn succ(&self) -> Option<Self> {
                self.checked_add(1)
            }
            #[inline]
            fn pred(&self) -> Option<Self> {
                self.checked_sub(1)
            }
        }
        impl CoverPreserving for $t {}
    };
}

int_domain!(u8 => u32, |v| u32::from(v), |w| u8::try_from(w).ok());
int_domain!(u16 => u32, |v| u32::from(v), |w| u16::try_from(w).ok());
int_domain!(i8 => u32, |v| u32::from((v as u8) ^ 0x80), |w| u8::try_from(w).ok().map(|n| (n ^ 0x80) as i8));
int_domain!(i16 => u32, |v| u32::from((v as u16) ^ 0x8000), |w| u16::try_from(w).ok().map(|n| (n ^ 0x8000) as i16));
int_domain!(i32 => u32, |v| (v as u32) ^ (1 << 31), |w| Some((w ^ (1 << 31)) as i32));
int_domain!(i64 => u64, |v| (v as u64) ^ (1 << 63), |w| Some((w ^ (1 << 63)) as i64));
// `usize`/`isize` encode into `u64`; lossless on targets up to 64 bits.
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
int_domain!(usize => u64, |v| v as u64, |w| usize::try_from(w).ok());
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
int_domain!(isize => u64, |v| (v as i64 as u64) ^ (1 << 63), |w| isize::try_from((w ^ (1 << 63)) as i64).ok());

/// 128-bit integers are discrete and bounded, with the identity encoding
/// (16 bytes per boundary, the size of the value). They have no `rank`: the
/// count of all values does not fit `u128`. Arbitrary-precision integers take
/// the same shape with no `MIN`/`MAX`.
macro_rules! wide_int_domain {
    ($($t:ty),*) => {$(
        impl Domain for $t {}
        impl OrderedDomain for $t {
            type Density = Discrete;
            const MIN: Option<Self> = Some(<$t>::MIN);
            const MAX: Option<Self> = Some(<$t>::MAX);
            crate::identity_encoding!();
        }
        impl DiscreteDomain for $t {
            #[inline] fn succ(&self) -> Option<Self> { self.checked_add(1) }
            #[inline] fn pred(&self) -> Option<Self> { self.checked_sub(1) }
        }
    )*};
}
wide_int_domain!(u128, i128);

/// Unsigned `NonZero` integers: cover-preserving, words start at 1.
macro_rules! nonzero_domain {
    ($($p:ty => $w:ty),*) => {$(
        impl Domain for NonZero<$p> {}
        impl OrderedDomain for NonZero<$p> {
            type Density = Discrete;
            type Word = $w;
            const MIN: Option<Self> = Some(NonZero::<$p>::MIN);
            const MAX: Option<Self> = Some(NonZero::<$p>::MAX);
            #[inline]
            #[allow(clippy::cast_lossless)]
            fn encode(&self) -> $w { self.get() as $w }
            #[inline]
            fn decode(word: &$w) -> Option<Self> { <$p>::try_from(*word).ok().and_then(NonZero::new) }
        }
        impl DiscreteDomain for NonZero<$p> {
            #[inline] fn succ(&self) -> Option<Self> { self.checked_add(1) }
            #[inline] fn pred(&self) -> Option<Self> { NonZero::new(self.get() - 1) }
        }
        impl CoverPreserving for NonZero<$p> {}
    )*};
}
nonzero_domain!(u8 => u32, u16 => u32, u32 => u32, u64 => u64);

/// `char` is discrete with a gap (surrogates). Its encoding is not
/// cover-preserving, so `rank` is written by hand.
impl Domain for char {}
impl OrderedDomain for char {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some('\0');
    const MAX: Option<Self> = Some(char::MAX);
    #[inline]
    fn encode(&self) -> u32 {
        u32::from(*self)
    }
    #[inline]
    fn decode(word: &u32) -> Option<Self> {
        char::from_u32(*word)
    }
}
impl DiscreteDomain for char {
    fn succ(&self) -> Option<Self> {
        match *self {
            '\u{D7FF}' => Some('\u{E000}'),
            c => char::from_u32(c as u32 + 1),
        }
    }
    fn pred(&self) -> Option<Self> {
        match *self {
            '\u{E000}' => Some('\u{D7FF}'),
            c => (c as u32).checked_sub(1).and_then(char::from_u32),
        }
    }
}
impl Ranked for char {
    fn rank(&self) -> u128 {
        let c = u128::from(u32::from(*self));
        if c > 0xD7FF { c - 0x800 } else { c }
    }
}

/// IPv4 addresses encode into `u32`.
impl Domain for Ipv4Addr {}
impl OrderedDomain for Ipv4Addr {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some(Ipv4Addr::UNSPECIFIED);
    const MAX: Option<Self> = Some(Ipv4Addr::BROADCAST);
    #[inline]
    fn encode(&self) -> u32 {
        u32::from(*self)
    }
    #[inline]
    fn decode(word: &u32) -> Option<Self> {
        Some(Ipv4Addr::from(*word))
    }
}
impl DiscreteDomain for Ipv4Addr {
    fn succ(&self) -> Option<Self> {
        u32::from(*self).checked_add(1).map(Ipv4Addr::from)
    }
    fn pred(&self) -> Option<Self> {
        u32::from(*self).checked_sub(1).map(Ipv4Addr::from)
    }
}
impl CoverPreserving for Ipv4Addr {}

/// `String` is dense with a minimum, `""`, and no maximum. Identity encoding:
/// a probe is compared by reference and never cloned.
impl Domain for String {}
impl OrderedDomain for String {
    type Density = Dense;
    const MIN: Option<Self> = Some(String::new());
    crate::identity_encoding!();
}
