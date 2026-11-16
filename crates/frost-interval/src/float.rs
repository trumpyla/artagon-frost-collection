//! `NotNanF32` and `NotNanF64`: floats as totally ordered domains with machine words.
//!
//! - `#[repr(transparent)]` over the primitive; no NaN; `-0.0` is stored as
//!   `+0.0`, so each number has one representation.
//! - The encoding is offset binary: `SIGN + magnitude` for `x >= 0` and
//!   `SIGN - magnitude` for `x < 0`. It is an order embedding into `u32`/`u64`
//!   and it is cover-preserving on `[-inf, +inf]`. Sets store the words and
//!   search them with integer compares.
//! - The domain is discrete: `succ` is the next representable float. So
//!   `[0.0, 1.0)` equals `[0.0, 1.0f64.next_down()]`, and `count()` is the exact
//!   number of representable values. These are sets of floats, not of reals.

use crate::domain::{
    CoverPreserving, Discrete, DiscreteDomain, Domain, OrderedDomain, OrderedDomainExt,
};
use crate::interval::Interval;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{Bound, RangeBounds};

/// The input was NaN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NanError;

macro_rules! float_domain {
    ($name:ident, $f:ty, $w:ty, $i:ty, $sign:expr, $inf:expr) => {
        #[doc = concat!("A `", stringify!($f), "` that is not NaN, with `-0.0` stored as `+0.0`.")]
        #[derive(Clone, Copy, Debug, Default)]
        #[repr(transparent)]
        pub struct $name($f);

        impl $name {
            const SIGN: $w = $sign;
            const INF_BITS: $w = $inf;
            /// Negative infinity.
            pub const NEG_INFINITY: Self = Self(<$f>::NEG_INFINITY);
            /// Positive infinity.
            pub const INFINITY: Self = Self(<$f>::INFINITY);
            /// Zero. Negative zero is stored as this value.
            pub const ZERO: Self = Self(0.0);

            /// `None` for NaN.
            #[inline]
            pub const fn new(x: $f) -> Option<Self> {
                if x.is_nan() {
                    None
                } else {
                    Some(Self(x + 0.0))
                }
            }
            #[inline]
            /// The float value.
            pub const fn get(self) -> $f {
                self.0
            }
            #[inline]
            #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
            const fn word(self) -> $w {
                // Branch-free: SIGN + m for x >= 0, SIGN - m for x < 0.
                let bits = self.0.to_bits();
                let neg = ((bits as $i) >> (<$w>::BITS - 1)) as $w; // all ones if negative
                let mag = bits & !Self::SIGN;
                Self::SIGN.wrapping_add((mag ^ neg).wrapping_sub(neg))
            }
            /// An interval from primitive bounds; `None` if a bound is NaN.
            pub fn interval<R: RangeBounds<$f>>(r: R) -> Option<Interval<Self>> {
                let conv = |b: Bound<&$f>| -> Option<Bound<Self>> {
                    Some(match b {
                        Bound::Included(&x) => Bound::Included(Self::new(x)?),
                        Bound::Excluded(&x) => Bound::Excluded(Self::new(x)?),
                        Bound::Unbounded => Bound::Unbounded,
                    })
                };
                Some(Self::from_bounds((
                    conv(r.start_bound())?,
                    conv(r.end_bound())?,
                )))
            }
        }

        impl PartialEq for $name {
            #[inline]
            fn eq(&self, o: &Self) -> bool {
                self.word() == o.word()
            }
        }
        impl Eq for $name {}
        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
                Some(self.cmp(o))
            }
        }
        impl Ord for $name {
            #[inline]
            fn cmp(&self, o: &Self) -> Ordering {
                self.word().cmp(&o.word())
            }
        }
        impl Hash for $name {
            fn hash<H: Hasher>(&self, h: &mut H) {
                self.word().hash(h);
            }
        }
        impl TryFrom<$f> for $name {
            type Error = NanError;
            fn try_from(x: $f) -> Result<Self, NanError> {
                Self::new(x).ok_or(NanError)
            }
        }
        impl From<$name> for $f {
            fn from(x: $name) -> $f {
                x.0
            }
        }

        impl $name {
            #[inline]
            fn from_word(w: $w) -> Option<Self> {
                let (neg, mag) = if w >= Self::SIGN {
                    (0, w - Self::SIGN)
                } else {
                    (Self::SIGN, Self::SIGN - w)
                };
                (mag <= Self::INF_BITS).then(|| Self(<$f>::from_bits(neg | mag)))
            }
        }
        impl Domain for $name {}
        impl OrderedDomain for $name {
            type Density = Discrete;
            type Word = $w;
            const MIN: Option<Self> = Some(Self::NEG_INFINITY);
            const MAX: Option<Self> = Some(Self::INFINITY);
            #[inline]
            fn encode(&self) -> $w {
                self.word()
            }
            #[inline]
            fn decode(word: &$w) -> Option<Self> {
                Self::from_word(*word)
            }
        }
        impl DiscreteDomain for $name {
            #[inline]
            fn succ(&self) -> Option<Self> {
                Self::from_word(self.word() + 1)
            }
            #[inline]
            fn pred(&self) -> Option<Self> {
                Self::from_word(self.word() - 1)
            }
        }
        impl CoverPreserving for $name {}
    };
}

float_domain!(NotNanF32, f32, u32, i32, 1 << 31, 0x7F80_0000);
float_domain!(NotNanF64, f64, u64, i64, 1 << 63, 0x7FF0_0000_0000_0000);
