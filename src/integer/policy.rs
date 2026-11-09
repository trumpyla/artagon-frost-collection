use core::fmt;

/// Requested representation for construction or an explicit operation result.
///
/// This does not expose a backend object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IntSetBackend {
    /// Choose with the bounded, deterministic memory-oriented planner.
    #[default]
    Auto,
    /// Store strictly ascending unsigned codes.
    Sorted,
    /// Store a dense local window, subject to the byte budget.
    Dense,
    /// Use the width-specific Rust Roaring collection, subject to its partition cap.
    Roaring,
    /// Store 2^16-code chunks as immutable shared containers. Algebra results
    /// share untouched containers with their operands.
    Chunked,
}

/// Observed representation, for diagnostics and workload measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IntSetRepr {
    /// Sorted unsigned codes.
    Sorted,
    /// A local window of machine-independent 64-bit bitmap words.
    Dense,
    /// Width-specific Roaring containers.
    Roaring,
    /// Shared immutable chunk containers.
    Chunked,
}

/// Representation limits retained by a frozen integer set and its results.
///
/// These cap dense payload bytes and Roaring upper-32-bit partitions. They do
/// not cap sorted or chunked storage, temporary allocations, or total process memory.
/// Union, intersection, and symmetric difference take the smaller limit from
/// each operand; difference keeps the left operand's limits. Explicit
/// `*_with_policy` operations replace these limits for their returned result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IntSetLimits {
    max_dense_bytes: usize,
    max_roaring_partitions: usize,
}

impl IntSetLimits {
    /// Maximum dense bitmap payload in bytes.
    pub const fn max_dense_bytes(self) -> usize {
        self.max_dense_bytes
    }

    /// Maximum distinct upper-32-bit partitions in a Roaring result.
    pub const fn max_roaring_partitions(self) -> usize {
        self.max_roaring_partitions
    }

    pub(crate) fn common(self, other: Self) -> Self {
        Self {
            max_dense_bytes: self.max_dense_bytes.min(other.max_dense_bytes),
            max_roaring_partitions: self
                .max_roaring_partitions
                .min(other.max_roaring_partitions),
        }
    }

    pub(crate) const fn automatic(self) -> IntSetPolicy {
        IntSetPolicy {
            backend: IntSetBackend::Auto,
            max_dense_bytes: self.max_dense_bytes,
            max_roaring_partitions: self.max_roaring_partitions,
        }
    }
}

impl Default for IntSetLimits {
    fn default() -> Self {
        IntSetPolicy::auto().limits()
    }
}

/// Explicit integer freeze plan and bounded storage-selection policy.
///
/// The dense budget limits bitmap payload bytes, not input staging, sorting,
/// allocator overhead, or total process memory. The Roaring cap bounds distinct
/// upper-32-bit partitions; it prevents automatic treemap explosion for scattered
/// `u64` values. These controls are not universal throughput recommendations.
/// Allocation failure follows ordinary Rust allocation behavior.
///
/// Constructed sets retain these two limits for algebra. The requested backend
/// controls this construction only. Result operations may select another legal
/// representation; their `*_with_policy` forms can explicitly override both the
/// representation and limits for one result.
///
/// ```
/// use frost::{TryFreezeWith, integer::IntSetPolicy};
/// let frozen = vec![3_i32, -1, 3].try_freeze_with(IntSetPolicy::auto()).unwrap();
/// assert_eq!(frozen.iter().collect::<Vec<_>>(), [-1, 3]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IntSetPolicy {
    pub(crate) backend: IntSetBackend,
    pub(crate) max_dense_bytes: usize,
    pub(crate) max_roaring_partitions: usize,
}

