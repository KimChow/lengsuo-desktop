"""分析卡运行的 jsonl 日志：统计工具调用分布、最后若干动作，判定 max_turns 原因。

用法: python3 analyze_card_log.py <task>
"""
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path.home() / "lengsuo-desktop-dev"

task = sys.argv[1] if len(sys.argv) > 1 else ""
if not task:
    print("用法: python3 analyze_card_log.py <task>")
    sys.exit(2)

p = ROOT / "logs" / f"{task}.jsonl"
if not p.is_file():
    print(f"无日志: {p}")
    sys.exit(1)

tools = Counter()
bashes = []
texts = []
assistant_turns = 0

with p.open(encoding="utf-8", errors="replace") as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except Exception:  # noqa: BLE001
            continue
        msg = ev.get("message") or {}
        if ev.get("type") == "assistant":
            assistant_turns += 1
            for blk in msg.get("content") or []:
                if not isinstance(blk, dict):
                    continue
                if blk.get("type") == "tool_use":
                    name = blk.get("name", "?")
                    tools[name] += 1
                    inp = blk.get("input") or {}
                    if name == "Bash":
                        bashes.append(str(inp.get("command", ""))[:180])
                    elif name in ("Read", "Glob", "Grep"):
                        tgt = inp.get("file_path") or inp.get("pattern") or ""
                        bashes.append(f"[{name}] {tgt}"[:180])
                elif blk.get("type") == "text":
                    t = (blk.get("text") or "").strip()
                    if t:
                        texts.append(t[:200])

print(f"assistant 消息数: {assistant_turns}")
print(f"工具调用分布   : {dict(tools.most_common())}")
print()
print("=== 最后 15 个动作 ===")
for x in bashes[-15:]:
    print(" ", x)
print()
print("=== 最后 3 段文本 ===")
for t in texts[-3:]:
    print(" ", t.replace("\n", " ")[:300])
