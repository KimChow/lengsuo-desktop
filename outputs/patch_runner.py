"""修补 bounded_worker_runner.py 的收尾判定：verify 失败时记录真实原因。

原文把 verify 的所有失败都写成"touches 越界"，但 verify 现在还会因
"空改动"（卡把预算全花在调研上、一行代码没写）而失败 —— 两者处理方式
完全不同，必须如实记录。

用法: python3 patch_runner.py <path-to-bounded_worker_runner.py>
"""
import sys
from pathlib import Path

OLD = '''        update_card_state("failed", exit_code=0, reason="touches 越界，见 verify 输出")'''
NEW = '''        update_card_state(
            "failed",
            exit_code=0,
            reason="验收未通过（touches 越界 / 空改动 / HEAD 未领先 base），见 verify 输出",
        )'''

p = Path(sys.argv[1])
s = p.read_text(encoding="utf-8")
if NEW.split("\n")[0] in s:
    print("已打过补丁")
    sys.exit(0)
if OLD not in s:
    print("未找到目标片段，未修改")
    sys.exit(1)
p.write_text(s.replace(OLD, NEW), encoding="utf-8")
print("已修补 runner 失败原因记录")
