//! Deterministic, parameterized workload factories shared by integration
//! tests and benchmark targets.
//!
//! The canonical source is `tests/support/workload.rs`; `benches/support`
//! includes the same file by path so both consumers run one implementation.
//! Everything here depends only on `std`. Generation is deterministic in the
//! spec (including its seed), every structural guarantee is exact, and invalid
//! parameters are rejected with a typed error rather than clamped or sampled
//! until something fits. Oracles are computed with `BTreeSet` from the emitted
//! values, independently of the generator's bookkeeping, and belong outside
//! any timed region.

#![allow(dead_code)] // Each consumer uses a subset of the factory surface.

use std::collections::BTreeSet;
use std::fmt;

/// `SplitMix64` (Steele, Lea, and Flood, "Fast splittable pseudorandom number
/// generators", OOPSLA 2014); the constants are the published reference values.
/// This is a deterministic stream for reproducible workloads, not a source of
/// cryptographic or statistically certified randomness.
#[derive(Clone, Debug)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound` by fixed-point multiplication (one draw, no
    /// rejection loop). The modulo bias is below `2^-64 * bound` and is
    /// irrelevant to the exact structural guarantees made by the factories.
    ///
    /// # Panics
    ///
    /// Panics when `bound` is zero; callers validate parameters first.
    pub(crate) fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "Rng::below requires a non-zero bound");
        let wide = u128::from(self.next_u64()) * u128::from(bound);
        u64::try_from(wide >> 64).expect("product shifted by 64 fits u64")
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        // Fisher-Yates; `below` makes each swap index deterministic.
        for i in (1..values.len()).rev() {
            let j = usize::try_from(self.below(u64::try_from(i + 1).expect("index fits u64")))
                .expect("index fits usize");
            values.swap(i, j);
        }
    }
}

/// Reason a specification was rejected. Every variant names the offending
/// parameter so a case id can be corrected rather than silently adjusted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkloadError {
    /// `shared` exceeds the smaller side's unique count.
    SharedExceedsSide {
        shared: usize,
        left: usize,
        right: usize,
    },
    /// Arithmetic on the given parameter would overflow `u64` or `usize`.
    Overflow(&'static str),
    /// The uniform span cannot hold the requested distinct count.
    SpanTooSmall { needed: u64, span: u64 },
    /// A parameter that must be at least one was zero.
    ZeroParameter(&'static str),
    /// `chunk_bits` must lie in `1..=63`.
    ChunkBitsOutOfRange(u32),
    /// A chunk cannot hold `per_chunk` distinct codes.
    PerChunkExceedsChunk { per_chunk: u64, chunk_bits: u32 },
    /// A block order must name each class exactly once.
    InvalidBlockOrder([Class; 3]),
    /// Duplicates were requested for a side with no members to duplicate.
    DuplicatesOfEmptySide(Side),
    /// A union or emitted side would exceed [`MAX_MEMBERS`]; rejected before
    /// any allocation so extreme counts never attempt an enormous buffer.
    ExceedsSupportedSize { requested: usize, limit: usize },
    /// A mapped value does not fit the requested key type.
    ValueOutOfRange { value: u64, target: &'static str },
}

impl fmt::Display for WorkloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SharedExceedsSide {
                shared,
                left,
                right,
            } => write!(f, "shared={shared} exceeds min(left={left}, right={right})"),
            Self::Overflow(what) => write!(f, "{what} overflows"),
            Self::SpanTooSmall { needed, span } => {
                write!(f, "span={span} cannot hold {needed} distinct codes")
            }
            Self::ZeroParameter(what) => write!(f, "{what} must be at least 1"),
            Self::ChunkBitsOutOfRange(bits) => write!(f, "chunk_bits={bits} not in 1..=63"),
            Self::PerChunkExceedsChunk {
                per_chunk,
                chunk_bits,
            } => write!(f, "per_chunk={per_chunk} exceeds a 2^{chunk_bits} chunk"),
            Self::InvalidBlockOrder(order) => {
                write!(f, "block order {order:?} is not a permutation")
            }
            Self::DuplicatesOfEmptySide(side) => {
                write!(f, "{side:?} side has no members to duplicate")
            }
            Self::ExceedsSupportedSize { requested, limit } => {
                write!(f, "{requested} members exceed the supported bound {limit}")
            }
            Self::ValueOutOfRange { value, target } => {
                write!(f, "value {value} does not fit {target}")
            }
        }
    }
}

