//! 应用入口组装：把 e1–e7 的接线层拼成一个可运行的工作台。
//!
//! 布局（标题栏 + 三区 + 输入区 + 底部终端抽屉）：
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │ 标题栏（WINDOW_TITLE）                        │
//! ├──────────┬──────────────────────┬────────────┤
//! │ Sidebar  │   SessionStream      │ RightPanel │
//! │          │                      │            │
//! │          ├──────────────────────┴────────────┤
//! │          │  InputAreaGpui（中右两列下方）       │
//! ├──────────┴───────────────────────────────────┤
//! │ 终端抽屉（默认收起，点标题条展开）              │  ← A13
//! └──────────────────────────────────────────────┘
//! ```
//!
//! 分层约定（与 [`crate::ui::env_gpui`] 黄金范例一致）：
//! - 根视图**不持有任何业务语义**：数据全部来自 [`crate::state`] 的 demo 构造，
//!   子视图各自负责「描述 → 元素」的映射。
//! - **版式唯一持有者是左分隔条**（[`DividerGpui`]）：三区宽度每次渲染都经
//!   [`sync::regions_for`] 从它持有的 [`WorkbenchLayout`] 读回，**不另存副本**；
//!   右侧 tab 条（[`RightPanelTabGpui`]）从同一份版式派生选中态，点击后经
//!   [`AppShell::sync_layout`] 就地写给右侧面板与右分隔条。
//! - 命令面板浮在最上层，快捷键经 `palette_gpui::hotkey_from_keystroke` 转换后
//!   交给 [`crate::ui::command_palette::handle_hotkey`]。
//! - 组织视图挂在主界面（e38，见下）。
//!
//! e35 环境管理挂主界面（对齐 A17）：侧栏新增「环境」入口 ⇒ 主区切到
//! [`EnvViewGpui`]（`MainView::Environments`）。
//! ⚠️ 环境列表当前是**演示数据**（`EnvViewGpui::demo()`）⇒ 真实来源在运行时侧。
//!
//! e38 组织与权限可见性挂主界面（对齐 A18）：侧栏新增「组织」入口 ⇒ 主区切到
//! [`OrgViewGpui`]（`MainView::Organization`）。
//! ⚠️ 组织清单当前是**演示数据**（[`demo::demo_org_view`]）⇒ 真实来源在运行时侧。
//!
//! e14 发送回显（**界面层本地回显，不投递**）：
//! - 输入区发送时把 `OutgoingInput` 入自己的 `outbox` 并 `notify()`；根视图**只读消费**
//!   输入区自己的 `outbox`（[`InputAreaGpui::take_pending`]），**不重新实现队列**。
//!   （未使用 `set_on_send`：它的回调拿不到 `cx`，无法写会话流。）
//! - 回显发生在 [`AppShell::drain_outbox`]（由 observe 输入区 + 每次 `render` 触发），
//!   经 [`sync::echo_message`] / [`sync::next_echo_seq`]
//!   变成一条 `User` 消息推进会话流。这一步**不写网络、不产生任何投递**，
//!   文案上只能称「本地回显 / 待运行时确认」。
//! - 回显时 **`in_flight` 保持 true**（描述层「等待确认期间禁重发」）。只有会话流
//!   出现非 `User` 的回应（[`sync::should_finish_send`]）时，才由
//!   [`AppShell::settle_pending`] 调 [`InputAreaGpui::finish_send`] 解除等待。
//!
//! e25 点文件跳预览：**会话流是意图的唯一产生者**，右栏预览 tab 只是**消费者**
//! （经 [`AppShell::sync_file_intent`] 下发）。⛔ 预览器本体未实现（P3，A14）。
//!
//! e29 侧栏导航入口切主区（对齐 A15）：侧栏的频道 / 智能体管理入口**不再是死按钮**
//! —— 根视图 `observe` 侧栏，经 [`AppShell::sync_main_view`] 把选中的入口
//! （[`crate::ui::main_view`]）同步到主区视图，渲染时由
//! [`main_area::render_main_area`] 换分支。⛔ 频道 / 智能体管理只显示**占位说明**
//! （数据源在运行时侧，本仓不实现）。
//!
//! e32 底部终端抽屉（对齐 A13）：抽屉本体在 [`crate::ui::terminal_gpui`]，
//! 根视图只做两件事 —— 把它挂在根渲染的最底部，以及经
//! [`AppShell::sync_terminal_drawer`] 消费它登记的「运行 / 中断」意图。
//! ⛔ 本仓**不 spawn 进程**、⛔ **不落盘**（抽屉状态只在内存里）。
//!
//! 文件划分：`demo.rs`（演示数据）、`launch.rs`（窗口启动）、`main_area.rs`（主区
//! 三分支渲染）、`sync.rs`（纯函数 + 根视图的同步方法）、`prefs_file.rs`（偏好落盘）、
//! `terminal_drawer.rs`（终端抽屉的宿主侧：意图消费 + 本地回显）、
//! `stop_intent.rs`（停止意图的宿主侧：消费 + 记账）、
//! `settings.rs`（设置窗口的状态快照）、`shell.rs`（根视图结构与生命周期方法）、
//! `render.rs`（标题栏 + 根元素树）、
//! `tests.rs` / `sync_tests.rs`（单元测试）。
#![cfg(feature = "ui")]

mod demo;
mod launch;
mod main_area;
mod palette_commands;
pub(crate) mod prefs_file;
mod render;
mod settings;
mod shell;
mod sidebar_collapse;
mod stop_intent;
mod sync;
#[cfg(test)]
mod sync_tests;
mod terminal_drawer;
#[cfg(test)]
mod tests;

pub use demo::{demo_session, secondary_views};
pub use launch::{launch, LaunchError, HOTKEY_HINT, NO_DISPLAY_MESSAGE, WINDOW_TITLE};

pub(super) use settings::{settings_snapshot, SettingsSnapshot};
pub(crate) use shell::AppShell;

/// 打开设置窗口的命令 id（命令表 `theme::palette` 里的 `app.settings`）。
const SETTINGS_COMMAND_ID: &str = "app.settings";
