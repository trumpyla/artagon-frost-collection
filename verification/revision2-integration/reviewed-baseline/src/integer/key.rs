use core::fmt;
use core::hash::Hash;
use core::num::{NonZeroU32, NonZeroU64};

/// A supported unsigned storage word: exactly `u32` or `u64`.
///
/// This trait is sealed by the private storage mapping. Applications implement
/// [`IntKey`] and select a primitive word; they never implement backend machinery.
/// The private bound keeps associated backend and iterator types out of the API.
///
/// ```compile_fail,E0277
/// use frost::integer::IntKey;
/// #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// struct TooWide(u128);
/// impl IntKey for TooWide {
///     type Word = u128;
///     fn encode(self) -> u128 { self.0 }
///     fn decode(word: u128) -> Option<Self> { Some(Self(word)) }
/// }
/// ```
#[allow(private_bounds)] // Deliberate sealing boundary; storage types stay private.
pub trait IntWord:
    Copy + Ord + Hash + fmt::Debug + Send + Sync + 'static + super::storage::Word
{
}

impl IntWord for u32 {}
impl IntWord for u64 {}

/// A logical key with one canonical, lossless, order-preserving integer code.
///
/// # Laws
///
/// - `decode(key.encode())` reconstructs the **complete original value**, not
///   merely a value considered equal by an `Eq` implementation that ignores data.
/// - Every accepted code is canonical: `decode(word).map(Self::encode)` is
///   `Some(word)` whenever decoding succeeds.
/// - Comparing keys has the same result as comparing their encoded words.
/// - Mapping is stable across calls, threads, collections, and time. Mutable
///   globals, runtime dictionaries, and per-set offsets do not belong in a codec.
///
/// Construction checks observed round trips, accepted-code canonicality, and
/// ordering of the normalized input. It cannot prove these laws for unobserved
/// values or detect metadata discarded by a custom equality relation.
///
/// This is a safe trait. Violating its laws can cause an error, a checked panic,
/// or incorrect logical answers, but Frost does not use codec promises to justify
/// unsafe memory access. User callback panics propagate. Iteration checks
/// decoding rather than constructing values through unchecked casts.
///
/// Use the ordinary frozen set family when stored representative metadata matters.
///
/// ```
/// use frost::integer::{FrozenIntSet, IntKey};
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// struct UserId(u64);
/// impl IntKey for UserId {
///     type Word = u64;
///     fn encode(self) -> u64 { self.0 }
///     fn decode(word: u64) -> Option<Self> { Some(Self(word)) }
/// }
/// let ids: FrozenIntSet<UserId> = [UserId(7), UserId(2)].into_iter().collect();
/// assert_eq!(ids.iter().collect::<Vec<_>>(), [UserId(2), UserId(7)]);
/// ```
pub trait IntKey: Copy + Ord {
    /// Canonical unsigned width, selected at compile time.
    type Word: IntWord;
    /// Encode the complete logical value, preserving its ordering.
    fn encode(self) -> Self::Word;
    /// Decode a valid canonical member; reject invalid or reserved codes.
    fn decode(word: Self::Word) -> Option<Self>;
}

macro_rules! unsigned_key {
    ($word:ty) => {
        impl IntKey for $word {
            type Word = $word;
            #[inline]
            fn encode(self) -> Self::Word {
                self
            }
            #[inline]
            fn decode(word: Self::Word) -> Option<Self> {
                Some(word)
            }
        }
    };
}

macro_rules! signed_key {
    ($key:ty, $word:ty, $sign:expr) => {
        impl IntKey for $key {
            type Word = $word;
            #[inline]
            #[allow(clippy::cast_sign_loss)] // Same-width two's-complement encoding.
            fn encode(self) -> Self::Word {
                (self as $word) ^ $sign
            }
            #[inline]
            #[allow(clippy::cast_possible_wrap)] // Inverse of the same-width encoding.
            fn decode(word: Self::Word) -> Option<Self> {
                Some((word ^ $sign) as $key)
            }
        }
    };
}

unsigned_key!(u32);
unsigned_key!(u64);
signed_key!(i32, u32, 1_u32 << 31);
signed_key!(i64, u64, 1_u64 << 63);

impl IntKey for NonZeroU32 {
    type Word = u32;
    fn encode(self) -> Self::Word {
        self.get()
    }
    fn decode(word: Self::Word) -> Option<Self> {
        Self::new(word)
    }
}

impl IntKey for NonZeroU64 {
    type Word = u64;
    fn encode(self) -> Self::Word {
        self.get()
    }
    fn decode(word: Self::Word) -> Option<Self> {
        Self::new(word)
    }
}
