# e51 · org_gpui/sections.rs 目录式拆分状态

**性质：纯重构（代码搬移），不改任何行为。**
前置先例：e48（`env_gpui.rs` → 目录）、e50（`gpui_app/mod.rs` → 目录）。
本卡按任务书要求先读先例与速查表；注：本 worktree（基于 `6e3dfe9`）中尚无
`docs/gpui-app-shell-split-status.md`，e50 的三条坑以任务书 §3/§4 转述为准并全部落实。

## 1. 拆分前后对照

| 文件 | 拆分前 | 拆分后 |
|---|---|---|
| `ui/org_gpui/sections.rs` | 506 | （已删除，`git rm`） |
| `ui/org_gpui/sections/mod.rs` | — | 96 |
| `ui/org_gpui/sections/controls.rs` | — | 190 |
| `ui/org_gpui/sections/lists.rs` | — | 256 |
| `ui/org_gpui/mod.rs` | 251 | 251（一行未改） |

验收：四份现存文件全部 ≤ 400 行 ✓（`check_line_limit.py` 硬限制 0 失败）。

## 2. 搬移边界（按函数名定位，行号参考原 506 行文件）

| 函数 | 去向 |
|---|---|
| `display_label` | `sections/mod.rs` |
| `status_block` | `sections/mod.rs` |
| `error_block` | `sections/mod.rs` |
| `render_notice_panel` | `sections/mod.rs` |
| `render_button` | `sections/controls.rs` |
| `render_tabs` | `sections/controls.rs` |
| `render_toolbar` | `sections/controls.rs` |
| `render_department_node` / `render_departments` | `sections/lists.rs` |
| `render_member_row` / `render_members_section` | `sections/lists.rs` |
| `render_role_row` / `render_roles_section` / `render_content` | `sections/lists.rs` |

再导出（`sections/mod.rs`，只导出 `org_gpui/mod.rs` 真正调用的符号）：

```rust
pub(in crate::ui::org_gpui) use controls::{render_tabs, render_toolbar};
pub(in crate::ui::org_gpui) use lists::render_content;
```

`render_button` / `status_block` / `error_block` / `render_notice_panel` 仅在
`sections` 子树内被调用，**不**再导出（unused 再导出会被 clippy `-D warnings` 拦下）。
`org_gpui/mod.rs` 的测试用 `use super::sections::display_label;` 路径不变，仍然成立。

## 3. 可见性处理

- **被搬进 `controls.rs` / `lists.rs` 的函数：`pub(super)` → `pub(in crate::ui::org_gpui)`。**
  原因：`sections.rs` 时代的 `pub(super)` = 对 `org_gpui` 可见；搬进
  `org_gpui::sections::controls` 后 `pub(super)` 降级为仅对 `org_gpui::sections` 可见，
  会导致 `org_gpui/mod.rs` 处 `E0603`（private function）以及受限再导出报
  `E0364/E0365`。`pub(in crate::ui::org_gpui)` 与原可见性**严格等价**。
- **留在 `sections/mod.rs` 的 4 个小工具保持 `pub(super)`**：它们仍在原深度，
  语义不变，且 `sections::controls` / `sections::lists` 作为 `org_gpui` 的后代
  同样可见，供 `lists.rs` 以 `use super::{display_label, error_block, status_block}`、
  `controls.rs` 以 `use super::render_notice_panel` 引用。

## 4. 行为不变证据（五道门禁实际输出摘要）

| 门禁 | 结果 |
|---|---|
| `python3 scripts/check_line_limit.py` | hard-limit failures: **0**（19 条 soft 警告全部为 `kernel_pipe` 等既有文件，与本卡无关） |
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy --features ui --workspace --all-targets -- -D warnings` | **0 error / 0 warning** |
| `cargo test --features ui --workspace` | **688 passed; 0 failed**（与拆分前完全一致，未删未改任何用例） |
| `cargo build --workspace` | Finished dev profile |

渲染内容零改动：函数体逐字搬移（仅 `cargo fmt` 规整了换行），未动任何文案、
颜色、顺序、id 字符串；`org_gpui/mod.rs` 调用侧一行未改。

## 5. 失败点与修法

- **E0603（function is private）**：已预判并规避——搬移函数一律直接写成
  `pub(in crate::ui::org_gpui)`，未走「先 `pub(super)` 再补」的弯路；
  首次 `cargo check` 即 0 error。
- **E0364/E0365（cannot re-export private item）**：已预判并规避——再导出语句
  与被导出项使用同一 `pub(in crate::ui::org_gpui)` 可见性，宽度一致，合法。
- **E0761（ambiguous module，`sections.rs` 与 `sections/` 并存）**：已预判并规避——
  写入新目录前先 `git rm crates/.../sections.rs`，两者从未并存。
- **clippy unused re-export**：已预判并规避——只再导出 `render_tabs` /
  `render_toolbar` / `render_content` 三个上层真实调用的入口，未加任何
  `#[allow(unused_imports)]`。

## 6. 如实声明

- 本卡是**纯重构**：未新增界面功能，未改协议 / 运行时 / 业务语义。
- 未做真机视觉验证（仅编译、clippy、单元测试证据）。
- 越界改动：无。`git diff` 只含 `crates/lengsuo-desktop/src/ui/org_gpui/` 下文件
  与本文档；共享区（Cargo.toml / Cargo.lock / rust-toolchain.toml / scripts / .github）未动。
