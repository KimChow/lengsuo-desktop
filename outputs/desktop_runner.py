#!/usr/bin/env python3
"""lengsuo-desktop 派卡器（47 研究机）。

与 agent 线 `~/lengsuo-agent-dev/bounded_worker_runner.py` 机制相同，差异：

  1. 根目录：`~/lengsuo-desktop-dev`（独立仓、独立 worktrees/targets/锁）
  2. 网关：192.168.1.151:18080（codex-as-api-fork，Anthropic 协议）
  3. 槽位环境变量：`LENGSUO_DESKTOP_SLOT`（与 bin/cargo wrapper 对应）
  4. 参考源不同：本线参考 `~/lengsuo-desktop-dev/GUIDE.md`（自有参考文档），
     不使用 agent 线的三源（codex / opencode / wb）。

用法：
    cd ~/lengsuo-desktop-dev && python3 bounded_worker_runner.py <task> [--dry-run]

任务约定（tasks/ 下）：
    <task>.json  {"worktree": "...", "slot": 0, "lane": "...", "max_turns": 60,
                  "touches": [...], "depends_on": [...], "reference": {...}}
    <task>.txt   任务书正文（喂给 claude -p）
"""
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path.home() / "lengsuo-desktop-dev"
GATEWAY = "http://192.168.1.151:18080"
# 用户指定：只用网关的 `auto` 档，路由由 192.168.1.151:18080 统一负责。
# 不要写死具体模型、也不要绕到 qoder:/reference-platform: 等渠道。
MODEL = os.environ.get("LENGSUO_DESKTOP_MODEL") or "auto"

# 本线不参考 agent 三源；参考件是任务书内明写的路径（见 build_prompt）。
REF_ROOTS: dict[str, str] = {}

if len(sys.argv) < 2:
    sys.exit("用法: python3 bounded_worker_runner.py <task> [--dry-run]")

task = sys.argv[1]
DRY_RUN = "--dry-run" in sys.argv
spec_path = ROOT / "tasks" / f"{task}.json"
if not spec_path.is_file():
    sys.exit(f"找不到任务单 {spec_path}")


def ref_block(s: dict) -> list[str]:
    """生成「参考实现」段落：强制按参考转写，禁止自创等价方案。"""
    ref = s.get("reference") or {}
    projects = ref.get("projects") or []
    paths = ref.get("paths") or []
    if not projects:
        return []

    roots = []
    for p in projects:
        root = REF_ROOTS.get(p)
        if root is None:
            roots.append(f"  - {p}（未登记，检查 REF_ROOTS）")
        else:
            roots.append(f"  - {p} → {root}")

    lines = [
        "",
        "【参考实现 · 强制约束】",
        "本卡不是自由设计：必须把参考实现转写/对齐到 Rust，禁止自创「等价」方案。",
        "参考根目录：",
        *roots,
    ]
    if paths:
        lines += ["", "本卡必须对齐的参考路径：", *[f"  - {p}" for p in paths]]
    if ref.get("note"):
        lines += ["", f"对齐说明：{ref['note']}"]

    lines += [
        "",
        "执行要求（按顺序）：",
        "  1. 先逐行读完上述参考路径的入口、数据结构、状态流转与错误分支；",
        "  2. 按参考件的**分层、命名、调用顺序**转写，不重排、不合并、不简化；",
        "  3. 参考里有的边界/失败处理必须一一对应，不许以「我觉得不需要」为由省略；",
        "  4. 因语言或运行时差异必须偏离的，逐条写进报告「偏离清单」并说明理由；",
        "  5. 禁止只读文档或摘要就动手实现。",
        "",
    ]
    return lines


def build_prompt(name: str, s: dict) -> str:
    """任务书前注入边界与参考约束。"""
    touches = s.get("touches", [])
    lines = [
        "【本卡边界 · 硬约束】",
        "你只允许修改以下路径，其余一律不许碰：",
        *[f"  - {t}" for t in touches],
        "",
        "禁止修改共享区：Cargo.toml / Cargo.lock / rust-toolchain.toml / scripts / .github",
        "  （这些由专门的基建卡独占负责；你若确需改动，写进报告，不要自己改。）",
        "禁止 push / merge / 直接改主仓；需要跨 crate 的改动一律写进报告。",
        "跑完请只提交本卡范围内的改动。超出边界的改动会在收尾时被自动校验拦下。",
    ]
    lines += ref_block(s)
    lines += ["---", ""]
    brief = (ROOT / "tasks" / f"{name}.txt").read_text()
    return "\n".join(lines) + brief


