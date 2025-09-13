use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{collections::HashSet, fs};

#[test]
fn create_with_gitignore() {
    setup();
    fs::create_dir_all("gitignore/source").unwrap();
    fs::write("gitignore/source/.gitignore", "*.log\n").unwrap();
    fs::write("gitignore/source/keep.txt", b"text").unwrap();
    fs::write("gitignore/source/skip.log", b"log").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/gitignore.pna",
        "--overwrite",
        "gitignore/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Expect: only `.gitignore` and `keep.txt` are present.
    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/gitignore.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in ["gitignore/source/.gitignore", "gitignore/source/keep.txt"] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_subdirs_and_negation() {
    // Matrix (path => expected with --gitignore):
    // - root:        keep.txt               => include (no rule)
    // - root:        skip.log               => exclude (*.log)
    // - root:        root_tmp.tmp           => include (no root *.tmp rule)
    // - build/:      build/output.txt       => exclude (build/)
    // - sub/:        sub/skip.log           => exclude (parent *.log; not whitelisted)
    // - sub/:        sub/keep.log           => include (!keep.log overrides parent)
    // - sub/:        sub/file.tmp           => exclude (child *.tmp)
    // - sub/nested/: sub/nested/deeper.log  => include (grandchild !deeper.log)
    // - a/b/:        a/b/secret.txt         => exclude (**/secret.txt)

    setup();
    fs::create_dir_all("gitignore/complex/source/sub/nested").unwrap();
    fs::create_dir_all("gitignore/complex/source/build").unwrap();
    fs::create_dir_all("gitignore/complex/source/tmponly").unwrap();
    fs::create_dir_all("gitignore/complex/source/a/b").unwrap();

    // Root files
    fs::write(
        "gitignore/complex/source/.gitignore",
        "*.log\nbuild/\n**/secret.txt\n",
    )
    .unwrap();
    fs::write("gitignore/complex/source/keep.txt", b"ok").unwrap();
    fs::write("gitignore/complex/source/skip.log", b"ignored").unwrap();
    fs::write("gitignore/complex/source/root_tmp.tmp", b"ok").unwrap();

    // Subdir rules and files
    fs::write(
        "gitignore/complex/source/sub/.gitignore",
        "!keep.log\n*.tmp\n",
    )
    .unwrap();
    fs::write("gitignore/complex/source/sub/skip.log", b"ignored").unwrap();
    fs::write("gitignore/complex/source/sub/keep.log", b"ok").unwrap();
    fs::write("gitignore/complex/source/sub/file.tmp", b"ignored").unwrap();

    // Grandchild override
    fs::write(
        "gitignore/complex/source/sub/nested/.gitignore",
        "!deeper.log\n",
    )
    .unwrap();
    fs::write("gitignore/complex/source/sub/nested/deeper.log", b"ok").unwrap();

    // Directory ignore and deep-glob
    fs::write("gitignore/complex/source/build/output.txt", b"ignored").unwrap();
    fs::write("gitignore/complex/source/a/b/secret.txt", b"ignored").unwrap();

    // Also a directory with its own ignore (sanity): only *.tmp ignored here
    fs::write("gitignore/complex/source/tmponly/.gitignore", "*.tmp\n").unwrap();
    fs::write("gitignore/complex/source/tmponly/file.tmp", b"ignored").unwrap();
    fs::write("gitignore/complex/source/tmponly/file.txt", b"ok").unwrap();

    // Create archive with --gitignore
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/complex/archive.pna",
        "--overwrite",
        "gitignore/complex/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify using entries inside the archive (no extraction).
    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/complex/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Exact set of expected entries.
    for required in [
        "gitignore/complex/source/.gitignore",
        "gitignore/complex/source/keep.txt",
        "gitignore/complex/source/root_tmp.tmp",
        "gitignore/complex/source/sub/.gitignore",
        "gitignore/complex/source/sub/keep.log",
        "gitignore/complex/source/sub/nested/.gitignore",
        "gitignore/complex/source/sub/nested/deeper.log",
        "gitignore/complex/source/tmponly/.gitignore",
        "gitignore/complex/source/tmponly/file.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_child_overrides_parent_ignore() {
    // Parent unignores (*.log), child re-ignores (*.log)
    setup();
    fs::create_dir_all("gitignore/child_overrides/source/child").unwrap();

    // Parent allows all .log
    fs::write("gitignore/child_overrides/source/.gitignore", "!*.log\n").unwrap();
    fs::write("gitignore/child_overrides/source/root.log", b"ok").unwrap();

    // Child ignores .log
    fs::write(
        "gitignore/child_overrides/source/child/.gitignore",
        "*.log\n",
    )
    .unwrap();
    fs::write(
        "gitignore/child_overrides/source/child/keep.log",
        b"ignored by child",
    )
    .unwrap();
    // Another .log in child should also be excluded
    fs::write(
        "gitignore/child_overrides/source/child/other.log",
        b"ignored too",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/child_overrides/archive.pna",
        "--overwrite",
        "gitignore/child_overrides/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/child_overrides/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/child_overrides/source/.gitignore",
        "gitignore/child_overrides/source/root.log",
        "gitignore/child_overrides/source/child/.gitignore",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_multi_level_toggle() {
    // Parent: *.log (ignore) -> Child: !*.log (unignore) -> Grandchild: *.log (ignore again)
    setup();
    fs::create_dir_all("gitignore/multi_toggle/source/child/nested").unwrap();

    fs::write("gitignore/multi_toggle/source/.gitignore", "*.log\n").unwrap();
    fs::write("gitignore/multi_toggle/source/root.log", b"drop").unwrap();

    fs::write("gitignore/multi_toggle/source/child/.gitignore", "!*.log\n").unwrap();
    fs::write("gitignore/multi_toggle/source/child/ok.log", b"keep").unwrap();

    fs::write(
        "gitignore/multi_toggle/source/child/nested/.gitignore",
        "*.log\n",
    )
    .unwrap();
    fs::write(
        "gitignore/multi_toggle/source/child/nested/back.log",
        b"drop",
    )
    .unwrap();
    // Another file under child that should be kept
    fs::write("gitignore/multi_toggle/source/child/extra.log", b"keep").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/multi_toggle/archive.pna",
        "--overwrite",
        "gitignore/multi_toggle/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/multi_toggle/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/multi_toggle/source/.gitignore",
        "gitignore/multi_toggle/source/child/ok.log",
        "gitignore/multi_toggle/source/child/extra.log",
        "gitignore/multi_toggle/source/child/.gitignore",
        "gitignore/multi_toggle/source/child/nested/.gitignore",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_last_match_wins() {
    // Within a single .gitignore, the last matching rule wins
    setup();
    fs::create_dir_all("gitignore/last_match/source/order_allow").unwrap();
    fs::create_dir_all("gitignore/last_match/source/order_deny").unwrap();

    // Case A: ignore then unignore (should be kept)
    fs::write(
        "gitignore/last_match/source/order_allow/.gitignore",
        "*.log\n!keep.log\n",
    )
    .unwrap();
    fs::write("gitignore/last_match/source/order_allow/keep.log", b"keep").unwrap();
    // This one should remain ignored
    fs::write("gitignore/last_match/source/order_allow/drop.log", b"drop").unwrap();

    // Case B: unignore then ignore (should be dropped)
    fs::write(
        "gitignore/last_match/source/order_deny/.gitignore",
        "!keep.log\n*.log\n",
    )
    .unwrap();
    fs::write("gitignore/last_match/source/order_deny/keep.log", b"drop").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/last_match/archive.pna",
        "--overwrite",
        "gitignore/last_match/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/last_match/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/last_match/source/order_allow/.gitignore",
        "gitignore/last_match/source/order_allow/keep.log",
        "gitignore/last_match/source/order_deny/.gitignore",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_child_anchored_slash() {
    // Leading slash in child .gitignore anchors to the child directory root
    setup();
    fs::create_dir_all("gitignore/child_anchor/source/child/nested").unwrap();

    // Child rule: only child/only_here.txt is excluded
    fs::write(
        "gitignore/child_anchor/source/child/.gitignore",
        "/only_here.txt\n",
    )
    .unwrap();
    fs::write("gitignore/child_anchor/source/child/only_here.txt", b"drop").unwrap();
    fs::write(
        "gitignore/child_anchor/source/child/nested/only_here.txt",
        b"keep",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/child_anchor/archive.pna",
        "--overwrite",
        "gitignore/child_anchor/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/child_anchor/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/child_anchor/source/child/.gitignore",
        "gitignore/child_anchor/source/child/nested/only_here.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_pruned_dir_cannot_unignore_inside() {
    // If parent prunes 'sub/', child '!keep.txt' cannot resurrect files inside
    setup();
    fs::create_dir_all("gitignore/pruned_dir/source/sub").unwrap();

    fs::write("gitignore/pruned_dir/source/.gitignore", "sub/\n").unwrap();
    fs::write("gitignore/pruned_dir/source/sub/.gitignore", "!keep.txt\n").unwrap();
    fs::write(
        "gitignore/pruned_dir/source/sub/keep.txt",
        b"should not be included",
    )
    .unwrap();
    // Another file under the pruned dir
    fs::write("gitignore/pruned_dir/source/sub/also.txt", b"not included").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/pruned_dir/archive.pna",
        "--overwrite",
        "gitignore/pruned_dir/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/pruned_dir/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in ["gitignore/pruned_dir/source/.gitignore"] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_pruned_dir_unignore_with_parent_exceptions() {
    // Re-inclusion works only if parent adds '!sub/' and '!sub/keep.txt'
    setup();
    fs::create_dir_all("gitignore/pruned_dir_fix/source/sub").unwrap();

    fs::write(
        "gitignore/pruned_dir_fix/source/.gitignore",
        "sub/\n!sub/\n!sub/keep.txt\n",
    )
    .unwrap();
    fs::write(
        "gitignore/pruned_dir_fix/source/sub/.gitignore",
        "!keep.txt\n",
    )
    .unwrap();
    fs::write(
        "gitignore/pruned_dir_fix/source/sub/keep.txt",
        b"should be included",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/pruned_dir_fix/archive.pna",
        "--overwrite",
        "gitignore/pruned_dir_fix/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/pruned_dir_fix/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/pruned_dir_fix/source/.gitignore",
        "gitignore/pruned_dir_fix/source/sub/.gitignore",
        "gitignore/pruned_dir_fix/source/sub/keep.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
}

#[test]
fn create_with_gitignore_excludes_gitignore_file_itself() {
    // A .gitignore rule can exclude the .gitignore file itself.
    // When the pattern contains ".gitignore", the file should not be archived.
    setup();
    fs::create_dir_all("gitignore/self_exclude/source").unwrap();

    // Ignore the .gitignore file itself.
    fs::write("gitignore/self_exclude/source/.gitignore", ".gitignore\n").unwrap();
    fs::write("gitignore/self_exclude/source/keep.txt", b"ok").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/self_exclude/archive.pna",
        "--overwrite",
        "gitignore/self_exclude/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/self_exclude/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in ["gitignore/self_exclude/source/keep.txt"] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_sibling_scopes_do_not_leak() {
    // Sibling directories each have their own .gitignore, and rules apply only to their subtree.
    // - A/.gitignore = "*.log"            -> A/a.log is excluded; A/keep.txt is included
    // - B/.gitignore = "*.tmp"            -> B/tmp.tmp is excluded; B/b.log is included
    setup();
    fs::create_dir_all("gitignore/sibling_scope/source/A").unwrap();
    fs::create_dir_all("gitignore/sibling_scope/source/B").unwrap();

    // A rules and files
    fs::write("gitignore/sibling_scope/source/A/.gitignore", "*.log\n").unwrap();
    fs::write("gitignore/sibling_scope/source/A/a.log", b"drop").unwrap();
    fs::write("gitignore/sibling_scope/source/A/keep.txt", b"ok").unwrap();

    // B rules and files
    fs::write("gitignore/sibling_scope/source/B/.gitignore", "*.tmp\n").unwrap();
    fs::write("gitignore/sibling_scope/source/B/b.log", b"ok").unwrap();
    fs::write("gitignore/sibling_scope/source/B/tmp.tmp", b"drop").unwrap();

    // Create archive with --gitignore
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/sibling_scope/archive.pna",
        "--overwrite",
        "gitignore/sibling_scope/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify exact set of entries; no leakage across siblings.
    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/sibling_scope/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/sibling_scope/source/A/.gitignore",
        "gitignore/sibling_scope/source/A/keep.txt",
        "gitignore/sibling_scope/source/B/.gitignore",
        "gitignore/sibling_scope/source/B/b.log",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_comment_and_escape() {
    // '#' starts a comment unless escaped; '\#file' matches a file literally starting with '#'.
    // Verify that comments don't act as patterns and escaped '#' does.
    setup();
    fs::create_dir_all("gitignore/comment_escape/source").unwrap();

    fs::write(
        "gitignore/comment_escape/source/.gitignore",
        "# this is a comment and should be ignored\n\\#secret.txt\n",
    )
    .unwrap();
    fs::write("gitignore/comment_escape/source/#secret.txt", b"drop").unwrap();
    fs::write("gitignore/comment_escape/source/file.tmp", b"keep").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/comment_escape/archive.pna",
        "--overwrite",
        "gitignore/comment_escape/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/comment_escape/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/comment_escape/source/.gitignore",
        "gitignore/comment_escape/source/file.tmp",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_gitignore_literal_bang_pattern() {
    // Leading '!' unignores patterns; to match a literal '!' in filename, it must be escaped.
    // Verify that '\!file.txt' ignores a file literally named '!file.txt'.
    setup();
    fs::create_dir_all("gitignore/literal_bang/source").unwrap();

    fs::write("gitignore/literal_bang/source/.gitignore", "\\!file.txt\n").unwrap();
    fs::write("gitignore/literal_bang/source/!file.txt", b"drop").unwrap();
    fs::write("gitignore/literal_bang/source/keep.txt", b"ok").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/literal_bang/archive.pna",
        "--overwrite",
        "gitignore/literal_bang/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("gitignore/literal_bang/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "gitignore/literal_bang/source/.gitignore",
        "gitignore/literal_bang/source/keep.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
