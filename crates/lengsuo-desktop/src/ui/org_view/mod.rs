//! 组织域视图（部门树 / 成员 / 角色 / 能力按钮）。
//!
//! 渲染语义层：只把 [`OrgListState`] 与服务端下发的 [`CapabilityFlags`]
//! 转成可直接绘制的行/节点。**不做授权推理**：
//!
//! - 按钮可用性来自服务端 `can_*` 标记（缺失即禁用）；
//! - 拒绝文案与引导来自服务端原因码，三种原因分别展示；
//! - `ai_assistant` 角色全局只读是产品不可变规则（服务端强制），
//!   界面如实反映，不依赖任何权限推理。
//!
//! 列表三态复用 [`OrgListState::should_show_empty_state`]：失败态绝不显示空态。
//!
//! 目录结构：`mod.rs`（本文件，模块声明与聚合）、`model.rs`（数据类型）、
//! `state.rs`（OrgViewState 及 impl）、`rows.rs`（行/节点构建辅助）、`tests.rs`（单测）。
//! 公开 API 与拆分前一字不变（见下方 `pub use`）。

mod model;
mod rows;
mod state;
#[cfg(test)]
mod tests;

pub use model::{
    denial_notice, ActionState, DenialNotice, DepartmentNode, MemberRow, RoleRow, SectionView,
};
pub use state::OrgViewState;
