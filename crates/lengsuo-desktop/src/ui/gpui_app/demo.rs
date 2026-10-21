//! 演示数据构造：会话流、快捷键表、次要视图。
//!
//! 数据全部来自 [`crate::state`]，根视图不持有业务语义。

use crate::state::hotkeys::{Binding, HotkeyRegistry};
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
/// 本批**不进主界面**（只做工作台四件），但要求代码编译进二进制。
/// 后续卡片接入导航时直接挂载这两个视图即可。
#[must_use]
pub fn secondary_views() -> (EnvViewGpui, OrgViewGpui) {
    (
        EnvViewGpui::demo(),
        OrgViewGpui::new(OrgViewState::default()),
    )
}