impl std::error::Error for WorkloadError {}

/// Largest union or emitted side the factories build: 2^26 codes (512 MiB of
/// `u64` per vector). Larger requests return
/// [`WorkloadError::ExceedsSupportedSize`] before any generation.
pub(crate) const MAX_MEMBERS: usize = 1 << 26;

/// Which side of a pair a parameter refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
}

/// Membership class of one code in the union of a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Class {
    LeftOnly,
    Shared,
    RightOnly,
}

impl Class {
    fn tag(self) -> &'static str {
        match self {
            Self::LeftOnly => "L",
            Self::Shared => "S",
            Self::RightOnly => "R",
        }
    }
}

/// Where the distinct codes of a workload live. Each layout gives exact
/// positions; `Uniform` is the only one that consumes the seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Layout {
    /// Distinct codes drawn without replacement from `base..base + span` using
    /// Floyd's algorithm (one random draw per code, no rejection loop).
    /// Density is `count / span`.
    Uniform { base: u64, span: u64 },
    /// Arithmetic progression `base + i * step`. `step = 1` is a dense run.
    Stride { base: u64, step: u64 },
    /// Runs of `run` consecutive codes separated by `gap` absent codes.
    Runs { base: u64, run: u64, gap: u64 },
    /// `per_chunk` codes in each aligned `2^chunk_bits` container, starting
    /// at container index `first_chunk`. With `per_chunk >= 2` every *full*
    /// container includes both its first and last code, so container
    /// boundaries (for example Roaring or Chunked 2^16 containers) are hit.
    /// A partially filled final container holds only its first
    /// `count % per_chunk` positions and therefore its start code but not
    /// its end code. The final `u64` container is supported; a request past
    /// it is rejected with `Overflow("chunk code")`.
    Chunks {
        first_chunk: u64,
        chunk_bits: u32,
        per_chunk: u64,
    },
}

impl Layout {
    fn id(self) -> String {
        match self {
            Self::Uniform { base, span } => format!("uniform:base={base}:span={span}"),
            Self::Stride { base, step } => format!("stride:base={base}:step={step}"),
            Self::Runs { base, run, gap } => format!("runs:base={base}:run={run}:gap={gap}"),
            Self::Chunks {
                first_chunk,
                chunk_bits,
                per_chunk,
            } => format!("chunks:first={first_chunk}:bits={chunk_bits}:per={per_chunk}"),
        }
    }

