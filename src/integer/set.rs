use alloc::vec::Vec;

use super::Shared as Arc;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter::FusedIterator;
use core::marker::PhantomData;

use super::dense::dense_bytes;
use super::storage::{Operation, Storage, Word, Words};
use super::{
    IntKey, IntSetError, IntSetLimits, IntSetPolicy, IntSetRepr, IntSetStatistics, IntWord,
};
use crate::family::{FrozenOrderedSet, MutableOrderedSet};
use crate::read::{Cardinality, ElementType, SetContains, ValueElements};
use crate::{Rejected, TryFreezeWith};

/// Validated codes in arrival order, with what the validation pass observed.
struct Staging<K: IntKey> {
    words: Vec<K::Word>,
    /// The codes arrived strictly ascending: already sorted and unique.
    ascending: bool,
    /// First arrival index where decoded order did not follow ascending code
    /// order. Meaningful only while `ascending` holds, where arrival index and
    /// sorted index coincide.
    nonmonotonic: Option<usize>,
    bounds: Option<(K::Word, K::Word)>,
    previous: Option<K>,
}

impl<K: IntKey> Staging<K> {
    const fn new() -> Self {
        Self {
            words: Vec::new(),
            ascending: true,
            nonmonotonic: None,
            bounds: None,
            previous: None,
        }
    }

    /// Record one code whose codec laws the caller has already checked.
    fn push(&mut self, index: usize, word: K::Word, decoded: K) {
        match self.bounds {
            None => self.bounds = Some((word, word)),
            Some((low, high)) => {
                if self.ascending {
                    if high < word {
                        // The same comparison the sorted pass makes at this index.
                        if self.nonmonotonic.is_none()
                            && self
                                .previous
                                .is_some_and(|prior| prior.cmp(&decoded) != Ordering::Less)
                        {
                            self.nonmonotonic = Some(index);
                        }
                    } else {
                        self.ascending = false;
                    }
                }
                self.bounds = Some((low.min(word), high.max(word)));
            }
        }
        self.previous = Some(decoded);
        self.words.push(word);
    }
}

/// Smallest input for which a temporary bitmap replaces a comparison sort.
const BITMAP_SORT_MIN: usize = 64;

/// Whether sorting through a bitmap is linear and bounded: the bitmap over the
/// observed span is no larger than the staged codes themselves, so temporary
/// staging at most doubles. This is a staging choice, not a representation;
/// the planner still selects the stored layout from the sorted codes.
fn bitmap_sort_pays<W: Word>(words: &[W], low: W, high: W) -> bool {
    words.len() >= BITMAP_SORT_MIN
        && dense_bytes(low.to_u64(), high.to_u64()) <= size_of_val(words) as u128
}

/// Sort and deduplicate codes within `[low, high]` by setting and reading bits.
fn bitmap_sort<W: Word>(words: &mut Vec<W>, low: W, high: W) {
    let base = low.to_u64() & !63;
    let blocks = usize::try_from(dense_bytes(low.to_u64(), high.to_u64()) / 8)
        .expect("the bitmap is no larger than the staged codes");
    let mut bits = alloc::vec![0_u64; blocks];
    for &word in words.iter() {
        let offset = word.to_u64() - base;
        bits[usize::try_from(offset >> 6).expect("offset within the bitmap")] |= 1 << (offset & 63);
    }
    words.clear();
    let mut block_base = base;
    for mut block in bits {
        while block != 0 {
            let code = block_base + u64::from(block.trailing_zeros());
            block &= block - 1;
            words.push(W::try_from_u64(code).expect("a staged code of this width"));
        }
        block_base = block_base.wrapping_add(64);
    }
}

