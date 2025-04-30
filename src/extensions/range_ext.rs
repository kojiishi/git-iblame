use std::ops::Range;

/// An extension trait to add `Range::intersect()`.
pub trait RangeExt<Idx> {
    /// Get the intersection of two `Range`s.
    /// ```
    /// use git_iblame::RangeExt;
    /// assert_eq!((2..4).intersect(1..6), 2..4);
    /// assert_eq!((2..4).intersect(3..6), 3..4);
    /// assert_eq!((2..4).intersect(1..3), 2..3);
    /// assert!((2..4).intersect(4..6).is_empty());
    /// ```
    fn intersect(&self, other: Range<Idx>) -> Range<Idx>;
}

impl<Idx: Copy + Ord> RangeExt<Idx> for Range<Idx> {
    fn intersect(&self, other: Range<Idx>) -> Range<Idx> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        start..end
    }
}
