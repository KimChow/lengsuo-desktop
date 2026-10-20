""""给指定任务书追加「上游不可用即停」约束段（幂等）。

用法: python3 add_upstream_guard.py <tasks_dir>
"""
import sys
from pathlib import Path

SECTION = """
## ⛔ 上游不可用时的行为（硬约束）

若你在执行中遇到以下任一情况，**立即停止重试并结束本轮**，不要反复重试、不要改写任务目标：

- `API Error: 403` / `Failed to authenticate` / `上游模型繁忙`
- `API returned an empty or malformed response` / `check for a proxy or gateway`
- 连续两次工具调用均因网络/网关错误失败

结束时的报告必须是：

```
STATUS: UPSTREAM_UNAVAILABLE
最后一次错误原文：<粘贴>
已完成的工作：<无 或 具体文件列表>
```

**理由**：历史上有卡因 403 空转 61 轮、零产物、整轮作废。宁可快速失败让主理人修网关，也不要把轮次消耗在重试上。**这不是你的错，如实上报即可。**
"""

MARKER = "## ⛔ 上游不可用时的行为（硬约束）"

TASKS = [
    "d1-ime-win32-recon.txt",
    "d2-gpui-skeleton.txt",
    "d3-workbench-layout.txt",
    "d5-env-view.txt",
    "d8-hotkey-command.txt",
]


def main() -> int:
    d = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    for name in TASKS:
        p = d / name
        if not p.is_file():
            print(f"  skip {name}（不存在）")
            continue
        text = p.read_text(encoding="utf-8")
        if MARKER in text:
            print(f"  = {name} 已有约束段")
            continue
        p.write_text(text.rstrip() + "\n" + SECTION, encoding="utf-8")
        print(f"  + {name} 已追加")
    return 0


if __name__ == "__main__":
    sys.exit(main())
