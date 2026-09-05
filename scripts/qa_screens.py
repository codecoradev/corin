#!/usr/bin/env python3
"""Corin visual QA journey (issue #299).

Drives the real app through every view, captures screenshots in dark+light
and desktop+mobile, and fails on console errors or horizontal overflow.

Usage:
    bun run dev            # terminal 1 (Vite on :1420, /api proxied to uteke-serve)
    uteke-serve --port 8767  # terminal 2 (sandbox, no auth)
    python3 scripts/qa_screens.py [--out /tmp/corin-qa] [--base http://localhost:1420]

Requires: playwright (pip install playwright + a chromium binary), or run under
the repo's standard toolchain on a machine with browsers installed.
"""
import argparse, glob, json, os, sys

def find_chromium():
    cands = glob.glob("/opt/hermes/.playwright/chromium_headless_shell-*/chrome-linux/headless_shell")
    cands += glob.glob(os.path.expanduser("~/.cache/ms-playwright/chromium-*/chrome-linux/chrome"))
    return sorted(cands)[-1] if cands else None

VIEWS = ["Dashboard", "Memories", "Namespaces", "Graph", "Rooms", "Documents", "Lifecycle", "Settings"]

def run(base, out):
    from playwright.sync_api import sync_playwright
    os.makedirs(out, exist_ok=True)
    failures = []
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=find_chromium(), args=["--no-sandbox"])
        for theme, init in [("dark", ""), ("light", "localStorage.setItem('corin.theme','light');")]:
            ctx = b.new_context(viewport={"width": 1440, "height": 900})
            if init:
                ctx.add_init_script(init)
            page = ctx.new_page()
            errors = []
            page.on("console", lambda m: errors.append(m.text[:200]) if m.type == "error" else None)
            page.on("pageerror", lambda e: errors.append(str(e)[:200]))
            page.goto(base, wait_until="load", timeout=30000)
            page.wait_for_timeout(4000)
            for i, v in enumerate(VIEWS):
                try:
                    if v != "Dashboard":
                        page.locator("aside button", has_text=v).first.click(timeout=5000)
                        page.wait_for_timeout(2000)
                    fn = f"{out}/{theme}-{i:02d}-{v.lower()}.png"
                    page.screenshot(path=fn)
                    ov = page.evaluate("() => ({sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth})")
                    if ov["sw"] > ov["cw"] + 2:
                        failures.append(f"{theme}/{v}: horizontal overflow {ov}")
                except Exception as e:
                    failures.append(f"{theme}/{v}: {str(e)[:120]}")
            if errors:
                failures.append(f"{theme}: console errors: {errors[:5]}")
            ctx.close()
        # mobile
        mctx = b.new_context(viewport={"width": 375, "height": 812})
        mpage = mctx.new_page()
        mpage.goto(base, wait_until="load", timeout=30000)
        mpage.wait_for_timeout(3500)
        mpage.screenshot(path=f"{out}/mobile-dashboard.png")
        mov = mpage.evaluate("() => ({sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth})")
        if mov["sw"] > mov["cw"] + 2:
            failures.append(f"mobile: horizontal overflow {mov}")
        mctx.close()
        b.close()
    return failures

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://localhost:1420")
    ap.add_argument("--out", default="/tmp/corin-qa")
    args = ap.parse_args()
    fails = run(args.base, args.out)
    print(json.dumps({"failures": fails}, indent=1))
    sys.exit(1 if fails else 0)