    /// Validate the layout parameters (always, even for an empty output) and
    /// return the inclusive upper bound of the `count` generated codes, which
    /// is the last code itself (`None` when `count == 0`). Every bound is
    /// computed with checked arithmetic; nothing here can wrap.
    fn validate(self, count: u64) -> Result<Option<u64>, WorkloadError> {
        match self {
            Self::Uniform { base, span } => {
                if span == 0 {
                    return Err(WorkloadError::ZeroParameter("span"));
                }
                if span < count {
                    return Err(WorkloadError::SpanTooSmall {
                        needed: count,
                        span,
                    });
                }
                base.checked_add(span - 1)
                    .ok_or(WorkloadError::Overflow("base + span"))?;
                // The drawn codes are distinct members of `base..base + span`.
                Ok((count > 0).then(|| base + (span - 1)))
            }
            Self::Stride { base, step } => {
                if step == 0 {
                    return Err(WorkloadError::ZeroParameter("step"));
                }
                if count == 0 {
                    return Ok(None);
                }
                let offset = (count - 1)
                    .checked_mul(step)
                    .ok_or(WorkloadError::Overflow("stride offset"))?;
                base.checked_add(offset)
                    .map(Some)
                    .ok_or(WorkloadError::Overflow("stride code"))
            }
            Self::Runs { base, run, gap } => {
                if run == 0 {
                    return Err(WorkloadError::ZeroParameter("run"));
                }
                let period = run
                    .checked_add(gap)
                    .ok_or(WorkloadError::Overflow("run + gap"))?;
                if count == 0 {
                    return Ok(None);
                }
                let last_index = count - 1;
                let offset = (last_index / run)
                    .checked_mul(period)
                    .and_then(|v| v.checked_add(last_index % run))
                    .ok_or(WorkloadError::Overflow("run offset"))?;
                base.checked_add(offset)
                    .map(Some)
                    .ok_or(WorkloadError::Overflow("run code"))
            }
            Self::Chunks {
                first_chunk,
                chunk_bits,
                per_chunk,
            } => {
                if !(1..=63).contains(&chunk_bits) {
                    return Err(WorkloadError::ChunkBitsOutOfRange(chunk_bits));
                }
                if per_chunk == 0 {
                    return Err(WorkloadError::ZeroParameter("per_chunk"));
                }
                if per_chunk > 1_u64 << chunk_bits {
                    return Err(WorkloadError::PerChunkExceedsChunk {
                        per_chunk,
                        chunk_bits,
                    });
                }
                // The first container coordinate must itself be addressable,
                // even when no code is generated.
                chunk_code(first_chunk, chunk_bits, per_chunk, 0)?;
                if count == 0 {
                    return Ok(None);
                }
                chunk_code(first_chunk, chunk_bits, per_chunk, count - 1).map(Some)
            }
        }
    }

    /// Generate `count` distinct ascending codes. `validate` must have passed.
    fn generate(self, count: u64, rng: &mut Rng) -> Vec<u64> {
        let count_usize = usize::try_from(count).expect("validated count fits usize");
        match self {
            Self::Uniform { base, span } => {
                // Floyd's algorithm (Bentley & Floyd, CACM 1987): exactly
                // `count` draws, each from a growing prefix of the span.
                let mut chosen = BTreeSet::new();
                for j in (span - count)..span {
                    let t = rng.below(j + 1);
                    if !chosen.insert(base + t) {
                        chosen.insert(base + j);
                    }
                }
                debug_assert_eq!(chosen.len(), count_usize);
                chosen.into_iter().collect()
            }
            Self::Stride { base, step } => (0..count).map(|i| base + i * step).collect(),
            Self::Runs { base, run, gap } => (0..count)
                .map(|i| base + (i / run) * (run + gap) + i % run)
                .collect(),
            Self::Chunks {
                first_chunk,
                chunk_bits,
                per_chunk,
            } => (0..count)
                .map(|i| {
                    chunk_code(first_chunk, chunk_bits, per_chunk, i)
                        .expect("validated last code bounds every earlier code")
                })
                .collect(),
        }
    }
}

/// The `index`-th code of a `Chunks` layout, with checked arithmetic. Codes
/// are monotonic in `index`, so validating the last index bounds all others.
fn chunk_code(
    first_chunk: u64,
    chunk_bits: u32,
    per_chunk: u64,
    index: u64,
) -> Result<u64, WorkloadError> {
    let last_offset = (1_u64 << chunk_bits) - 1;
    let chunk = first_chunk
        .checked_add(index / per_chunk)
        .ok_or(WorkloadError::Overflow("chunk index"))?;
    let j = index % per_chunk;
    let offset = if per_chunk >= 2 && j == per_chunk - 1 {
        last_offset
    } else if per_chunk >= 2 {
        j * (last_offset / (per_chunk - 1))
    } else {
        0
    };
    chunk
        .checked_shl(chunk_bits)
        .filter(|_| chunk >> (64 - chunk_bits) == 0)
        .and_then(|start| start.checked_add(offset))
        .ok_or(WorkloadError::Overflow("chunk code"))
}

