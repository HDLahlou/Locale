"""Toolchain probe behind `just doctor`.

Only looks: never installs, changes files or uses the network.
Run as: python3 -I scripts/doctor.py [--json] [--only NAME[,NAME...]]
Exit 0 when every check needed for M1 is ok, 1 otherwise.
Each external command gets LOCALE_DOCTOR_TIMEOUT seconds (default 20).
"""

import argparse
import functools
import json
import os
import re
import shutil
import signal
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OK, MISSING, WARN = "ok", "missing", "warn"
ALL_PLATFORMS = ("darwin", "win32", "linux")
MACOS = ("darwin",)
WINDOWS = ("win32",)
NOT_WINDOWS = ("darwin", "linux")
DISK_NEEDED_BYTES = 20 * 1024**3
RUST_TARGETS = {
    "aarch64-apple-ios": NOT_WINDOWS,
    "aarch64-apple-ios-sim": NOT_WINDOWS,
    "aarch64-linux-android": ALL_PLATFORMS,
    "armv7-linux-androideabi": ALL_PLATFORMS,
    "x86_64-linux-android": ALL_PLATFORMS,
}
UUID_RE = re.compile(r"[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}")


class CmdError(Exception):
    """An external command could not run or timed out; reported as missing."""


class Check:
    def __init__(self, name, fn, needed, platforms, hint):
        self.name, self.fn, self.needed, self.platforms, self.hint = name, fn, needed, platforms, hint

    def applies(self, platform):
        return platform in self.platforms


CHECKS = []


def check(name, hint, needed=True, platforms=ALL_PLATFORMS):
    """Register a check. The decorated function returns (status, detail)."""

    def deco(fn):
        CHECKS.append(Check(name, fn, needed, platforms, hint))
        return fn

    return deco


def timeout_seconds():
    return float(os.environ.get("LOCALE_DOCTOR_TIMEOUT", "20"))


def run(prog, *args):
    """Run a command; return (exit code, combined output). Raises CmdError if it cannot run."""
    path = shutil.which(prog)
    if path is None and os.path.isabs(prog) and os.path.isfile(prog):
        path = prog
    if path is None:
        raise CmdError(f"{prog} not found on PATH")
    limit = timeout_seconds()
    extra = {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {"start_new_session": True}
    try:
        proc = subprocess.Popen(
            [path, *args],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            errors="replace",
            stdin=subprocess.DEVNULL,
            **extra,
        )
    except OSError as err:
        raise CmdError(f"{prog} failed to start: {err}")
    try:
        stdout, stderr = proc.communicate(timeout=limit)
    except subprocess.TimeoutExpired:
        kill_tree(proc)
        try:
            proc.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        raise CmdError(f"{prog} timed out after {limit:g}s")
    return proc.returncode, stdout + stderr


def kill_tree(proc):
    """Kill the child and everything it started, so no grandchild keeps the pipes open."""
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/T", "/F", "/PID", str(proc.pid)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL,
        )
    else:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except OSError:
            pass
    proc.kill()


def first_line(text):
    for line in text.splitlines():
        if line.strip():
            return line.strip()
    return ""


def from_version(prog, *args):
    """Status from a version command: ok when it exits 0."""
    try:
        rc, out = run(prog, *args)
    except CmdError as err:
        return MISSING, str(err)
    if rc != 0:
        return MISSING, f"exit {rc}: {first_line(out)}"
    return OK, first_line(out)


def version_tuple(text):
    match = re.search(r"(\d+)\.(\d+)", text)
    return (int(match.group(1)), int(match.group(2))) if match else None


def which_status(prog):
    path = shutil.which(prog)
    return (OK, path) if path else (MISSING, f"{prog} not found on PATH")


# --- Rust ---------------------------------------------------------------

@check("cargo", "Install Rust with rustup: https://rustup.rs")
def _cargo():
    return from_version("cargo", "--version")


@check("rustc", "Install Rust with rustup: https://rustup.rs")
def _rustc():
    return from_version("rustc", "--version")


