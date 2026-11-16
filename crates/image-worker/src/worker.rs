//! Parent side: run one job in a fresh `image-worker` child process.
//!
//! The child gets a clean environment, `/` as its working directory, the
//! input file as stdin, no stderr, and the path of a temporary output file the
//! parent created next to the final output. The parent kills the child when
//! the time limit passes or (where the OS lets us measure it) when its memory
//! goes over the limit. Every abnormal end becomes [`WorkerError::InvalidImage`],
//! which carries only the format and a size bucket.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::codec::{CodecError, Facts, ImageFormat};
use crate::job::{Job, JobResult, OutputSize};

/// Default time limit per job (spec §8.4).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default memory limit per job (spec §8.4).
pub const DEFAULT_MEMORY_LIMIT: u64 = 1 << 30;
/// How often the parent checks the child's exit, clock and memory.
const POLL: Duration = Duration::from_millis(10);
/// Most bytes of child stdout the parent reads; a result line is far smaller.
const MAX_RESULT_LINE: u64 = 64 * 1024;

/// Upload size bucket for error reports (PH §8). Sizes in MiB, matching the
/// 50 MB (52,428,800-byte) limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum SizeBucket {
    #[serde(rename = "<5MB")]
    Under5Mb,
    #[serde(rename = "5-20MB")]
    From5To20Mb,
    /// 20 MiB and over (inputs over 50 MB are refused before this point).
    #[serde(rename = "20-50MB")]
    From20To50Mb,
}

impl SizeBucket {
    pub fn for_size(bytes: u64) -> SizeBucket {
        const MIB: u64 = 1024 * 1024;
        match bytes {
            b if b < 5 * MIB => SizeBucket::Under5Mb,
            b if b < 20 * MIB => SizeBucket::From5To20Mb,
            _ => SizeBucket::From20To50Mb,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SizeBucket::Under5Mb => "<5MB",
            SizeBucket::From5To20Mb => "5-20MB",
            SizeBucket::From20To50Mb => "20-50MB",
        }
    }
}

/// The image is invalid, or the worker ended abnormally on it. Callers
/// translate this to `invalid_image`. Carries no path, name or content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct InvalidImage {
    /// The format the uploader claimed (the content may be something else).
    pub format: ImageFormat,
    pub size_bucket: SizeBucket,
}

/// Why a worker job did not produce a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerError {
    /// Decode failed, or the child timed out, crashed, went over its memory
    /// limit, exited non-zero or printed a malformed result.
    InvalidImage(InvalidImage),
    /// No codec back end on this operating system.
    UnsupportedPlatform,
    /// The worker executable path is not absolute (no `PATH` search).
    WorkerPathNotAbsolute,
    /// A host-side I/O step failed (opening the input, creating the output,
    /// starting the child). Not the image's fault.
    Io(std::io::ErrorKind),
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkerError::InvalidImage(i) => write!(
                f,
                "invalid image (format {}, size {})",
                i.format.as_str(),
                i.size_bucket.as_str()
            ),
            WorkerError::UnsupportedPlatform => f.write_str("no image codec on this platform"),
            WorkerError::WorkerPathNotAbsolute => f.write_str("worker path must be absolute"),
            WorkerError::Io(kind) => write!(f, "image worker I/O error: {kind}"),
        }
    }
}

impl std::error::Error for WorkerError {}

impl From<std::io::Error> for WorkerError {
    fn from(e: std::io::Error) -> Self {
        WorkerError::Io(e.kind())
    }
}

/// What a successful job returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobOutput {
    pub facts: Facts,
    /// Size of the written image, for jobs that write one.
    pub output: Option<OutputSize>,
}

/// A job to run. Test-only kinds exist in debug builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    Decode,
    Preview,
    Compatible,
    #[cfg(debug_assertions)]
    TestHang,
    #[cfg(debug_assertions)]
    TestAbort,
    #[cfg(debug_assertions)]
    TestAlloc(u64),
    #[cfg(debug_assertions)]
    TestGarbage,
    #[cfg(debug_assertions)]
    TestExitNonZero,
}

impl Task {
    fn writes_output(self) -> bool {
        match self {
            Task::Preview | Task::Compatible => true,
            #[cfg(debug_assertions)]
            Task::TestHang | Task::TestAbort => true,
            _ => false,
        }
    }

    fn job(self, claimed: ImageFormat, output: Option<PathBuf>) -> Job {
        let out = || output.clone().unwrap_or_default();
        match self {
            Task::Decode => Job::Decode { claimed },
            Task::Preview => Job::Preview {
                claimed,
                output: out(),
            },
            Task::Compatible => Job::Compatible {
                claimed,
                output: out(),
            },
            #[cfg(debug_assertions)]
            Task::TestHang => Job::TestHang { output: out() },
            #[cfg(debug_assertions)]
            Task::TestAbort => Job::TestAbort { output: out() },
            #[cfg(debug_assertions)]
            Task::TestAlloc(bytes) => Job::TestAlloc { bytes },
            #[cfg(debug_assertions)]
            Task::TestGarbage => Job::TestGarbage,
            #[cfg(debug_assertions)]
            Task::TestExitNonZero => Job::TestExitNonZero,
        }
    }
}

