# e52 · ui/input_area.rs 目录式拆分状态

**性质：纯重构（代码搬移），不改任何行为。**
前置先例：e48（`env_gpui.rs` → 目录）、e51（`org_gpui/sections.rs` → 目录）。
注：本 worktree（基于 `19818c0`）中无 `docs/gpui-app-shell-split-status.md`，
e50 的三条坑以任务书 §3/§4 转述为准并全部落实。

## 1. 拆分前后对照

| 文件 | 拆分前 | 拆分后 |
|---|---|---|
| `ui/input_area.rs` | 495 | （已删除，`git rm`） |
| `ui/input_area/mod.rs` | — | 120 |
| `ui/input_area/state.rs` | — | 115 |
| `ui/input_area/mention.rs` | — | 82 |
| `ui/input_area/tests.rs` | — | 194 |

验收：四份现存文件全部 ≤ 400 行 ✓（`check_line_limit.py` hard-limit failures: 0）。

## 2. 搬移边界（按函数名定位）

| 内容 | 去向 |
|---|---|
| `//!` 模块文档（原 1–11 行，原文保留 + 补目录结构说明） | `mod.rs` |
| `InputMode` + `impl InputMode`（`ALL` / `label` / `id` / `from_id`） | `mod.rs` |
| `StopIntent` / `SendButtonKind` / `SendButton` / `OutgoingInput` | `mod.rs` |
| `InputAreaState` 字段定义 | `mod.rs` |
| `new` / `draft` / `set_draft` / `push_str` / `line_count` / `mode` / `set_mode` / `set_mode_id` / `is_in_flight` / `finish_send` / `can_send` / `send_button` / `take_stop` / `submit` | `state.rs` |
| `set_candidates` / `candidates` / `filter_candidates` / `mention_query` / `mention_index` / `insert_mention` | `mention.rs` |
| `#[cfg(test)] mod tests`（14 条用例，逐条保留） | `tests.rs` |

`use std::collections::HashSet;` 仅 `mention.rs` 需要，随之搬走；`mod.rs` 不保留未用 import。

## 3. 可见性处理

- **`InputAreaState` 的四个字段 → `pub(super)`**。
  原因：字段定义留在 `input_area/mod.rs`，而 `impl` 拆进兄弟/子模块
  `state.rs` / `mention.rs`。Rust 中父模块结构体的私有字段对子模块**不可见**，
  不提升可见性会集中爆 `E0616`（field is private）。`pub(super)` 在 `mod.rs`
  中即「对 `input_area` 全部后代可见」，与拆分前的模块内可见范围等价。
- 类型与方法本身保持 `pub`，与拆分前一致；无新增 `#[allow(...)]`。

## 4. 旧路径兼容

六个类型仍**直接定义在 `input_area/mod.rs`**，因此
`crate::ui::input_area::{InputAreaState, InputMode, OutgoingInput, SendButton, SendButtonKind, StopIntent}`
全部照旧可导入，**无需 `pub use`、也没有可被判 unused 的再导出**。
`ui/mod.rs` 的 `pub mod input_area;` 一行未改（Rust 2018 自动指向目录）。
外部调用方（`layout/`、`input_gpui/`、`gpui_app/`）零改动。

## 5. 行为不变证据（五道门禁实际输出摘要）

| 门禁 | 结果 |
|---|---|
| `python3 scripts/check_line_limit.py` | hard-limit failures: **0**（18 条 soft 警告均为既有文件，与本卡无关） |
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy --features ui --workspace --all-targets -- -D warnings` | **0 error / 0 warning** |
| `cargo test --features ui --workspace` | **688 passed; 0 failed**（与拆分前完全一致，未删未改任何用例） |
| `cargo build --workspace` | Finished dev profile |

函数体逐字搬移（仅 `cargo fmt` 规整了 `state.rs` 的 import 换行），
未动任何文案、模式 id、默认值或断言。

## 6. 失败点与修法

- **E0616（field is private）**：已预判并规避——`InputAreaState` 字段直接写成
  `pub(super)`，首次 `cargo check` 即 0 error，未走「先报错再补」的弯路。
- **unused re-export**：已预判并规避——类型留在 `mod.rs`，不写 `pub use`，
  不存在未用再导出；未加 `#[allow(unused_imports)]`。
- **E0761（ambiguous module）**：已预判并规避——写入目录后立刻 `git rm` 旧
  `ui/input_area.rs`，两者从未并存。

## 7. 如实声明

- 本卡是**纯重构**：⛔ 未新增界面功能，⛔ 未改协议 / 运行时 / 业务语义。
- ⛔ 未做真机视觉验证（仅编译、clippy、单元测试证据）。
- 越界改动：无。`git diff` 只含 `crates/lengsuo-desktop/src/ui/input_area/` 下文件
  与本文档；共享区（Cargo.toml / Cargo.lock / rust-toolchain.toml / scripts / .github）未动。
