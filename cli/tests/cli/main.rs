#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
#[macro_use]
pub mod utils;
mod acl;
mod append;
#[cfg(not(target_family = "wasm"))]
mod bsdtar;
#[cfg(not(target_family = "wasm"))]
mod bugreport;
mod chmod;
mod chown;
mod chunk;
#[cfg(not(target_family = "wasm"))]
mod combination;
#[cfg(not(target_family = "wasm"))]
mod complete;
mod concat;
mod create;
mod delete;
#[cfg(not(target_family = "wasm"))]
mod diff;
mod encrypt;
mod extract;
mod flag_pairs;
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
mod stdio_arbitration;
mod strip;
mod update;
#[cfg(not(target_family = "wasm"))]
mod verify;
mod xattr;

use clap::CommandFactory;
use portable_network_archive::cli::Cli;

#[test]
fn clap_configuration_remains_valid() {
    Cli::command().debug_assert();
}
