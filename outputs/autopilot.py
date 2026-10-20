#!/usr/bin/env python3
"""lengsuo-desktop 自动驾驶调度器（47 研究机，每小时跑一次）。

做三件事，全部幂等：
1. 推进：把「待跑」的卡按 lane 派出去（lane 忙则跳过）
2. 收敛：对已产出代码的卡做 verify → merge（门禁全绿才合）
3. 推送：把 47 的 main 推到 GitHub（无凭据则跳过，交给本机窗口推）

设计约束（来自真实翻车记录）：
- **绝不重派已 merged 的卡**（台账是唯一真相）
- **空改动 / HEAD 未领先 base 的卡判失败**（card_guard.verify 已修）
- **上游 403 时不重派**：网关探针不通就整轮跳过，等下次
- **lane 串行**：同 lane 同时只跑 1 张，避免 worktree/构建槽位打架
- **失败卡不自动重派**：连续失败 2 次就停止，等人看

用法:
  python3 autopilot.py            # 跑一轮
  python3 autopilot.py --daemon   # 常驻，每 N 秒跑一轮（cron 用这个）
  python3 autopilot.py --status   # 只看状态
"""

from __future__ import annotations

import json
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path.home() / "lengsuo-desktop-dev"
REPO = ROOT / "repo"
TASKS = ROOT / "tasks"
STATE = ROOT / "state"
RESULTS = ROOT / "results"
LOGS = ROOT / "logs"
GUARD = REPO / "scripts" / "card_guard.py"
RUNNER = ROOT / "bounded_worker_runner.py"
GATEWAY = "http://192.168.1.151:18080"

# 每张卡最多自动重派次数；超过就等人
MAX_RETRY = 2
# 自动派卡的顺序（e8 依赖其他，必须最后）
DISPATCH_ORDER = [
    "e1-sidebar-gpui",
    "e2-session-gpui",
    "e3-input-gpui",
    "e4-right-panel-gpui",
    "e5-palette-gpui",
    "e6-env-gpui",
    "e7-org-gpui",
    "e8-app-shell",
]
# 有依赖的卡：前置全部 merged 才派
DEPENDS = {
    "e8-app-shell": [
        "e1-sidebar-gpui",
        "e2-session-gpui",
        "e3-input-gpui",
        "e4-right-panel-gpui",
        "e5-palette-gpui",
        "e6-env-gpui",
        "e7-org-gpui",
    ],
}


def log(msg: str) -> None:
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def run(cmd: list[str], cwd: Path | None = None, timeout: int = 300) -> tuple[int, str]:
    try:
        p = subprocess.run(
            cmd, cwd=str(cwd) if cwd else None, capture_output=True, text=True, timeout=timeout
        )
        return p.returncode, (p.stdout or "") + (p.stderr or "")
    except subprocess.TimeoutExpired:
        return 124, "timeout"
    except Exception as exc:  # noqa: BLE001
        return 1, str(exc)


def guard(cmd: str, task: str) -> tuple[int, str]:
    return run([sys.executable, str(GUARD), cmd, task], cwd=ROOT)


def read_state(task: str) -> dict:
    p = STATE / f"{task}.json"
    if not p.is_file():
        return {}
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except Exception:  # noqa: BLE001
        return {}


def read_spec(task: str) -> dict:
    p = TASKS / f"{task}.json"
    if not p.is_file():
        return {}
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except Exception:  # noqa: BLE001
        return {}


def all_tasks() -> list[str]:
    if not TASKS.is_dir():
        return []
    out = []
    for p in TASKS.glob("*.json"):
        stem = p.stem
        # 排除 runner 自动生成的 settings 文件（<task>-settings.json）
        if stem.endswith("-settings"):
            continue
        if stem.startswith(("d", "e")):
            out.append(stem)
    return sorted(out)


