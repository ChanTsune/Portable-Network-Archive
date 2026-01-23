use std::collections::HashMap;

/// Reorders `(usize, T)` items into index order, buffering out-of-order items.
///
/// Used to preserve original argument order when parallel processing may
/// produce results in arbitrary order.
///
/// # Preconditions
///
/// This iterator expects indices to be contiguous starting at `start`.
/// If any index in the expected sequence is missing from the input,
/// subsequent items will be buffered but never yielded (silently dropped).
///
/// Each index should appear at most once. If duplicate indices are provided,
/// only the last item with that index will be retained (earlier items are
/// silently overwritten).
#[derive(Debug)]
pub(crate) struct OrderedByIndex<I, T>
where
    I: Iterator<Item = (usize, T)>,
{
    iter: I,
    next: usize,
    buffer: HashMap<usize, T>,
}

impl<I, T> OrderedByIndex<I, T>
where
    I: Iterator<Item = (usize, T)>,
{
    /// Creates an `OrderedByIndex` that yields items in ascending index order starting from 0.
    ///
    /// `iter` is the underlying iterator that yields `(index, item)` pairs;
    /// out-of-order items are buffered until their index becomes the next expected value.
    pub(crate) fn new(iter: I) -> Self {
        Self::with_start(iter, 0)
    }

    /// Creates an `OrderedByIndex` iterator that yields items starting from `start`
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
        }
    }
}

impl<I, T> Iterator for OrderedByIndex<I, T>
where
    I: Iterator<Item = (usize, T)>,
{
    type Item = T;

    /// Advances the ordered iterator and yields the next item whose index matches
    /// the current expected index.
    ///
    /// Returns the next buffered or incoming item with the smallest contiguous index
    /// equal to the iterator's current expectation; when the underlying iterator is
    /// exhausted and no matching buffered item exists, iteration ends.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.buffer.remove(&self.next) {
                self.next += 1;
                return Some(item);
            }

            match self.iter.next() {
                Some((idx, item)) => {
                    if idx == self.next {
                        self.next += 1;
                        return Some(item);
                    }
                    self.buffer.insert(idx, item);
                }
                None => return None,
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
        let mut ordered = OrderedByIndex::new(iter);
        assert_eq!(ordered.next(), Some(10));
        assert_eq!(ordered.next(), Some(20));
        assert_eq!(ordered.next(), None);
    }

    #[test]
    fn with_start_reorders_from_given_index() {
        let src = vec![(2, "a"), (4, "c"), (3, "b")];
        let out: Vec<_> = OrderedByIndex::with_start(src.into_iter(), 2).collect();
        assert_eq!(out, vec!["a", "b", "c"]);
    }

    #[test]
    fn handles_already_ordered_items() {
        let items = vec![(0, 'a'), (1, 'b'), (2, 'c')];
        let out: Vec<_> = OrderedByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['a', 'b', 'c']);
    }

    #[test]
    fn handles_reverse_order() {
        let items = vec![(2, 'c'), (1, 'b'), (0, 'a')];
        let out: Vec<_> = OrderedByIndex::new(items.into_iter()).collect();
        assert_eq!(out, vec!['a', 'b', 'c']);
    }

    #[test]
    fn handles_empty_iterator() {
        let items: Vec<(usize, i32)> = vec![];
        let out: Vec<_> = OrderedByIndex::new(items.into_iter()).collect();
        assert!(out.is_empty());
    }
}
