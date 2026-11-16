//! What the `image-worker` binary runs: one job, then exit.
//!
//! The child reads the image only from the handle it was given (stdin),
//! writes output only to the file the parent created and named in the job,
//! never opens a network socket and never logs names or paths.

use std::fs::OpenOptions;
use std::io::{Read, Write};

use crate::codec::{platform_codec, CodecError, Encoded, ImageCodec, ImageFormat};
use crate::job::{Job, JobResult, OutputSize};
use crate::sniff::sniff;

/// Largest input the child reads (the 50 MB upload limit). Anything longer is
/// refused without decoding.
pub const MAX_INPUT_BYTES: u64 = 52_428_800;

/// Exit code when the job argument is missing or malformed.
pub const EXIT_BAD_JOB: i32 = 2;
/// Exit code when the input or output handle failed.
pub const EXIT_IO: i32 = 4;

/// Run the job described by `args` (exactly one JSON argument) on `input`,
/// printing the result line to `out`. Returns the process exit code.
pub fn run(args: &[String], input: impl Read, out: impl Write) -> i32 {
    let [job] = args else {
        return EXIT_BAD_JOB;
    };
    let Ok(job) = serde_json::from_str::<Job>(job) else {
        return EXIT_BAD_JOB;
    };
    match run_job(job, input) {
        Ok(result) => print_result(out, &result),
        Err(code) => code,
    }
}

fn print_result(mut out: impl Write, result: &JobResult) -> i32 {
    let Ok(mut line) = serde_json::to_vec(result) else {
        return EXIT_IO;
    };
    line.push(b'\n');
    match out.write_all(&line).and_then(|()| out.flush()) {
        Ok(()) => 0,
        Err(_) => EXIT_IO,
    }
}

fn read_input(input: impl Read) -> Result<Option<Vec<u8>>, i32> {
    let mut bytes = Vec::new();
    input
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| EXIT_IO)?;
    Ok((bytes.len() as u64 <= MAX_INPUT_BYTES).then_some(bytes))
}

/// Read the input and check its content against the claimed type before any
/// decoder sees it. Outer error: I/O exit code; inner error: refusal.
fn read_checked(
    input: impl Read,
    claimed: ImageFormat,
) -> Result<Result<Vec<u8>, CodecError>, i32> {
    let Some(bytes) = read_input(input)? else {
        return Ok(Err(CodecError::DecodeFailed));
    };
    Ok(match sniff(&bytes) {
        Ok(format) if format == claimed => Ok(bytes),
        Ok(_) => Err(CodecError::FormatMismatch),
        Err(_) => Err(CodecError::NotAnImage),
    })
}

fn write_output(path: &std::path::Path, bytes: &[u8]) -> Result<(), i32> {
    // The parent created this file; the child never creates files.
    let mut f = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|_| EXIT_IO)?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|_| EXIT_IO)
}

fn run_job(job: Job, input: impl Read) -> Result<JobResult, i32> {
    let codec = platform_codec();
    match job {
        Job::Decode { claimed } => {
            let bytes = match read_checked(input, claimed)? {
                Ok(bytes) => bytes,
                Err(e) => return Ok(error(e)),
            };
            Ok(match codec.decode(&bytes) {
                Ok(facts) => JobResult::Ok {
                    facts,
                    output: None,
                },
                Err(e) => error(e),
            })
        }
        Job::Preview { claimed, output } => {
            encode_job(codec.as_ref(), input, claimed, &output, |c, b| c.preview(b))
        }
        Job::Compatible { claimed, output } => {
            encode_job(codec.as_ref(), input, claimed, &output, |c, b| {
                c.compatible(b)
            })
        }
        #[cfg(debug_assertions)]
        Job::TestHang { output } => {
            write_output(&output, b"x")?;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        #[cfg(debug_assertions)]
        Job::TestAbort { output } => {
            write_output(&output, b"x")?;
            std::process::abort();
        }
        #[cfg(debug_assertions)]
        Job::TestAlloc { bytes } => {
            let mut v: Vec<u8> = Vec::new();
            let chunk = 16 * 1024 * 1024;
            let mut left = bytes as usize;
            let mut keep = Vec::new();
            while left > 0 {
                let n = left.min(chunk);
                v.resize(n, 1);
                keep.push(std::mem::take(&mut v));
                left -= n;
            }
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                std::hint::black_box(&keep);
            }
        }
        #[cfg(debug_assertions)]
        Job::TestGarbage => Err(print_raw(b"{\"status\":\"not a result\"}\n")),
        #[cfg(debug_assertions)]
        Job::TestExitNonZero => Err(3),
        #[cfg(debug_assertions)]
        Job::TestEnv => Ok(JobResult::Env {
            names: std::env::vars_os()
                .map(|(k, _)| k.to_string_lossy().into_owned())
                .collect(),
        }),
    }
}

