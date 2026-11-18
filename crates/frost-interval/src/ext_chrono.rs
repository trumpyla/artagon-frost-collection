//! chrono 0.4 domains (feature `chrono`).
//!
//! | Type | Density | Word |
//! |---|---|---|
//! | `NaiveDate` | discrete, 1 day | `u32` day number, cover-preserving |
//! | `NaiveTime`, `NaiveDateTime`, `DateTime<Utc>` | dense | identity |
//!
//! chrono represents a leap second as a nanosecond field of
//! `1_000_000_000..2_000_000_000` in second 59 of any minute, and its
//! arithmetic skips those values. So `+ 1 ns` is not a lawful `succ`, and the time-of-day types are
//! declared dense. Their true maximum is the leap-second form of the last
//! second, which is above `NaiveTime`'s own (private) `MAX`.

use crate::domain::{CoverPreserving, Dense, Discrete, DiscreteDomain, Domain, OrderedDomain};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

const SIGN: u32 = 1 << 31;

impl Domain for NaiveDate {}
impl OrderedDomain for NaiveDate {
    type Density = Discrete;
    type Word = u32;
    const MIN: Option<Self> = Some(NaiveDate::MIN);
    const MAX: Option<Self> = Some(NaiveDate::MAX);
    #[inline]
    #[allow(clippy::cast_sign_loss)]
    fn encode(&self) -> u32 {
        (self.to_epoch_days() as u32) ^ SIGN
    }
    #[inline]
    #[allow(clippy::cast_possible_wrap)]
    fn decode(word: &u32) -> Option<Self> {
        NaiveDate::from_epoch_days((*word ^ SIGN) as i32)
    }
}
impl DiscreteDomain for NaiveDate {
    #[inline]
    fn succ(&self) -> Option<Self> {
        self.succ_opt()
    }
    #[inline]
    fn pred(&self) -> Option<Self> {
        self.pred_opt()
    }
}
impl CoverPreserving for NaiveDate {}

const LAST_TIME: NaiveTime = match NaiveTime::from_hms_nano_opt(23, 59, 59, 1_999_999_999) {
    Some(t) => t,
    None => panic!("the leap-second form of 23:59:59 is a valid NaiveTime"),
};
const LAST_DATETIME: NaiveDateTime = NaiveDateTime::new(NaiveDate::MAX, LAST_TIME);

impl Domain for NaiveTime {}
impl OrderedDomain for NaiveTime {
    type Density = Dense;
    const MIN: Option<Self> = Some(NaiveTime::MIN);
    const MAX: Option<Self> = Some(LAST_TIME);
    crate::identity_encoding!();
}

impl Domain for NaiveDateTime {}
impl OrderedDomain for NaiveDateTime {
    type Density = Dense;
    const MIN: Option<Self> = Some(NaiveDateTime::MIN);
    const MAX: Option<Self> = Some(LAST_DATETIME);
    crate::identity_encoding!();
}

impl Domain for DateTime<Utc> {}
impl OrderedDomain for DateTime<Utc> {
    type Density = Dense;
    const MIN: Option<Self> = Some(DateTime::<Utc>::MIN_UTC);
    const MAX: Option<Self> = Some(LAST_DATETIME.and_utc());
    crate::identity_encoding!();
}
