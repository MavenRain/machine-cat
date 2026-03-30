//! Trace-to-sumcheck bridge: proving AIR constraint satisfaction.
//!
//! Converts an AIR's transition constraints over an execution trace
//! into a sumcheck claim and delegates to proof-cat for the proof.
//!
//! [`air_prove`] produces an [`AirProof`]; [`air_verify`] checks it.
//! The current protocol opens all trace values (not zero-knowledge).

use proof_cat::commit::merkle::{MerkleProof, MerkleRoot, MerkleTree};
use proof_cat::poly::MultilinearPoly;
use proof_cat::sumcheck::{SumcheckClaim, SumcheckProof, sumcheck_prove, sumcheck_verify};
use proof_cat::transcript::Transcript;
use proof_cat::FieldBytes;

use plonkish_cat::Field;

use crate::air::Air;
use crate::air_expr::AirExpr;
use crate::column::Column;
use crate::error::Error;
use crate::trace::Trace;

/// Domain separation label for the AIR proof transcript.
const TRANSCRIPT_LABEL: &[u8] = b"machine-cat-v0.1";

// ── Proof types ──────────────────────────────────────────────

/// An opened column: all values in a column with their Merkle proofs.
#[derive(Debug, Clone)]
pub struct ColumnOpening<F: Field> {
    column_index: usize,
    values: Vec<F>,
    merkle_proofs: Vec<MerkleProof>,
}

impl<F: Field> ColumnOpening<F> {
    /// The column index.
    #[must_use]
    pub fn column_index(&self) -> usize {
        self.column_index
    }

    /// The opened values (one per row).
    #[must_use]
    pub fn values(&self) -> &[F] {
        &self.values
    }

    /// The Merkle proofs (one per value).
    #[must_use]
    pub fn merkle_proofs(&self) -> &[MerkleProof] {
        &self.merkle_proofs
    }
}

/// A proof that an AIR's constraints hold over a given trace.
///
/// # Examples
///
/// ```
/// use plonkish_cat::F101;
/// use machine_cat::{Air, FibonacciAir, FibonacciInput, StepCount};
/// use machine_cat::bridge::{air_prove, air_verify};
///
/// let air = FibonacciAir;
/// let input = FibonacciInput::new(
///     F101::new(1), F101::new(1), StepCount::new(7),
/// );
/// let trace = air.generate_trace(&input)?;
///
/// let proof = air_prove(&air, &trace)?;
/// assert!(air_verify(&air, &proof)?);
/// # Ok::<(), machine_cat::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct AirProof<F: Field> {
    trace_commitment: MerkleRoot,
    sumcheck: SumcheckProof<F>,
    column_openings: Vec<ColumnOpening<F>>,
    row_count: usize,
}

impl<F: Field> AirProof<F> {
    /// The Merkle root committing to the trace.
    #[must_use]
    pub fn trace_commitment(&self) -> &MerkleRoot {
        &self.trace_commitment
    }

    /// The sumcheck proof.
    #[must_use]
    pub fn sumcheck(&self) -> &SumcheckProof<F> {
        &self.sumcheck
    }

    /// The opened column values with Merkle proofs.
    #[must_use]
    pub fn column_openings(&self) -> &[ColumnOpening<F>] {
        &self.column_openings
    }

    /// The trace row count.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.row_count
    }
}

// ── Prove ────────────────────────────────────────────────────

