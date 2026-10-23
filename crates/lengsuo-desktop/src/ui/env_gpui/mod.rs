//! 环境视图的 GPUI 接线层：把 [`crate::ui::env_view`] 的渲染描述接到元素树。
//!
//! **这是本仓接线层的黄金范例** —— 其他 `*_gpui.rs` 照这个文件的结构写。
//!
//! 分层约定：
//! - **描述层**（`env_view`）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（本目录）：把描述变成 GPUI 元素，**不重新实现**任何业务判断。
//!
//! 本目录内部分两半（与 [`crate::ui::sidebar_gpui`] 同构）：
//! - `mapping.rs` **无** `feature = "ui"` 门控：描述字段 → 界面语义（状态色槽位、
//!   主区模式、空态文案、分组行标识、选中态写回），纯数据，随 `cargo test` 一起跑。
//! - `render.rs` 带 `feature = "ui"` 门控：GPUI 元素树，只能编译验证。
//! - `tests.rs`：原 `mod tests` 整体迁出，显式 `use` 符号（不用 `super::*`）。
//!
//! 硬约束：
//! - fail-closed：加载失败必须显示错误，**不得**退化成空列表。
//! - 「真实为空」与「筛选无结果」用不同文案（描述层已区分，此处不得合并）。
//! - 描述层说「不显示空态」（`empty_text == None`）时，界面**不得**补默认文案。
//! - 文案只有一个来源：状态 / 空态 / 分组名都取自描述层，本文件不另造一套。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
//!
//! 目录结构：`env_gpui/mod.rs` + `mapping.rs` + `render.rs` + `tests.rs`。
//! 公开 API 与拆分前一字不变（见下方 `pub use`）。

#[cfg(feature = "ui")]
mod filter_bar;
#[cfg(test)]
#[cfg(feature = "ui")]
mod filter_render_tests;
mod mapping;
#[cfg(feature = "ui")]
mod render;
#[cfg(test)]
mod tests;

pub use mapping::{
    active_group_row, clear_filter, demo_state, empty_recovery, empty_state_text, filter_key,
    filter_summary, main_pane, select_card, set_group_filter, status_tone, EmptyRecovery,
    FilterSummary, MainPane, StatusTone, CLEAR_FILTER_LABEL, SEARCH_PLACEHOLDER,
};
#[cfg(feature = "ui")]
pub use render::EnvViewGpui;
