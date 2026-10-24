//! 浮层视图族（波次 6）。
//!
//! 与 `panel_views` 同构：**一卡一文件**，每张波次 6 卡只改自己那个模块，
//! 互不触碰 ⇒ `card_rules.overlaps` 判定零冲突，可真并行。
//!
//! 分层约定（与 `panel_views` / `sidebar_gpui` 同构）：
//! - **纯数据层**（各子模块上半，无 GPUI 依赖）：条目 / 空态 / 诚实占位文案；
//! - **GPUI 层**（子模块的 `view` / `bar` / `card`）：把描述接到元素树，颜色一律走主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**；
//! - 子模块的 `pub mod` 声明由主理人一次性落盘，卡不得改本文件。
#![cfg(feature = "ui")]
pub mod batch_bar;
pub mod idle_capability;
pub mod network_check;
pub mod project_invitation;
pub mod self_check;
pub mod share_selection;
pub mod task_starter;
pub mod task_status;
pub mod todo_list;
pub mod update_flow;
pub mod voice_states;
pub mod worktree_branch;
