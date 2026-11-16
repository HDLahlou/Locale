"""Tests for scripts/doctor.py. They run the script as a subprocess with a PATH
that holds only fake tools, so results do not depend on what is installed."""

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SCRIPT = os.path.join(ROOT, "scripts", "doctor.py")
WINDOWS = os.name == "nt"


def load_doctor():
    spec = importlib.util.spec_from_file_location("doctor", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def make_fake(directory, name, body):
    """Create a fake tool. Posix: a shell script. Windows: a .cmd file."""
    if WINDOWS:
        path = os.path.join(directory, name + ".cmd")
        content = "@echo off\r\n" + body + "\r\n"
    else:
        path = os.path.join(directory, name)
        content = "#!/bin/sh\n" + body + "\n"
    with open(path, "w") as handle:
        handle.write(content)
    os.chmod(path, 0o755)


def fake_version_tool(directory, name):
    make_fake(directory, name, f"echo {name} 1.0.0")


def run_doctor(path_dir, *args, extra_env=None):
    env = {"PATH": path_dir, "HOME": path_dir}
    if WINDOWS:
        env["SYSTEMROOT"] = os.environ.get("SYSTEMROOT", "")
    env.update(extra_env or {})
    return subprocess.run(
        [sys.executable, "-I", SCRIPT, *args],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
    )


def row_for(output, name):
    for line in output.splitlines():
        fields = line.split()
        if fields and fields[0] == name:
            return line
    raise AssertionError(f"no row for {name} in:\n{output}")


class DoctorTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.empty = os.path.join(self._tmp.name, "empty")
        os.makedirs(self.empty)

    def test_needed_tool_missing_exits_1_and_row_says_missing(self):
        result = run_doctor(self.empty, "--only", "cargo")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        row = row_for(result.stdout, "cargo")
        self.assertIn(" missing ", row)
        self.assertIn(" yes ", row)
        self.assertIn("Install Rust", row)

    def test_missing_not_needed_tool_keeps_exit_0(self):
        result = run_doctor(self.empty, "--only", "cargo-ndk")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        row = row_for(result.stdout, "cargo-ndk")
        self.assertIn(" missing ", row)
        self.assertIn(" no ", row)

    def test_present_needed_tool_is_ok(self):
        fakes = os.path.join(self._tmp.name, "fakes")
        os.makedirs(fakes)
        fake_version_tool(fakes, "just")
        result = run_doctor(fakes, "--only", "just")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(" ok ", row_for(result.stdout, "just"))
        self.assertIn("just 1.0.0", row_for(result.stdout, "just"))

    def test_nonzero_exit_is_missing_with_reason(self):
        fakes = os.path.join(self._tmp.name, "fakes")
        os.makedirs(fakes)
        make_fake(fakes, "just", "echo broken >&2\nexit 3")
        result = run_doctor(fakes, "--only", "just")
        self.assertEqual(result.returncode, 1)
        row = row_for(result.stdout, "just")
        self.assertIn(" missing ", row)
        self.assertIn("exit 3", row)

    def test_hanging_tool_is_reported_after_timeout(self):
        fakes = os.path.join(self._tmp.name, "fakes")
        os.makedirs(fakes)
        if WINDOWS:
            make_fake(fakes, "just", f'"{sys.executable}" -c "import time; time.sleep(60)"')
        else:
            make_fake(fakes, "just", f"exec {shutil.which('sleep')} 60")
        started = time.monotonic()
        result = run_doctor(fakes, "--only", "just", extra_env={"LOCALE_DOCTOR_TIMEOUT": "1"})
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        row = row_for(result.stdout, "just")
        self.assertIn(" missing ", row)
        self.assertIn("timed out after 1s", row)
        self.assertLess(elapsed, 60)

    def test_json_parses_with_one_entry_per_check(self):
        result = run_doctor(self.empty, "--json")
        entries = json.loads(result.stdout)
        doctor = load_doctor()
        applicable = [c for c in doctor.CHECKS if c.applies(sys.platform)]
        self.assertEqual(len(entries), len(applicable))
        self.assertEqual([e["name"] for e in entries], [c.name for c in applicable])
        for entry in entries:
            self.assertEqual(
                set(entry), {"name", "status", "detail", "needed_m1", "hint"}
            )
            self.assertIn(entry["status"], ("ok", "missing", "warn"))
            if entry["status"] != "ok":
                self.assertTrue(entry["hint"])

    def test_json_exit_code_matches_table(self):
        result = run_doctor(self.empty, "--json", "--only", "cargo,gh")
        self.assertEqual(result.returncode, 1)
        entries = json.loads(result.stdout)
        self.assertEqual({e["name"] for e in entries}, {"cargo", "gh"})

    def test_unknown_only_name_is_rejected(self):
        result = run_doctor(self.empty, "--only", "no-such-tool")
        self.assertEqual(result.returncode, 2)
        self.assertIn("unknown check", result.stderr)

    def test_checks_are_registered_once_per_name(self):
        doctor = load_doctor()
        names = [c.name for c in doctor.CHECKS]
        self.assertEqual(len(names), len(set(names)))

    def test_ios_targets_skipped_on_windows(self):
        doctor = load_doctor()
        ios = [c for c in doctor.CHECKS if "apple-ios" in c.name]
        self.assertEqual(len(ios), 2)
        self.assertFalse(any(c.applies("win32") for c in ios))
        self.assertTrue(all(c.applies("darwin") for c in ios))

    def test_m1_flags_match_brief(self):
        doctor = load_doctor()
        needed = {c.name for c in doctor.CHECKS if c.needed}
        not_needed = {c.name for c in doctor.CHECKS if not c.needed}
        self.assertTrue({"cargo", "rustc", "just", "node", "pnpm", "flutter", "dart", "jdk", "adb"} <= needed)
        self.assertEqual(not_needed, {"cargo-ndk", "gh", "playwright-browsers", "libheif", "disk-space"})

    def test_disk_space_is_warn_only(self):
        doctor = load_doctor()
        disk = next(c for c in doctor.CHECKS if c.name == "disk-space")
        self.assertFalse(disk.needed)
        original = doctor.shutil.disk_usage
        try:
            doctor.shutil.disk_usage = lambda _path: type("U", (), {"free": 1024})()
            self.assertEqual(doctor.run_check(disk)["status"], "warn")
        finally:
            doctor.shutil.disk_usage = original


if __name__ == "__main__":
    unittest.main()