/// Prove that a trace satisfies an AIR's constraints.
///
/// # Protocol
///
/// 1. Validate the trace dimensions and constraint satisfaction.
/// 2. Commit the trace (all values, row-major) to a Merkle tree.
/// 3. Squeeze random challenges for batching constraints.
/// 4. Compute the random-linear-combination of constraint evaluations
///    at each row pair, producing a vector of length `N-1`.
/// 5. Pad to a power of two, build a multilinear polynomial, and
///    run sumcheck (claim: sum = 0).
/// 6. Open all trace column values with Merkle proofs.
///
/// # Errors
///
/// Returns an error if the trace does not satisfy the constraints,
/// or if any internal step fails.
pub fn air_prove<F: FieldBytes, A: Air<F>>(
    air: &A,
    trace: &Trace<F>,
) -> Result<AirProof<F>, Error> {
    // 1. Validate dimensions.
    validate_trace(air, trace)?;

    let constraints = air.constraints();
    if constraints.is_empty() {
        Err(Error::NoConstraints)
    } else {
        // 2. Validate constraint satisfaction at every row pair.
        validate_constraints(&constraints, trace)?;

        // 3. Commit trace (row-major flat data).
        let tree = MerkleTree::from_field_values(trace.data());

        // 4. Initialize transcript.
        let transcript = Transcript::new(TRANSCRIPT_LABEL)
            .absorb_bytes(tree.root().as_bytes())
            .absorb_bytes(&air.column_count().count().to_le_bytes())
            .absorb_bytes(&constraints.len().to_le_bytes());

        // 5. Squeeze combination challenges (one per constraint).
        let (alphas, transcript) = squeeze_challenges(constraints.len(), transcript)?;

        // 6. Compute combined constraint evaluations at each row pair.
        let combined_evals = compute_combined_evals(&constraints, &alphas, trace)?;

        // 7. Pad to power of two and build MLE.
        let padded = pad_to_power_of_two(combined_evals);
        let poly = MultilinearPoly::from_evals(padded)?;

        // 8. Run sumcheck.
        let (sumcheck, _, _) = sumcheck_prove(
            &SumcheckClaim::new(poly, F::zero()),
            transcript,
        )?;

        // 9. Open all columns.
        let column_openings = open_all_columns(air, trace, &tree)?;

        Ok(AirProof {
            trace_commitment: tree.root(),
            sumcheck,
            column_openings,
            row_count: trace.row_count().count(),
        })
    }
}

// ── Verify ───────────────────────────────────────────────────

