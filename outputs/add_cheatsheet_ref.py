"""给批二任务书插入「速查表」指引（幂等）。

根治手段：卡反复 grep registry 源码查 gpui API（e4-e7 各花 44+ 次调用），
把答案前置成 docs/gpui-wiring-cheatsheet.md，任务书直接指向它。
"""
import sys
from pathlib import Path

MARKER = "## 📖 API 速查表（先看这个，别查源码）"

SECTION = """
## 📖 API 速查表（先看这个，别查源码）

**`docs/gpui-wiring-cheatsheet.md` 已把 gpui-kit / gpui-pre 的用法核实完毕。**

它包含：最小骨架代码、全部 flex/尺寸/间距/圆角/文本样式方法名、
`.child()` vs `.children()` 的区别、`on_click` 签名、`Entity`/`Context` 用法、
主题色字段清单、常见编译错误速查表。

**用法**：
1. **先读这张速查表**（1-2 次 Read 足够）
2. **直接按表写代码**，不要去 grep `~/.cargo/registry` 查 API
3. 只有当速查表里确实没有你需要的 API 时，才允许查源码（**最多 12 次调用**）

⛔ 上一批 e4/e5/e6/e7 四张卡把 45 次预算全花在 grep registry 源码上，
一行代码没写就撞上限，**产物被静默丢弃**。不要重复这个错误。
"""

TASKS = [
    "e1-sidebar-gpui.txt",
    "e2-session-gpui.txt",
    "e3-input-gpui.txt",
    "e4-right-panel-gpui.txt",
    "e5-palette-gpui.txt",
    "e6-env-gpui.txt",
    "e7-org-gpui.txt",
    "e8-app-shell.txt",
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
            print(f"  = {name} 已有速查表指引")
            continue
        # 插到「执行纪律」段之前（纪律段在后，收尾时看）
        anchor = "## ⛔ 执行纪律（本批新增，违反即作废）"
        if anchor in text:
            text = text.replace(anchor, SECTION.strip() + "\n\n" + anchor, 1)
        else:
            text = text.rstrip() + "\n" + SECTION
        p.write_text(text, encoding="utf-8")
        print(f"  + {name} 已插入速查表指引")
    return 0


if __name__ == "__main__":
    sys.exit(main())
