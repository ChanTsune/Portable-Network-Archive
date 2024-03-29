use std::path::{Path, PathBuf};

#[inline]
pub(crate) fn part_name<P: AsRef<Path>>(p: P, n: usize) -> Option<PathBuf> {
    #[inline]
    fn with_ext(p: &Path, n: usize) -> Option<PathBuf> {
        let name = PathBuf::from(p.file_stem()?);
        if let Some(ext) = p.extension() {
            if ext.eq_ignore_ascii_case("pna") {
                Some(name.with_extension(format!("part{n}.{}", ext.to_string_lossy())))
            } else {
                Some(name.with_extension(format!("part{n}")))
            }
        } else {
            Some(name.with_extension(format!("part{n}")))
        }
    }
    let p = p.as_ref();
    if let Some(parent) = p.parent() {
        with_ext(p, n).map(|i| parent.join(i))
    } else {
        with_ext(p, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn non_part_to_part_with_extension() {
        assert_eq!(part_name("a.pna", 1), Some(PathBuf::from("a.part1.pna")));
        assert_eq!(
            part_name("parent/a.pna", 1),
            Some(PathBuf::from("parent/a.part1.pna"))
        );
    }

    #[test]
    fn part_to_part_with_extension() {
        assert_eq!(
            part_name("a.part1.pna", 2),
            Some(PathBuf::from("a.part2.pna"))
        );
        assert_eq!(
            part_name("parent/a.part1.pna", 2),
            Some(PathBuf::from("parent/a.part2.pna"))
        );
    }

    #[test]
    fn non_part_to_part_without_extension() {
        assert_eq!(part_name("a", 1), Some(PathBuf::from("a.part1")));
        assert_eq!(
            part_name("parent/a", 1),
            Some(PathBuf::from("parent/a.part1"))
        );
    }

    #[test]
    fn part_to_part_without_extension() {
        assert_eq!(part_name("a.part1", 2), Some(PathBuf::from("a.part2")));
        assert_eq!(
            part_name("parent/a.part1", 2),
            Some(PathBuf::from("parent/a.part2"))
        );
    }
}
