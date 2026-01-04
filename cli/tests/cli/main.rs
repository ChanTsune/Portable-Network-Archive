#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
mod acl;
mod append;
mod bugreport;
#[cfg(not(target_family = "wasm"))]
mod cd_option;
mod chmod;
mod chown;
mod chunk;
#[cfg(not(target_family = "wasm"))]
mod combination;
mod complete;
mod concat;
mod create;
mod delete;
#[cfg(not(target_family = "wasm"))]
mod diff;
mod encrypt;
mod extract;
mod fflag;
mod hardlink;
mod keep_acl;
mod keep_all;
mod list;
mod migrate;
mod multipart;
#[cfg(not(target_family = "wasm"))]
mod nodump;
mod restore_acl;
mod restore_acl_0_19_1;
mod solid_mode;
mod sort;
mod split;
#[cfg(not(target_family = "wasm"))]
mod stdio;
mod strip;
mod update;
pub mod utils;
mod xattr;

use clap::CommandFactory;
use portable_network_archive::cli::Cli;

#[test]
fn clap_configuration_remains_valid() {
    Cli::command().debug_assert();
}
