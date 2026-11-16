//! The wire format between the parent and the `image-worker` child.
//!
//! The parent passes one [`Job`] as a single JSON argument, the input image as
//! the child's stdin (an already-open file handle), and, for jobs that write
//! an image, the path of a temporary file the parent created. The child
//! prints exactly one [`JobResult`] JSON line on stdout and exits 0.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::codec::{CodecError, Facts, ImageFormat};

/// One job for one child process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Job {
    /// Full decode check and facts. `claimed` is the type the uploader
    /// declared; the content must match it.
    Decode { claimed: ImageFormat },
    /// Decode check, then write a preview JPEG to `output`.
    Preview {
        claimed: ImageFormat,
        output: PathBuf,
    },
    /// Decode check, then write a compatible JPEG to `output`.
    Compatible {
        claimed: ImageFormat,
        output: PathBuf,
    },
    /// Test only: write a byte to `output`, then sleep forever.
    #[cfg(debug_assertions)]
    TestHang { output: PathBuf },
    /// Test only: write a byte to `output`, then abort.
    #[cfg(debug_assertions)]
    TestAbort { output: PathBuf },
    /// Test only: allocate and touch `bytes` bytes, then sleep forever.
    #[cfg(debug_assertions)]
    TestAlloc { bytes: u64 },
    /// Test only: print a line that is not a valid result.
    #[cfg(debug_assertions)]
    TestGarbage,
    /// Test only: exit 3 without printing anything.
    #[cfg(debug_assertions)]
    TestExitNonZero,
    /// Test only: report the names of the environment variables.
    #[cfg(debug_assertions)]
    TestEnv,
}

/// Size of an encoded output image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSize {
    pub width: u32,
    pub height: u32,
}

/// The one line the child prints.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum JobResult {
    /// The job succeeded. `output` is set for jobs that write an image.
    Ok {
        facts: Facts,
        output: Option<OutputSize>,
    },
    /// The codec refused the image or the platform.
    Error { error: CodecError },
    /// Test only: answer to [`Job::TestEnv`].
    #[cfg(debug_assertions)]
    Env { names: Vec<String> },
}
