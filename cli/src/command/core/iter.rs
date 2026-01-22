use std::collections::HashMap;

/// Reorders `(usize, T)` items into index order, buffering out-of-order items.
///
/// This iterator expects indices to be contiguous starting at `start`.
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
    /// Creates an OrderedByIndex that yields items in ascending index order starting from 0.
    ///
    /// `iter` is the underlying iterator that yields `(index, item)` pairs; out-of-order items are buffered until their index becomes the next expected value.
    ///
    /// # Examples
    ///
    /// ```
    /// let iter = vec![(1, 20), (0, 10)].into_iter();
    /// let mut ordered = OrderedByIndex::new(iter);
    /// assert_eq!(ordered.next(), Some(10));
    /// assert_eq!(ordered.next(), Some(20));
    /// assert_eq!(ordered.next(), None);
    /// ```
    pub(crate) fn new(iter: I) -> Self {
        Self::with_start(iter, 0)
    }

    /// Creates an OrderedByIndex iterator that yields items starting from `start`
    /// and buffers out-of-order items until their index becomes the next expected.
    ///
    /// `iter` must produce `(usize, T)` pairs where the `usize` is the item's
    /// original index. The resulting iterator emits `T` values in ascending index
    /// order beginning at `start`.
    ///
    /// # Examples
    ///
    /// ```
    /// let src = vec![(2, "a"), (4, "c"), (3, "b")];
    /// let out: Vec<_> = OrderedByIndex::with_start(src.into_iter(), 2).collect();
    /// assert_eq!(out, vec!["a", "b", "c"]);
    /// ```
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

    /// Advances the ordered iterator and yields the next item whose index matches the current expected index.
    ///
    /// The method returns the next buffered or incoming item with the smallest contiguous index equal to the iterator's current expectation; when the underlying iterator is exhausted and no matching buffered item exists, iteration ends.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::iter::FromIterator;
    ///
    /// // Assume OrderedByIndex is in scope and constructed from an iterator of (usize, T).
    /// let items = vec![(0, 'a'), (2, 'c'), (1, 'b')];
    /// let mut ordered = crate::command::core::iter::OrderedByIndex::new(items.into_iter());
    ///
    /// assert_eq!(ordered.next(), Some('a'));
    /// assert_eq!(ordered.next(), Some('b'));
    /// assert_eq!(ordered.next(), Some('c'));
    /// assert_eq!(ordered.next(), None);
    /// ```
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