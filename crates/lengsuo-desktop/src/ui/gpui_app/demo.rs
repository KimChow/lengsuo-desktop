//! 演示数据构造：会话流、快捷键表、次要视图。
//!
//! 数据全部来自 [`crate::state`]，根视图不持有业务语义。

use crate::state::hotkeys::{Binding, HotkeyRegistry};
use crate::state::session_stream::{
    MessageRole, PendingApproval, SessionMessage, SessionStreamState,
};
use crate::ui::env_gpui::EnvViewGpui;
use crate::ui::org_gpui::OrgViewGpui;
use crate::ui::org_view::OrgViewState;

/// 一根演示消息。
fn message(role: MessageRole, content: &str, tool: Option<&str>) -> SessionMessage {
    SessionMessage {
        role,
        content: content.into(),
        tool: tool.map(str::to_owned),
        ts_ms: 0,
    }
}

/// 演示用会话流：手工构造，**不接真实数据源**。
///
/// 覆盖四类消息与一个挂起审批，供根视图组装时验证各渲染分支都有内容。
#[must_use]
pub fn demo_session() -> SessionStreamState {
    let mut state = SessionStreamState::new();
    state.push_message(
        1,
        message(MessageRole::User, "帮我看一下构建脚本为什么失败", None),
    );
    state.push_message(
        2,
        message(MessageRole::Assistant, "好的，我先读取入口脚本。", None),
    );
    state.push_message(
        3,
        message(MessageRole::ToolCall, "scripts/card_guard.py", Some("Read")),
    );
    state.push_message(
        4,
        message(
            MessageRole::ToolResult,
            "#!/usr/bin/env python3\nprint('ok')",
            Some("Read"),
        ),
    );
    state.push_approval(PendingApproval {
        id: "demo-approval-1".into(),
        tool: "Bash".into(),
        summary: "运行 cargo test --workspace".into(),
        resolved: None,
    });
    state
}

/// 内置快捷键表；内置表若自相冲突则退化为空表（不 panic）。
pub(super) fn default_registry() -> HotkeyRegistry {
    HotkeyRegistry::with_defaults().unwrap_or_else(|_| {
        HotkeyRegistry::from_bindings(std::iter::empty::<Binding>()).expect("空绑定表必定无冲突")
    })
}

/// 环境 / 组织视图的演示实例。
///
/// 本批**不进主界面**（只做工作台四件），但要求代码编译进二进制。
/// 后续卡片接入导航时直接挂载这两个视图即可。
#[must_use]
pub fn secondary_views() -> (EnvViewGpui, OrgViewGpui) {
    (
        EnvViewGpui::demo(),
        OrgViewGpui::new(OrgViewState::default()),
    )
}
