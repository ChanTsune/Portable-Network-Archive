#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
mod acl;
mod append;
mod chmod;
mod chown;
#[cfg(not(target_family = "wasm"))]
mod combination;
mod concat;
mod create;
mod delete;
mod encrypt;
mod extract;
mod hardlink;
mod keep_acl;
mod keep_all;
mod list;
mod multipart;
mod restore_acl;
mod restore_acl_0_19_1;
mod solid_mode;
mod split;
mod strip;
mod symlink;
mod update;
mod user_group;
pub mod utils;
mod xattr;

#[cfg(windows)]
mod windows_attributes;

#[cfg(windows)]
mod windows_properties;