type SpawnObserver = Arc<dyn Fn(u32) + Send + Sync>;

/// Runs image jobs in `image-worker` child processes, one process per job.
#[derive(Clone)]
pub struct Worker {
    exe: PathBuf,
    timeout: Duration,
    memory_limit: u64,
    on_spawn: Option<SpawnObserver>,
}

impl fmt::Debug for Worker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The executable path is deliberately left out.
        f.debug_struct("Worker")
            .field("timeout", &self.timeout)
            .field("memory_limit", &self.memory_limit)
            .finish_non_exhaustive()
    }
}

impl Worker {
    /// A worker that runs the executable at `exe`, which must be an absolute
    /// path (tests pass `env!("CARGO_BIN_EXE_image-worker")`, the packaged app
    /// passes the path of its bundled binary). `PATH` is never searched.
    pub fn new(exe: impl Into<PathBuf>) -> Result<Worker, WorkerError> {
        let exe = exe.into();
        if !exe.is_absolute() {
            return Err(WorkerError::WorkerPathNotAbsolute);
        }
        Ok(Worker {
            exe,
            timeout: DEFAULT_TIMEOUT,
            memory_limit: DEFAULT_MEMORY_LIMIT,
            on_spawn: None,
        })
    }

    /// Change the time limit (default 10 s).
    pub fn with_timeout(mut self, timeout: Duration) -> Worker {
        self.timeout = timeout;
        self
    }

    /// Change the memory limit in bytes (default 1 GiB).
    pub fn with_memory_limit(mut self, bytes: u64) -> Worker {
        self.memory_limit = bytes;
        self
    }

    /// Call `f` with each child's process ID right after it starts (used by
    /// tests to check that no child outlives its job).
    pub fn with_spawn_observer(mut self, f: impl Fn(u32) + Send + Sync + 'static) -> Worker {
        self.on_spawn = Some(Arc::new(f));
        self
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Full decode check of the file at `input`, whose uploader claimed it is
    /// `claimed`; returns its facts.
    pub fn decode(&self, input: &Path, claimed: ImageFormat) -> Result<Facts, WorkerError> {
        self.run(Task::Decode, input, claimed, None)
            .map(|o| o.facts)
    }

    /// Decode check, then write the preview JPEG to `output`.
    pub fn preview(
        &self,
        input: &Path,
        claimed: ImageFormat,
        output: &Path,
    ) -> Result<JobOutput, WorkerError> {
        self.run(Task::Preview, input, claimed, Some(output))
    }

    /// Decode check, then write the compatible JPEG to `output`.
    pub fn compatible(
        &self,
        input: &Path,
        claimed: ImageFormat,
        output: &Path,
    ) -> Result<JobOutput, WorkerError> {
        self.run(Task::Compatible, input, claimed, Some(output))
    }

    /// Run `task` on the file at `input`. For tasks that write an image,
    /// `output` is where it ends up; it is written only on success, and no
    /// temporary file is left behind on failure.
    pub fn run(
        &self,
        task: Task,
        input: &Path,
        claimed: ImageFormat,
        output: Option<&Path>,
    ) -> Result<JobOutput, WorkerError> {
        let file = File::open(input)?;
        let invalid = WorkerError::InvalidImage(InvalidImage {
            format: claimed,
            size_bucket: SizeBucket::for_size(file.metadata()?.len()),
        });

        let temp = if task.writes_output() {
            let output = output.ok_or(WorkerError::Io(std::io::ErrorKind::InvalidInput))?;
            let dir = match output.parent() {
                Some(d) if !d.as_os_str().is_empty() => d,
                _ => Path::new("."),
            };
            Some(
                tempfile::Builder::new()
                    .prefix(".image-worker-")
                    .suffix(".partial")
                    .tempfile_in(dir)?,
            )
        } else {
            None
        };
        let job = task.job(claimed, temp.as_ref().map(|t| t.path().to_path_buf()));
        let result = self.execute(&job, file)?.ok_or(invalid)?;
        match (result, task.writes_output(), temp) {
            (JobResult::Ok { facts, output: out }, false, None) if out.is_none() => Ok(JobOutput {
                facts,
                output: None,
            }),
            (
                JobResult::Ok {
                    facts,
                    output: Some(out),
                },
                true,
                Some(temp),
            ) => {
                let dest = output.ok_or(WorkerError::Io(std::io::ErrorKind::InvalidInput))?;
                temp.persist(dest).map_err(|e| WorkerError::from(e.error))?;
                Ok(JobOutput {
                    facts,
                    output: Some(out),
                })
            }
            (
                JobResult::Error {
                    error: CodecError::UnsupportedPlatform,
                },
                _,
                _,
            ) => Err(WorkerError::UnsupportedPlatform),
            _ => Err(invalid),
        }
    }

    /// Test only: the names of the environment variables a worker child sees.
    #[cfg(debug_assertions)]
    pub fn child_env_var_names(&self) -> Result<Vec<String>, WorkerError> {
        let input = File::open("/dev/null")?;
        match self.execute(&Job::TestEnv, input)? {
            Some(JobResult::Env { names }) => Ok(names),
            _ => Err(WorkerError::Io(std::io::ErrorKind::InvalidData)),
        }
    }

    /// Spawn one child for `job` with `input` as its stdin and supervise it.
    /// `Ok(None)` means it ended abnormally: killed, crashed, non-zero exit or
    /// no well-formed result line.
    fn execute(&self, job: &Job, input: File) -> Result<Option<JobResult>, WorkerError> {
        let job = serde_json::to_string(job)
            .map_err(|_| WorkerError::Io(std::io::ErrorKind::InvalidData))?;
        let mut child = Command::new(&self.exe)
            .arg(job)
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::from(input))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        if let Some(f) = &self.on_spawn {
            f(child.id());
        }
        let stdout = child.stdout.take();
        let reader = thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(out) = stdout {
                let _ = out.take(MAX_RESULT_LINE).read_to_end(&mut buf);
            }
            buf
        });
        let status = self.supervise(&mut child);
        // The child is dead and reaped here, so its stdout is closed.
        let line = reader.join().unwrap_or_default();
        Ok(match status {
            Some(status) if status.success() => parse_result(&line),
            _ => None,
        })
    }

    /// Wait for the child, killing it on timeout or memory overrun. Returns
    /// its exit status, or `None` if it was killed by us. Always reaps it.
    fn supervise(&self, child: &mut Child) -> Option<ExitStatus> {
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Some(status),
                Ok(None) => {}
                Err(_) => break,
            }
            if start.elapsed() >= self.timeout || self.over_memory(child.id()) {
                break;
            }
            thread::sleep(POLL);
        }
        let _ = child.kill();
        let _ = child.wait();
        None
    }

    fn over_memory(&self, pid: u32) -> bool {
        memory_footprint(pid).is_some_and(|bytes| bytes > self.memory_limit)
    }
}

