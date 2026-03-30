//! Symbolic constraint expressions with row-relative addressing.
//!
//! [`AirExpr<F>`] mirrors plonkish-cat's [`Expression`](plonkish_cat::Expression)
//! but uses [`ColumnRef`] (current row / next row) instead of absolute
//! [`Wire`](plonkish_cat::Wire) indices.  Constraints built from
//! `AirExpr` must evaluate to zero at every consecutive row pair
//! in the execution trace.

use crate::column::{Column, ColumnRef};
use crate::error::Error;
use plonkish_cat::Field;

/// A symbolic polynomial expression over row-relative column references.
///
/// Used to define AIR transition constraints: expressions that must
/// equal zero for every consecutive row pair `(row[i], row[i+1])`.
///
/// # Examples
///
/// ```
/// use plonkish_cat::F101;
/// use machine_cat::{AirExpr, Column};
///
/// // Constraint: next_a - current_b = 0
/// let current_b = AirExpr::<F101>::current(Column::new(1));
/// let next_a = AirExpr::<F101>::next(Column::new(0));
/// let constraint = next_a - current_b;
/// ```
#[derive(Debug, Clone)]
pub enum AirExpr<F: Field> {
    /// A field constant.
    Constant(F),
    /// A row-relative column reference.
    Ref(ColumnRef),
    /// Negation.
    Neg(Box<AirExpr<F>>),
    /// Sum of two expressions.
    Sum(Box<AirExpr<F>>, Box<AirExpr<F>>),
    /// Product of two expressions.
    Product(Box<AirExpr<F>>, Box<AirExpr<F>>),
}

impl<F: Field> AirExpr<F> {
    /// A constant expression.
    #[must_use]
    pub fn constant(c: F) -> Self {
        Self::Constant(c)
    }

    /// Reference a column in the current row.
    #[must_use]
    pub fn current(col: Column) -> Self {
        Self::Ref(ColumnRef::Current(col))
    }

    /// Reference a column in the next row.
    #[must_use]
    pub fn next(col: Column) -> Self {
        Self::Ref(ColumnRef::Next(col))
    }

    /// Evaluate this expression given a row-pair assignment.
    ///
    /// The assignment maps each [`ColumnRef`] to a field value
    /// for a specific `(row[i], row[i+1])` pair.
    ///
    /// # Errors
    ///
    /// Returns an error if the assignment fails for any
    /// referenced column (e.g., column out of bounds).
    pub fn evaluate<A: Fn(ColumnRef) -> Result<F, Error>>(
        &self,
        assignment: &A,
    ) -> Result<F, Error> {
        match self {
            Self::Constant(c) => Ok(c.clone()),
            Self::Ref(cr) => assignment(*cr),
            Self::Neg(inner) => inner.evaluate(assignment).map(|v| -v),
            Self::Sum(left, right) => {
                let l = left.evaluate(assignment)?;
                let r = right.evaluate(assignment)?;
                Ok(l + r)
            }
            Self::Product(left, right) => {
                let l = left.evaluate(assignment)?;
                let r = right.evaluate(assignment)?;
                Ok(l * r)
            }
        }
    }
}

impl<F: Field> std::ops::Add for AirExpr<F> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::Sum(Box::new(self), Box::new(rhs))
    }
}

impl<F: Field> std::ops::Sub for AirExpr<F> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl<F: Field> std::ops::Mul for AirExpr<F> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::Product(Box::new(self), Box::new(rhs))
    }
}

impl<F: Field> std::ops::Neg for AirExpr<F> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::Neg(Box::new(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plonkish_cat::F101;

    fn two_col_assignment(
        curr: Vec<F101>,
        next: Vec<F101>,
    ) -> impl Fn(ColumnRef) -> Result<F101, Error> {
        move |cr| match cr {
            ColumnRef::Current(c) => curr.get(c.index()).cloned().ok_or(
                Error::ColumnOutOfBounds {
                    index: c.index(),
                    column_count: curr.len(),
                },
            ),
            ColumnRef::Next(c) => next.get(c.index()).cloned().ok_or(
                Error::ColumnOutOfBounds {
                    index: c.index(),
                    column_count: next.len(),
                },
            ),
        }
    }

    #[test]
    fn constant_evaluates_to_itself() -> Result<(), Error> {
        let e = AirExpr::constant(F101::new(42));
        let assign = two_col_assignment(vec![], vec![]);
        assert_eq!(e.evaluate(&assign)?, F101::new(42));
        Ok(())
    }

    #[test]
    fn current_ref_evaluates() -> Result<(), Error> {
        let e = AirExpr::<F101>::current(Column::new(1));
        let assign = two_col_assignment(
            vec![F101::new(10), F101::new(20)],
            vec![F101::new(30), F101::new(40)],
        );
        assert_eq!(e.evaluate(&assign)?, F101::new(20));
        Ok(())
    }

    #[test]
    fn next_ref_evaluates() -> Result<(), Error> {
        let e = AirExpr::<F101>::next(Column::new(0));
        let assign = two_col_assignment(
            vec![F101::new(10), F101::new(20)],
            vec![F101::new(30), F101::new(40)],
        );
        assert_eq!(e.evaluate(&assign)?, F101::new(30));
        Ok(())
    }

    #[test]
    fn arithmetic_works() -> Result<(), Error> {
        // next_b - current_a - current_b = 0
        // With curr = [3, 5], next = [5, 8]: 8 - 3 - 5 = 0
        let current_a = AirExpr::<F101>::current(Column::new(0));
        let current_b = AirExpr::<F101>::current(Column::new(1));
        let next_b = AirExpr::<F101>::next(Column::new(1));
        let expr = next_b - current_a - current_b;

        let assign = two_col_assignment(
            vec![F101::new(3), F101::new(5)],
            vec![F101::new(5), F101::new(8)],
        );
        assert_eq!(expr.evaluate(&assign)?, F101::zero());
        Ok(())
    }

    #[test]
    fn product_evaluates() -> Result<(), Error> {
        // current_a * current_b = 3 * 5 = 15
        let expr = AirExpr::<F101>::current(Column::new(0))
            * AirExpr::current(Column::new(1));
        let assign = two_col_assignment(
            vec![F101::new(3), F101::new(5)],
            vec![],
        );
        assert_eq!(expr.evaluate(&assign)?, F101::new(15));
        Ok(())
    }

    #[test]
    fn out_of_bounds_column_fails() {
        let e = AirExpr::<F101>::current(Column::new(5));
        let assign = two_col_assignment(vec![F101::new(1)], vec![]);
        assert!(e.evaluate(&assign).is_err());
    }
}