/// How the union's codes are assigned to left-only, shared, and right-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Partition {
    /// Classes are spread as evenly as the counts allow across the ascending
    /// union (an exact rational merge, not a repeating pattern).
    Interleaved,
    /// Contiguous blocks in the named order, for head/tail/separated shapes.
    Blocks([Class; 3]),
    /// A seeded Fisher-Yates permutation of the exact class multiset.
    Shuffled,
}

impl Partition {
    fn id(self) -> String {
        match self {
            Self::Interleaved => "interleaved".to_string(),
            Self::Blocks(order) => format!(
                "blocks:{}{}{}",
                order[0].tag(),
                order[1].tag(),
                order[2].tag()
            ),
            Self::Shuffled => "shuffled".to_string(),
        }
    }

    fn validate(self) -> Result<(), WorkloadError> {
        if let Self::Blocks(order) = self {
            let mut seen = [false; 3];
            for class in order {
                seen[class as usize] = true;
            }
            if seen != [true; 3] {
                return Err(WorkloadError::InvalidBlockOrder(order));
            }
        }
        Ok(())
    }

    fn assign(self, counts: [(Class, usize); 3], rng: &mut Rng) -> Vec<Class> {
        let total: usize = counts.iter().map(|(_, n)| n).sum();
        match self {
            Self::Interleaved => {
                // Position the k-th member of a class with `n` members at the
                // rational `(2k + 1) / (2n)` of the sequence and merge all
                // classes by that key; ties keep the class order.
                let mut keyed: Vec<(u128, u128, Class)> = Vec::with_capacity(total);
                for (class, n) in counts {
                    for k in 0..n {
                        keyed.push((2 * k as u128 + 1, 2 * n as u128, class));
                    }
                }
                keyed.sort_by(|(an, ad, ac), (bn, bd, bc)| {
                    (an * bd).cmp(&(bn * ad)).then(ac.cmp(bc))
                });
                keyed.into_iter().map(|(_, _, class)| class).collect()
            }
            Self::Blocks(order) => {
                let mut classes = Vec::with_capacity(total);
                for class in order {
                    let (_, n) = counts
                        .iter()
                        .find(|(candidate, _)| *candidate == class)
                        .expect("validated block order names every class");
                    classes.extend(std::iter::repeat_n(class, *n));
                }
                classes
            }
            Self::Shuffled => {
                let mut classes = Vec::with_capacity(total);
                for (class, n) in counts {
                    classes.extend(std::iter::repeat_n(class, n));
                }
                rng.shuffle(&mut classes);
                classes
            }
        }
    }
}

/// Emission order of one side's values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Order {
    Ascending,
    Descending,
    Shuffled,
}

impl Order {
    fn tag(self) -> &'static str {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
            Self::Shuffled => "shuf",
        }
    }
}

/// How one side is emitted: its order and how many extra copies of existing
/// members are appended before ordering. Copy `k` duplicates member `k mod n`
/// of the ascending unique members, so the duplicate set is exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Emit {
    pub(crate) order: Order,
    pub(crate) duplicates: usize,
}

impl Emit {
    pub(crate) const ASCENDING: Self = Self {
        order: Order::Ascending,
        duplicates: 0,
    };

    pub(crate) const SHUFFLED: Self = Self {
        order: Order::Shuffled,
        duplicates: 0,
    };

    fn id(self) -> String {
        format!("{}+{}", self.order.tag(), self.duplicates)
    }

    /// Reject an impossible emission for a side with `unique` distinct members
    /// and return the emitted length. Called before any generation.
    fn validate(self, unique: usize, side: Side) -> Result<usize, WorkloadError> {
        if self.duplicates > 0 && unique == 0 {
            return Err(WorkloadError::DuplicatesOfEmptySide(side));
        }
        let total = unique
            .checked_add(self.duplicates)
            .ok_or(WorkloadError::Overflow("side length"))?;
        bounded(total)
    }