/// Verify an AIR proof.
///
/// Replays the transcript, checks the sumcheck proof, verifies
/// all Merkle openings, and confirms the final evaluation matches.
///
/// # Errors
///
/// Returns an error if any verification step fails structurally.
pub fn air_verify<F: FieldBytes, A: Air<F>>(
    air: &A,
    proof: &AirProof<F>,
) -> Result<bool, Error> {
    let constraints = air.constraints();
    if constraints.is_empty() {
        Err(Error::NoConstraints)
    } else {
        // 1. Replay transcript.
        let transcript = Transcript::new(TRANSCRIPT_LABEL)
            .absorb_bytes(proof.trace_commitment.as_bytes())
            .absorb_bytes(&air.column_count().count().to_le_bytes())
            .absorb_bytes(&constraints.len().to_le_bytes());

        let (alphas, transcript) = squeeze_challenges(constraints.len(), transcript)?;

        // 2. Compute padded length and num_vars for sumcheck.
        let num_row_pairs = proof.row_count.saturating_sub(1);
        let padded_len = pad_to_power_of_two_len(num_row_pairs);
        let num_vars = usize::try_from(padded_len.trailing_zeros())
            .map_err(|_| Error::TraceNotPowerOfTwo { row_count: padded_len })?;

        // 3. Run sumcheck verifier.
        let (final_eval, challenges, _) = sumcheck_verify(
            proof.sumcheck(),
            &F::zero(),
            proof_cat::NumVars::new(num_vars),
            transcript,
        )?;

        // 4. Verify Merkle openings.
        if verify_merkle_openings(proof) {
            // 5. Reconstruct trace from openings and re-evaluate.
            let trace = reconstruct_trace(air, proof)?;
            let combined_evals = compute_combined_evals(&constraints, &alphas, &trace)?;
            let padded = pad_to_power_of_two(combined_evals);
            let poly = MultilinearPoly::from_evals(padded)?;

            // 6. Check MLE evaluation at challenges.
            let expected = poly.evaluate(&challenges)?;
            Ok(expected == final_eval)
        } else {
            Err(Error::ProofCat(proof_cat::Error::MerkleVerificationFailed))
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────

/// Validate that the trace matches the AIR's column count and has enough rows.
fn validate_trace<F: Field, A: Air<F>>(air: &A, trace: &Trace<F>) -> Result<(), Error> {
    if trace.column_count() != air.column_count() {
        Err(Error::ColumnCountMismatch {
            expected: air.column_count().count(),
            actual: trace.column_count().count(),
        })
    } else if trace.row_count().count() < 2 {
        Err(Error::InsufficientRows {
            row_count: trace.row_count().count(),
        })
    } else {
        Ok(())
    }
}

/// Validate that all constraints hold at every row pair.
fn validate_constraints<F: Field>(
    constraints: &[AirExpr<F>],
    trace: &Trace<F>,
) -> Result<(), Error> {
    (0..trace.row_count().count() - 1).try_for_each(|row| {
        let assign = trace.row_pair_assignment(row)?;
        constraints.iter().try_for_each(|c| {
            let val = c.evaluate(&assign)?;
            if val == F::zero() {
                Ok(())
            } else {
                Err(Error::UnsatisfiedAirConstraint { row })
            }
        })
    })
}

/// Squeeze `count` challenges from the transcript.
fn squeeze_challenges<F: FieldBytes>(
    count: usize,
    transcript: Transcript,
) -> Result<(Vec<F>, Transcript), Error> {
    (0..count).try_fold((Vec::with_capacity(count), transcript), |(alphas, t), _| {
        let (challenge, t): (F, Transcript) = t.squeeze_challenge()?;
        Ok((
            alphas.into_iter().chain(core::iter::once(challenge)).collect(),
            t,
        ))
    })
}

/// Compute the random-linear-combination of constraint evaluations.
///
/// For each row pair `(i, i+1)`, computes `sum_j alpha_j * P_j(row_i, row_{i+1})`.
fn compute_combined_evals<F: Field>(
    constraints: &[AirExpr<F>],
    alphas: &[F],
    trace: &Trace<F>,
) -> Result<Vec<F>, Error> {
    (0..trace.row_count().count() - 1)
        .map(|row| {
            let assign = trace.row_pair_assignment(row)?;
            constraints
                .iter()
                .zip(alphas.iter())
                .try_fold(F::zero(), |acc, (c, alpha)| {
                    let val = c.evaluate(&assign)?;
                    Ok(acc + alpha.clone() * val)
                })
        })
        .collect()
}

/// Open all columns from the Merkle tree.
fn open_all_columns<F: FieldBytes, A: Air<F>>(
    air: &A,
    trace: &Trace<F>,
    tree: &MerkleTree,
) -> Result<Vec<ColumnOpening<F>>, Error> {
    let cols = air.column_count().count();
    let rows = trace.row_count().count();
    (0..cols)
        .map(|col_idx| {
            let values = trace.column_values(Column::new(col_idx))?;
            let merkle_proofs: Result<Vec<MerkleProof>, Error> = (0..rows)
                .map(|row| {
                    let flat_idx = row * cols + col_idx;
                    tree.open(flat_idx).map_err(Error::from)
                })
                .collect();
            Ok(ColumnOpening {
                column_index: col_idx,
                values,
                merkle_proofs: merkle_proofs?,
            })
        })
        .collect()
}

/// Verify all Merkle openings in the proof.
fn verify_merkle_openings<F: FieldBytes>(proof: &AirProof<F>) -> bool {
    let cols = proof.column_openings.len();
    proof.column_openings.iter().all(|opening| {
        opening
            .values
            .iter()
            .enumerate()
            .all(|(row, value)| {
                let flat_idx = row * cols + opening.column_index;
                MerkleTree::verify_opening(
                    &proof.trace_commitment,
                    flat_idx,
                    value,
                    &opening.merkle_proofs[row],
                )
            })
    })
}

/// Reconstruct a Trace from the opened column values.
fn reconstruct_trace<F: Field, A: Air<F>>(
    air: &A,
    proof: &AirProof<F>,
) -> Result<Trace<F>, Error> {
    let cols = air.column_count().count();
    let rows = proof.row_count;
    let row_vecs: Vec<Vec<F>> = (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| {
                    proof.column_openings
                        .get(c)
                        .and_then(|opening| opening.values.get(r).cloned())
                        .ok_or(Error::ColumnOutOfBounds {
                            index: c,
                            column_count: cols,
                        })
                })
                .collect::<Result<Vec<F>, Error>>()
        })
        .collect::<Result<Vec<Vec<F>>, Error>>()?;
    Trace::from_rows(air.column_count(), &row_vecs)
}

/// Pad a vector to the next power of two with `F::zero()`.
fn pad_to_power_of_two<F: Field>(v: Vec<F>) -> Vec<F> {
    let target = pad_to_power_of_two_len(v.len());
    let padding = target - v.len();
    v.into_iter()
        .chain((0..padding).map(|_| F::zero()))
        .collect()
}

/// Next power of two >= n (minimum 1).
fn pad_to_power_of_two_len(n: usize) -> usize {
    if n <= 1 { 1 } else { n.next_power_of_two() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fibonacci::{FibonacciAir, FibonacciInput, StepCount};
    use plonkish_cat::F101;

    #[test]
    fn fibonacci_prove_verify_roundtrip() -> Result<(), Error> {
        let air = FibonacciAir;
        let input = FibonacciInput::new(F101::new(1), F101::new(1), StepCount::new(7));
        let trace = air.generate_trace(&input)?;

        let proof = air_prove(&air, &trace)?;
        assert!(air_verify(&air, &proof)?);
        Ok(())
    }

    #[test]
    fn fibonacci_small_trace() -> Result<(), Error> {
        let air = FibonacciAir;
        // Minimum: 2 rows (1 step).
        let input = FibonacciInput::new(F101::new(1), F101::new(1), StepCount::new(1));
        let trace = air.generate_trace(&input)?;

        let proof = air_prove(&air, &trace)?;
        assert!(air_verify(&air, &proof)?);
        Ok(())
    }

    #[test]
    fn fibonacci_different_initial_values() -> Result<(), Error> {
        let air = FibonacciAir;
        let input = FibonacciInput::new(F101::new(3), F101::new(5), StepCount::new(3));
        let trace = air.generate_trace(&input)?;

        let proof = air_prove(&air, &trace)?;
        assert!(air_verify(&air, &proof)?);
        Ok(())
    }

    #[test]
    fn invalid_trace_column_count_rejected() {
        let air = FibonacciAir;
        // 3 columns instead of 2.
        let trace = Trace::from_rows(
            crate::column::ColumnCount::new(3),
            &[
                vec![F101::new(1), F101::new(1), F101::new(0)],
                vec![F101::new(1), F101::new(2), F101::new(0)],
            ],
        );
        match trace {
            Ok(t) => assert!(air_prove::<F101, _>(&air, &t).is_err()),
            Err(_) => {} // Also acceptable
        }
    }

    #[test]
    fn tampered_trace_rejected() {
        let air = FibonacciAir;
        // Valid first row, invalid second row.
        let trace = Trace::from_rows(
            crate::column::ColumnCount::new(2),
            &[
                vec![F101::new(1), F101::new(1)],
                vec![F101::new(1), F101::new(99)], // Should be 2, not 99
            ],
        );
        match trace {
            Ok(t) => assert!(air_prove::<F101, _>(&air, &t).is_err()),
            Err(_) => {}
        }
    }
}
