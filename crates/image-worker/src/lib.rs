//! Image decode and convert worker.
//!
//! - [`sniff`]: pure-Rust content sniffing (usable on phones).
//! - [`structure`]: pure-Rust container completeness checks.
//! - [`strip`]: pure-Rust, size-preserving removal of location metadata
//!   (usable on phones).
//! - [`codec`]: the [`ImageCodec`] trait and the per-OS back end.
//! - [`worker`]: the parent-side API that runs one job per child process with
//!   time and memory limits.
//! - [`child`]: the code the `image-worker` binary runs.

pub mod child;
pub mod codec;
pub mod job;
pub mod sniff;
pub mod strip;
pub mod structure;
pub mod worker;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod macos;

pub use codec::{platform_codec, CodecError, Encoded, Facts, ImageCodec, ImageFormat};
#[cfg(target_os = "macos")]
pub use macos::ImageIoCodec;
pub use sniff::{sniff, NotAnImage};
pub use worker::{InvalidImage, JobOutput, SizeBucket, Task, Worker, WorkerError};