    fn apply(self, unique: &[u64], side: Side, rng: &mut Rng) -> Result<Vec<u64>, WorkloadError> {
        let total = self.validate(unique.len(), side)?;
        let mut values = Vec::with_capacity(total);
        values.extend_from_slice(unique);
        for k in 0..self.duplicates {
            values.push(unique[k % unique.len()]);
        }
        match self.order {
            Order::Ascending => values.sort_unstable(),
            Order::Descending => {
                values.sort_unstable();
                values.reverse();
            }
            Order::Shuffled => rng.shuffle(&mut values),
        }
        Ok(values)
    }
}

/// A specification that can build one deterministic workload. Implementors
/// for new domains (maps, sequences, strings) follow the same contract: the
/// id embeds every parameter, `build` validates before generating, and the
/// output carries its oracle inputs.
pub(crate) trait WorkloadSpec: fmt::Debug + Clone {
    type Output;

    /// A reproducible case id containing every parameter, including the seed.
    /// It contains no commas, so it can be placed in a CSV field.
    fn id(&self) -> String;

    fn build(&self) -> Result<Self::Output, WorkloadError>;
}

/// Two sets with an exact number of shared members.
///
/// Skew is expressed by `left` versus `right`; density by the layout; overlap
/// by `shared`; ordering and duplicates by each side's `Emit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetPairSpec {
    pub(crate) seed: u64,
    /// Distinct members on the left side.
    pub(crate) left: usize,
    /// Distinct members on the right side.
    pub(crate) right: usize,
    /// Exact size of the intersection of the distinct members.
    pub(crate) shared: usize,
    pub(crate) layout: Layout,
    pub(crate) partition: Partition,
    pub(crate) left_emit: Emit,
    pub(crate) right_emit: Emit,
}

impl SetPairSpec {
    /// Ascending emission, interleaved partition: the shape most benchmarks use.
    pub(crate) fn new(seed: u64, left: usize, right: usize, shared: usize, layout: Layout) -> Self {
        Self {
            seed,
            left,
            right,
            shared,
            layout,
            partition: Partition::Interleaved,
            left_emit: Emit::ASCENDING,
            right_emit: Emit::ASCENDING,
        }
    }

    pub(crate) fn with_partition(mut self, partition: Partition) -> Self {
        self.partition = partition;
        self
    }

    pub(crate) fn with_emit(mut self, left: Emit, right: Emit) -> Self {
        self.left_emit = left;
        self.right_emit = right;
        self
    }

    fn union_len(&self) -> Result<usize, WorkloadError> {
        if self.shared > self.left.min(self.right) {
            return Err(WorkloadError::SharedExceedsSide {
                shared: self.shared,
                left: self.left,
                right: self.right,
            });
        }
        self.left
            .checked_add(self.right)
            .map(|sum| sum - self.shared)
            .ok_or(WorkloadError::Overflow("left + right"))
            .and_then(bounded)
    }
}

fn bounded(requested: usize) -> Result<usize, WorkloadError> {
    if requested > MAX_MEMBERS {
        Err(WorkloadError::ExceedsSupportedSize {
            requested,
            limit: MAX_MEMBERS,
        })
    } else {
        Ok(requested)
    }
}

impl WorkloadSpec for SetPairSpec {
    type Output = SetPair;

    fn id(&self) -> String {
        format!(
            "seed={};left={};right={};shared={};layout={};partition={};lemit={};remit={}",
            self.seed,
            self.left,
            self.right,
            self.shared,
            self.layout.id(),
            self.partition.id(),
            self.left_emit.id(),
            self.right_emit.id()
        )
    }

