pub(crate) fn join_with_capacity(
    mut iter: impl Iterator<Item = impl AsRef<str>>,
    sep: &str,
    capacity: usize,
) -> String {
    match iter.next() {
        None => String::new(),
        Some(first) => {
            let mut result = String::with_capacity(capacity);
            result.push_str(first.as_ref());
            for item in iter {
                result.push_str(sep);
                result.push_str(item.as_ref());
            }
            result
        }
    }
}
