//! The Air trait: the core abstraction for algebraic intermediate representations.
//!
//! An [`Air`] defines a set of columns and transition constraints
//! that must hold at every consecutive row pair of an execution trace.
//! This is the STARK analog of a plonkish-cat gate, operating on
//! trace tables rather than individual wires.

use crate::air_expr::AirExpr;
use crate::column::ColumnCount;
use crate::error::Error;
use crate::trace::Trace;
use plonkish_cat::Field;

/// An Algebraic Intermediate Representation.
///
/// Defines the **columns** (trace width) and **transition constraints**
/// (polynomial expressions that must be zero at every consecutive
/// row pair) for a provable computation.
///
/// Implementations also provide [`generate_trace`](Air::generate_trace)
/// to compute the execution trace from a given input.
///
/// # Categorical interpretation
///
/// Each `Air` is a morphism in a category where objects are
/// trace shapes ([`ColumnCount`]).  Parallel composition of
/// independent AIRs corresponds to the tensor product (column
/// concatenation).  This structure is documented here but not
/// encoded at the type level in v0.1; see the machine-cat
/// roadmap for future categorical enforcement.
pub trait Air<F: Field> {
    /// The type of input this AIR computes over.
    type Input;

    /// The number of columns in this AIR's trace.
    fn column_count(&self) -> ColumnCount;

    /// The transition constraints.
    ///
    /// Each [`AirExpr`] must evaluate to zero for every consecutive
    /// row pair `(row[i], row[i+1])` in a valid trace.
    fn constraints(&self) -> Vec<AirExpr<F>>;

    /// Generate the execution trace from an input.
    ///
    /// The returned trace must have [`column_count()`](Air::column_count)
    /// columns.  The number of rows is determined by the input.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is invalid or trace generation
    /// fails.
    fn generate_trace(&self, input: &Self::Input) -> Result<Trace<F>, Error>;
}
