pub(crate) mod path;
pub(crate) mod str;
pub(crate) mod utf8path;

pub(crate) mod slice {
    #[inline]
    pub(crate) fn skip_while<E, P>(s: &[E], mut predicate: P) -> &[E]
    where
        P: FnMut(&E) -> bool,
    {
        let mut s = s;
        while s.first().is_some_and(&mut predicate) {
            s = &s[1..];
        }
        s
    }
}
