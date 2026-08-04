#!/usr/bin/env python3
"""
novai-auto-fix.py — self-healing CI script.

Flow:
  1. Read the most-recent GitHub Actions run that failed (via the GitHub API).
  2. Download the failed log.
  3. Match known error patterns against the log:
       a. Rust compile error  -> fix via simple heuristic patches (e.g. add
          missing `use`, swap a renamed API, gate a feature).
       b. Missing system dep  -> append to scripts/build-*.sh.
       c. Kernel config drift -> scripts/config --enable for missing symbol.
  4. Apply the patch in-place, commit, push back to main, and re-trigger CI.

If no rule matches, it just reports and exits non-zero so the human is
notified (instead of looping forever).

Run with:
  GH_TOKEN=<token> REPO=salom600/NovaiOS python3 scripts/auto-fix.py
"""

from __future__ import annotations
import os, re, sys, json, urllib.request, urllib.error, subprocess, base64, time
from pathlib import Path

REPO       = os.environ.get("REPO", "salom600/NovaiOS")
GH_TOKEN   = os.environ.get("GH_TOKEN", "")
ROOT       = Path(__file__).resolve().parent.parent
GIT_BRANCH = os.environ.get("GIT_BRANCH", "main")

API = "https://api.github.com"

def api(method: str, path: str, body: dict | None = None, raw: bool = False):
    url = f"{API}/repos/{REPO}/{path.lstrip('/')}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, method=method, data=data, headers={
        "Authorization":  f"Bearer {GH_TOKEN}",
        "Accept":         "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "Content-Type":   "application/json",
    })
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            if raw: return r.read()
            return json.loads(r.read().decode() or "{}")
    except urllib.error.HTTPError as e:
        print(f"[api] {method} {path} -> {e.code}: {e.reason}", file=sys.stderr)
        if e.code == 404: return None
        body = e.read().decode(errors="replace")
        print(body[:600], file=sys.stderr)
        return None

def most_recent_failed_run() -> dict | None:
    runs = api("GET", "actions/runs?status=failure&per_page=10") or {}
    for r in runs.get("workflow_runs", []):
        if r["head_branch"] == GIT_BRANCH and r["event"] in ("push","workflow_run","schedule"):
            return r
    return None

def fetch_log(jobs_url: str) -> str:
    # jobs_url is a full API URL like https://api.github.com/repos/OWNER/REPO/actions/runs/N/jobs
    # Convert to a path our api() helper understands.
    if jobs_url.startswith(API):
        path = jobs_url[len(API) + 1:]  # strip "https://api.github.com/"
        # path is now "repos/OWNER/REPO/actions/runs/N/jobs"
        # api() prepends "/repos/{REPO}/", so strip "repos/" if present
        if path.startswith("repos/"):
            path = path[len("repos/"):]
        # path is now "OWNER/REPO/actions/runs/N/jobs"
        # We want to keep OWNER/REPO intact since REPO env may not match
        # Actually api() does f"/repos/{REPO}/{path}", so we need to strip OWNER/REPO too
        # Simpler: hit the URL directly with urllib
    jobs = _http_get_json(jobs_url) or {}
    out = []
    for job in jobs.get("jobs", []):
        for step in job.get("steps", []):
            if step.get("conclusion") != "failure":
                continue
            log_url = f"{API}/repos/{REPO}/actions/jobs/{job['id']}/logs"
            try:
                req = urllib.request.Request(log_url, headers={
                    "Authorization": f"Bearer {GH_TOKEN}",
                    "Accept": "application/vnd.github+json",
                })
                with urllib.request.urlopen(req, timeout=60) as r:
                    out.append(r.read().decode(errors="replace"))
            except Exception as e:
                out.append(f"# failed to fetch logs for {job['name']}: {e}\n")
    return "\n".join(out)

def _http_get_json(url: str) -> dict | None:
    try:
        req = urllib.request.Request(url, headers={
            "Authorization": f"Bearer {GH_TOKEN}",
            "Accept": "application/vnd.github+json",
        })
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.loads(r.read().decode() or "{}")
    except urllib.error.HTTPError as e:
        print(f"[http] GET {url} -> {e.code}: {e.reason}", file=sys.stderr)
        return None
    except Exception as e:
        print(f"[http] GET {url} -> {e}", file=sys.stderr)
        return None

# ---------------------------------------------------------------------------
# Rule engine
# ---------------------------------------------------------------------------

RULES: list[tuple[str, re.Pattern, callable]] = []

def rule(name: str, pattern: str):
    def deco(fn):
        RULES.append((name, re.compile(pattern, re.MULTILINE), fn))
        return fn
    return deco

@rule("missing-system-dep", r"command not found:\s*(\S+)|package '(\S+)' not found")
def _missing_dep(m: re.Match) -> bool:
    pkg = m.group(1) or m.group(2)
    print(f"[fix] adding missing system dep {pkg!r}")
    sh = ROOT / "scripts/build-iso.sh"
    txt = sh.read_text()
    # Insert into the pacstrap call.
    needle = "pacstrap -c -M -G \"$ROOTFS\" \\"
    if needle in txt and pkg not in txt:
        txt = txt.replace(needle, f"{needle}\n  {pkg} # auto-added")
        sh.write_text(txt)
        return True
    return False

