//! time 0.3 domains (feature `time`).
//!
//! | Type | Density | Word |
//! |---|---|---|
//! | `Date` | discrete, 1 day | `u32` Julian day, cover-preserving |
//! | `Time` | discrete, 1 ns | `u64` nanosecond of day, cover-preserving |
//! | `PrimitiveDateTime`, `UtcDateTime`, `Duration` | discrete, 1 ns | identity |
//!
//! `OffsetDateTime` is not covered: values with different offsets compare
//! equal, so one instant has several representations.

use crate::domain::{CoverPreserving, Discrete, DiscreteDomain, Domain, OrderedDomain};
use time::{Date, Duration, PrimitiveDateTime, Time, UtcDateTime};

const SIGN: u32 = 1 << 31;

impl Domain for Date {}
impl OrderedDomain for Date {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some(Date::MIN);
    const MAX: Option<Self> = Some(Date::MAX);
    #[inline]
    #[allow(clippy::cast_sign_loss)]
    fn encode(&self) -> u32 {
        (self.to_julian_day() as u32) ^ SIGN
    }
    #[inline]
    #[allow(clippy::cast_possible_wrap)]
    fn decode(word: &u32) -> Option<Self> {
        Date::from_julian_day((*word ^ SIGN) as i32).ok()
    }
}
impl DiscreteDomain for Date {
    #[inline]
    fn succ(&self) -> Option<Self> {
        self.next_day()
    }
    #[inline]
    fn pred(&self) -> Option<Self> {
        self.previous_day()
    }
}
impl CoverPreserving for Date {}

impl Domain for Time {}
impl OrderedDomain for Time {
    type Density = Discrete;
    type Word = u64;
    const MIN: Option<Self> = Some(Time::MIDNIGHT);
    const MAX: Option<Self> = Some(Time::MAX);
    #[inline]
    fn encode(&self) -> u64 {
        let (h, m, s, ns) = self.as_hms_nano();
        ((u64::from(h) * 60 + u64::from(m)) * 60 + u64::from(s)) * 1_000_000_000 + u64::from(ns)
    }
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn decode(word: &u64) -> Option<Self> {
        let w = *word;
        if w >= 86_400_000_000_000 {
            return None;
        }
        let (secs, ns) = (w / 1_000_000_000, (w % 1_000_000_000) as u32);
        Time::from_hms_nano(
            (secs / 3600) as u8,
            (secs / 60 % 60) as u8,
            (secs % 60) as u8,
            ns,
        )
        .ok()
    }
}
impl DiscreteDomain for Time {
    #[inline]
    fn succ(&self) -> Option<Self> {
        Self::decode(&(self.encode() + 1))
    }
    #[inline]
    fn pred(&self) -> Option<Self> {
        self.encode().checked_sub(1).and_then(|w| Self::decode(&w))
    }
}
impl CoverPreserving for Time {}

macro_rules! time_nanos {
    ($($t:ty),*) => {$(
        impl Domain for $t {}
        impl OrderedDomain for $t {
            type Density = Discrete;
            const MIN: Option<Self> = Some(<$t>::MIN);
            const MAX: Option<Self> = Some(<$t>::MAX);
            crate::identity_encoding!();
        }
        impl DiscreteDomain for $t {
            #[inline]
            fn succ(&self) -> Option<Self> {
                self.checked_add(Duration::NANOSECOND)
            }
            #[inline]
            fn pred(&self) -> Option<Self> {
                self.checked_sub(Duration::NANOSECOND)
            }
        }
    )*};
}
time_nanos!(PrimitiveDateTime, UtcDateTime, Duration);