/// A structurally frozen set of losslessly encoded integer-domain values.
///
/// Storage is private and selected when constructed. Each instance has sorted,
/// dense-window, width-specific Roaring, or shared chunked storage. Feature unification never
/// adds representations to an already available profile. The `int-set` feature
/// enables the complete profile with `alloc`. The optional `std` feature enables
/// supported runtime CPU dispatch in the Roaring dependency.
///
/// Cloning shares the immutable storage in O(1). An algebra result equal to an
/// operand shares that operand's allocation when its representation satisfies
/// the result policy. Chunked storage additionally shares untouched 2^16-code
/// containers between versions. Reference counts are atomic where the target
/// supports pointer-width atomics and non-atomic otherwise.
///
/// Iteration reconstructs `K` by value. There is no borrowed `&K`, mutable view,
/// implicit numeric-range fill, or complement over all possible word patterns.
/// Use `valid_universe.difference(&set)` for a domain-relative complement.
///
/// Counts are `u64`; this type implements [`Cardinality`], not [`crate::read::Len`].
/// [`try_len`](Self::try_len) makes conversion to a target's `usize` explicit.
/// The complete `u64` universe has 2^64 members and cannot be represented by
/// this cardinality contract. A sparse set can still span that entire universe;
/// [`IntSetStatistics::span`] uses `u128` to describe it without overflow.
/// The phantom key preserves its lifetimes and auto-trait restrictions even
/// though only unsigned codes are physically stored.
#[must_use]
pub struct FrozenIntSet<K: IntKey> {
    storage: Arc<Storage<K::Word>>,
    limits: IntSetLimits,
    key: PhantomData<K>,
}

impl<K: IntKey> FrozenIntSet<K> {
    /// Empty sorted set with a shared storage header and no member buffer.
    pub fn new() -> Self {
        Self::from_storage(Storage::empty(), IntSetLimits::default())
    }

    fn from_storage(storage: Storage<K::Word>, limits: IntSetLimits) -> Self {
        Self::from_shared(Arc::new(storage), limits)
    }

    fn from_shared(storage: Arc<Storage<K::Word>>, limits: IntSetLimits) -> Self {
        Self {
            storage,
            limits,
            key: PhantomData,
        }
    }

    /// Encode keys, validate their observed codec laws, normalize, and plan storage.
    ///
    /// Duplicates collapse according to the canonical integer code. This profile
    /// reconstructs full keys under the [`IntKey`] laws; it cannot retain metadata
    /// that a codec discards. Normalization needs O(n) staging storage. Input that
    /// arrives strictly ascending is not sorted again; unsorted input whose span
    /// fits a bitmap no larger than the staged codes is normalized in linear time
    /// with that much additional temporary storage; other input is sorted once in
    /// O(n log n) time. Checked validation adds linear codec work. Policy limits
    /// cover the documented candidate costs, not staging storage or the allocator.
    ///
    /// # Errors
    /// Returns a codec-law violation observed in the input, a rejected explicit
    /// representation, or an unrepresentable cardinality. Consumed input is not
    /// returned; use [`TryFreezeWith`] on an owned source to recover it on failure.
    /// User codec/ordering panics and allocator failures are not returned as errors.
    pub fn try_from_iter<I: IntoIterator<Item = K>>(
        input: I,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        let mut staging = Staging::new();
        for (index, key) in input.into_iter().enumerate() {
            let word = key.encode();
            let decoded = K::decode(word).ok_or(IntSetError::InvalidCode { index })?;
            if decoded != key || decoded.cmp(&key) != Ordering::Equal {
                return Err(IntSetError::CodecRoundTrip { index });
            }
            if decoded.encode() != word {
                return Err(IntSetError::NonCanonicalCode { index });
            }
            staging.push(index, word, decoded);
        }
        Self::normalize(staging, policy)
    }

