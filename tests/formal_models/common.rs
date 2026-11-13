//! Shared model vocabulary. The finite domains themselves come from `exhaust`
//! and `itertools`; nothing here enumerates. Sets over at most eight keys are
//! bit masks: bit `k` is set when key `k` is a member, so every subset of `n`
//! keys is the range `0..1 << n`.

use exhaust::Exhaust;

/// Three keys, for typed domains enumerated by `exhaust`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Exhaust)]
pub(crate) enum Key3 {
    K0,
    K1,
    K2,
}

impl Key3 {
    pub(crate) fn index(self) -> u32 {
        self as u32
    }
}

/// Members of `set` among the first `n` keys, ascending.
pub(crate) fn members(set: u32, n: u32) -> impl Iterator<Item = u32> {
    (0..n).filter(move |key| set & (1 << key) != 0)
}

/// Whether every member of `a` is a member of `b`.
pub(crate) fn is_subset(a: u32, b: u32) -> bool {
    a & !b == 0
}
