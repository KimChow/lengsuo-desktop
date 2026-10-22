//! 演示数据构造：会话流、快捷键表、次要视图。
//!
//! 数据全部来自 [`crate::state`]，根视图不持有业务语义。

use crate::state::hotkeys::{Binding, HotkeyRegistry};
use crate::state::org_perm::CapabilityFlags;
use crate::state::session_stream::SessionStreamState;
use crate::ui::env_gpui::EnvViewGpui;
use crate::ui::org_gpui::OrgViewGpui;
use crate::ui::org_view::OrgViewState;

/// 演示用会话流：五类工具调用 + 一个挂起审批，**不接真实数据源**。
///
/// 数据来自 [`crate::state::session_demo`]（e15 产出）：五类工具调用各一条，
/// 用来在真机窗口里验证工具卡片的所有渲染分支。
/// 主界面与 [`crate::ui::session_gpui::SessionStreamGpui::demo`] 消费**同一份**数据
/// —— 否则「工具卡片五类真实触发」在窗口里看不到，即「实现齐了但没人用」。
/// 详见 `docs/session-render-status.md` §9.6。
#[must_use]
pub fn demo_session() -> SessionStreamState {
    crate::state::session_demo::demo_session()
}
/// 内置快捷键表；内置表若自相冲突则退化为空表（不 panic）。
pub(super) fn default_registry() -> HotkeyRegistry {
    HotkeyRegistry::with_defaults().unwrap_or_else(|_| {
        HotkeyRegistry::from_bindings(std::iter::empty::<Binding>()).expect("空绑定表必定无冲突")
    })
}

/// 环境 / 组织视图的演示实例。
///
/// ⚠️ 这里返回的组织视图是**空态**（`OrgViewState::default()` ⇒ 三张列表都是
/// `Loading`）⇒ 只用来验证「视图可构造」，**不用于主界面**。
/// 主界面挂的是 [`demo_org_view`]（有内容的演示数据）。
#[must_use]
pub fn secondary_views() -> (EnvViewGpui, OrgViewGpui) {
    (
        EnvViewGpui::demo(),
        OrgViewGpui::new(OrgViewState::default()),
    )
}

/// 组织域的演示视图：部门 / 成员 / 角色各有几条**演示**条目。
///
/// ⛔ **演示数据**：真实成员 / 角色 / 部门由服务端下发，本仓暂不接入。
/// ⛔ **不伪造权限**：能力位用 [`CapabilityFlags::default`]（空标记）
/// ⇒ 界面显示「服务端未下发能力标记」——这是**对的**（服务端确实没下发）。
///
/// 字段名对齐描述层 [`OrgViewState`]（`member_rows` / `role_rows` /
/// `department_rows` 的取值规则），⛔ 不另造一套。
#[must_use]
pub fn demo_org_view() -> OrgViewGpui {
    use serde_json::json;

    let mut state = OrgViewState::new(CapabilityFlags::default());
    state.departments_mut().apply(vec![
        json!({"id": "d1", "name": "研发一部"}),
        json!({"id": "d2", "name": "华南组"}),
    ]);
    state.members_mut().apply(vec![
        json!({"id": "u1", "name": "张三", "email": "zhang@example.com"}),
        json!({"id": "u2", "name": "李四", "email": "li@example.com"}),
    ]);
    state.roles_mut().apply(vec![
        json!({"id": "r1", "name": "管理员", "preset_key": "admin", "is_builtin": true}),
        json!({"id": "r2", "name": "AI 助手", "preset_key": "ai_assistant", "is_ai_assistant": true}),
    ]);
    OrgViewGpui::new(state)
}
