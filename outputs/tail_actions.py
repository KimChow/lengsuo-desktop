"""打印指定卡 jsonl 日志的最后 N 个工具调用动作。

用法: python3 tail_actions.py <task> [n]
"""
import json
import sys
from pathlib import Path

ROOT = Path.home() / "lengsuo-desktop-dev"
task = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 10

p = ROOT / "logs" / f"{task}.jsonl"
if not p.is_file():
    print(f"无日志: {p}")
    sys.exit(1)

actions: list[str] = []
for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
    try:
        ev = json.loads(line)
    except Exception:  # noqa: BLE001
        continue
    if ev.get("type") != "assistant":
        continue
    for blk in (ev.get("message") or {}).get("content") or []:
        if not isinstance(blk, dict) or blk.get("type") != "tool_use":
            continue
        name = blk.get("name", "?")
        inp = blk.get("input") or {}
        if name == "Bash":
            actions.append("BASH  " + str(inp.get("command", ""))[:100].replace("\n", " "))
        elif name in ("Read", "Write", "Edit"):
            actions.append(f"{name:5s} {str(inp.get('file_path', ''))[-70:]}")
        else:
            actions.append(f"{name:5s} {str(inp)[:80]}")

print(f"=== {task} 共 {len(actions)} 个动作，最后 {n} 个 ===")
for a in actions[-n:]:
    print("  ", a)
