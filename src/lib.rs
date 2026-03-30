//! machine-cat: generic AIR chip framework built on proof-cat.
//!
//! Generalizes from plonkish-cat's wire-indexed constraints to
//! **execution trace tables** where constraint polynomials must
//! hold at every consecutive row pair.  This is the standard
//! STARK/AIR (Algebraic Intermediate Representation) model.
//!
//! # Architecture
//!
//! ```text
//! Air::generate_trace(input) -> Trace<F>
//!              |
//!    bridge::air_prove(air, trace) -> AirProof<F>
//!              |
//!    bridge::air_verify(air, proof) -> Ok(true)
//! ```
//!
//! The [`Air`] trait defines columns and transition constraints.
//! [`Trace`] is the 2D execution witness.  The [`bridge`] module
//! converts constraint satisfaction into a sumcheck proof via
//! proof-cat.
//!
//! # Modules
//!
//! - [`column`] -- `Column`, `ColumnCount`, `ColumnRef` newtypes
//! - [`air_expr`] -- `AirExpr<F>`: constraint expressions with row-relative addressing
//! - [`trace`] -- `Trace<F>`: 2D table of field elements
//! - [`air`] -- `Air<F>` trait: the core abstraction
//! - [`fibonacci`] -- `FibonacciAir`: a concrete example
//! - [`bridge`] -- `air_prove` / `air_verify`: trace-to-sumcheck bridge

pub mod air;
pub mod air_expr;
pub mod bridge;
pub mod column;
pub mod error;
pub mod fibonacci;
pub mod trace;

pub use air::Air;
pub use air_expr::AirExpr;
pub use column::{Column, ColumnCount, ColumnRef};
pub use error::Error;
pub use fibonacci::{FibonacciAir, FibonacciInput, StepCount};
pub use trace::{RowCount, Trace};
