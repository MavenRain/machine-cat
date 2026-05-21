//! Project-wide error type.

/// All errors that can arise in machine-cat.
#[derive(Debug)]
pub enum Error {
    /// An error propagated from proof-cat.
    ProofCat(proof_cat::Error),
    /// An error propagated from plonkish-cat.
    Plonkish(plonkish_cat::Error),
    /// An error propagated from field-cat (field arithmetic or byte encoding).
    FieldCat(field_cat::Error),
    /// Column index out of bounds.
    ColumnOutOfBounds {
        /// The column index that was accessed.
        index: usize,
        /// The total number of columns.
        column_count: usize,
    },
    /// Trace has zero rows.
    EmptyTrace,
    /// Trace column count does not match the AIR's column count.
    ColumnCountMismatch {
        /// The expected column count (from the AIR).
        expected: usize,
        /// The actual column count (from the trace).
        actual: usize,
    },
    /// Trace row count is not a power of two.
    TraceNotPowerOfTwo {
        /// The row count.
        row_count: usize,
    },
    /// An AIR constraint was not satisfied at a row pair.
    UnsatisfiedAirConstraint {
        /// The row index of the failing pair.
        row: usize,
    },
    /// AIR has no constraints.
    NoConstraints,
    /// Trace has fewer than 2 rows.
    InsufficientRows {
        /// The row count.
        row_count: usize,
    },
    /// Row length does not match column count.
    RowLengthMismatch {
        /// The row index.
        row: usize,
        /// The expected length.
        expected: usize,
        /// The actual length.
        actual: usize,
    },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ProofCat(e) => write!(f, "proof-cat error: {e}"),
            Self::Plonkish(e) => write!(f, "plonkish-cat error: {e}"),
            Self::FieldCat(e) => write!(f, "field-cat error: {e}"),
            Self::ColumnOutOfBounds {
                index,
                column_count,
            } => write!(
                f,
                "column index {index} out of bounds (column count: {column_count})"
            ),
            Self::EmptyTrace => write!(f, "trace has zero rows"),
            Self::ColumnCountMismatch { expected, actual } => {
                write!(
                    f,
                    "column count mismatch: expected {expected}, got {actual}"
                )
            }
            Self::TraceNotPowerOfTwo { row_count } => {
                write!(f, "trace row count {row_count} is not a power of two")
            }
            Self::UnsatisfiedAirConstraint { row } => {
                write!(
                    f,
                    "AIR constraint not satisfied at row pair ({row}, {})",
                    row + 1
                )
            }
            Self::NoConstraints => write!(f, "AIR has no constraints"),
            Self::InsufficientRows { row_count } => {
                write!(f, "trace has {row_count} rows, need at least 2")
            }
            Self::RowLengthMismatch {
                row,
                expected,
                actual,
            } => write!(f, "row {row} has {actual} elements, expected {expected}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProofCat(e) => Some(e),
            Self::Plonkish(e) => Some(e),
            Self::FieldCat(e) => Some(e),
            Self::ColumnOutOfBounds { .. }
            | Self::EmptyTrace
            | Self::ColumnCountMismatch { .. }
            | Self::TraceNotPowerOfTwo { .. }
            | Self::UnsatisfiedAirConstraint { .. }
            | Self::NoConstraints
            | Self::InsufficientRows { .. }
            | Self::RowLengthMismatch { .. } => None,
        }
    }
}

impl From<proof_cat::Error> for Error {
    fn from(e: proof_cat::Error) -> Self {
        Self::ProofCat(e)
    }
}

impl From<plonkish_cat::Error> for Error {
    fn from(e: plonkish_cat::Error) -> Self {
        Self::Plonkish(e)
    }
}

impl From<field_cat::Error> for Error {
    fn from(e: field_cat::Error) -> Self {
        Self::FieldCat(e)
    }
}