    fn build(&self) -> Result<SetPair, WorkloadError> {
        let union_len = self.union_len()?;
        let union_count = u64::try_from(union_len).map_err(|_| WorkloadError::Overflow("union"))?;
        let max_code = self.layout.validate(union_count)?;
        self.partition.validate()?;
        // Emission bounds are checked before the union is generated so an
        // impossible or oversized side never triggers an allocation.
        self.left_emit.validate(self.left, Side::Left)?;
        self.right_emit.validate(self.right, Side::Right)?;
        let mut seeds = Rng::new(self.seed);
        let mut layout_rng = Rng::new(seeds.next_u64());
        let mut partition_rng = Rng::new(seeds.next_u64());
        let mut left_rng = Rng::new(seeds.next_u64());
        let mut right_rng = Rng::new(seeds.next_u64());

        let union = self.layout.generate(union_count, &mut layout_rng);
        let classes = self.partition.assign(
            [
                (Class::LeftOnly, self.left - self.shared),
                (Class::Shared, self.shared),
                (Class::RightOnly, self.right - self.shared),
            ],
            &mut partition_rng,
        );
        let mut left_unique = Vec::with_capacity(self.left);
        let mut right_unique = Vec::with_capacity(self.right);
        let mut shared = Vec::with_capacity(self.shared);
        for (&code, &class) in union.iter().zip(&classes) {
            match class {
                Class::LeftOnly => left_unique.push(code),
                Class::Shared => {
                    left_unique.push(code);
                    right_unique.push(code);
                    shared.push(code);
                }
                Class::RightOnly => right_unique.push(code),
            }
        }
        let left = self
            .left_emit
            .apply(&left_unique, Side::Left, &mut left_rng)?;
        let right = self
            .right_emit
            .apply(&right_unique, Side::Right, &mut right_rng)?;
        Ok(SetPair {
            id: self.id(),
            spec: self.clone(),
            max_code,
            left,
            right,
            left_unique,
            right_unique,
            shared,
        })
    }
}

/// A built pair. `left` and `right` are the emitted inputs (with duplicates
/// and ordering applied); the `*_unique` and `shared` vectors are ascending
/// and describe the exact structure the spec guaranteed.
#[derive(Clone, Debug)]
pub(crate) struct SetPair {
    pub(crate) id: String,
    pub(crate) spec: SetPairSpec,
    /// Inclusive upper bound of every generated code, `None` when empty.
    pub(crate) max_code: Option<u64>,
    pub(crate) left: Vec<u64>,
    pub(crate) right: Vec<u64>,
    pub(crate) left_unique: Vec<u64>,
    pub(crate) right_unique: Vec<u64>,
    pub(crate) shared: Vec<u64>,
}

impl SetPair {
    /// Independent oracle over the emitted inputs.
    pub(crate) fn oracle(&self) -> SetOracle<u64> {
        SetOracle::from_slices(&self.left, &self.right)
    }

    /// Reinterpret the codes in another key domain (for example `UserId` or
    /// a shifted signed range). The mapping must be injective to preserve the
    /// exact guarantees; the oracle is recomputed from the mapped inputs.
    pub(crate) fn map<K: Ord + Clone>(&self, f: impl Fn(u64) -> K) -> MappedPair<K> {
        MappedPair {
            id: self.id.clone(),
            left: self.left.iter().map(|&v| f(v)).collect(),
            right: self.right.iter().map(|&v| f(v)).collect(),
        }
    }

    /// Checked narrowing to `u32`; rejects instead of truncating.
    pub(crate) fn to_u32(&self) -> Result<MappedPair<u32>, WorkloadError> {
        self.try_map(|v| {
            u32::try_from(v).map_err(|_| WorkloadError::ValueOutOfRange {
                value: v,
                target: "u32",
            })
        })
    }