@rule("rust-missing-use", r"error\[E0433\]: failed to resolve: use of unresolved module or unlinked crate `(\w+)`")
def _missing_use(m: re.Match) -> bool:
    crate = m.group(1)
    print(f"[fix] adding missing `use {crate}` heuristic")
    # Scan source files for the failed identifier and add `use {crate};`
    changed = False
    for rs in ROOT.rglob("src/**/*.rs"):
        txt = rs.read_text()
        if f"use {crate}" not in txt and crate in txt:
            new = f"use {crate};\n" + txt
            rs.write_text(new)
            changed = True
    return changed

@rule("rust-feature-missing", r"error: the cargo feature `(\w+)` is not enabled")
def _feature_missing(m: re.Match) -> bool:
    feat = m.group(1)
    print(f"[fix] adding missing cargo feature {feat!r}")
    for cargo in ROOT.rglob("Cargo.toml"):
        txt = cargo.read_text()
        # Find the [dependencies.<dep>] section and add the feature
        m2 = re.search(r'\[dependencies\.(\w+)\]\n(.*?)(?=\n\[|\Z)', txt, re.DOTALL)
        if m2 and feat not in m2.group(0):
            block = m2.group(0)
            new_block = block.rstrip() + f'\n{feat} = ["full"]\n'
            txt = txt.replace(block, new_block)
            cargo.write_text(txt)
            return True
    return False

@rule("kernel-config-missing", r"warning: .*CONFIG_(\w+).*not set|CONFIG_(\w+): symbol not found")
def _kernel_cfg(m: re.Match) -> bool:
    sym = m.group(1) or m.group(2)
    print(f"[fix] enabling missing kernel CONFIG_{sym}")
    sh = ROOT / "scripts/build-kernel.sh"
    txt = sh.read_text()
    if "scripts/config --enable" in txt:
        txt = txt.replace(
            "scripts/config --enable CONFIG_RUST --enable CONFIG_BUILD_RUST",
            f"scripts/config --enable CONFIG_RUST --enable CONFIG_BUILD_RUST --enable CONFIG_{sym}"
        )
        sh.write_text(txt)
        return True
    return False

@rule("wget-404", r"ERROR 404: Not Found")
def _wget_404(m: re.Match) -> bool:
    print("[fix] a download 404'd — bumping KVER to latest LTS")
    sh = ROOT / "scripts/build-kernel.sh"
    txt = sh.read_text()
    # Try a few common LTS versions
    for v in ["6.12.10","6.12.11","6.12.12","6.13.0","6.11.13"]:
        txt2 = txt.replace(f'KVER="${{KVER:-6.12.10}}"', f'KVER="${{KVER:-{v}}}"')
        if txt2 != txt:
            sh.write_text(txt2); return True
    return False

@rule("cargo-lock-drift", r"error: the lock file .* needs to be updated")
def _cargo_lock(m: re.Match) -> bool:
    print("[fix] running `cargo update -p <drifted crate>`")
    subprocess.run(["cargo","update"], cwd=ROOT, check=False)
    return True

# ---------------------------------------------------------------------------
# Apply loop
# ---------------------------------------------------------------------------

def apply_rules(log: str) -> list[str]:
    applied = []
    for name, pat, fn in RULES:
        m = pat.search(log)
        if m:
            try:
                if fn(m):
                    applied.append(name)
            except Exception as e:
                print(f"[fix] rule {name} raised {e}", file=sys.stderr)
    return applied

def git_commit_push(msg: str):
    env = dict(os.environ, GIT_AUTHOR_NAME="novai-bot", GIT_AUTHOR_EMAIL="bot@novaios.org",
               GIT_COMMITTER_NAME="novai-bot", GIT_COMMITTER_EMAIL="bot@novaios.org")
    subprocess.run(["git","add","-A"], cwd=ROOT, check=True, env=env)
    if subprocess.run(["git","diff","--cached","--quiet"], cwd=ROOT).returncode != 0:
        subprocess.run(["git","commit","-m",msg], cwd=ROOT, check=True, env=env)
        subprocess.run(["git","push","origin",GIT_BRANCH], cwd=ROOT, check=True, env=env)
        return True
    return False

def main():
    if not GH_TOKEN:
        print("GH_TOKEN env required"); sys.exit(2)
    run = most_recent_failed_run()
    if not run:
        print("no failed run on branch", GIT_BRANCH); return
    print(f"inspecting run #{run['run_number']} ({run['name']})")
    log = fetch_log(run["jobs_url"])
    if not log:
        print("no log available"); sys.exit(3)

    applied = apply_rules(log)
    if not applied:
        print("no rule matched — opening issue for human triage")
        api("POST", "issues", {
            "title": f"CI failed with unrecognised error in run #{run['run_number']}",
            "body":  f"Workflow `{run['name']}` failed.\n\nLogs:\n```\n{log[:4000]}\n```\n\nRun: {run['html_url']}",
            "labels": ["ci-failure","auto-fix-skipped"],
        })
        sys.exit(1)

    print(f"applied fixes: {applied}")
    msg = "auto-fix: " + ", ".join(applied) + f" (run #{run['run_number']})"
    if git_commit_push(msg):
        print("pushed fix; CI will re-run on push")
    else:
        print("no changes to commit (rules fired but produced no diff)")

if __name__ == "__main__":
    main()
