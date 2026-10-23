//! 设置窗口：15 个面板的导航 + 改主题即时生效。
//!
//! 分层约定（与 [`crate::ui::palette_gpui`] 等接线层一致）：
//! - **描述层**（[`view`]）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（[`window`] / [`nav`] / [`theme_panel`]）：`feature = "ui"` 隔离，
//!   只做「描述字段 → 样式 / 事件」的映射。
//!
//! 文件拆分（每个文件单一职责，均 ≤ 400 行）：
//! - `view/`：15 面板清单、面板 id、主题取值、能力位展示文案、快捷键展示、账号成员 ——
//!   **不门控**（无 `ui` feature 也能测）。
//! - `window.rs`：设置窗口视图本体（`SettingsGpui`）+ `Render`。
//! - `nav.rs`：左栏 15 项导航。
//! - `theme_panel.rs`：「外观」面板的主题三档选择。
//! - `security_panel.rs`：「安全中心」面板的能力位**只读**清单。
//! - `system_perm_panel.rs`：「系统权限」面板的拒绝原因细分 + 引导路径（只读）。
//! - `hotkey_panel.rs`：「个性化」面板的快捷键**只读**清单。
//! - `data_panel.rs`：「数据管理」面板的环境分组**只读**清单。
//! - `org_panel.rs`：「账号」面板的组织成员**只读**清单。
//! - `help_panel.rs`：「帮助与反馈」面板的版本 / 快捷键 / 命令表**只读**清单。
//! - 波次 1 的 8 个面板接线层：`models_panel.rs` / `software_panel.rs` /
//!   `extensions_panel.rs` / `memory_panel.rs` / `subscription_panel.rs` /
//!   `claw_panel.rs` / `laboratory_panel.rs` / `mcp_consent_panel.rs`
//!   （描述层是 `view/` 下的同名模块）——**当前均为占位实现**，
//!   由 `ui-settings-*` 卡补成完整面板。
//! - `tests.rs`：单元测试（显式导入，不用 `use super::*`）。
//!
//! 硬约定：
//! - **改即生效，无保存按钮**：点主题档位立刻调 `Theme::change`，全窗口重绘。
//! - **颜色一律取自 `cx.theme()`**，不硬编码。
//! - **不引入任何组件库**（`Setting` / `Select` 一律不用），全部用 `div()` 自搭。
//! - 除「外观」「安全中心」「个性化」「系统权限」「数据管理」「账号」「帮助与反馈」外的 8 个面板
//!   **只有导航 + 占位文案**，功能待实现，详见 `docs/settings-window-status.md`。
//! - 「安全中心」**只读**展示服务端下发的 `can_*` 标记，⛔ 不做授权推理。
//! - 「个性化」**只读**展示本仓真实的快捷键绑定表，⛔ 不提供改键入口。
//! - 「系统权限」按拒绝原因细分展示 `can_*`，点击引导入口只**上报意图**，⛔ 不做授权推理。
//! - 「数据管理」「账号」**只读**展示服务端下发的清单，⛔ 不提供增删改。
//! - 「帮助与反馈」**只读**展示版本 / 快捷键 / 命令表，
//!   ⛔ 不实现反馈提交与网络检查（要网络与运行时）。

pub mod view;

#[cfg(feature = "ui")]
mod data_panel;
#[cfg(feature = "ui")]
mod help_panel;
#[cfg(feature = "ui")]
mod hotkey_panel;
#[cfg(feature = "ui")]
mod nav;
#[cfg(feature = "ui")]
mod org_panel;
#[cfg(feature = "ui")]
mod security_panel;
#[cfg(feature = "ui")]
mod system_perm_panel;
#[cfg(feature = "ui")]
mod theme_panel;
// === 波次 1：8 个设置面板的接线层（占位落盘，由 ui-settings-* 卡补齐）===
#[cfg(feature = "ui")]
mod claw_panel;
#[cfg(feature = "ui")]
mod extensions_panel;
#[cfg(feature = "ui")]
mod laboratory_panel;
#[cfg(feature = "ui")]
mod mcp_consent_panel;
#[cfg(feature = "ui")]
mod memory_panel;
#[cfg(feature = "ui")]
mod models_panel;
#[cfg(feature = "ui")]
mod software_panel;
#[cfg(feature = "ui")]
mod subscription_panel;
#[cfg(feature = "ui")]
mod window;

#[cfg(test)]
mod tests;

#[cfg(feature = "ui")]
pub use window::SettingsGpui;
