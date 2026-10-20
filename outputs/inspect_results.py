"""读取各卡 results/*-run.json 的关键字段，用于判定失败根因。

用法: python3 inspect_results.py [task ...]
"""
import json
import sys
from pathlib import Path

ROOT = Path.home() / "lengsuo-desktop-dev"
RESULTS = ROOT / "results"


def show(task: str) -> None:
    p = RESULTS / f"{task}-run.json"
    print(f"===== {task} =====")
    if not p.is_file():
        print("  (无 result 文件)")
        return
    try:
        d = json.loads(p.read_text(encoding="utf-8", errors="replace"))
    except Exception as exc:  # noqa: BLE001
        print(f"  解析失败: {exc}")
        return
    r = d.get("result") or {}
    print(f"  exit_code : {d.get('exit_code')}")
    print(f"  subtype   : {r.get('subtype')}")
    print(f"  is_error  : {r.get('is_error')}")
    print(f"  num_turns : {r.get('num_turns')}")
    txt = str(r.get("result") or "")
    print(f"  result    : {txt[:900]}")
    err = str(d.get("stderr") or "")
    if err.strip():
        print(f"  stderr    : {err[:500]}")
    # 常见根因关键字扫描
    low = (txt + " " + err).lower()
    hits = [
        k
        for k in (
            "403",
            "subscription_not_found",
            "econnrefused",
            "proxy",
            "assertion failed",
            "no such file",
            "error[",
            "error:",
            "ledgererror",
            "permission denied",
            "timed out",
        )
        if k in low
    ]
    if hits:
        print(f"  关键字    : {hits}")
    print()


def main() -> None:
    tasks = sys.argv[1:]
    if not tasks:
        tasks = sorted(p.name[: -len("-run.json")] for p in RESULTS.glob("*-run.json"))
    for t in tasks:
        show(t)


if __name__ == "__main__":
    main()