@check("rustfmt", "Run: rustup component add rustfmt")
def _rustfmt():
    return from_version("cargo", "fmt", "--version")


@check("clippy", "Run: rustup component add clippy")
def _clippy():
    return from_version("cargo", "clippy", "--version")


@functools.lru_cache(maxsize=None)
def installed_rust_targets():
    rc, out = run("rustup", "target", "list", "--installed")
    if rc != 0:
        raise CmdError(f"rustup target list failed: {first_line(out)}")
    return frozenset(out.split())


def target_check(target):
    def fn():
        try:
            installed = installed_rust_targets()
        except CmdError as err:
            return MISSING, str(err)
        return (OK, "installed") if target in installed else (MISSING, "not installed")

    return fn


for _target, _platforms in RUST_TARGETS.items():
    CHECKS.append(
        Check(_target, target_check(_target), True, _platforms, f"Run: rustup target add {_target}")
    )


# --- Tools --------------------------------------------------------------

@check("just", "Install just: https://just.systems")
def _just():
    return from_version("just", "--version")


@check("node", "Install Node.js 20 or later: https://nodejs.org")
def _node():
    try:
        rc, out = run("node", "--version")
    except CmdError as err:
        return MISSING, str(err)
    if rc != 0:
        return MISSING, f"exit {rc}: {first_line(out)}"
    version = first_line(out)
    match = re.match(r"v(\d+)", version)
    if match and int(match.group(1)) >= 20:
        return OK, version
    return MISSING, f"{version} found, need 20 or later"


@check("pnpm", "Install pnpm 12.18 (see package.json), e.g. corepack enable")
def _pnpm():
    return from_version("pnpm", "--version")


@check("flutter", "Install Flutter 3.47.7 (stable): https://docs.flutter.dev/install")
def _flutter():
    return from_version("flutter", "--version")


@check("dart", "Installed with Flutter; put the Flutter bin folder on PATH")
def _dart():
    return from_version("dart", "--version")


@check("flutter_rust_bridge_codegen", "Run: cargo install flutter_rust_bridge_codegen")
def _frb():
    return from_version("flutter_rust_bridge_codegen", "--version")


@check("python3", "Install Python 3.9 or later")
def _python3():
    try:
        rc, out = run("python3", "--version")
    except CmdError as err:
        return MISSING, str(err)
    if rc != 0:
        return MISSING, f"exit {rc}: {first_line(out)}"
    version = first_line(out)
    found = version_tuple(version)
    if found and found >= (3, 9):
        return OK, version
    return MISSING, f"{version} found, need 3.9 or later"


@check("xcode", "Install Xcode, then run: sudo xcode-select -s /Applications/Xcode.app", platforms=MACOS)
def _xcode():
    return from_version("xcodebuild", "-version")


@check("ios-simulator", "Open Xcode > Settings > Platforms and download an iOS simulator runtime", platforms=MACOS)
def _ios_simulator():
    try:
        rc, out = run("xcrun", "simctl", "list", "devices", "available")
    except CmdError as err:
        return MISSING, str(err)
    if rc != 0:
        return MISSING, f"exit {rc}: {first_line(out)}"
    section, count = "", 0
    for line in out.splitlines():
        stripped = line.strip()
        if stripped.startswith("--"):
            section = stripped
        elif "iOS" in section and UUID_RE.search(line):
            count += 1
    if count:
        return OK, f"{count} available iOS simulator(s)"
    return MISSING, "no available iOS simulator"


@check("sips", "sips ships with macOS; check that /usr/bin is on PATH", platforms=MACOS)
def _sips():
    return which_status("sips")


def android_sdk_root():
    for var in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        value = os.environ.get(var)
        if value and os.path.isdir(value):
            return value
    adb = shutil.which("adb")
    if adb:
        root = os.path.dirname(os.path.dirname(adb))
        if os.path.isdir(os.path.join(root, "platform-tools")):
            return root
    return None


