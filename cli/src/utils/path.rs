use std::path::{Path, PathBuf};

#[inline]
pub(crate) fn part_name<P: AsRef<Path>>(p: P, n: u32) -> Option<PathBuf> {
    #[inline]
    fn with_ext(p: &Path, n: u32) -> Option<PathBuf> {
        let name = p.file_stem()?;
        if let Some(ext) = p.extension() {
            Some(PathBuf::from(name).with_extension(format!("part{n}.{}", ext.to_string_lossy())))
        } else {
            Some(PathBuf::from(name).with_extension(format!("part{n}")))
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
}