    /// Import encoded members with checked decoding and canonicality validation.
    ///
    /// This accepts logical words, not an unchecked serialized backend. Gaps in
    /// a restricted key domain are rejected. Duplicates and input order normalize
    /// exactly as in [`try_from_iter`](Self::try_from_iter).
    ///
    /// # Errors
    /// Returns an invalid/noncanonical code, observed order violation, or rejected
    /// storage policy. The consumed source is not returned. Allocation failures
    /// and panics in user code follow ordinary Rust behavior.
    pub fn try_from_encoded<I: IntoIterator<Item = K::Word>>(
        input: I,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        let mut staging = Staging::new();
        for (index, word) in input.into_iter().enumerate() {
            let key = K::decode(word).ok_or(IntSetError::InvalidCode { index })?;
            if key.encode() != word {
                return Err(IntSetError::NonCanonicalCode { index });
            }
            staging.push(index, word, key);
        }
        Self::normalize(staging, policy)
    }

    /// Sort, deduplicate, and check that decoded order follows code order.
    ///
    /// Input that arrived strictly ascending is already sorted and unique, and
    /// the first pass compared its decoded keys in that same order, so no
    /// second pass is needed. Unsorted input whose span is no larger than the
    /// staged codes is normalized through a temporary bitmap in linear time;
    /// other input is sorted by comparison.
    fn normalize(staging: Staging<K>, policy: IntSetPolicy) -> Result<Self, IntSetError> {
        let Staging {
            mut words,
            ascending,
            nonmonotonic,
            bounds,
            ..
        } = staging;
        if ascending {
            if let Some(index) = nonmonotonic {
                return Err(IntSetError::NonMonotonicEncoding { index });
            }
        } else {
            match bounds {
                Some((low, high)) if bitmap_sort_pays(&words, low, high) => {
                    bitmap_sort(&mut words, low, high);
                }
                _ => {
                    words.sort_unstable();
                    words.dedup();
                }
            }
            let mut previous = None;
            for (index, &word) in words.iter().enumerate() {
                let key = K::decode(word).ok_or(IntSetError::InvalidCode { index })?;
                if key.encode() != word {
                    return Err(IntSetError::NonCanonicalCode { index });
                }
                if previous.is_some_and(|prior: K| prior.cmp(&key) != Ordering::Less) {
                    return Err(IntSetError::NonMonotonicEncoding { index });
                }
                previous = Some(key);
            }
        }
        Storage::from_sorted(words, policy)
            .map(|storage| Self::from_storage(storage, policy.limits()))
    }

    /// Rebuild the same logical codes under an explicit policy, without decoding.
    ///
    /// This uses representation-aware conversion and retains the supplied limits
    /// for subsequent algebra. It may allocate or enumerate codes and is not a
    /// cheap view conversion. Storage may be shared when the existing
    /// representation already satisfies the policy. A forced backend applies
    /// to this operation only; its limits persist in later algebra.
    ///
    /// # Errors
    /// Returns the selected representation's budget or partition rejection. This
    /// set remains unchanged, including when rebuilding fails.
    pub fn with_policy(&self, policy: IntSetPolicy) -> Result<Self, IntSetError> {
        self.storage
            .replan_shared(policy)
            .map(|storage| Self::from_shared(storage, policy.limits()))
    }

    /// Representation limits retained by this set and composed by default algebra.
    pub const fn limits(&self) -> IntSetLimits {
        self.limits
    }

    /// Automatic result policy carrying this set's retained representation limits.
    ///
    /// This does not recover the backend requested during construction. Default
    /// union, intersection, and symmetric difference take the smaller limits;
    /// difference keeps the left operand's limits.
    pub const fn policy(&self) -> IntSetPolicy {
        self.limits.automatic()
    }

    /// Estimated payload bytes of the selected representation, for diagnostics.
    ///
    /// Sorted and dense payloads are exact. Roaring uses container statistics
    /// to estimate its buffer and index storage; chunked storage counts container
    /// payloads and index entries. This is not a resident-memory measurement.
    /// This estimate excludes storage headers, reference counts, allocator
    /// overhead, temporary allocations, and memory retained by other versions.
    pub fn payload_bytes(&self) -> u64 {
        self.storage.payload_bytes()
    }