impl IntSetPolicy {
    /// Automatic choice, at most 8 MiB dense payload and 4,096 Roaring partitions.
    pub const fn auto() -> Self {
        Self {
            backend: IntSetBackend::Auto,
            max_dense_bytes: 8 * 1024 * 1024,
            max_roaring_partitions: 4096,
        }
    }
    /// Explicit sorted storage.
    pub const fn sorted() -> Self {
        Self {
            backend: IntSetBackend::Sorted,
            ..Self::auto()
        }
    }
    /// Explicit dense storage; an excessive span is a checked error.
    pub const fn dense() -> Self {
        Self {
            backend: IntSetBackend::Dense,
            ..Self::auto()
        }
    }
    /// Explicit Roaring storage; an excessive partition count is a checked error.
    pub const fn roaring() -> Self {
        Self {
            backend: IntSetBackend::Roaring,
            ..Self::auto()
        }
    }
    /// Explicit chunked storage with structurally shared containers.
    ///
    /// Cloning and algebra share untouched 2^16-code containers between
    /// versions, which suits snapshot-style workloads deriving many sets from
    /// a common ancestor. Automatic construction never selects it. Neither the
    /// dense-byte limit nor the Roaring-partition limit caps chunked storage;
    /// each chunk carries index and allocation overhead, so scattered codes may
    /// be better served by sorted storage.
    pub const fn chunked() -> Self {
        Self {
            backend: IntSetBackend::Chunked,
            ..Self::auto()
        }
    }
    /// Set the dense payload allocation limit in bytes.
    #[must_use]
    pub const fn with_dense_budget(self, bytes: usize) -> Self {
        Self {
            max_dense_bytes: bytes,
            ..self
        }
    }
    /// Set the number of upper-32-bit partitions eligible for Roaring storage.
    #[must_use]
    pub const fn with_roaring_partition_limit(self, partitions: usize) -> Self {
        Self {
            max_roaring_partitions: partitions,
            ..self
        }
    }
    /// Requested representation for construction or an explicit operation result.
    pub const fn backend(self) -> IntSetBackend {
        self.backend
    }
    /// Dense payload limit in bytes.
    pub const fn max_dense_bytes(self) -> usize {
        self.max_dense_bytes
    }
    /// Roaring upper-32-bit partition limit.
    pub const fn max_roaring_partitions(self) -> usize {
        self.max_roaring_partitions
    }

    /// Representation limits retained by a set constructed with this policy.
    pub const fn limits(self) -> IntSetLimits {
        IntSetLimits {
            max_dense_bytes: self.max_dense_bytes,
            max_roaring_partitions: self.max_roaring_partitions,
        }
    }
}

impl Default for IntSetPolicy {
    fn default() -> Self {
        Self::auto()
    }
}

/// Checked construction failure. No variant promises recovery from allocator OOM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IntSetError {
    /// A key did not survive its observed encode/decode round trip.
    CodecRoundTrip {
        /// Zero-based input position.
        index: usize,
    },
    /// An encoded input was rejected by its key type.
    InvalidCode {
        /// Zero-based position in the validation pass: original input, or the
        /// sorted unique sequence if an unstable codec fails revalidation.
        index: usize,
    },
    /// Decoding and re-encoding changed an accepted code.
    NonCanonicalCode {
        /// Zero-based position in the validation pass: original input, or the
        /// sorted unique sequence if an unstable codec fails revalidation.
        index: usize,
    },
    /// Adjacent distinct codes did not decode to strictly ascending keys.
    NonMonotonicEncoding {
        /// Position in the sorted unique code sequence.
        index: usize,
    },
    /// The dense window exceeds its explicit payload limit.
    DenseBudgetExceeded {
        /// Required bitmap payload bytes, computed without narrowing.
        required_bytes: u128,
        /// Effective byte limit, capped by the target's addressable allocation.
        max_bytes: usize,
    },
    /// The Roaring candidate would contain too many upper-32-bit partitions.
    RoaringPartitionLimit {
        /// Number of distinct partitions.
        partitions: u64,
        /// Configured limit.
        max_partitions: usize,
    },
    /// A cardinality would exceed the public `u64` domain.
    CardinalityOverflow,
}

impl fmt::Display for IntSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CodecRoundTrip { index } => {
                write!(f, "integer key at input {index} did not round-trip")
            }
            Self::InvalidCode { index } => {
                write!(f, "invalid integer code at validation position {index}")
            }
            Self::NonCanonicalCode { index } => {
                write!(
                    f,
                    "noncanonical integer code at validation position {index}"
                )
            }
            Self::NonMonotonicEncoding { index } => write!(
                f,
                "integer codec violates ascending order at normalized position {index}"
            ),
            Self::DenseBudgetExceeded {
                required_bytes,
                max_bytes,
            } => write!(
                f,
                "dense payload requires {required_bytes} bytes; limit is {max_bytes}"
            ),
            Self::RoaringPartitionLimit {
                partitions,
                max_partitions,
            } => write!(
                f,
                "Roaring requires {partitions} upper-32-bit partitions; limit is {max_partitions}"
            ),
            Self::CardinalityOverflow => f.write_str("integer set cardinality exceeds u64"),
        }
    }
}
impl core::error::Error for IntSetError {}

/// Distribution of encoded members. Querying this scans the encoded values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IntSetStatistics {
    /// Number of members, independent of the logical span.
    pub cardinality: u64,
    /// Inclusive encoded bounds, or `None` for an empty set.
    pub encoded_bounds: Option<(u64, u64)>,
    /// Inclusive span; `u128` represents the complete `u64` universe.
    pub span: u128,
    /// Number of maximal consecutive runs of encoded members.
    pub runs: u64,
    /// Number of distinct upper-32-bit partitions.
    pub high32_partitions: u64,
}
