//! Column newtypes for AIR trace tables.
//!
//! - [`Column`]: a column index within a trace table
//! - [`ColumnCount`]: the number of columns (the AIR's "shape")
//! - [`ColumnRef`]: a row-relative column reference (current or next row)

/// A column index within a trace table.
///
/// Analogous to [`Wire`](plonkish_cat::Wire) in plonkish-cat,
/// but in the context of execution traces rather than flat witness vectors.
///
/// # Examples
///
/// ```
/// use machine_cat::Column;
///
/// let col = Column::new(0);
/// assert_eq!(col.index(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Column(usize);

impl Column {
    /// Create a new column index.
    #[must_use]
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// The underlying index.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for Column {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "c{}", self.0)
    }
}

/// The number of columns in a trace table.
///
/// This is the "shape" of an AIR: two AIRs with the same
/// [`ColumnCount`] operate on trace tables of the same width.
///
/// # Examples
///
/// ```
/// use machine_cat::ColumnCount;
///
/// let a = ColumnCount::new(2);
/// let b = ColumnCount::new(3);
/// assert_eq!((a + b).count(), 5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnCount(usize);

impl ColumnCount {
    /// Create a new column count.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self(n)
    }

    /// The underlying count.
    #[must_use]
    pub fn count(self) -> usize {
        self.0
    }

    /// Zero columns (the unit object for tensor product).
    #[must_use]
    pub fn zero() -> Self {
        Self(0)
    }

    /// Tensor product: parallel composition of column spaces.
    #[must_use]
    pub fn tensor(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::Add for ColumnCount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.tensor(rhs)
    }
}

impl core::fmt::Display for ColumnCount {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A row-relative column reference.
///
/// AIR constraint expressions use [`ColumnRef`] to address
/// values in the **current** row or the **next** row.
/// This is the key distinction from plonkish-cat's absolute
/// [`Wire`](plonkish_cat::Wire) references.
///
/// # Examples
///
/// ```
/// use machine_cat::{Column, ColumnRef};
///
/// let curr = ColumnRef::Current(Column::new(0));
/// let next = ColumnRef::Next(Column::new(1));
/// assert_eq!(curr.column().index(), 0);
/// assert_eq!(next.column().index(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnRef {
    /// The value of a column in the current row.
    Current(Column),
    /// The value of a column in the next row.
    Next(Column),
}

impl ColumnRef {
    /// The referenced column, regardless of row position.
    #[must_use]
    pub fn column(self) -> Column {
        match self {
            Self::Current(c) | Self::Next(c) => c,
        }
    }

    /// Whether this references the current row.
    #[must_use]
    pub fn is_current(self) -> bool {
        matches!(self, Self::Current(_))
    }

    /// Whether this references the next row.
    #[must_use]
    pub fn is_next(self) -> bool {
        matches!(self, Self::Next(_))
    }
}

impl core::fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Current(c) => write!(f, "curr.{c}"),
            Self::Next(c) => write!(f, "next.{c}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_count_tensor_is_addition() {
        let a = ColumnCount::new(3);
        let b = ColumnCount::new(4);
        assert_eq!(a.tensor(b), ColumnCount::new(7));
        assert_eq!(a + b, ColumnCount::new(7));
    }

    #[test]
    fn column_ref_accessors() {
        let curr = ColumnRef::Current(Column::new(2));
        assert!(curr.is_current());
        assert!(!curr.is_next());
        assert_eq!(curr.column(), Column::new(2));

        let next = ColumnRef::Next(Column::new(5));
        assert!(next.is_next());
        assert!(!next.is_current());
        assert_eq!(next.column(), Column::new(5));
    }

    #[test]
    fn column_display() {
        assert_eq!(format!("{}", Column::new(3)), "c3");
    }

    #[test]
    fn column_ref_display() {
        assert_eq!(
            format!("{}", ColumnRef::Current(Column::new(0))),
            "curr.c0"
        );
        assert_eq!(
            format!("{}", ColumnRef::Next(Column::new(1))),
            "next.c1"
        );
    }
}
