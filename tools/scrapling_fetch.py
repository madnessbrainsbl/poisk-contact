"""Render one public contact page in an isolated Scrapling browser, returning JSON."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import urlsplit

MAX_HTML = 4 * 1024 * 1024


def validate_url(value):
    parsed = urlsplit(value)
    if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password:
        raise ValueError("Scrapling requires an HTTP(S) URL without credentials")
    return value


def python_path():
    configured = os.environ.get("POISK_SCRAPLING_PYTHON")
    if configured is not None:
        return configured or None
    root = Path(__file__).resolve().parent.parent
    path = root / ".scrapling" / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    return str(path) if path.is_file() else None


def needs_browser(html):
    text = re.sub(r"(?is)<(script|style)\b[^>]*>.*?</\1\s*>", " ", html)
    text = re.sub(r"<[^>]*>", " ", text)
    if "@" in text or len(re.sub(r"\D", "", text)) >= 7:
        return False
    lowered = html.lower()
    return ("enable javascript" in lowered or "включите javascript" in lowered or
            (len(text.strip()) < 400 and "<script" in lowered and
             any(marker in lowered for marker in ('id="root"', 'id="app"', 'id="__next"', "__next_data__"))))


def guard_browser_processes():
    # Closing this process also closes its job, including Chromium descendants.
    if os.name != "nt":
        return
    import ctypes
    from ctypes import wintypes

    class BasicLimits(ctypes.Structure):
        _fields_ = [("process_time", ctypes.c_int64), ("job_time", ctypes.c_int64),
                    ("flags", wintypes.DWORD), ("min_working_set", ctypes.c_size_t),
                    ("max_working_set", ctypes.c_size_t), ("active_processes", wintypes.DWORD),
                    ("affinity", ctypes.c_size_t), ("priority", wintypes.DWORD),
                    ("scheduling", wintypes.DWORD)]

    class Limits(ctypes.Structure):
        _fields_ = [("basic", BasicLimits), ("io", ctypes.c_uint64 * 6),
                    ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                    ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t)]

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
    kernel.CreateJobObjectW.restype = wintypes.HANDLE
    kernel.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
    kernel.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    kernel.GetCurrentProcess.restype = wintypes.HANDLE
    job = kernel.CreateJobObjectW(None, None)
    limits = Limits()
    limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    if not job or not kernel.SetInformationJobObject(job, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
        raise ctypes.WinError(ctypes.get_last_error())
    if not kernel.AssignProcessToJobObject(job, kernel.GetCurrentProcess()):
        raise ctypes.WinError(ctypes.get_last_error())


def render(url):
    validate_url(url)
    from scrapling.fetchers import StealthyFetcher

    page = StealthyFetcher.fetch(
        url, headless=True, timeout=10_000, wait=500, network_idle=False,
        disable_resources=True, google_search=False, solve_cloudflare=False,
        real_chrome=(os.name == "nt" and Path(os.environ.get("PROGRAMFILES", "C:/Program Files"),
                    "Google/Chrome/Application/chrome.exe").is_file()),
    )
    validate_url(page.url)
    if not 200 <= page.status < 300:
        raise ValueError(f"Scrapling page returned HTTP {page.status}")
    if len(page.body) > MAX_HTML:
        raise ValueError("Scrapling HTML exceeds 4 MiB")
    html = page.body.decode("utf-8", errors="replace")
    if not html.strip():
        raise ValueError("Scrapling returned empty HTML")
    return {"url": page.url, "status": page.status, "html": html}


def render_isolated(url):
    validate_url(url)
    python = python_path()
    if not python:
        raise RuntimeError("Scrapling is not installed; run tools/setup_scrapling.ps1")
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    child = subprocess.Popen(
        [python, "-X", "utf8", str(Path(__file__).resolve())],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        encoding="utf-8", creationflags=flags,
    )
    try:
        stdout, stderr = child.communicate(json.dumps({"url": url}), timeout=20)
    except subprocess.TimeoutExpired as error:
        child.kill()
        child.communicate()
        raise RuntimeError("Scrapling exceeded its 20-second page budget") from error
    if child.returncode:
        raise RuntimeError(f"Scrapling failed: {stderr[-600:]}")
    response = json.loads(stdout)
    validate_url(response["url"])
    if not 200 <= response["status"] < 300 or not response["html"].strip():
        raise ValueError("Invalid Scrapling page response")
    return response


def main():
    try:
        request = json.loads(sys.stdin.buffer.read(16_384))
        validate_url(request["url"])
        guard_browser_processes()
        result = render(request["url"])
        sys.stdout.write(json.dumps(result, ensure_ascii=False))
        sys.stdout.flush()
        return 0
    except Exception as error:
        print(f"Scrapling: {type(error).__name__}: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
