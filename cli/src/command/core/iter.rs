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
    pub(crate) fn new(iter: I) -> Self {
        Self::with_start(iter, 0)
    }

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