    pub(crate) fn try_map<K: Ord + Clone>(
        &self,
        f: impl Fn(u64) -> Result<K, WorkloadError>,
    ) -> Result<MappedPair<K>, WorkloadError> {
        Ok(MappedPair {
            id: self.id.clone(),
            left: self.left.iter().map(|&v| f(v)).collect::<Result<_, _>>()?,
            right: self.right.iter().map(|&v| f(v)).collect::<Result<_, _>>()?,
        })
    }
}

/// A pair in a caller-chosen key domain.
#[derive(Clone, Debug)]
pub(crate) struct MappedPair<K> {
    pub(crate) id: String,
    pub(crate) left: Vec<K>,
    pub(crate) right: Vec<K>,
}

impl<K: Ord + Clone> MappedPair<K> {
    pub(crate) fn oracle(&self) -> SetOracle<K> {
        SetOracle::from_slices(&self.left, &self.right)
    }
}

/// Reference answers computed with `std::collections::BTreeSet`. Build it
/// before timing; compare inside tests, never inside a measured kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetOracle<K> {
    pub(crate) left: Vec<K>,
    pub(crate) right: Vec<K>,
    pub(crate) intersection: Vec<K>,
    pub(crate) union: Vec<K>,
    pub(crate) difference: Vec<K>,
    pub(crate) symmetric_difference: Vec<K>,
    pub(crate) is_subset: bool,
    pub(crate) is_superset: bool,
    pub(crate) is_disjoint: bool,
}

impl<K: Ord + Clone> SetOracle<K> {
    pub(crate) fn from_slices(left: &[K], right: &[K]) -> Self {
        let l: BTreeSet<K> = left.iter().cloned().collect();
        let r: BTreeSet<K> = right.iter().cloned().collect();
        Self {
            intersection: l.intersection(&r).cloned().collect(),
            union: l.union(&r).cloned().collect(),
            difference: l.difference(&r).cloned().collect(),
            symmetric_difference: l.symmetric_difference(&r).cloned().collect(),
            is_subset: l.is_subset(&r),
            is_superset: l.is_superset(&r),
            is_disjoint: l.is_disjoint(&r),
            left: l.into_iter().collect(),
            right: r.into_iter().collect(),
        }
    }
}

/// One sequence of codes: the single-side counterpart of `SetPairSpec`, for
/// construction workloads and map keys. `duplicates` make last-value-wins
/// observable through `entries`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SequenceSpec {
    pub(crate) seed: u64,
    pub(crate) unique: usize,
    pub(crate) layout: Layout,
    pub(crate) emit: Emit,
}

impl WorkloadSpec for SequenceSpec {
    type Output = Sequence;

    fn id(&self) -> String {
        format!(
            "seed={};unique={};layout={};emit={}",
            self.seed,
            self.unique,
            self.layout.id(),
            self.emit.id()
        )
    }

    fn build(&self) -> Result<Sequence, WorkloadError> {
        let pair = SetPairSpec {
            seed: self.seed,
            left: self.unique,
            right: 0,
            shared: 0,
            layout: self.layout,
            partition: Partition::Interleaved,
            left_emit: self.emit,
            right_emit: Emit::ASCENDING,
        }
        .build()?;
        Ok(Sequence {
            id: self.id(),
            max_code: pair.max_code,
            unique: pair.left_unique,
            values: pair.left,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Sequence {
    pub(crate) id: String,
    pub(crate) max_code: Option<u64>,
    /// Emitted values, including duplicates, in emission order.
    pub(crate) values: Vec<u64>,
    /// Ascending distinct members.
    pub(crate) unique: Vec<u64>,
}

impl Sequence {
    /// `(key, position)` entries in emission order. For a family that keeps
    /// the last value, the expected value of a key is its last position.
    pub(crate) fn entries(&self) -> Vec<(u64, usize)> {
        self.values.iter().copied().zip(0..).collect()
    }

    /// The expected map after inserting `entries` with last-value-wins.
    pub(crate) fn last_value_oracle(&self) -> std::collections::BTreeMap<u64, usize> {
        self.entries().into_iter().collect()
    }
}