#[cfg(debug_assertions)]
fn print_raw(bytes: &[u8]) -> i32 {
    let mut out = std::io::stdout();
    let _ = out.write_all(bytes);
    let _ = out.flush();
    0
}

fn error(error: CodecError) -> JobResult {
    JobResult::Error { error }
}

fn encode_job(
    codec: &dyn ImageCodec,
    input: impl Read,
    claimed: ImageFormat,
    output: &std::path::Path,
    f: impl Fn(&dyn ImageCodec, &[u8]) -> Result<Encoded, CodecError>,
) -> Result<JobResult, i32> {
    let bytes = match read_checked(input, claimed)? {
        Ok(bytes) => bytes,
        Err(e) => return Ok(error(e)),
    };
    let encoded = match f(codec, &bytes) {
        Ok(e) => e,
        Err(e) => return Ok(error(e)),
    };
    write_output(output, &encoded.bytes)?;
    Ok(JobResult::Ok {
        facts: encoded.facts,
        output: Some(OutputSize {
            width: encoded.width,
            height: encoded.height,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECODE_JPEG: &str = r#"{"kind":"decode","claimed":"jpeg"}"#;

    fn run_str(args: &[&str], input: &[u8]) -> (i32, String) {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        let code = run(&args, input, &mut out);
        (code, String::from_utf8(out).unwrap())
    }

    #[test]
    fn bad_arguments_exit_without_output() {
        assert_eq!(run_str(&[], b""), (EXIT_BAD_JOB, String::new()));
        assert_eq!(run_str(&["{}"], b""), (EXIT_BAD_JOB, String::new()));
        assert_eq!(
            run_str(&[DECODE_JPEG, "extra"], b""),
            (EXIT_BAD_JOB, String::new())
        );
    }

    #[test]
    fn non_image_gives_one_error_line() {
        let (code, out) = run_str(&[DECODE_JPEG], b"hello");
        assert_eq!(code, 0);
        assert_eq!(out.lines().count(), 1);
        let r: JobResult = serde_json::from_str(out.trim_end()).unwrap();
        assert_eq!(
            r,
            JobResult::Error {
                error: CodecError::NotAnImage
            }
        );
    }

    #[test]
    fn content_must_match_claimed_type() {
        let png = b"\x89PNG\r\n\x1a\n....";
        let (code, out) = run_str(&[DECODE_JPEG], png);
        assert_eq!(code, 0);
        let r: JobResult = serde_json::from_str(out.trim_end()).unwrap();
        assert_eq!(
            r,
            JobResult::Error {
                error: CodecError::FormatMismatch
            }
        );
    }

    #[test]
    fn oversized_input_is_refused_unread() {
        let big = std::io::repeat(0xFF).take(MAX_INPUT_BYTES + 1);
        let mut out = Vec::new();
        let code = run(&[DECODE_JPEG.to_string()], big, &mut out);
        assert_eq!(code, 0);
        let r: JobResult = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            r,
            JobResult::Error {
                error: CodecError::DecodeFailed
            }
        );
    }
}
