//! Internal utility helpers shared across modules.
//!
//! The contents of this module are crate-internal.
pub(crate) mod str;
pub(crate) mod utf8path;

pub(crate) mod slice {
    #[inline]
    pub(crate) fn skip_while<E, P>(mut s: &[E], mut predicate: P) -> &[E]
    where
        P: FnMut(&E) -> bool,
    {
        while let Some((first, rest)) = s.split_first() {
            if !predicate(first) {
                break;
            }
            s = rest;
        }
        s
    }
}
