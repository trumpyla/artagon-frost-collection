//! Third-party temporal domains. Run with `--features chrono-0_4,jiff-0_2,time-0_3`.
#![allow(
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_bitwise_bool,
    clippy::items_after_statements,
    unreachable_pub,
    unused_imports,
    reason = "test code"
)]
mod support;
use frost_interval::*;
use support::laws;

#[cfg(feature = "chrono-0_4")]
mod chrono_domains {
    use super::*;
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

    #[test]
    fn naive_date_exhaustive_walk() {
        // Every NaiveDate from MIN to MAX: about 191 million days.
        laws::check_cover_preserving_exhaustive::<NaiveDate>(usize::MAX);
    }

    #[test]
    fn naive_date_sets() {
        let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
        assert_eq!(
            std::mem::size_of::<FrozenIntervalSet<NaiveDate>>(),
            std::mem::size_of::<(bool, Vec<u32>)>()
        );
        let feb: FrozenIntervalSet<NaiveDate> = FrozenIntervalSet::of(d(2024, 2, 1)..d(2024, 3, 1));
        assert_eq!(feb.count(), 29);
        assert_eq!(feb.closed_intervals(), [(d(2024, 2, 1), d(2024, 2, 29))]);
        let q: FrozenIntervalSet<NaiveDate> = FrozenIntervalSet::from_intervals([
            d(2024, 1, 1)..=d(2024, 1, 31),
            d(2024, 2, 1)..=d(2024, 2, 29),
        ]);
        assert_eq!(q.interval_count(), 1); // adjacent days coalesce
        assert_eq!(
            NaiveDate::closed(NaiveDate::MIN, NaiveDate::MAX),
            NaiveDate::all()
        );
        // Oracle: chrono's own date arithmetic.
        let days = NaiveDate::MAX
            .signed_duration_since(NaiveDate::MIN)
            .num_days()
            + 1;
        assert_eq!(FrozenIntervalSet::<NaiveDate>::all().count(), days as u128);
        assert_eq!(days, 191_491_529);
    }

    #[test]
    fn time_of_day_types_are_dense_and_bounded() {
        let t = |h, m, s, n| NaiveTime::from_hms_nano_opt(h, m, s, n).unwrap();
        let leap = t(23, 59, 59, 1_500_000_000);
        let last = t(23, 59, 59, 1_999_999_999);
        let times = [
            NaiveTime::MIN,
            t(0, 0, 0, 1),
            t(12, 0, 0, 0),
            t(12, 0, 59, 1_000_000_000),
            t(12, 1, 0, 0),
            t(23, 59, 59, 999_999_999),
            leap,
            last,
        ];
        laws::check_ordered(&times);
        assert_eq!(NaiveTime::max_cut(), Cut::Above(last));
        assert_eq!(NaiveTime::closed(NaiveTime::MIN, last), NaiveTime::all());
        // The leap-second form lies above 23:59:59.999999999 and inside the domain.
        let evening: FrozenIntervalSet<NaiveTime> = FrozenIntervalSet::of(t(18, 0, 0, 0)..);
        assert!(evening.contains(&leap) && evening.contains(&last));
        // chrono accepts the leap-second form only in second 59.
        assert!(NaiveTime::from_hms_nano_opt(12, 0, 0, 1_000_000_000).is_none());

        let dt = |y, mo, d, h| {
            NaiveDate::from_ymd_opt(y, mo, d)
                .unwrap()
                .and_hms_opt(h, 0, 0)
                .unwrap()
        };
        let top = NaiveDate::MAX.and_time(last);
        laws::check_ordered(&[
            NaiveDateTime::MIN,
            dt(1970, 1, 1, 0),
            dt(2024, 6, 30, 23),
            NaiveDateTime::MAX,
            top,
        ]);
        laws::check_ordered(&[
            DateTime::<Utc>::MIN_UTC,
            dt(1970, 1, 1, 0).and_utc(),
            DateTime::<Utc>::MAX_UTC,
            top.and_utc(),
        ]);
        let s: FrozenIntervalSet<DateTime<Utc>> =
            FrozenIntervalSet::from_intervals([
                dt(2024, 1, 1, 0).and_utc()..dt(2024, 1, 2, 0).and_utc()
            ]);
        assert!(
            s.contains(&dt(2024, 1, 1, 23).and_utc()) && !s.contains(&dt(2024, 1, 2, 0).and_utc())
        );
        let m: FrozenIntervalMap<NaiveDateTime, &str> =
            [(dt(2024, 1, 1, 9)..dt(2024, 1, 1, 17), "open")]
                .into_iter()
                .collect();
        assert_eq!(m.get(&dt(2024, 1, 1, 12)), Some(&"open"));
        assert_eq!(m.get(&dt(2024, 1, 1, 17)), None);
    }
}

#[cfg(feature = "jiff-0_2")]
mod jiff_domains {
    use super::*;
    use jiff::civil::{Date, DateTime, Time, date, time};
    use jiff::{SignedDuration, Timestamp};

    #[test]
    fn civil_date_exhaustive_walk() {
        // Every date from -9999-01-01 to 9999-12-31.
        laws::check_cover_preserving_exhaustive::<Date>(8_000_000);
        assert_eq!(FrozenIntervalSet::<Date>::all().count(), 7_304_484);
    }

