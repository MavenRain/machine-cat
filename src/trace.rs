//! Execution traces: 2D tables of field elements.
//!
//! A [`Trace<F>`] is the witness for an AIR.  It has a fixed
//! number of columns (the AIR's shape) and a variable number
//! of rows (the computation length).  Stored in row-major
//! flat layout for cache locality.

use crate::column::{Column, ColumnCount, ColumnRef};
use crate::error::Error;
use plonkish_cat::Field;

/// A row count newtype.
///
/// # Examples
///
/// ```
/// use machine_cat::RowCount;
///
/// let rc = RowCount::new(8);
/// assert_eq!(rc.count(), 8);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowCount(usize);

impl RowCount {
    /// Create a new row count.
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

impl core::fmt::Display for RowCount {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An execution trace: a 2D table of field elements.
///
/// Stored in row-major order: `data[row * column_count + col]`.
/// Each row has exactly `column_count` elements.
///
/// # Examples
///
/// ```
/// use plonkish_cat::F101;
/// use machine_cat::{Column, ColumnCount, Trace};
///
/// let trace = Trace::from_rows(
///     ColumnCount::new(2),
///     &[
///         vec![F101::new(1), F101::new(1)],
///         vec![F101::new(1), F101::new(2)],
///         vec![F101::new(2), F101::new(3)],
///         vec![F101::new(3), F101::new(5)],
///     ],
/// )?;
///
/// assert_eq!(trace.get(2, Column::new(0))?, F101::new(2));
/// assert_eq!(trace.get(2, Column::new(1))?, F101::new(3));
/// # Ok::<(), machine_cat::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Trace<F: Field> {
    data: Vec<F>,
    column_count: ColumnCount,
    row_count: RowCount,
}

impl<F: Field> Trace<F> {
    /// Construct a trace from a vector of rows.
    ///
    /// Each inner vector must have exactly `column_count` elements.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyTrace`] if `rows` is empty, or
    /// [`Error::RowLengthMismatch`] if any row has the wrong length.
    pub fn from_rows(column_count: ColumnCount, rows: &[Vec<F>]) -> Result<Self, Error> {
        if rows.is_empty() {
            Err(Error::EmptyTrace)
        } else {
            // Validate all row lengths, then flatten.
            let data: Result<Vec<F>, Error> = rows
                .iter()
                .enumerate()
                .try_fold(Vec::with_capacity(rows.len() * column_count.count()), |acc, (i, row)| {
                    if row.len() == column_count.count() {
                        Ok(acc.into_iter().chain(row.iter().cloned()).collect())
                    } else {
                        Err(Error::RowLengthMismatch {
                            row: i,
                            expected: column_count.count(),
                            actual: row.len(),
                        })
                    }
                });
            Ok(Self {
                data: data?,
                column_count,
                row_count: RowCount::new(rows.len()),
            })
        }
    }

    /// The number of columns.
    #[must_use]
    pub fn column_count(&self) -> ColumnCount {
        self.column_count
    }

    /// The number of rows.
    #[must_use]
    pub fn row_count(&self) -> RowCount {
        self.row_count
    }

    /// Get the value at `(row, col)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColumnOutOfBounds`] if the row or column
    /// is out of range.
    pub fn get(&self, row: usize, col: Column) -> Result<F, Error> {
        if row >= self.row_count.count() || col.index() >= self.column_count.count() {
            Err(Error::ColumnOutOfBounds {
                index: col.index(),
                column_count: self.column_count.count(),
            })
        } else {
            Ok(self.data[row * self.column_count.count() + col.index()].clone())
        }
    }

    /// Build an assignment function for a row pair `(row, row+1)`.
    ///
    /// The returned closure maps [`ColumnRef::Current`] to values
    /// in `row` and [`ColumnRef::Next`] to values in `row + 1`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientRows`] if `row + 1 >= row_count`.
    pub fn row_pair_assignment(
        &self,
        row: usize,
    ) -> Result<impl Fn(ColumnRef) -> Result<F, Error> + '_, Error> {
        if row + 1 >= self.row_count.count() {
            Err(Error::InsufficientRows {
                row_count: self.row_count.count(),
            })
        } else {
            Ok(move |cr: ColumnRef| {
                let (r, c) = match cr {
                    ColumnRef::Current(col) => (row, col),
                    ColumnRef::Next(col) => (row + 1, col),
                };
                self.get(r, c)
            })
        }
    }

    /// Extract all values from a single column.
    ///
    /// Returns a vector of length `row_count`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ColumnOutOfBounds`] if `col` is out of range.
    pub fn column_values(&self, col: Column) -> Result<Vec<F>, Error> {
        if col.index() >= self.column_count.count() {
            Err(Error::ColumnOutOfBounds {
                index: col.index(),
                column_count: self.column_count.count(),
            })
        } else {
            Ok((0..self.row_count.count())
                .map(|r| self.data[r * self.column_count.count() + col.index()].clone())
                .collect())
        }
    }

    /// The raw data as a flat slice (row-major order).
    #[must_use]
    pub fn data(&self) -> &[F] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plonkish_cat::F101;

    fn fib_trace() -> Result<Trace<F101>, Error> {
        Trace::from_rows(
            ColumnCount::new(2),
            &[
                vec![F101::new(1), F101::new(1)],
                vec![F101::new(1), F101::new(2)],
                vec![F101::new(2), F101::new(3)],
                vec![F101::new(3), F101::new(5)],
            ],
        )
    }

    #[test]
    fn from_rows_and_get() -> Result<(), Error> {
        let t = fib_trace()?;
        assert_eq!(t.row_count(), RowCount::new(4));
        assert_eq!(t.column_count(), ColumnCount::new(2));
        assert_eq!(t.get(0, Column::new(0))?, F101::new(1));
        assert_eq!(t.get(3, Column::new(1))?, F101::new(5));
        Ok(())
    }

    #[test]
    fn empty_rows_fails() {
        let result = Trace::<F101>::from_rows(ColumnCount::new(2), &[]);
        assert!(result.is_err());
    }

    #[test]
    fn wrong_row_length_fails() {
        let result = Trace::from_rows(
            ColumnCount::new(2),
            &[vec![F101::new(1), F101::new(2)], vec![F101::new(3)]],
        );
        assert!(result.is_err());
    }

    #[test]
    fn column_values_extraction() -> Result<(), Error> {
        let t = fib_trace()?;
        let col0 = t.column_values(Column::new(0))?;
        assert_eq!(
            col0,
            vec![F101::new(1), F101::new(1), F101::new(2), F101::new(3)]
        );
        Ok(())
    }

    #[test]
    fn row_pair_assignment_works() -> Result<(), Error> {
        let t = fib_trace()?;
        let assign = t.row_pair_assignment(1)?;
        // Current row 1: [1, 2], Next row 2: [2, 3]
        assert_eq!(assign(ColumnRef::Current(Column::new(0)))?, F101::new(1));
        assert_eq!(assign(ColumnRef::Current(Column::new(1)))?, F101::new(2));
        assert_eq!(assign(ColumnRef::Next(Column::new(0)))?, F101::new(2));
        assert_eq!(assign(ColumnRef::Next(Column::new(1)))?, F101::new(3));
        Ok(())
    }

    #[test]
    fn row_pair_at_last_row_fails() -> Result<(), Error> {
        let t = fib_trace()?;
        // Row 3 is the last row; no "next" row exists.
        let result = t.row_pair_assignment(3);
        assert!(result.is_err());
        Ok(())
    }
}
