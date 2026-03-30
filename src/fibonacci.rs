//! Fibonacci AIR: a concrete example of the [`Air`] trait.
//!
//! Proves correct computation of the Fibonacci sequence.
//! The trace has 2 columns (`a`, `b`) and 2 transition constraints:
//!
//! - `next_a - current_b = 0`
//! - `next_b - current_a - current_b = 0`
//!
//! Given initial values `(a_0, b_0)`, the trace computes:
//!
//! | Row | a | b |
//! |-----|---|---|
//! | 0   | a_0 | b_0 |
//! | 1   | b_0 | a_0 + b_0 |
//! | 2   | a_0 + b_0 | a_0 + 2*b_0 |
//! | ... | ... | ... |

use crate::air::Air;
use crate::air_expr::AirExpr;
use crate::column::{Column, ColumnCount};
use crate::error::Error;
use crate::trace::Trace;
use plonkish_cat::Field;

/// Number of computation steps (rows = steps + 1).
///
/// # Examples
///
/// ```
/// use machine_cat::StepCount;
///
/// let steps = StepCount::new(7);
/// assert_eq!(steps.count(), 7);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepCount(usize);

impl StepCount {
    /// Create a new step count.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self(n)
    }

    /// The underlying count.
    #[must_use]
    pub fn count(self) -> usize {
        self.0
    }
}

/// Input to the Fibonacci AIR: initial values and step count.
///
/// # Examples
///
/// ```
/// use plonkish_cat::F101;
/// use machine_cat::{FibonacciInput, StepCount};
///
/// let input = FibonacciInput::new(F101::new(1), F101::new(1), StepCount::new(7));
/// ```
#[derive(Debug, Clone)]
pub struct FibonacciInput<F: Field> {
    initial_a: F,
    initial_b: F,
    num_steps: StepCount,
}

impl<F: Field> FibonacciInput<F> {
    /// Create a Fibonacci input.
    #[must_use]
    pub fn new(initial_a: F, initial_b: F, num_steps: StepCount) -> Self {
        Self {
            initial_a,
            initial_b,
            num_steps,
        }
    }

    /// The initial value of column `a`.
    #[must_use]
    pub fn initial_a(&self) -> &F {
        &self.initial_a
    }

    /// The initial value of column `b`.
    #[must_use]
    pub fn initial_b(&self) -> &F {
        &self.initial_b
    }

    /// The number of computation steps.
    #[must_use]
    pub fn num_steps(&self) -> StepCount {
        self.num_steps
    }
}

/// The Fibonacci AIR.
///
/// Two columns (`a` = column 0, `b` = column 1) with transition
/// constraints ensuring each row is the Fibonacci successor of
/// the previous row.
///
/// # Examples
///
/// ```
/// use plonkish_cat::{F101, Field};
/// use machine_cat::{Air, FibonacciAir, FibonacciInput, StepCount};
///
/// let air = FibonacciAir;
/// let input = FibonacciInput::new(
///     F101::new(1), F101::new(1), StepCount::new(3),
/// );
/// let trace = air.generate_trace(&input)?;
///
/// // 4 rows: (1,1), (1,2), (2,3), (3,5)
/// assert_eq!(trace.row_count().count(), 4);
/// # Ok::<(), machine_cat::Error>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FibonacciAir;

impl<F: Field> Air<F> for FibonacciAir {
    type Input = FibonacciInput<F>;

    fn column_count(&self) -> ColumnCount {
        ColumnCount::new(2)
    }

    fn constraints(&self) -> Vec<AirExpr<F>> {
        let current_a = AirExpr::current(Column::new(0));
        let current_b = AirExpr::current(Column::new(1));
        let next_a = AirExpr::next(Column::new(0));
        let next_b = AirExpr::next(Column::new(1));

        vec![
            // next_a - current_b = 0
            next_a - current_b.clone(),
            // next_b - current_a - current_b = 0
            next_b - current_a - current_b,
        ]
    }

    fn generate_trace(&self, input: &FibonacciInput<F>) -> Result<Trace<F>, Error> {
        let num_rows = input.num_steps.0 + 1;
        // Build rows via successors: (a, b) -> (b, a+b).
        let rows: Vec<Vec<F>> = std::iter::successors(
            Some((input.initial_a.clone(), input.initial_b.clone())),
            |prev| Some((prev.1.clone(), prev.0.clone() + prev.1.clone())),
        )
        .take(num_rows)
        .map(|(a, b)| vec![a, b])
        .collect();

        Trace::from_rows(ColumnCount::new(2), &rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::Column;
    use plonkish_cat::F101;

    #[test]
    fn fibonacci_trace_correctness() -> Result<(), Error> {
        let air = FibonacciAir;
        let input = FibonacciInput::new(F101::new(1), F101::new(1), StepCount::new(7));
        let trace = air.generate_trace(&input)?;

        // 8 rows: (1,1), (1,2), (2,3), (3,5), (5,8), (8,13), (13,21), (21,34)
        assert_eq!(trace.row_count().count(), 8);
        assert_eq!(trace.get(0, Column::new(0))?, F101::new(1));
        assert_eq!(trace.get(0, Column::new(1))?, F101::new(1));
        assert_eq!(trace.get(7, Column::new(0))?, F101::new(21));
        assert_eq!(trace.get(7, Column::new(1))?, F101::new(34));
        Ok(())
    }

    #[test]
    fn fibonacci_constraints_satisfied() -> Result<(), Error> {
        let air = FibonacciAir;
        let input = FibonacciInput::new(F101::new(1), F101::new(1), StepCount::new(7));
        let trace = air.generate_trace(&input)?;
        let constraints = air.constraints();

        // Check constraints at every row pair.
        (0..trace.row_count().count() - 1).try_for_each(|row| {
            let assign = trace.row_pair_assignment(row)?;
            constraints.iter().try_for_each(|c| {
                let val = c.evaluate(&assign)?;
                if val == F101::zero() {
                    Ok(())
                } else {
                    Err(Error::UnsatisfiedAirConstraint { row })
                }
            })
        })
    }

    #[test]
    fn fibonacci_single_step() -> Result<(), Error> {
        let air = FibonacciAir;
        let input = FibonacciInput::new(F101::new(3), F101::new(5), StepCount::new(1));
        let trace = air.generate_trace(&input)?;

        // 2 rows: (3, 5), (5, 8)
        assert_eq!(trace.row_count().count(), 2);
        assert_eq!(trace.get(0, Column::new(0))?, F101::new(3));
        assert_eq!(trace.get(0, Column::new(1))?, F101::new(5));
        assert_eq!(trace.get(1, Column::new(0))?, F101::new(5));
        assert_eq!(trace.get(1, Column::new(1))?, F101::new(8));
        Ok(())
    }
}
