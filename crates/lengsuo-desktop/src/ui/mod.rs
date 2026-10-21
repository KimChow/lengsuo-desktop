//! GPUI 渲染层（阶段二，`feature = "ui"`）。
//!
//! 本模块分两类：
//! - **描述层**（`sidebar` / `session_render` / `layout` / `input_area` /
//!   `right_panel` / `command_palette` / `env_view` / `org_view` / `tool_card` /
//!   `approval_card`）：产出纯数据的渲染描述，**不依赖 GPUI**，无窗口也能测。
//! - **接线层**（`*_gpui` 后缀）：把描述接到 GPUI 元素树，受 `feature = "ui"` 隔离。
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
pub mod main_view;
pub mod org_view;
pub mod right_panel;
pub mod session_render;
pub mod sidebar;
pub mod tool_card;

// === 批二：GPUI 接线层 ===
// 上面的模块产出**渲染描述**（纯数据、无窗口可测）；下面的模块把描述
// 真正接到 GPUI 元素树上。分开是为了让描述层保持可测、接线层可替换。
pub mod env_gpui;
pub mod input_gpui;
pub mod layout_gpui;
pub mod org_gpui;
pub mod palette_gpui;
pub mod right_panel_gpui;
pub mod session_gpui;
pub mod settings_gpui;
pub mod sidebar_gpui;
