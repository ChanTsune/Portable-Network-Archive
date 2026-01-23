use std::collections::HashMap;

/// Reorders `(usize, T)` items into index order, buffering out-of-order items.
///
/// Used to preserve original argument order when parallel processing may
/// produce results in arbitrary order.
///
/// # Gap Handling
///
/// This iterator expects indices to be contiguous starting at `start`.
/// If any index in the expected sequence is missing from the input (a "gap"),
/// the iterator skips to the next available index, ensuring all buffered
/// items are eventually yielded.
///
/// # Duplicate Indices
///
/// Each index should appear at most once. If duplicate indices are provided,
/// only the last item with that index will be retained (earlier items are
/// silently overwritten).
#[derive(Debug)]
pub(crate) struct ReorderByIndex<I, T>
where
    I: Iterator<Item = (usize, T)>,
{
    iter: I,
    next: usize,
    buffer: HashMap<usize, T>,
    exhausted: bool,
}

impl<I, T> ReorderByIndex<I, T>
where
    I: Iterator<Item = (usize, T)>,
{
    /// Creates a `ReorderByIndex` that yields items in ascending index order starting from 0.
    ///
    /// `iter` is the underlying iterator that yields `(index, item)` pairs;
    /// out-of-order items are buffered until their index becomes the next expected value.
    pub(crate) fn new(iter: I) -> Self {
        Self::with_start(iter, 0)
    }

    /// Creates a `ReorderByIndex` iterator that yields items starting from `start`
    /// and buffers out-of-order items until their index becomes the next expected.
    ///
    /// `iter` must produce `(usize, T)` pairs where the `usize` is the item's
    /// original index. The resulting iterator emits `T` values in ascending index
    /// order beginning at `start`.
    ///
    /// See the struct-level documentation for preconditions regarding index
    /// contiguity and uniqueness.
    pub(crate) fn with_start(iter: I, start: usize) -> Self {
        Self {
            iter,
            next: start,
            buffer: HashMap::new(),
            exhausted: false,
        }
    }
}

impl<I, T> Iterator for ReorderByIndex<I, T>
where
    I: Iterator<Item = (usize, T)>,
{
    type Item = T;

    /// Advances the iterator and yields the next item whose index matches
    /// the current expected index.
    ///
    /// Returns the next buffered or incoming item in ascending index order.
    /// If a gap is detected (missing index), skips to the next available index.
    /// When the underlying iterator is exhausted and no buffered items remain,
    /// iteration ends.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.buffer.remove(&self.next) {
                self.next += 1;
                return Some(item);
            }

            if self.exhausted {
                let min_idx = *self.buffer.keys().min()?;
                self.next = min_idx;
                continue;
            }

            match self.iter.next() {
                Some((idx, item)) => {
                    if idx == self.next {
                        self.next += 1;
                        return Some(item);
                    }
                    self.buffer.insert(idx, item);
                }
                None => self.exhausted = true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorders_out_of_order_items() {
        let iter = vec![(1, 20), (0, 10)].into_iter();
        let mut ordered = ReorderByIndex::new(iter);
        assert_eq!(ordered.next(), Some(10));
        assert_eq!(ordered.next(), Some(20));
        assert_eq!(ordered.next(), None);
    }

    #[test]
    fn with_start_reorders_from_given_index() {
        let src = vec![(2, "a"), (4, "c"), (3, "b")];
        let out: Vec<_> = ReorderByIndex::with_start(src.into_iter(), 2).collect();
        assert_eq!(out, vec!["a", "b", "c"]);
    }

    #[test]
    fn handles_already_ordered_items() {
        let items = vec![(0, 'a'), (1, 'b'), (2, 'c')];
        let out: Vec<_> = ReorderByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['a', 'b', 'c']);
    }

    #[test]
    fn handles_reverse_order() {
        let items = vec![(2, 'c'), (1, 'b'), (0, 'a')];
        let out: Vec<_> = ReorderByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['a', 'b', 'c']);
    }

    #[test]
    fn handles_empty_iterator() {
        let items: Vec<(usize, i32)> = vec![];
        let out: Vec<_> = ReorderByIndex::new(items.into_iter()).collect();
        assert!(out.is_empty());
    }

    #[test]
    fn handles_gap_in_indices() {
        // Missing index 1: should yield 'a', skip gap, then yield 'c', 'd'
        let items = vec![(0, 'a'), (2, 'c'), (3, 'd')];
        let out: Vec<_> = ReorderByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['a', 'c', 'd']);
    }

    #[test]
    fn handles_multiple_gaps() {
        // Missing indices 1 and 3
        let items = vec![(0, 'a'), (2, 'c'), (4, 'e')];
        let out: Vec<_> = ReorderByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['a', 'c', 'e']);
    }

    #[test]
    fn handles_gap_at_start() {
        // Missing index 0
        let items = vec![(1, 'b'), (2, 'c')];
        let out: Vec<_> = ReorderByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['b', 'c']);
    }

    #[test]
    fn handles_single_item_after_gap() {
        // Only index 5, missing 0-4
        let items = vec![(5, 'f')];
        let out: Vec<_> = ReorderByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['f']);
    }
}