    #[test]
    fn civil_time_and_instants() {
        let samples = [
            Time::MIN,
            time(0, 0, 0, 1),
            time(0, 0, 59, 999_999_999),
            time(0, 59, 59, 999_999_999),
            time(12, 0, 0, 0),
            time(23, 59, 59, 999_999_998),
            Time::MAX,
        ];
        laws::check_cover_preserving_sampled(&samples, 50);
        assert_eq!(FrozenIntervalSet::<Time>::all().count(), 86_400_000_000_000);
        assert_eq!(
            std::mem::size_of::<FrozenIntervalSet<Time>>(),
            std::mem::size_of::<(bool, Vec<u64>)>()
        );
        let work: FrozenIntervalSet<Time> = FrozenIntervalSet::from_intervals([
            time(9, 0, 0, 0)..time(12, 0, 0, 0),
            time(12, 0, 0, 0)..time(17, 0, 0, 0),
        ]);
        assert_eq!(work.interval_count(), 1);
        assert_eq!(work.count(), 8 * 3_600_000_000_000);

        let ns = SignedDuration::from_nanos(1);
        let ts = [
            Timestamp::MIN,
            Timestamp::MIN.checked_add(ns).unwrap(),
            Timestamp::UNIX_EPOCH,
            Timestamp::MAX.checked_sub(ns).unwrap(),
            Timestamp::MAX,
        ];
        laws::check_ordered(&ts);
        laws::check_discrete(&ts);
        let dts = [
            DateTime::MIN,
            DateTime::MIN.checked_add(ns).unwrap(),
            date(2024, 2, 29).at(23, 59, 59, 999_999_999),
            DateTime::MAX.checked_sub(ns).unwrap(),
            DateTime::MAX,
        ];
        laws::check_ordered(&dts);
        laws::check_discrete(&dts);
        let ds = [
            SignedDuration::MIN,
            SignedDuration::MIN + ns,
            SignedDuration::ZERO,
            ns,
            SignedDuration::MAX - ns,
            SignedDuration::MAX,
        ];
        laws::check_ordered(&ds);
        laws::check_discrete(&ds);
        // Discrete at 1 ns: [a, b] and (b, c] coalesce.
        let (a, b, c) = (
            Timestamp::UNIX_EPOCH,
            Timestamp::from_second(10).unwrap(),
            Timestamp::from_second(20).unwrap(),
        );
        let s: FrozenIntervalSet<Timestamp> =
            [Timestamp::closed(a, b), Timestamp::open_closed(b, c)]
                .into_iter()
                .collect();
        assert_eq!(s.closed_intervals(), [(a, c)]);
        let feb: FrozenIntervalSet<Date> =
            FrozenIntervalSet::of(date(2024, 2, 1)..date(2024, 3, 1));
        assert_eq!(feb.count(), 29);
    }
}

#[cfg(feature = "time-0_3")]
mod time_domains {
    use super::*;
    use time::{Date, Duration, Month, PrimitiveDateTime, Time, UtcDateTime};

    #[test]
    fn date_exhaustive_walk() {
        laws::check_cover_preserving_exhaustive::<Date>(8_000_000);
    }

    #[test]
    fn time_and_date_times() {
        let t = |h, m, s, n| Time::from_hms_nano(h, m, s, n).unwrap();
        let samples = [
            Time::MIDNIGHT,
            t(0, 0, 0, 1),
            t(0, 0, 59, 999_999_999),
            t(0, 59, 59, 999_999_999),
            t(12, 0, 0, 0),
            Time::MAX,
        ];
        laws::check_cover_preserving_sampled(&samples, 50);
        assert_eq!(FrozenIntervalSet::<Time>::all().count(), 86_400_000_000_000);

        let ns = Duration::NANOSECOND;
        let d = Date::from_calendar_date(2024, Month::February, 29).unwrap();
        let p = [
            PrimitiveDateTime::MIN,
            PrimitiveDateTime::MIN + ns,
            PrimitiveDateTime::new(d, Time::MAX),
            PrimitiveDateTime::MAX - ns,
            PrimitiveDateTime::MAX,
        ];
        laws::check_ordered(&p);
        laws::check_discrete(&p);
        let u = [
            UtcDateTime::MIN,
            UtcDateTime::MIN + ns,
            UtcDateTime::new(d, Time::MAX),
            UtcDateTime::MAX - ns,
            UtcDateTime::MAX,
        ];
        laws::check_ordered(&u);
        laws::check_discrete(&u);
        let ds = [
            Duration::MIN,
            Duration::MIN + ns,
            Duration::ZERO,
            ns,
            Duration::MAX - ns,
            Duration::MAX,
        ];
        laws::check_ordered(&ds);
        laws::check_discrete(&ds);
        let feb: FrozenIntervalSet<Date> = FrozenIntervalSet::of(
            Date::from_calendar_date(2024, Month::February, 1).unwrap()
                ..Date::from_calendar_date(2024, Month::March, 1).unwrap(),
        );
        assert_eq!(feb.count(), 29);
    }
}