if DRY_RUN:
    _spec = json.loads(spec_path.read_text())
    print(f"=== {task} 派卡预览（dry-run，未认领）===")
    print(build_prompt(task, _spec))
    sys.exit(0)

# 派卡前先认领：路径重叠 / 依赖未合 / worktree 复用 一律拒绝。
guard = ROOT / "repo" / "scripts" / "card_guard.py"
claim = subprocess.run([sys.executable, str(guard), "claim", task])
if claim.returncode != 0:
    sys.exit(f"派卡被拒（card_guard）：{task}")

spec = json.loads(spec_path.read_text())
wt = Path(spec["worktree"])
slot = str(spec.get("slot", 0))

env = os.environ.copy()
env.update(
    {
        "PATH": f"{ROOT / 'bin'}:/home/ubuntu/.local/bin:/usr/local/bin:/usr/bin:/bin",
        "LENGSUO_DESKTOP_SLOT": slot,
        "CARGO_TARGET_DIR": str(ROOT / "targets" / task),
        "CARGO_BUILD_JOBS": "2",
        "CARGO_INCREMENTAL": "0",
        "CARGO_HTTP_TIMEOUT": "45",
        "CARGO_NET_RETRY": "1",
        "ANTHROPIC_BASE_URL": GATEWAY,
        "ANTHROPIC_AUTH_TOKEN": "local",
        "ANTHROPIC_MODEL": MODEL,
        "ANTHROPIC_DEFAULT_HAIKU_MODEL": MODEL,
        "ANTHROPIC_DEFAULT_SONNET_MODEL": MODEL,
        "ANTHROPIC_DEFAULT_OPUS_MODEL": MODEL,
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
    }
)

# 只把 Anthropic / Claude 相关变量写进 settings，避免整份环境落盘
settings = ROOT / "tasks" / f"{task}-settings.json"
settings.write_text(
    json.dumps(
        {k: v for k, v in env.items() if k.startswith(("ANTHROPIC_", "CLAUDE_CODE_"))}
    )
)
settings.chmod(0o600)

state = ROOT / "results" / f"{task}-run.json"
state.write_text(json.dumps({"status": "running", "started": time.time(), "worktree": str(wt)}))


def update_card_state(new_state: str, **extra) -> None:
    """更新调度台账（已 claim 过，别人抢不进来，无需再加锁）。"""
    p = ROOT / "state" / f"{task}.json"
    p.parent.mkdir(parents=True, exist_ok=True)
    cur = {}
    if p.is_file():
        try:
            cur = json.loads(p.read_text())
        except ValueError:
            cur = {}
    cur.update({"state": new_state, "updated_at": time.time(), **extra})
    p.write_text(json.dumps(cur, ensure_ascii=False, indent=2))


update_card_state("running", pid=os.getpid())


args = [
    "/home/ubuntu/.local/bin/claude",
    "-p",
    build_prompt(task, spec),
    "--setting-sources",
    "",
    "--settings",
    str(settings),
]
# `--model auto` 会被 Claude Code 的模型名校验拒掉；ANTHROPIC_MODEL=auto 走网关正常。
if MODEL != "auto":
    args += ["--model", MODEL]
args += [
    "--permission-mode",
    "acceptEdits",
    "--allowedTools",
    "Read,Write,Edit,Glob,Grep,Bash",
    "--tools",
    "Read,Write,Edit,Glob,Grep,Bash",
    "--max-turns",
    str(spec.get("max_turns", 60)),
    "--output-format",
    "stream-json",
    "--verbose",
]
if spec.get("resume"):
    args += ["--resume", spec["resume"]]

# 上游网关会瞬时 400/繁忙，做有限次指数退避重试；非瞬时错误立即退出。
RETRIES = int(spec.get("retries", 3))
TRANSIENT_MARKERS = (
    "上游模型繁忙",
    "API Error",
    "overloaded",
    "rate limit",
    "429",
    "502",
    "503",
    "504",
)