    /// Whether both sets use the same immutable storage allocation.
    ///
    /// Equal sets with different result limits may share storage if that
    /// representation satisfies both sets' limits.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storage, &other.storage)
    }

    /// Number of 2^16-code containers physically shared with `other`.
    ///
    /// Only chunked storage shares containers; every other pair reports zero.
    pub fn shared_chunks(&self, other: &Self) -> u64 {
        self.storage.shared_chunks(&other.storage)
    }

    /// Number of chunk containers, or zero for other representations.
    pub fn chunk_count(&self) -> u64 {
        self.storage.chunk_count()
    }

    /// Exact member-buffer payload bytes for sorted and dense storage.
    ///
    /// Returns `None` for Roaring or chunked noncontiguous storage. This excludes the set
    /// wrapper, allocator headers, temporary allocations, and process overhead.
    pub fn contiguous_payload_bytes(&self) -> Option<usize> {
        self.storage.contiguous_payload_bytes()
    }

    /// Actual representation, for diagnostics.
    pub fn representation(&self) -> IntSetRepr {
        self.storage.representation()
    }

    /// Number of distinct members, cached independently of iteration.
    pub fn cardinality(&self) -> u64 {
        self.storage.cardinality()
    }

    /// Cardinality if it fits this target's address-sized integer.
    pub fn try_len(&self) -> Option<usize> {
        usize::try_from(self.cardinality()).ok()
    }

    /// Whether the set has no members.
    pub fn is_empty(&self) -> bool {
        self.cardinality() == 0
    }

    /// Membership lookup, encoding the query once.
    pub fn contains(&self, key: &K) -> bool {
        self.storage.contains(key.encode())
    }

    /// Inclusive logical bounds, reconstructed from cached encoded bounds.
    ///
    /// # Panics
    /// Panics if a previously valid code no longer decodes because the codec
    /// violates its stability law, or if user decoding itself panics.
    pub fn bounds(&self) -> Option<(K, K)> {
        self.storage
            .bounds()
            .map(|(first, last)| (decode::<K>(first), decode::<K>(last)))
    }

    /// Ascending logical members, reconstructed by value as requested.
    ///
    /// Decoding is checked; a law-breaking decoder can panic during traversal.
    pub fn iter(&self) -> IntSetIter<'_, K> {
        IntSetIter {
            words: self.storage.iter(),
            key: PhantomData,
        }
    }

    /// Ascending canonical codes, without reconstructing keys or exposing a backend.
    pub fn encoded_values(&self) -> EncodedValues<'_, K::Word> {
        EncodedValues {
            words: self.storage.iter(),
        }
    }

    /// Scan encoded members to explain their distribution, without decoding keys.
    pub fn statistics(&self) -> IntSetStatistics {
        let encoded_bounds = self
            .storage
            .bounds()
            .map(|(low, high)| (low.to_u64(), high.to_u64()));
        let mut statistics = IntSetStatistics {
            cardinality: self.cardinality(),
            encoded_bounds,
            span: encoded_bounds.map_or(0, |(low, high)| u128::from(high) - u128::from(low) + 1),
            runs: 0,
            high32_partitions: 0,
        };
        let mut previous: Option<u64> = None;
        for word in self.encoded_values() {
            let word = word.to_u64();
            if previous.and_then(|previous| previous.checked_add(1)) != Some(word) {
                statistics.runs += 1;
            }
            if previous.is_none_or(|previous| previous >> 32 != word >> 32) {
                statistics.high32_partitions += 1;
            }
            previous = Some(word);
        }
        statistics
    }

    /// Number of common members, using encoded backend kernels without decoding.
    ///
    /// Dense/Roaring pairs may allocate temporary compressed storage when a
    /// bounded byte conversion replaces many membership probes. Chunked/Dense
    /// pairs may allocate temporary chunks for the dense window. These
    /// conversions do not materialize a result set or mutate either operand.
    pub fn intersection_len(&self, other: &Self) -> u64 {
        if Arc::ptr_eq(&self.storage, &other.storage) {
            self.cardinality()
        } else {
            self.storage.intersection_len(&other.storage)
        }
    }

    /// Cardinality of the union, without allocating a result.
    ///
    /// May allocate temporary conversion storage as in [`intersection_len`](Self::intersection_len).
    ///
    /// # Panics
    /// Panics if the mathematical cardinality exceeds `u64::MAX`.
    pub fn union_len(&self, other: &Self) -> u64 {
        self.cardinality()
            .checked_add(other.cardinality() - self.intersection_len(other))
            .expect("integer set union cardinality exceeds u64")
    }

    /// Cardinality of members present only in this set.
    ///
    /// May allocate temporary conversion storage as in [`intersection_len`](Self::intersection_len).
    pub fn difference_len(&self, other: &Self) -> u64 {
        self.cardinality() - self.intersection_len(other)
    }

    /// Cardinality of members present in exactly one operand.
    ///
    /// May allocate temporary conversion storage as in [`intersection_len`](Self::intersection_len).
    ///
    /// # Panics
    /// Panics if the mathematical cardinality exceeds `u64::MAX`.
    pub fn symmetric_difference_len(&self, other: &Self) -> u64 {
        let common = self.intersection_len(other);
        (self.cardinality() - common)
            .checked_add(other.cardinality() - common)
            .expect("integer set symmetric difference cardinality exceeds u64")
    }

    /// Whether every member is present in `other`.
    pub fn is_subset(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storage, &other.storage) || self.storage.is_subset(&other.storage)
    }
    /// Whether every member of `other` is present in this set.
    pub fn is_superset(&self, other: &Self) -> bool {
        other.is_subset(self)
    }
    /// Whether the sets have no common member.
    pub fn is_disjoint(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.storage, &other.storage) {
            self.is_empty()
        } else {
            self.storage.is_disjoint(&other.storage)
        }
    }

    /// Materialize the encoded intersection without decoding logical keys.
    ///
    /// Uses the smaller dense-byte and Roaring-partition limit from the two
    /// operands. Result storage preserves or promotes compact representations.
    /// Use [`intersection_with_policy`](Self::intersection_with_policy) to choose
    /// explicit limits or a forced result representation.
    pub fn intersection(&self, other: &Self) -> Self {
        self.binary_default(
            other,
            Operation::Intersection,
            self.limits.common(other.limits),
        )
    }

    /// Materialize the encoded union without synthesizing new domain codes.
    ///
    /// Uses the smaller dense-byte and Roaring-partition limit from the operands.
    /// Dense windows beyond that budget can promote to Roaring; sorted storage
    /// remains a fallback when compressed candidates are ineligible.
    ///
    /// # Panics
    /// Panics if the mathematical cardinality exceeds `u64::MAX`.
    pub fn union(&self, other: &Self) -> Self {
        self.binary_default(other, Operation::Union, self.limits.common(other.limits))
    }

    /// Materialize this set minus `other`, keeping this set's representation limits.
    pub fn difference(&self, other: &Self) -> Self {
        self.binary_default(other, Operation::Difference, self.limits)
    }

    /// Materialize members present in exactly one operand.
    ///
    /// Uses the smaller dense-byte and Roaring-partition limit from the operands.
    ///
    /// # Panics
    /// Panics if the mathematical cardinality exceeds `u64::MAX`.
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        self.binary_default(
            other,
            Operation::SymmetricDifference,
            self.limits.common(other.limits),
        )
    }

    /// Materialize an intersection with explicit result representation and limits.
    ///
    /// The supplied policy replaces inherited limits for the returned set.
    /// Neither input changes. Codec values remain encoded throughout the operation.
    ///
    /// # Errors
    /// Returns a rejected forced representation or unrepresentable cardinality.
    /// Allocation failures are not converted into returned errors.
    pub fn intersection_with_policy(
        &self,
        other: &Self,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        self.binary_checked(other, Operation::Intersection, policy)
    }

    /// Materialize a union with explicit result representation and limits.
    ///
    /// # Errors
    /// Returns a rejected forced representation or cardinality exceeding u64.
    /// Allocation failures are not converted into returned errors.
    pub fn union_with_policy(
        &self,
        other: &Self,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        self.binary_checked(other, Operation::Union, policy)
    }

    /// Materialize a difference with explicit result representation and limits.
    ///
    /// # Errors
    /// Returns a rejected forced representation or unrepresentable cardinality.
    /// Allocation failures are not converted into returned errors.
    pub fn difference_with_policy(
        &self,
        other: &Self,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        self.binary_checked(other, Operation::Difference, policy)
    }

    /// Materialize a symmetric difference with explicit result policy.
    ///
    /// # Errors
    /// Returns a rejected forced representation or cardinality exceeding u64.
    /// Allocation failures are not converted into returned errors.
    pub fn symmetric_difference_with_policy(
        &self,
        other: &Self,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        self.binary_checked(other, Operation::SymmetricDifference, policy)
    }

    fn binary_default(&self, other: &Self, operation: Operation, limits: IntSetLimits) -> Self {
        self.binary_checked(other, operation, limits.automatic())
            .unwrap_or_else(|error| panic!("integer set operation failed: {error}"))
    }

    fn binary_checked(
        &self,
        other: &Self,
        operation: Operation,
        policy: IntSetPolicy,
    ) -> Result<Self, IntSetError> {
        if matches!(operation, Operation::Union | Operation::SymmetricDifference)
            && self
                .cardinality()
                .checked_add(other.cardinality())
                .is_none()
        {
            let common = self.intersection_len(other);
            let count = match operation {
                Operation::Union => self.cardinality().checked_add(other.cardinality() - common),
                Operation::SymmetricDifference => {
                    (self.cardinality() - common).checked_add(other.cardinality() - common)
                }
                Operation::Intersection | Operation::Difference => unreachable!(),
            };
            count.ok_or(IntSetError::CardinalityOverflow)?;
        }
        self.storage
            .binary(&other.storage, operation, policy)
            .map(|storage| Self::from_shared(storage, policy.limits()))
    }
}

