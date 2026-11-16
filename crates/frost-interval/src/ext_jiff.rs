//! jiff 0.2 domains (feature `jiff`).
//!
//! | Type | Density | Word |
//! |---|---|---|
//! | `civil::Date` | discrete, 1 day | `u32` day number from `Date::MIN`, cover-preserving |
//! | `civil::Time` | discrete, 1 ns | `u64` nanosecond of day, cover-preserving |
//! | `Timestamp`, `civil::DateTime`, `SignedDuration` | discrete, 1 ns | identity (the interval exceeds `u64`) |
//!
//! `Zoned` is not covered: a civil interval in a time zone is not an order
//! encoding into instants (DST gaps and folds).

use crate::domain::{CoverPreserving, Discrete, DiscreteDomain, Domain, OrderedDomain};
use jiff::civil::{Date, DateTime, Time};
use jiff::{SignedDuration, Timestamp};

/// Days from 1970-01-01 in the proleptic Gregorian calendar.
const fn days_from_civil(y: i32, m: i32, d: i32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let (era, yoe) = (y.div_euclid(400), y.rem_euclid(400));
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
const fn civil_from_days(z: i32) -> (i32, i32, i32) {
    let z = z + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if m <= 2 {
            yoe + era * 400 + 1
        } else {
            yoe + era * 400
        },
        m,
        d,
    )
}
const FIRST_DAY: i32 = days_from_civil(-9999, 1, 1);

impl Domain for Date {}
impl OrderedDomain for Date {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some(Date::MIN);
    const MAX: Option<Self> = Some(Date::MAX);
    #[inline]
    #[allow(clippy::cast_sign_loss)]
    fn encode(&self) -> u32 {
        (days_from_civil(
            i32::from(self.year()),
            i32::from(self.month()),
            i32::from(self.day()),
        ) - FIRST_DAY) as u32
    }
    #[inline]
    fn decode(word: &u32) -> Option<Self> {
        let z = i32::try_from(*word).ok()?.checked_add(FIRST_DAY)?;
        let (y, m, d) = civil_from_days(z);
        Date::new(
            i16::try_from(y).ok()?,
            i8::try_from(m).ok()?,
            i8::try_from(d).ok()?,
        )
        .ok()
    }
}
impl DiscreteDomain for Date {
    #[inline]
    fn succ(&self) -> Option<Self> {
        self.tomorrow().ok()
    }
    #[inline]
    fn pred(&self) -> Option<Self> {
        self.yesterday().ok()
    }
}
impl CoverPreserving for Date {}

const NANOS_PER_DAY: u64 = 86_400_000_000_000;

impl Domain for Time {}
impl OrderedDomain for Time {
    type Density = Discrete;
    type Word = u64;
    const MIN: Option<Self> = Some(Time::MIN);
    const MAX: Option<Self> = Some(Time::MAX);
    #[inline]
    #[allow(clippy::cast_sign_loss)]
    fn encode(&self) -> u64 {
        let secs = (self.hour() as u64 * 60 + self.minute() as u64) * 60 + self.second() as u64;
        secs * 1_000_000_000 + self.subsec_nanosecond() as u64
    }
    #[inline]
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn decode(word: &u64) -> Option<Self> {
        let w = *word;
        if w >= NANOS_PER_DAY {
            return None;
        }
        let (secs, ns) = (w / 1_000_000_000, (w % 1_000_000_000) as i32);
        Time::new(
            (secs / 3600) as i8,
            (secs / 60 % 60) as i8,
            (secs % 60) as i8,
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

macro_rules! jiff_nanos {
    ($t:ty, |$v:ident| $succ:expr, |$w:ident| $pred:expr) => {
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
                let $v = *self;
                $succ
            }
            #[inline]
            fn pred(&self) -> Option<Self> {
                let $w = *self;
                $pred
            }
        }
    };
}
const NS: SignedDuration = SignedDuration::from_nanos(1);
jiff_nanos!(Timestamp, |v| v.checked_add(NS).ok(), |v| v
    .checked_sub(NS)
    .ok());
jiff_nanos!(DateTime, |v| v.checked_add(NS).ok(), |v| v
    .checked_sub(NS)
    .ok());
jiff_nanos!(SignedDuration, |v| v.checked_add(NS), |v| v.checked_sub(NS));
