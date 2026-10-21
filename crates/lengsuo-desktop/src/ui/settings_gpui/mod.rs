//! 设置窗口：15 个面板的导航 + 改主题即时生效。
//!
//! 分层约定（与 [`crate::ui::palette_gpui`] 等接线层一致）：
//! - **描述层**（[`view`]）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（[`window`] / [`nav`] / [`theme_panel`]）：`feature = "ui"` 隔离，
//!   只做「描述字段 → 样式 / 事件」的映射。
//!
//! 文件拆分（每个文件单一职责，均 ≤ 400 行）：
//! - `view.rs`：15 面板清单、面板 id、主题取值 —— **不门控**（无 `ui` feature 也能测）。
//! - `window.rs`：设置窗口视图本体（`SettingsGpui`）+ `Render`。
//! - `nav.rs`：左栏 15 项导航。
//! - `theme_panel.rs`：「外观」面板的主题三档选择。
//! - `tests.rs`：单元测试（显式导入，不用 `use super::*`）。
//!
//! 硬约定：
//! - **改即生效，无保存按钮**：点主题档位立刻调 `Theme::change`，全窗口重绘。
//! - **颜色一律取自 `cx.theme()`**，不硬编码。
//! - **不引入任何组件库**（`Setting` / `Select` 一律不用），全部用 `div()` 自搭。
//! - 除「外观」外的 14 个面板**只有导航 + 占位文案**，功能待实现，
//!   详见 `docs/settings-window-status.md`。

pub mod view;

#[cfg(feature = "ui")]
mod nav;
#[cfg(feature = "ui")]
mod theme_panel;
#[cfg(feature = "ui")]
mod window;

#[cfg(test)]
mod tests;

#[cfg(feature = "ui")]
pub use window::SettingsGpui;