/// Memory in use by process `pid`, where this OS lets us read it.
fn memory_footprint(pid: u32) -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::phys_footprint(pid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        None
    }
}

/// Exactly one non-empty JSON line, nothing after it.
fn parse_result(bytes: &[u8]) -> Option<JobResult> {
    let text = std::str::from_utf8(bytes).ok()?;
    let line = text.strip_suffix('\n')?;
    if line.is_empty() || line.contains('\n') {
        return None;
    }
    serde_json::from_str(line).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_buckets() {
        const MIB: u64 = 1024 * 1024;
        assert_eq!(SizeBucket::for_size(0), SizeBucket::Under5Mb);
        assert_eq!(SizeBucket::for_size(5 * MIB - 1), SizeBucket::Under5Mb);
        assert_eq!(SizeBucket::for_size(5 * MIB), SizeBucket::From5To20Mb);
        assert_eq!(SizeBucket::for_size(20 * MIB), SizeBucket::From20To50Mb);
        assert_eq!(SizeBucket::for_size(52_428_800), SizeBucket::From20To50Mb);
        assert_eq!(
            serde_json::to_string(&SizeBucket::From5To20Mb).unwrap(),
            "\"5-20MB\""
        );
    }

    #[test]
    fn relative_worker_path_is_refused() {
        assert_eq!(
            Worker::new("image-worker").unwrap_err(),
            WorkerError::WorkerPathNotAbsolute
        );
        assert_eq!(
            Worker::new("target/debug/image-worker").unwrap_err(),
            WorkerError::WorkerPathNotAbsolute
        );
    }

    #[test]
    fn result_line_must_be_exactly_one_line() {
        let ok = br#"{"status":"error","error":"decode_failed"}"#;
        let mut line = ok.to_vec();
        assert!(parse_result(&line).is_none(), "no newline");
        line.push(b'\n');
        assert!(parse_result(&line).is_some());
        let mut two = line.clone();
        two.extend_from_slice(&line);
        assert!(parse_result(&two).is_none());
        assert!(parse_result(b"\n").is_none());
        assert!(parse_result(b"{\"status\":\"ok\"}\n").is_none());
    }

    #[test]
    fn error_text_has_no_path() {
        let e = WorkerError::InvalidImage(InvalidImage {
            format: ImageFormat::Heic,
            size_bucket: SizeBucket::Under5Mb,
        });
        assert_eq!(e.to_string(), "invalid image (format heic, size <5MB)");
    }
}
