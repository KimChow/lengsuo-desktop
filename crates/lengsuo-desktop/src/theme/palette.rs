//! 命令面板（`Ctrl+K`）的命令表与过滤（纯逻辑，无渲染）。

/// 命令类别，决定分组展示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Session,
    Environment,
    Organization,
    App,
}

/// 一条命令。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteCommand {
    /// 稳定标识（用于派发，不用标题派发）。
    pub id: &'static str,
    /// 展示标题。
    pub title: &'static str,
    pub kind: CommandKind,
}

/// 内置命令表。
pub const COMMANDS: &[PaletteCommand] = &[
    PaletteCommand {
        id: "session.new",
        title: "新建会话",
        kind: CommandKind::Session,
    },
    PaletteCommand {
        id: "session.list",
        title: "会话列表",
        kind: CommandKind::Session,
    },
    PaletteCommand {
        id: "env.list",
        title: "环境列表",
        kind: CommandKind::Environment,
    },
    PaletteCommand {
        id: "org.member",
        title: "成员管理",
        kind: CommandKind::Organization,
    },
    PaletteCommand {
        id: "org.role",
        title: "角色管理",
        kind: CommandKind::Organization,
    },
    PaletteCommand {
        id: "app.settings",
        title: "设置",
        kind: CommandKind::App,
    },
];

/// 按关键词过滤命令：大小写不敏感的子串匹配；空关键词返回全部。
#[must_use]
pub fn filter_commands(keyword: &str, commands: &[PaletteCommand]) -> Vec<PaletteCommand> {
    let kw = keyword.trim().to_lowercase();
    if kw.is_empty() {
        return commands.to_vec();
    }
    commands
        .iter()
        .filter(|c| c.title.to_lowercase().contains(&kw) || c.id.to_lowercase().contains(&kw))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_keyword_returns_all() {
        let all = filter_commands("", COMMANDS);
        assert_eq!(all.len(), COMMANDS.len());
        assert_eq!(filter_commands("   ", COMMANDS).len(), COMMANDS.len());
    }

    #[test]
    fn filters_by_title_substring() {
        let hit = filter_commands("会话", COMMANDS);
        assert_eq!(hit.len(), 2);
        assert!(hit.iter().all(|c| c.kind == CommandKind::Session));
    }

    #[test]
    fn filters_by_id_as_well() {
        let hit = filter_commands("org.", COMMANDS);
        assert_eq!(hit.len(), 2);
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert_eq!(filter_commands("ENV.LIST", COMMANDS).len(), 1);
    }

    #[test]
    fn no_match_returns_empty() {
        assert!(filter_commands("zzz-nothing", COMMANDS).is_empty());
    }

    #[test]
    fn command_ids_are_unique() {
        let mut ids: Vec<&str> = COMMANDS.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "命令 id 必须唯一，否则派发会歧义");
    }
}