def gateway_ok() -> bool:
    """探针模型网关。不通就别派卡（否则白烧一轮）。"""
    body = json.dumps(
        {
            "model": "auto",
            "max_tokens": 8,
            "messages": [{"role": "user", "content": "ok"}],
        }
    ).encode()
    req = urllib.request.Request(
        f"{GATEWAY}/v1/messages",
        data=body,
        headers={
            "content-type": "application/json",
            "anthropic-version": "2023-06-01",
            "authorization": "Bearer local",
            "x-api-key": "local",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return resp.status == 200 and bool(resp.read().strip())
    except Exception:  # noqa: BLE001
        return False


def retry_count(task: str) -> int:
    """数已产生的失败轮次。"""
    n = 0
    for p in sorted(RESULTS.glob(f"{task}-run*.json")):
        try:
            d = json.loads(p.read_text(encoding="utf-8", errors="replace"))
        except Exception:  # noqa: BLE001
            continue
        if d.get("exit_code") not in (0, None):
            n += 1
    return n


def worktree_dirty(task: str) -> bool:
    spec = read_spec(task)
    wt = Path(spec.get("worktree", ""))
    if not wt.is_dir():
        return False
    code, out = run(["git", "status", "--porcelain"], cwd=wt, timeout=60)
    return code == 0 and bool(out.strip())


def step_merge() -> int:
    """对产出代码的卡做 verify → merge。"""
    merged = 0
    for task in all_tasks():
        st = read_state(task).get("state")
        if st not in ("verifying", "running", "failed"):
            continue
        # running 的卡正在跑，别碰
        if st == "running":
            continue
        spec = read_spec(task)
        wt = Path(spec.get("worktree", ""))
        if not wt.is_dir():
            continue
        # 有未提交改动 = 卡没提交，verify 会判空改动
        if worktree_dirty(task):
            log(f"  {task}: worktree 有未提交改动，跳过（等人工或卡自己收尾）")
            continue
        code, out = guard("verify", task)
        if code != 0:
            first = next((x for x in out.splitlines() if x.strip()), "")
            log(f"  {task}: verify 未过 → {first[:110]}")
            continue
        code, out = guard("merge", task)
        if code == 0:
            merged += 1
            log(f"  {task}: ✓ 已合入")
        else:
            first = next((x for x in out.splitlines() if x.strip()), "")
            log(f"  {task}: merge 失败 → {first[:110]}")
    return merged


def step_dispatch() -> int:
    """按 lane 派「待跑」的卡。"""
    if not gateway_ok():
        log("模型网关不可用，本轮不派卡（等下次）")
        return 0
    lanes_busy: set[str] = set()
    for task in all_tasks():
        if read_state(task).get("state") == "running":
            spec = read_spec(task)
            lane = spec.get("lane", "?")
            lanes_busy.add(lane)
    dispatched = 0
    for task in DISPATCH_ORDER:
        st = read_state(task).get("state")
        if st in ("running", "verifying", "merged"):
            continue
        if st not in (None, "pending", "failed"):
            continue
        if st == "failed" and retry_count(task) >= MAX_RETRY:
            log(f"  {task}: 失败已达 {MAX_RETRY} 次，不再自动重派（等人看）")
            continue
        spec = read_spec(task)
        if not spec:
            continue
        lane = spec.get("lane", "?")
        if lane in lanes_busy:
            continue
        deps = DEPENDS.get(task, [])
        if deps and not all(read_state(d).get("state") == "merged" for d in deps):
            log(f"  {task}: 依赖未满足，等前置合入")
            continue
        wt = Path(spec.get("worktree", ""))
        if not wt.is_dir():
            log(f"  {task}: worktree 不存在，跳过")
            continue
        # 重置到 main，保证基线干净（上一轮的半成品不留）
        if st == "failed":
            code, _ = run(["git", "reset", "--hard", "main"], cwd=wt, timeout=120)
            if code != 0:
                log(f"  {task}: reset 失败")
                continue
            run(["git", "clean", "-fd"], cwd=wt, timeout=120)
        log(f"  {task}: 派卡（lane={lane}）")
        with open(LOGS / f"{task}.out", "a", encoding="utf-8") as fh:
            subprocess.Popen(
                [sys.executable, str(RUNNER), task],
                cwd=str(ROOT),
                stdout=fh,
                stderr=subprocess.STDOUT,
                stdin=subprocess.DEVNULL,
                start_new_session=True,
            )
        lanes_busy.add(lane)
        dispatched += 1
        time.sleep(2)
    return dispatched


def step_push() -> None:
    """把 47 的 main 推到 GitHub。

    47 已配仓库专用凭据（~/.git-credentials-desktop，600，仅本仓生效），
    所以这里能直接推，不必等本机窗口。

    推送前先 fetch + 合并远端 —— 远端可能已被本机窗口推过新提交，
    直接 push 会因非快进被拒。合并用 --no-edit（不打断自动流程）。
    推送后必须核对远端 SHA 与本地一致 —— 静默失败是这类脚本最常见的坑。
    """
    fcode, fout = run(["git", "fetch", "origin", "main"], cwd=REPO, timeout=180)
    if fcode != 0:
        first = next((x for x in fout.splitlines() if x.strip()), "")
        log(f"  fetch 失败 → {first[:100]}")
        return

    code, out = run(
        ["git", "rev-list", "--left-right", "--count", "HEAD...origin/main"],
        cwd=REPO,
    )
    if code != 0:
        log("  无法比较与 origin/main 的差异（跳过推送）")
        return
    counts = out.split()
    if len(counts) == 2 and counts[0] == "0" and counts[1] == "0":
        log("  47 main 与 origin/main 已同步，无需推送")
        return

    # 远端领先 ⇒ 先合并远端（可能冲突，冲突则停下等人）
    if len(counts) == 2 and counts[1] != "0":
        mcode, mout = run(
            ["git", "merge", "--no-edit", "origin/main"], cwd=REPO, timeout=300
        )
        if mcode != 0:
            first = next((x for x in mout.splitlines() if x.strip()), "")
            log(f"  ⚠ 合并 origin/main 冲突，停下等人：{first[:100]}")
            return
        log("  已合并远端新提交")

    pcode, pout = run(["git", "push", "origin", "main"], cwd=REPO, timeout=300)
    if pcode != 0:
        first = next((x for x in pout.splitlines() if x.strip()), "")
        log(f"  推送失败 → {first[:110]}")
        return

    # 推送后核对：远端 SHA 必须与本地一致
    lcode, lout = run(["git", "rev-parse", "HEAD"], cwd=REPO)
    rcode, rout = run(["git", "ls-remote", "origin", "refs/heads/main"], cwd=REPO, timeout=120)
    local_sha = lout.strip()[:8] if lcode == 0 else "?"
    remote_sha = rout.split()[0][:8] if rcode == 0 and rout.split() else "?"
    if local_sha == remote_sha and local_sha != "?":
        log(f"  ✓ 已推送并核对：{local_sha}")
    else:
        log(f"  ⚠ 推送后核对不一致：local={local_sha} remote={remote_sha}")


def step_report() -> None:
    for task in all_tasks():
        st = read_state(task).get("state", "?")
        if st != "merged":
            log(f"  {task}: {st}")


def cycle() -> None:
    log("=== 自动驾驶一轮开始 ===")
    step_report()
    m = step_merge()
    d = step_dispatch()
    step_push()
    log(f"=== 本轮：合入 {m} 张，派卡 {d} 张 ===")


def main() -> int:
    args = sys.argv[1:]
    if "--status" in args:
        step_report()
        return 0
    if "--daemon" in args:
        interval = 3600
        if "--interval" in args:
            interval = int(args[args.index("--interval") + 1])
        while True:
            try:
                cycle()
            except Exception as exc:  # noqa: BLE001
                log(f"轮次异常：{exc}")
            log(f"下一轮在 {interval}s 后")
            time.sleep(interval)
    cycle()
    return 0


if __name__ == "__main__":
    sys.exit(main())
