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
// === 波次 7：浮层第二批（一卡一文件；声明由主理人一次性落盘） ===
pub mod account_checkin;
pub mod account_credits;
pub mod account_menu;
pub mod archive;
pub mod artifact_forward;
pub mod artifact_slot;
pub mod automation_ds_form;
pub mod automation_error;
pub mod automation_schedule;
pub mod automation_templates;
pub mod background_task;
pub mod batch_bar;
pub mod cloud_upgrade;
pub mod cloud_welcome;
pub mod collab_activity;
pub mod collab_config;
pub mod collab_connectors;
pub mod collab_members;
pub mod collab_plan_create;
pub mod collab_plan_detail;
pub mod collab_plan_filter;
pub mod collab_plan_status;
pub mod collab_plan_subtodo;
pub mod collab_quota;
pub mod collab_task_handoff;
pub mod context_menu;
pub mod conversation_list;
pub mod credit_hint;
pub mod deeplink_landing;
pub mod delete;
pub mod detail_panel;
pub mod error_feedback;
pub mod file_version;
pub mod goods_purchase;
pub mod home_schedule_create;
pub mod home_schedule_sidebar;
pub mod idle_capability;
pub mod media_preview;
pub mod mention_popover;
pub mod mobile_connect;
pub mod network_check;
pub mod project_invitation;
pub mod queue_progress;
pub mod sandbox_intercept;
pub mod search_filter;
pub mod security_audit_config;
pub mod security_audit_network;
pub mod security_cmd_safety;
pub mod security_file_safety;
pub mod self_check;
pub mod share_selection;
pub mod share_task;
pub mod shortcut_cheatsheet;
pub mod task_hover_card;
pub mod task_invitation;
pub mod task_starter;
pub mod task_status;
pub mod todo_list;
pub mod tool_approval;
pub mod unified_market_bar;
pub mod update_flow;
pub mod voice_states;
pub mod workspace_trust;
pub mod worktree_branch;