@check("android-sdk", "Install Android Studio or the command-line SDK, then set ANDROID_HOME")
def _android_sdk():
    root = android_sdk_root()
    if root:
        return OK, root
    return MISSING, "no ANDROID_HOME, ANDROID_SDK_ROOT or adb on PATH"


@check("android-ndk", "In Android Studio SDK Manager, install the NDK (Side by side)")
def _android_ndk():
    root = android_sdk_root()
    if root is None:
        return MISSING, "Android SDK not found"
    ndk_dir = os.path.join(root, "ndk")
    if os.path.isdir(ndk_dir) and os.listdir(ndk_dir):
        return OK, sorted(os.listdir(ndk_dir))[-1]
    if os.path.isdir(os.path.join(root, "ndk-bundle")):
        return OK, "ndk-bundle"
    return MISSING, f"no NDK directory under {root}"


@check("adb", "Install platform-tools in the Android SDK Manager")
def _adb():
    return from_version("adb", "version")


@check("emulator", "Install the Android Emulator and create an AVD in Device Manager")
def _emulator():
    root = android_sdk_root()
    emulator = shutil.which("emulator")
    if emulator is None and root:
        name = "emulator.exe" if sys.platform == "win32" else "emulator"
        candidate = os.path.join(root, "emulator", name)
        if os.path.isfile(candidate):
            emulator = candidate
    if emulator is None:
        return MISSING, "emulator not found"
    try:
        rc, out = run(emulator, "-list-avds")
    except CmdError as err:
        return MISSING, str(err)
    if rc != 0:
        return MISSING, f"exit {rc}: {first_line(out)}"
    avds = [
        line.strip()
        for line in out.splitlines()
        if line.strip() and not line.startswith(("INFO", "WARNING"))
    ]
    if avds:
        return OK, f"{len(avds)} AVD(s): {', '.join(avds)}"
    return MISSING, "no AVD created"


@check("jdk", "Install a JDK 17 or later (e.g. Temurin 17)")
def _jdk():
    try:
        rc, out = run("java", "-version")
    except CmdError as err:
        return MISSING, str(err)
    match = re.search(r'version "([^"]+)"', out)
    if rc != 0 or not match:
        return MISSING, f"no working java: {first_line(out)}"
    parts = match.group(1).split(".")
    major = int(parts[1]) if parts[0] == "1" and len(parts) > 1 else int(parts[0])
    if major >= 17:
        return OK, f"Java {match.group(1)}"
    return MISSING, f"Java {match.group(1)} found, need 17 or later"


@check("cargo-ndk", "Run: cargo install cargo-ndk", needed=False)
def _cargo_ndk():
    return from_version("cargo-ndk", "--version")


@check("gh", "Install GitHub CLI and run: gh auth login", needed=False)
def _gh():
    status, detail = from_version("gh", "--version")
    if status != OK:
        return status, detail
    try:
        rc, _ = run("gh", "auth", "status")
    except CmdError as err:
        return MISSING, str(err)
    if rc != 0:
        return MISSING, "not signed in"
    return OK, detail


@check("playwright-browsers", "Run: pnpm exec playwright install chromium", needed=False)
def _playwright():
    override = os.environ.get("PLAYWRIGHT_BROWSERS_PATH")
    if override:
        cache = override
    elif sys.platform == "darwin":
        cache = os.path.expanduser("~/Library/Caches/ms-playwright")
    elif sys.platform == "win32":
        cache = os.path.join(os.environ.get("LOCALAPPDATA", ""), "ms-playwright")
    else:
        cache = os.path.expanduser("~/.cache/ms-playwright")
    if os.path.isdir(cache):
        browsers = [name for name in os.listdir(cache) if name.startswith("chromium")]
        if browsers:
            return OK, ", ".join(sorted(browsers))
    return MISSING, f"no chromium under {cache}"


@check("libheif", "Only bundled for Windows builds; for local use install libheif", needed=False)
def _libheif():
    try:
        rc, out = run("pkg-config", "--modversion", "libheif")
    except CmdError as err:
        return MISSING, str(err)
    if rc != 0:
        return MISSING, "not found by pkg-config"
    return OK, "libheif " + first_line(out)


