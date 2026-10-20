"""给 bounded_worker_runner.py 打补丁：卡失败/撞上限时自动提交残留改动。

背景：卡撞 max_turns 时往往已经写了大半代码（如 e4 写了 580 行），但因为
没走到「提交」那一步，worktree 里留着未提交改动 —— 重派时会被 reset 掉，
产物白费。

补丁在收尾段（写 state 之后、判断 exit_code 之前）插入：
- worktree 有未提交改动 ⇒ 用一个明确的 WIP 提交信息提交下来
- 提交后 verify 会看到「有改动」，卡就能进 verifying → merge 流程
- 若改动编译不过，集成门禁会拦住它 —— 比默默丢掉强

用法: python3 patch_runner_autocommit.py <path-to-bounded_worker_runner.py>
"""
import sys
from pathlib import Path

ANCHOR = """state.write_text(
    json.dumps(
        {
            "status": "finished" if proc.returncode == 0 else "error","""

INJECT = '''# 撞上限/异常退出时，卡往往已经写了大半代码但没走到提交那一步。
# 这里把残留改动提交下来（标 WIP），让产物不至于被重派时的 reset 抹掉。
# 编译不过由集成门禁拦住 —— 比默默丢掉强。
try:
    _dirty = subprocess.run(
        ["git", "status", "--porcelain"], cwd=wt, capture_output=True, text=True
    ).stdout.strip()
    if _dirty:
        subprocess.run(
            ["git", "add", "-A"],
            cwd=wt,
            capture_output=True,
            text=True,
        )
        _msg = "wip(desktop): 卡未完成，保留已写下的实现（待复核）"
        subprocess.run(
            ["git", "commit", "-m", _msg],
            cwd=wt,
            capture_output=True,
            text=True,
        )
        print(f"→ {task}: 已提交残留改动（{len(_dirty.splitlines())} 项）")
except Exception as _exc:  # noqa: BLE001
    print(f"→ {task}: 提交残留改动失败：{_exc}")

'''


def main() -> int:
    p = Path(sys.argv[1])
    s = p.read_text(encoding="utf-8")
    if "已提交残留改动" in s:
        print("已打过补丁")
        return 0
    if ANCHOR not in s:
        print("未找到锚点，未修改")
        return 1
    s = s.replace(ANCHOR, INJECT + ANCHOR, 1)
    p.write_text(s, encoding="utf-8")
    print("已插入残留改动自动提交")
    return 0


if __name__ == "__main__":
    sys.exit(main())
