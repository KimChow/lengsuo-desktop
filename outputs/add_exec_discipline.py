"""给批二任务书补「先落码后调研 + 增量编译」硬约束（幂等）。

背景：e4/e5/e6/e7 四张卡全部把 45 次工具调用预算花在调研 gpui-kit API 上，
一行代码没写就撞上限，exit 0 被误判为成功 → 台账标 merged → 产物静默丢失。
e1/e2 则是写完 259/595 行但没编译过，15/16 个编译错误。

本补丁给任务书加三条硬约束：
1. 调研预算上限（最多 12 次调用查 API）
2. 增量编译：每写完一个渲染分支就 cargo check 一次
3. 工具调用预算提到 70，并要求「先写最简可编译版本，再逐步完善」
"""
import sys
from pathlib import Path

MARKER = "## ⛔ 执行纪律（本批新增，违反即作废）"

SECTION = """
## ⛔ 执行纪律（本批新增，违反即作废）

**上一批 e4/e5/e6/e7 四张卡全部作废**：它们把 45 次工具调用预算全花在调研
gpui-kit 的 API 上，一行代码没写就撞上限，退出码却是 0 —— 台账因此误标成
"已合入"，产物静默丢失（4 张卡的工作全部白做）。

**e1/e2 也不行**：写完 259 / 595 行但一次都没编译过，留了 15 / 16 个编译错误。

### 规则 1：调研预算 ≤ 12 次调用

查 gpui-kit API 的工具调用**最多 12 次**，用完就必须开始写代码。

不需要提前把 API 查全 —— 编译器会告诉你缺什么。**先写一个能编译的最小版本，
再按报错补**。这比先调研后实现快一个数量级。

### 规则 2：增量编译，每 5-10 分钟必查一次

**每写完一个渲染分支，立刻跑一次**：

```bash
cargo check -p lengsuo-desktop --features ui
```

⚠️ 首次编译约 1-2 分钟（gpui 依赖已编译好，之后 10-30 秒）。
**不要攒到最后才编译** —— 上批 e1/e2 就是攒到最后，16 个错误一起爆，预算耗尽。

### 规则 3：先落最简可编译版本

顺序固定：

1. 写最小骨架（一个 `impl Render` + 一个 `div()`，先保证能编译）
2. 立刻 `cargo check` 确认
3. 逐个加渲染分支，**每加一个就 check**
4. 最后补测试与文档

**任何时候预算即将耗尽，就把当前能编译的版本提交下来** ——
宁可交一个 60 分但能编译的版本，也不要交一个 95 分但编不过的版本。
空改动（没写任何代码）等于这张卡白跑，会被 verify 判失败。

### 规则 4：verify 会拒绝空改动

主理人已修 `card_guard.verify`：**改动文件数为 0 直接判失败**，
`HEAD` 未领先 base 也判失败。所以「调研到底、一行没写」是必然作废的，
不要再试图这样交差。
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
            print(f"  = {name} 已有执行纪律段")
            continue
        # 旧的预算行同步上调
        text = text.replace("**最多 45 次。**", "**最多 70 次。**")
        text = text.replace("**最多 45 次**", "**最多 70 次**")
        text = text.replace("**最多 50 次**（组装卡稍宽）", "**最多 70 次**")
        p.write_text(text.rstrip() + "\n" + SECTION, encoding="utf-8")
        print(f"  + {name} 已追加执行纪律")
    return 0


if __name__ == "__main__":
    sys.exit(main())
