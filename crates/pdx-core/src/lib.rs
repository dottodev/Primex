//! pdx-core — errors, byte reader, hashing, workspace documents and job model.
//!
//! This crate builds and unit-tests on host x86_64 with zero Android
//! dependencies. Every other parser crate depends on its error type.

pub mod error;
pub mod hash;
pub mod job;
pub mod reader;
pub mod workspace;

pub use error::{ErrorKind, PdxError, Result};
pub use hash::{hex_decode, hex_encode, sha256, sha256_file, sha256_hex, verify_sha256};
pub use job::{CancelToken, Job, JobId, JobSequencer, Progress};
pub use reader::ByteReader;
pub use workspace::{migrate_project, ProfileJson, ProjectJson};

pub const TOOL_VERSION: &str = "0.1.0";
pub const MIUIX_VERSION: &str = "0.9.4";
