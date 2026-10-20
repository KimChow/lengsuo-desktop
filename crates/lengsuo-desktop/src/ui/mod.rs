//! GPUI 渲染层（阶段二，`feature = "ui"`）。
//!
//! 阶段一（本批）：本模块**只有占位**，不存在任何 GPUI 依赖。
//! 阶段二：`ui = ["dep:gpui"]` 打开后在此实现视图。
//!
//! 渲染层约束：
//! - 数据只来自 [`crate::state`] 与 [`crate::theme`]，**不实现业务语义**。
//! - **不持有凭据**；不直连运行时。
//! - 中文输入依赖 Windows IME 修复，修复前不得声称输入可用。
//!
//! 下面的 `pub mod` 声明由主理人一次性落盘，使各任务卡只需改自己新建的
//! 文件，避免多卡共改本文件造成路径归属冲突。

pub mod approval_card;
pub mod command_palette;
pub mod env_view;
pub mod gpui_app;
pub mod gpui_input;
pub mod input_area;
pub mod layout;
pub mod org_view;
pub mod right_panel;
pub mod session_render;
pub mod sidebar;
pub mod tool_card;
