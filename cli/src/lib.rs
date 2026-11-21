#![doc(html_root_url = "https://docs.rs/portable-network-archive/0.41.1")]
#![doc = include_str!("../README.md")]
#[cfg(all(feature = "acl", target_os = "linux"))]
mod acl_draft;
#[cfg(all(feature = "acl", target_os = "macos"))]
mod acl_draft_darwin;
#[cfg(all(feature = "acl", target_os = "freebsd"))]
mod acl_draft_freebsd;
#[cfg(all(feature = "acl", windows))]
mod acl_draft_windows;
mod chunk;
pub mod cli;
pub mod command;
mod ext;
mod utils;
