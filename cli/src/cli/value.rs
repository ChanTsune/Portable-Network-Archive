//! Custom value types for CLI argument parsing.
//!
//! This module provides specialized types that implement [`std::str::FromStr`] for use
//! with clap's argument parsing. Each type handles validation and conversion of
//! command-line string inputs into strongly-typed values.

mod argon2id_params;
mod color_choice;
mod compression_level;
mod datetime;
mod name_id_pair;
mod options;
mod pbkdf2_sha256_params;
mod private_chunk_type;

pub(crate) use argon2id_params::Argon2idParams;
pub(crate) use color_choice::ColorChoice;
pub(crate) use compression_level::{DeflateLevel, XzLevel, ZstdLevel};
pub use datetime::DateTime;
pub(crate) use name_id_pair::NameIdPair;
pub(crate) use options::ArchiveOptions;
pub(crate) use pbkdf2_sha256_params::Pbkdf2Sha256Params;
pub(crate) use private_chunk_type::PrivateChunkType;
