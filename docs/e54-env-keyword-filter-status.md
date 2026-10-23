# E54：环境视图关键词搜索框 + 筛选空态恢复入口

## 本卡做了什么

环境管理视图（`EnvViewGpui`）此前只能按分组筛选，**没有任何输入框**：
`EnvGroupState::set_keyword` / `clear_keyword` / `clear_filter` / `is_filter_active`
在全仓没有真实调用者，用户进入 `FilteredOut` 空态后**没有恢复入口**。

本次把已有的筛选能力接到界面上：

- **关键词搜索框**：主区顶部无条件渲染一个 [`Textarea`]（`filter_bar.rs`）。
  首次渲染时延迟创建编辑器并订阅 `InputEvent::Change`；内容变化即写回
  `EnvGroupState::set_keyword`（关键词 → `visible_entries` 过滤，行为完全复用 state 层）。
- **筛选空态的「清除筛选」按钮**：仅当描述层空态为 `FilteredOut` 且确有筛选时显示。
  点击走 `mapping::clear_filter`（清分组 + 关键词）并同步清空输入框。
  `NoData`、失败态、加载态**不显示**该按钮（避免假按钮）。
- 空态**文案**仍只取自 `env_view` 描述层（`empty_state_text`），未新增第二套文案。

纯逻辑（`empty_recovery` / `filter_summary` / `clear_filter`）放在 `mapping.rs`，
无窗口可单测；`filter_bar.rs` 只做元素接线。

## 验证状态（如实）

- **编译**：`cargo check -p lengsuo-desktop --features ui --all-targets` 通过。
- **单测**：新增 4 个 mapping 层测试（空态恢复判定、清筛选、空白关键词、关键词收窄可见条目）。
- **真实渲染未验证**：本卡只做了编译 + 单测验证，**没有窗口实测截图**，
  搜索框的布局 / 焦点 / 按钮点击路径均未在真实窗口中观察。
- **IME 未验证**：沿用仓内既有口径（`palette_gpui` 的 `Textarea` 注释同样声明），
  **不得**据此声称中文输入可用；Windows 中文 IME 取决于独立的 IME 修复卡。
- **未改 state 层任何行为**：`state/env_group.rs` 一字未改，也未改描述层
  `ui/env_view.rs` 的判定语义。

## 涉及文件

- `crates/lengsuo-desktop/src/ui/env_gpui/filter_bar.rs`（新增）
- `crates/lengsuo-desktop/src/ui/env_gpui/mapping.rs`（追加纯逻辑 + 测试用判定）
- `crates/lengsuo-desktop/src/ui/env_gpui/mod.rs`（挂模块 / 导出）
- `crates/lengsuo-desktop/src/ui/env_gpui/render.rs`（接线，≤ 400 行）
- `crates/lengsuo-desktop/src/ui/env_gpui/tests.rs`（新增单测）
