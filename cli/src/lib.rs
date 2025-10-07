//! The `portable-network-archive` crate provides a command-line interface
//! for interacting with PNA archives.
//!
//! This crate is the binary entry point for the PNA tool, and it handles
//! argument parsing, command execution, and user interaction. It is built

#![doc = include_str!("../README.md")]
mod chunk;
pub mod cli;
pub mod command;
mod ext;
mod utils;