def _last_result(jl: Path) -> dict:
    """取日志里最后一条 result 记录（结构化判定用，不扫正文）。"""
    if not jl.is_file():
        return {}
    last = {}
    for line in jl.read_text(errors="replace").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except Exception:
            continue
        if row.get("type") == "result":
            last = row
    return last


def _transient(rc: int, jl: Path) -> tuple[bool, str]:
    """退出码非 0 且结构化结果指向上游瞬时错误 ⇒ 可重试。

    只看 result 记录，绝不在日志正文里找标记：正文含执行体读到的代码，
    会出现假命中。
    """
    if rc == 0:
        return False, ""
    row = _last_result(jl)
    status = row.get("api_error_status")
    subtype = str(row.get("subtype") or "")
    if subtype == "error_max_turns":
        return False, ""
    if status in (429, 502, 503, 504):
        return True, f"api_error_status={status} subtype={subtype}"
    blob = str(row.get("result") or "")
    hit = next((m for m in TRANSIENT_MARKERS if m in blob), None)
    if hit is None:
        return False, ""
    idx = blob.find(hit)
    return True, blob[max(0, idx - 80) : idx + 160].replace(chr(10), " ")[:240]


proc = None
for attempt in range(1, RETRIES + 1):
    with (ROOT / "logs" / f"{task}.jsonl").open("w") as log, (
        ROOT / "logs" / f"{task}.stderr"
    ).open("w") as err:
        proc = subprocess.run(args, cwd=wt, env=env, stdout=log, stderr=err)
    retry, snippet = _transient(proc.returncode, ROOT / "logs" / f"{task}.jsonl")
    if not retry:
        break
    if attempt == RETRIES:
        print(f"✗ {task}: 上游瞬时错误重试 {RETRIES} 次仍失败：{snippet}")
        break
    wait = 30 * (2 ** (attempt - 1))
    print(f"⟳ {task}: 第 {attempt} 次遇上游瞬时错误，{wait}s 后重试：{snippet}")
    time.sleep(wait)

last = {}
for line in (ROOT / "logs" / f"{task}.jsonl").read_text().splitlines():
    try:
        d = json.loads(line)
        if d.get("type") == "result":
            last = d
    except ValueError:
        pass

# 撞上限/异常退出时，卡往往已经写了大半代码但没走到提交那一步。
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

state.write_text(
    json.dumps(
        {
            "status": "finished" if proc.returncode == 0 else "error",
            "exit_code": proc.returncode,
            "finished": time.time(),
            "result": last,
        },
        ensure_ascii=False,
        indent=2,
    )
)

# 收尾：先校验改动是否越界，再决定台账状态。
if proc.returncode == 0:
    checked = subprocess.run([sys.executable, str(guard), "verify", task])
    if checked.returncode == 0:
        update_card_state("verifying", exit_code=0)
    else:
        update_card_state("failed", exit_code=0, reason="touches 越界，见 verify 输出")
        print(task, proc.returncode)
        sys.exit(0)
else:
    update_card_state("failed", exit_code=proc.returncode)
    print(task, proc.returncode)
    sys.exit(0)

# 通过验收后自动集成。
cfg_path = ROOT / "config.json"
cfg = {}
if cfg_path.is_file():
    try:
        cfg = json.loads(cfg_path.read_text())
    except ValueError:
        cfg = {}

if not cfg.get("auto_integrate", True):
    print(f"{task}: 已通过验收，按配置不自动集成（可手动 card_guard.py merge {task}）")
    print(task, proc.returncode)
    sys.exit(0)

print(f"→ {task} 开始自动集成（含门禁）")
merged = subprocess.run([sys.executable, str(guard), "merge", task])
if merged.returncode != 0:
    print(f"✗ {task} 集成失败，占用保留，需人工处理")
    print(task, proc.returncode)
    sys.exit(0)

if cfg.get("auto_push"):
    repo = ROOT / "repo"
    push = subprocess.run(
        ["git", "-C", str(repo), "push", "origin", "main"],
        capture_output=True,
        text=True,
    )
    if push.returncode == 0:
        print("→ 已推送 origin/main")
    else:
        print(f"✗ 推送失败（本地已合入，需人工处理）：{push.stderr.strip()[:300]}")
else:
    print("→ 已合入本地 main（未配置 auto_push，需手动推送）")

print(task, proc.returncode)
