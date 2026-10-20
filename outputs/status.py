#!/usr/bin/env python3
"""查看 desktop 线各卡状态（在 47 上运行）。"""
import json
from pathlib import Path

ROOT = Path.home() / "lengsuo-desktop-dev"
TASKS = [
    "d1-ime-win32-recon",
    "d2-gpui-skeleton",
    "d3-workbench-layout",
    "d4-org-permission-view",
    "d5-env-view",
    "d6-session-render",
    "d7-theme-tokens",
    "d8-hotkey-command",
]

print(f"{'task':28s} {'state':10s} {'exit':5s} {'turns':6s} reason")
print("-" * 90)
for t in TASKS:
    st = {}
    sp = ROOT / "state" / f"{t}.json"
    if sp.is_file():
        try:
            st = json.loads(sp.read_text())
        except ValueError:
            st = {}
    rp = ROOT / "results" / f"{t}-run.json"
    res = {}
    if rp.is_file():
        try:
            res = json.loads(rp.read_text())
        except ValueError:
            res = {}
    inner = res.get("result") or {}
    state = st.get("state") or res.get("status") or "-"
    exit_code = res.get("exit_code", "-")
    turns = inner.get("num_turns", "-")
    reason = str(st.get("reason") or inner.get("subtype") or "")[:34]
    print(f"{t:28s} {str(state):10s} {str(exit_code):5s} {str(turns):6s} {reason}")
