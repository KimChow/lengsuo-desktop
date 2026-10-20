#!/usr/bin/env python3
"""lengsuo-desktop 自动驾驶守护进程（47 研究机）。

以 nohup 方式常驻，每小时跑一轮 autopilot.py。不依赖 cron / systemd
（47 无 cron 守护，systemd unit 需 root）。

自愈设计：
- 守护进程自身崩溃 → 由外层 `while` 循环拉起（见 run_autopilot.sh）
- autopilot 单轮异常 → 捕获后记日志，不退出整轮
- 连续 3 轮全失败（网关挂了等）→ 退避到 30 分钟一轮，恢复后自动回到 1 小时

用法: python3 autopilot_daemon.py [--interval 3600]
"""
from __future__ import annotations

import subprocess
import sys
import time
from pathlib import Path

ROOT = Path.home() / "lengsuo-desktop-dev"
LOG = ROOT / "logs" / "autopilot.log"


def log(msg: str) -> None:
    line = f"[daemon {time.strftime('%H:%M:%S')}] {msg}"
    print(line, flush=True)
    try:
        LOG.parent.mkdir(parents=True, exist_ok=True)
        with LOG.open("a", encoding="utf-8") as fh:
            fh.write(line + "\n")
    except Exception:  # noqa: BLE001
        pass


def main() -> int:
    interval = 3600
    if "--interval" in sys.argv:
        interval = int(sys.argv[sys.argv.index("--interval") + 1])

    log(f"自动驾驶守护启动，间隔 {interval}s")
    fails = 0
    while True:
        try:
            p = subprocess.run(
                [sys.executable, str(ROOT / "autopilot.py")],
                cwd=str(ROOT),
                capture_output=True,
                text=True,
                timeout=1800,
            )
            out = (p.stdout or "") + (p.stderr or "")
            for line in out.splitlines():
                log(line)
            if p.returncode == 0 and "派卡" in out:
                fails = 0
            else:
                fails += 1
        except subprocess.TimeoutExpired:
            log("本轮超时（>30min），跳过")
            fails += 1
        except Exception as exc:  # noqa: BLE001
            log(f"本轮异常：{exc}")
            fails += 1

        wait = interval
        if fails >= 3:
            wait = 1800
            log(f"连续 {fails} 轮异常，退避到 30 分钟一轮")
        elif fails:
            wait = 900
        log(f"下一轮在 {wait}s 后")
        time.sleep(wait)
    return 0


if __name__ == "__main__":
    sys.exit(main())
