# pytest + pywinauto end-to-end harness for the oca desktop app.
#
# Drives the real target/debug/ui.exe through Windows UI Automation (pywinauto's "uia"
# backend), which works here because `ui/Cargo.toml` builds eframe with the `accesskit`
# feature — egui's own widgets (ui.button/ui.label/...) publish an accessible name/role for
# free. Hand-painted controls that skip egui's widget API don't get this automatically; see
# the comment on nav_rail.rs's `rail_button` for the one place that was fixed to add it. Any
# other custom-painted control (media library items, timeline clips) still isn't reachable by
# name yet — coordinate-based pywinauto calls are the fallback there until they get the same
# treatment.
#
# Requires the app to already be built (`cargo build -p ui` / `make build`) and the FFmpeg/
# GStreamer runtime DLL directories on PATH for the *launched process* — same requirement as
# running target/debug/ui.exe by hand (see CLAUDE.md's Commands section). `make test-e2e`
# exports both before invoking pytest; running `pytest` directly needs them exported first.
from __future__ import annotations

import subprocess
import time
from pathlib import Path

import pytest
from pywinauto import Desktop

REPO_ROOT = Path(__file__).resolve().parents[1]
APP_EXE = REPO_ROOT / "target" / "debug" / "ui.exe"
WINDOW_TITLE = "oca"
LAUNCH_TIMEOUT_SECS = 20


@pytest.fixture
def oca_window():
    """Launches a fresh ui.exe, waits for its main window, yields it, then tears it down."""
    if not APP_EXE.exists():
        pytest.fail(f"{APP_EXE} not found — build it first: cargo build -p ui")

    process = subprocess.Popen([str(APP_EXE)], cwd=REPO_ROOT)
    try:
        window = _wait_for_window(process)
        window.wait("visible", timeout=LAUNCH_TIMEOUT_SECS)
        # Without this, click_input() sends real OS-level mouse events at the right
        # coordinates but they land on whatever window is actually foreground (often the
        # terminal that launched pytest) — the click never reaches oca at all, no exception,
        # it just silently does nothing.
        window.set_focus()
        yield window
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()


def _wait_for_window(process: subprocess.Popen):
    deadline = time.monotonic() + LAUNCH_TIMEOUT_SECS
    while time.monotonic() < deadline:
        if process.poll() is not None:
            pytest.fail(
                f"ui.exe exited early with code {process.returncode} — check PATH has the "
                "FFmpeg/GStreamer runtime DLL dirs (CLAUDE.md's Commands section)"
            )
        window = Desktop(backend="uia").window(title=WINDOW_TITLE)
        if window.exists():
            return window
        time.sleep(0.25)
    pytest.fail(f"{WINDOW_TITLE!r} window did not appear within {LAUNCH_TIMEOUT_SECS}s")
