mod atime;
mod ctime;
mod entry_order;
mod exclude;
mod exclude_from;
mod exclude_vcs;
mod files_from;
#[cfg(not(target_family = "wasm"))]
mod files_from_stdin;
mod include;
mod mtime;
mod numeric_owner;
mod option_gitignore;
#[cfg(any(windows, target_os = "macos"))]
mod option_newer_ctime;
mod option_newer_ctime_than;
mod option_newer_mtime;
mod option_newer_mtime_than;
mod option_no_recursive;
#[cfg(any(windows, target_os = "macos"))]
mod option_older_ctime;
mod option_older_ctime_than;
mod option_older_mtime;
mod option_older_mtime_than;
#[cfg(unix)]
mod option_one_file_system;
mod option_strip_components;
mod password_from_file;
mod password_hash;
mod sanitize_parent_components;
#[cfg(unix)]
mod sparse;
mod substitution;
mod symlink;
mod transform;
mod user_group;
mod without_overwrite;
