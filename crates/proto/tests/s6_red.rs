//! S6 proof only: lives on ci/s6-red, never merged.
#[cfg(windows)]
#[test]
fn s6_red_windows_only_failure() {
    panic!("S6 red proof: intentional Windows-only failure");
}