fn decode<K: IntKey>(word: K::Word) -> K {
    K::decode(word).expect("IntKey::decode rejected a previously validated code")
}

impl<K: IntKey> Default for FrozenIntSet<K> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K: IntKey> Clone for FrozenIntSet<K> {
    /// O(1): increments the immutable storage's reference count.
    fn clone(&self) -> Self {
        Self::from_shared(Arc::clone(&self.storage), self.limits)
    }
}

/// Automatic, checked construction. Panics on an observed codec-law violation.
impl<K: IntKey> FromIterator<K> for FrozenIntSet<K> {
    fn from_iter<I: IntoIterator<Item = K>>(input: I) -> Self {
        Self::try_from_iter(input, IntSetPolicy::auto())
            .unwrap_or_else(|error| panic!("integer set construction failed: {error}"))
    }
}

impl<K: IntKey> PartialEq for FrozenIntSet<K> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storage, &other.storage) || self.storage.equals(&other.storage)
    }
}
impl<K: IntKey> Eq for FrozenIntSet<K> {}
impl<K: IntKey> PartialOrd for FrozenIntSet<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<K: IntKey> Ord for FrozenIntSet<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        if Arc::ptr_eq(&self.storage, &other.storage) {
            Ordering::Equal
        } else {
            self.storage.compare(&other.storage)
        }
    }
}
impl<K: IntKey> Hash for FrozenIntSet<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.storage.hash_into(state);
    }
}
impl<K: IntKey + fmt::Debug> fmt::Debug for FrozenIntSet<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// Borrowing iterator yielding reconstructed logical values in ascending order.
///
/// Cardinality can exceed `usize`; no `ExactSizeIterator` promise is made.
pub struct IntSetIter<'a, K: IntKey> {
    words: Words<'a, K::Word>,
    key: PhantomData<K>,
}
impl<K: IntKey> Clone for IntSetIter<'_, K> {
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
            key: PhantomData,
        }
    }
}
impl<K: IntKey> Iterator for IntSetIter<'_, K> {
    type Item = K;
    fn next(&mut self) -> Option<K> {
        self.words.next().map(decode::<K>)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.words.size_hint()
    }
    /// Select the representation once for the whole traversal. `for_each`,
    /// `sum`, and other adapters built on `fold` take this path; `for` loops
    /// and `collect` still call `next`.
    fn fold<B, F: FnMut(B, K) -> B>(self, init: B, mut f: F) -> B {
        self.words
            .fold(init, |accumulator, word| f(accumulator, decode::<K>(word)))
    }
}
impl<K: IntKey> FusedIterator for IntSetIter<'_, K> {}
impl<K: IntKey> fmt::Debug for IntSetIter<'_, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntSetIter")
            .field("size_hint", &self.size_hint())
            .finish_non_exhaustive()
    }
}

