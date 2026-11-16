//! Temporal domains from `std::time`.
//!
//! | Type | Density | Extrema | Word |
//! |---|---|---|---|
//! | `Duration` | discrete, 1 ns | `ZERO..=MAX` | identity word (a 96-bit value does not fit a word) |
//! | `SystemTime`, `Instant` | dense | none exposed by std | identity word + side bit |
//!
//! `SystemTime` and `Instant` have a platform-specific resolution and no public
//! minimum or maximum, so the lawful claim is a dense, unbounded domain.

#[cfg(feature = "std")]
use crate::domain::Dense;
use crate::domain::{Discrete, DiscreteDomain, Domain, OrderedDomain};
use core::time::Duration;
#[cfg(feature = "std")]
use std::time::{Instant, SystemTime};

impl Domain for Duration {}
impl OrderedDomain for Duration {
    type Density = Discrete;
    const MIN: Option<Self> = Some(Duration::ZERO);
    const MAX: Option<Self> = Some(Duration::MAX);
    crate::identity_encoding!();
}
impl DiscreteDomain for Duration {
    #[inline]
    fn succ(&self) -> Option<Self> {
        self.checked_add(Duration::from_nanos(1))
    }
    #[inline]
    fn pred(&self) -> Option<Self> {
        self.checked_sub(Duration::from_nanos(1))
    }
}

#[cfg(feature = "std")]
impl Domain for SystemTime {}
#[cfg(feature = "std")]
impl OrderedDomain for SystemTime {
    type Density = Dense;
    crate::identity_encoding!();
}

#[cfg(feature = "std")]
impl Domain for Instant {}
#[cfg(feature = "std")]
impl OrderedDomain for Instant {
    type Density = Dense;
    crate::identity_encoding!();
}
