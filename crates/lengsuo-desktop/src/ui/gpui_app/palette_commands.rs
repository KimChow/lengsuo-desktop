//! 命令面板里**导航类命令** → 导航目标的映射（纯数据，无 GPUI 依赖）。
//!
//! 命令表（`theme/palette.rs` 的 `COMMANDS`）是**唯一**的 id 来源；本模块只回答
//! 「这个 id 该把界面切到哪儿」，⛔ 不做业务动作（不建会话、不发消息）。
//!
//! ⛔ 未知 id 一律 `None` ⇒ 调用方静默忽略（fail-closed：不猜、不兜底到某个视图）。
/// 导航类命令的目标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavigateTarget {
    /// 回到会话流（清空侧栏导航选中）。
    Sessions,
    /// 浏览器环境视图（B 域）。
    Environments,
    /// 组织视图 · 成员表（A 域）。
    OrgMembers,
    /// 组织视图 · 角色表（A 域）。
    OrgRoles,
}
/// 命令 id → 导航目标。
///
/// `app.settings` **不在此列**（它开窗，由 `on_execute` 直接处理）；
/// 未知 id 返回 `None`。
#[must_use]
pub(crate) fn navigate_target(id: &str) -> Option<NavigateTarget> {
    match id {
        "session.list" | "session.new" => Some(NavigateTarget::Sessions),
        "env.list" => Some(NavigateTarget::Environments),
        "org.member" => Some(NavigateTarget::OrgMembers),
        "org.role" => Some(NavigateTarget::OrgRoles),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`（父模块 `use gpui_kit::*` 会把 GPUI 的 `test` 宏
    //    带进来遮蔽标准库 `#[test]`，见速查表 §12.3）。
    use crate::ui::gpui_app::palette_commands::{navigate_target, NavigateTarget};

    #[test]
    fn session_commands_return_to_sessions() {
        assert_eq!(
            navigate_target("session.list"),
            Some(NavigateTarget::Sessions)
        );
        assert_eq!(
            navigate_target("session.new"),
            Some(NavigateTarget::Sessions)
        );
    }

    #[test]
    fn env_command_switches_to_environments() {
        assert_eq!(
            navigate_target("env.list"),
            Some(NavigateTarget::Environments)
        );
    }

    #[test]
    fn org_commands_pick_their_own_section() {
        assert_eq!(
            navigate_target("org.member"),
            Some(NavigateTarget::OrgMembers)
        );
        assert_eq!(navigate_target("org.role"), Some(NavigateTarget::OrgRoles));
        assert_ne!(
            navigate_target("org.member"),
            navigate_target("org.role"),
            "成员与角色必须是不同目标"
        );
    }

    #[test]
    fn settings_and_unknown_ids_are_not_navigation() {
        assert_eq!(navigate_target("app.settings"), None, "开窗命令不走导航");
        assert_eq!(
            navigate_target("no.such.command"),
            None,
            "未知 id 必须 None"
        );
        assert_eq!(navigate_target(""), None);
    }
}