/// Borrowing iterator over canonical unsigned codes, independent of the backend.
pub struct EncodedValues<'a, W: IntWord> {
    words: Words<'a, W>,
}
impl<W: IntWord> Clone for EncodedValues<'_, W> {
    fn clone(&self) -> Self {
        Self {
            words: self.words.clone(),
        }
    }
}
impl<W: IntWord> Iterator for EncodedValues<'_, W> {
    type Item = W;
    fn next(&mut self) -> Option<W> {
        self.words.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.words.size_hint()
    }
    /// Select the representation once for the whole traversal.
    fn fold<B, F: FnMut(B, W) -> B>(self, init: B, f: F) -> B {
        self.words.fold(init, f)
    }
}
impl<W: IntWord> FusedIterator for EncodedValues<'_, W> {}
impl<W: IntWord> fmt::Debug for EncodedValues<'_, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncodedValues")
            .field("size_hint", &self.size_hint())
            .finish_non_exhaustive()
    }
}

impl<'a, K: IntKey> IntoIterator for &'a FrozenIntSet<K> {
    type Item = K;
    type IntoIter = IntSetIter<'a, K>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<K: IntKey> ElementType for FrozenIntSet<K> {
    type Item = K;
}
impl<K: IntKey> Cardinality for FrozenIntSet<K> {
    fn cardinality(&self) -> u64 {
        self.cardinality()
    }
}
impl<K: IntKey> SetContains<K> for FrozenIntSet<K> {
    fn contains(&self, key: &K) -> bool {
        self.contains(key)
    }
}
impl<K: IntKey> ValueElements for FrozenIntSet<K> {
    type Values<'a>
        = IntSetIter<'a, K>
    where
        Self: 'a;
    fn values(&self) -> Self::Values<'_> {
        self.iter()
    }
}

macro_rules! integer_freeze_plan {
    ($source:ty) => {
        impl<K: IntKey> TryFreezeWith<IntSetPolicy> for $source {
            type Output = FrozenIntSet<K>;
            type Error = Rejected<Self, IntSetError>;
            fn try_freeze_with(self, policy: IntSetPolicy) -> Result<Self::Output, Self::Error> {
                match FrozenIntSet::try_from_iter(self.iter().copied(), policy) {
                    Ok(set) => Ok(set),
                    Err(error) => Err(Rejected { input: self, error }),
                }
            }
        }
    };
}
integer_freeze_plan!(Vec<K>);
integer_freeze_plan!(MutableOrderedSet<K>);
integer_freeze_plan!(FrozenOrderedSet<K>);