# --- Windows ------------------------------------------------------------

@check("msvc-build-tools", "Install Visual Studio Build Tools with the C++ workload", platforms=WINDOWS)
def _msvc():
    program_files = os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")
    vswhere = os.path.join(program_files, "Microsoft Visual Studio", "Installer", "vswhere.exe")
    if not os.path.isfile(vswhere):
        return MISSING, "vswhere.exe not found; Visual Studio installer missing"
    try:
        rc, out = run(
            vswhere, "-latest", "-products", "*",
            "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property", "installationPath",
        )
    except CmdError as err:
        return MISSING, str(err)
    path = first_line(out)
    if rc == 0 and path:
        return OK, path
    return MISSING, "no MSVC C++ build tools found"


@check("webview2", "Install the Microsoft Edge WebView2 Runtime", platforms=WINDOWS)
def _webview2():
    key = r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDC-59929ED6C0A4}"
    try:
        rc, out = run("reg", "query", key, "/v", "pv")
    except CmdError as err:
        return MISSING, str(err)
    match = re.search(r"pv\s+REG_SZ\s+(\S+)", out)
    if rc == 0 and match:
        return OK, f"WebView2 {match.group(1)}"
    return MISSING, "WebView2 runtime not registered"


# --- Warnings -----------------------------------------------------------

@check("disk-space", "Free at least 20 GB at the repository before a full build", needed=False)
def _disk():
    free = shutil.disk_usage(ROOT).free
    gib = f"{free / 1024**3:.1f} GB free"
    return (OK, gib) if free >= DISK_NEEDED_BYTES else (WARN, gib)


# --- Runner -------------------------------------------------------------

def run_check(item):
    try:
        status, detail = item.fn()
    except Exception as err:  # a probe must never crash the report
        status, detail = MISSING, f"probe error: {err}"
    return {
        "name": item.name,
        "status": status,
        "detail": detail,
        "needed_m1": item.needed,
        "hint": "" if status == OK else item.hint,
    }


def print_table(results):
    headers = ("CHECK", "STATUS", "M1", "DETAIL", "FIX")
    rows = [
        (r["name"], r["status"], "yes" if r["needed_m1"] else "no", r["detail"], r["hint"])
        for r in results
    ]
    widths = [max(len(str(row[i])) for row in [headers, *rows]) for i in range(4)]
    for row in [headers, *rows]:
        cells = [str(row[i]).ljust(widths[i]) for i in range(4)] + [str(row[4])]
        print("  ".join(cells).rstrip())


def main(argv=None):
    parser = argparse.ArgumentParser(description="Locale toolchain probe (read-only).")
    parser.add_argument("--json", action="store_true", help="print results as a JSON array")
    parser.add_argument("--only", help="comma-separated check names to run")
    args = parser.parse_args(argv)

    selected = [c for c in CHECKS if c.applies(sys.platform)]
    if args.only:
        wanted = [name.strip() for name in args.only.split(",") if name.strip()]
        known = {c.name for c in CHECKS}
        unknown = [name for name in wanted if name not in known]
        if unknown:
            parser.error("unknown check(s): " + ", ".join(unknown))
        selected = [c for c in selected if c.name in wanted]

    results = [run_check(item) for item in selected]
    blocking = [r for r in results if r["needed_m1"] and r["status"] != OK]
    counts = {s: sum(1 for r in results if r["status"] == s) for s in (OK, MISSING, WARN)}

    if args.json:
        print(json.dumps(results, indent=2))
    else:
        print_table(results)
        print()
        print(
            f"{counts[OK]} ok, {counts[MISSING]} missing, {counts[WARN]} warn; "
            + (f"{len(blocking)} M1 tool(s) not ready" if blocking else "all M1 tools ready")
        )
    return 1 if blocking else 0


if __name__ == "__main__":
    sys.exit(main())
