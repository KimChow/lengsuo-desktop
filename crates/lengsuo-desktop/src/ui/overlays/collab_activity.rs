//! 协作动态（波次 8，卡 `ui-collab-activity-feed`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.activity`。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（卡面目标名，与既有占位保持一致）。
pub const TITLE: &str = "协作动态";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.activity";
/// 命名空间键总数（据文案全表实测）。
pub const NS_KEY_COUNT: usize = 149;

/// 文案表（逐字取自文案全表命名空间 `collab.activity`；本卡收录 78 键，任务书要求 ≥40）。
pub const TEXTS: &[(&str, &str)] = &[
    (
        "collab.activity.action.connectorPublicAuthorized",
        "完成了 {name} 的授权，现在项目成员都可以使用",
    ),
    ("collab.activity.action.fileDeletedFallback", "删除了文件"),
    (
        "collab.activity.action.fileDeletedInFolder",
        "删除了「{folder}」下的 {name}",
    ),
    (
        "collab.activity.action.fileDeletedMulti",
        "删除了 {count} 个文件",
    ),
    ("collab.activity.action.fileFolderRootLabel", "全部文件"),
    ("collab.activity.action.fileFolderSeparator", " › "),
    (
        "collab.activity.action.fileMoved",
        "将 {name} 移动到「{targetPath}」",
    ),
    ("collab.activity.action.fileMovedFallback", "移动了文件"),
    (
        "collab.activity.action.fileMovedFromTo",
        "将 {name} 从「{sourceFolder}」移动到「{targetPath}」",
    ),
    (
        "collab.activity.action.fileMovedMulti",
        "移动了 {count} 个文件",
    ),
    ("collab.activity.action.fileMovedRootLabel", "全部文件"),
    (
        "collab.activity.action.fileRenamed",
        "将 {oldName} 重命名为 {newName}",
    ),
    ("collab.activity.action.fileRenamedFallback", "重命名了文件"),
    (
        "collab.activity.action.fileRenamedInFolder",
        "将「{folder}」下的 {oldName} 重命名为 {newName}",
    ),
    (
        "collab.activity.action.fileRenamedMulti",
        "重命名了 {count} 个文件",
    ),
    ("collab.activity.action.fileSuffixMore", " 等文件"),
    ("collab.activity.action.fileUpdated", "更新了 {name}"),
    ("collab.activity.action.fileUpdatedFallback", "更新了文件"),
    (
        "collab.activity.action.fileUpdatedInFolder",
        "更新了「{folder}」下的 {name}",
    ),
    (
        "collab.activity.action.fileUpdatedMulti",
        "更新了 {count} 个文件",
    ),
    ("collab.activity.action.fileUploaded", "上传了 {name}"),
    ("collab.activity.action.fileUploadedFallback", "上传了文件"),
    (
        "collab.activity.action.fileUploadedInFolder",
        "在「{folder}」上传了 {name}",
    ),
    (
        "collab.activity.action.fileUploadedMulti",
        "上传了 {count} 个文件",
    ),
    (
        "collab.activity.action.instructionChanged",
        "更改了项目指令",
    ),
    ("collab.activity.action.memberInvited", "邀请了 {name}"),
    ("collab.activity.approval.action.approve", "同意"),
    ("collab.activity.approval.action.reject", "拒绝"),
    ("collab.activity.approval.role.admin", "管理员"),
    ("collab.activity.approval.role.member", "成员"),
    ("collab.activity.approval.rolePrefix", "申请权限：{role}"),
    ("collab.activity.approval.status.approved", "已同意"),
    ("collab.activity.approval.status.cancelled", "已撤回"),
    ("collab.activity.approval.status.expired", "已过期"),
    ("collab.activity.approval.status.invalid", "已失效"),
    ("collab.activity.approval.status.rejected", "已拒绝"),
    (
        "collab.activity.approval.toast.fallback",
        "操作失败，请稍后重试",
    ),
    ("collab.activity.detailDialog.addToChat", "添加至任务"),
    ("collab.activity.detailDialog.close", "关闭"),
    ("collab.activity.detailDialog.emptyFiles", "暂无关联文件"),
    ("collab.activity.detailDialog.fileDeleted", "文件已被删除"),
    ("collab.activity.detailDialog.fileSectionTitle", "文件"),
    (
        "collab.activity.detailDialog.movedSourceTarget",
        " · 从「{sourceFolder}」移动到「{targetPath}」",
    ),
    (
        "collab.activity.detailDialog.movedTargetPath",
        " · 移动到「{targetPath}」",
    ),
    ("collab.activity.detailDialog.title", "动态详情"),
    ("collab.activity.mapper.fallbackUnnamedFile", "未命名文件"),
    ("collab.activity.memberFilter.allMembers", "全部成员"),
    (
        "collab.activity.memberFilter.allMembersCount",
        "全部成员（{selected}/{total}）",
    ),
    ("collab.activity.memberFilter.clear", "重置"),
    ("collab.activity.memberFilter.confirm", "确定"),
    ("collab.activity.memberFilter.countLabel", "{count}个成员"),
    ("collab.activity.memberFilter.defaultLabel", "成员筛选"),
    (
        "collab.activity.memberFilter.othersSuffix",
        "{first} 等 {count} 人",
    ),
    ("collab.activity.memberFilter.searchPlaceholder", "搜索成员"),
    ("collab.activity.memberFilter.title", "筛选成员"),
    ("collab.activity.memberFilter.viewAllTasks", "查看所有任务"),
    ("collab.activity.message.cancel", "取消"),
    ("collab.activity.message.deleteFailed", "留言删除失败"),
    ("collab.activity.message.editFailed", "留言修改失败"),
    ("collab.activity.message.mentionAllHint", "通知项目全体成员"),
    (
        "collab.activity.message.placeholder",
        "发条留言同步给项目成员，输入 @ 可提及成员",
    ),
    ("collab.activity.message.replyFailed", "回复失败"),
    ("collab.activity.message.send", "发送"),
    ("collab.activity.message.submitFailed", "留言发送失败"),
    ("collab.activity.nameSeparator", "、"),
    ("collab.activity.panel.empty", "暂无与我有关的动态"),
    ("collab.activity.panel.emptyMember", "暂无动态"),
    (
        "collab.activity.panel.emptyMemberFiltered",
        "没有匹配的动态",
    ),
    (
        "collab.activity.panel.emptyMemberFilterMessage",
        "清除成员筛选后查看全部动态",
    ),
    ("collab.activity.panel.loadFailed", "加载失败"),
    (
        "collab.activity.panel.loadFailedRetryMessage",
        "加载失败，请稍后重试",
    ),
    ("collab.activity.panel.loading", "加载中…"),
    ("collab.activity.panel.memberTab", "成员动态"),
    ("collab.activity.panel.mineTab", "与我相关"),
    ("collab.activity.panel.noMore", "没有更多了"),
    ("collab.activity.panel.postMessage", "发布留言"),
    ("collab.activity.panel.retry", "点击重试"),
    ("collab.activity.todoEvent.time.justNow", "刚刚"),
];

/// 按键取文案（纯函数；缺键返回 `None`，调用方自行 fail-closed）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 按键取文案；键在本卡静态表内，缺键属程序性错误。
fn must_text(key: &str) -> &'static str {
    text(key).expect("collab.activity 文案表缺键")
}

/// 浮层/面板状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发，正常呈现（条目空集 ⇒ 空态，见 [`body_label`]）。
    Ready,
    /// 拉取中。
    Loading,
    /// 拉取失败（fail-closed，必须显示错误 + 重试）。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 由运行时输入解析初始态（纯函数）。
pub fn resolve(delivered: Option<bool>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(false) => State::LoadFailed,
        Some(true) => State::Ready,
    }
}

/// 动作文案（模板 + 参数，渲染成句由 [`ActionText::render`] 完成）。
#[derive(Clone, Debug, PartialEq)]
pub enum ActionText {
    /// 上传了 {name}
    FileUploaded { name: String },
    /// 在「{folder}」上传了 {name}
    FileUploadedInFolder { folder: String, name: String },
    /// 删除了「{folder}」下的 {name}
    FileDeletedInFolder { folder: String, name: String },
    /// 删除了 {count} 个文件
    FileDeletedMulti { count: usize },
    /// 将 {oldName} 重命名为 {newName}
    FileRenamed { old_name: String, new_name: String },
    /// 将 {name} 从「{sourceFolder}」移动到「{targetPath}」
    FileMovedFromTo {
        name: String,
        source_folder: String,
        target_path: String,
    },
    /// 更新了 {count} 个文件
    FileUpdatedMulti { count: usize },
    /// 邀请了 {name}
    MemberInvited { name: String },
    /// 更改了项目指令
    InstructionChanged,
    /// 完成了 {name} 的授权，现在项目成员都可以使用
    ConnectorPublicAuthorized { name: String },
}

/// 用 `{placeholder}` 对替换模板占位符（纯函数）。
fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (placeholder, value) in pairs {
        out = out.replace(placeholder, value);
    }
    out
}

impl ActionText {
    /// 渲染成句（模板逐字取自文案表；⛔ 不自编中文）。
    pub fn render(&self) -> String {
        match self {
            Self::FileUploaded { name } => fill(
                must_text("collab.activity.action.fileUploaded"),
                &[("{name}", name.as_str())],
            ),
            Self::FileUploadedInFolder { folder, name } => fill(
                must_text("collab.activity.action.fileUploadedInFolder"),
                &[("{folder}", folder.as_str()), ("{name}", name.as_str())],
            ),
            Self::FileDeletedInFolder { folder, name } => fill(
                must_text("collab.activity.action.fileDeletedInFolder"),
                &[("{folder}", folder.as_str()), ("{name}", name.as_str())],
            ),
            Self::FileDeletedMulti { count } => fill(
                must_text("collab.activity.action.fileDeletedMulti"),
                &[("{count}", &count.to_string())],
            ),
            Self::FileRenamed { old_name, new_name } => fill(
                must_text("collab.activity.action.fileRenamed"),
                &[
                    ("{oldName}", old_name.as_str()),
                    ("{newName}", new_name.as_str()),
                ],
            ),
            Self::FileMovedFromTo {
                name,
                source_folder,
                target_path,
            } => fill(
                must_text("collab.activity.action.fileMovedFromTo"),
                &[
                    ("{name}", name.as_str()),
                    ("{sourceFolder}", source_folder.as_str()),
                    ("{targetPath}", target_path.as_str()),
                ],
            ),
            Self::FileUpdatedMulti { count } => fill(
                must_text("collab.activity.action.fileUpdatedMulti"),
                &[("{count}", &count.to_string())],
            ),
            Self::MemberInvited { name } => fill(
                must_text("collab.activity.action.memberInvited"),
                &[("{name}", name.as_str())],
            ),
            Self::InstructionChanged => {
                must_text("collab.activity.action.instructionChanged").to_string()
            }
            Self::ConnectorPublicAuthorized { name } => fill(
                must_text("collab.activity.action.connectorPublicAuthorized"),
                &[("{name}", name.as_str())],
            ),
        }
    }
}

/// 单条动态（纯数据；由调用方供数，本卡无数据源 ⇒ 不构造）。
#[derive(Clone, Debug, PartialEq)]
pub struct ActivityEntry {
    /// 操作人显示名。
    pub actor: String,
    /// 动作文案。
    pub action: ActionText,
}

/// 动态面板状态 + 条目集（纯数据）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {
    /// 面板状态机。
    pub state: State,
    /// 条目集（`Ready` 且空 ⇒ 空态文案，≠ 未下发）。
    pub entries: Vec<ActivityEntry>,
}

impl Feed {
    /// 已下发 + 给定条目集。
    pub fn ready(entries: Vec<ActivityEntry>) -> Self {
        Self {
            state: State::Ready,
            entries,
        }
    }
}

/// 正文主文案（纯函数）：`Some(label)` ⇒ 单行提示分支；`None` ⇒ 条目列表分支。
/// fail-closed：`NotDelivered` 映射 [`NOT_DELIVERED`]，⛔ 不与空态混同。
pub fn body_label(feed: &Feed) -> Option<&'static str> {
    match feed.state {
        State::NotDelivered => Some(NOT_DELIVERED),
        State::Loading => Some(must_text("collab.activity.panel.loading")),
        State::LoadFailed => Some(must_text("collab.activity.panel.loadFailed")),
        State::Ready if feed.entries.is_empty() => Some(must_text("collab.activity.panel.empty")),
        State::Ready => None,
    }
}

// ===== GPUI 层 =====
/// 主区/浮层呈现入口（fail-closed：无数据源 ⇒ 固定「未下发」分支）。
pub fn main_area(theme: &Theme) -> AnyElement {
    let state = resolve(None);
    let title = SharedString::from(format!(
        "{}（{}）",
        TITLE,
        match state {
            State::Ready => "就绪",
            State::Loading => "加载中",
            State::LoadFailed => "加载失败",
            State::NotDelivered => NOT_DELIVERED,
        }
    ));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground)
        .child(title)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        body_label, resolve, text, ActionText, ActivityEntry, Feed, State, NOT_DELIVERED, TEXTS,
    };

    #[test]
    fn resolve_and_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(resolve(Some(false)), State::LoadFailed);
        assert_eq!(resolve(Some(true)), State::Ready);
    }

    #[test]
    fn texts_verbatim() {
        assert_eq!(
            text("collab.activity.action.fileRenamed"),
            Some("将 {oldName} 重命名为 {newName}")
        );
        assert_eq!(
            text("collab.activity.action.fileFolderSeparator"),
            Some(" › ")
        );
        assert_eq!(
            text("collab.activity.panel.empty"),
            Some("暂无与我有关的动态")
        );
        assert!(text("collab.activity.message.placeholder").is_some());
        let n = TEXTS.len();
        assert!(n >= 40, "文案覆盖不足: {n}");
    }

    #[test]
    fn texts_unique_and_prefixed() {
        let mut seen = std::collections::HashSet::new();
        for (key, value) in TEXTS {
            assert!(key.starts_with("collab.activity."), "越界键: {key}");
            assert!(!value.is_empty(), "空文案: {key}");
            assert!(seen.insert(*key), "重复键: {key}");
        }
    }

    #[test]
    fn action_renders_verbatim() {
        let invited = ActionText::MemberInvited {
            name: "李四".into(),
        };
        assert_eq!(invited.render(), "邀请了 李四");
        let renamed = ActionText::FileRenamed {
            old_name: "a.md".into(),
            new_name: "b.md".into(),
        };
        assert_eq!(renamed.render(), "将 a.md 重命名为 b.md");
        let moved = ActionText::FileMovedFromTo {
            name: "规格.md".into(),
            source_folder: "全部文件".into(),
            target_path: "归档".into(),
        };
        assert_eq!(moved.render(), "将 规格.md 从「全部文件」移动到「归档」");
    }

    #[test]
    fn body_branches_covered() {
        let undelivered = Feed::default();
        assert_eq!(body_label(&undelivered), Some(NOT_DELIVERED));
        let loading = Feed {
            state: State::Loading,
            entries: vec![],
        };
        assert_eq!(body_label(&loading), Some("加载中…"));
        let failed = Feed {
            state: State::LoadFailed,
            entries: vec![],
        };
        assert_eq!(body_label(&failed), Some("加载失败"));
        let empty_ready = Feed::ready(vec![]);
        assert_eq!(body_label(&empty_ready), Some("暂无与我有关的动态"));
        let fed = Feed::ready(vec![ActivityEntry {
            actor: "张三".into(),
            action: ActionText::InstructionChanged,
        }]);
        assert_eq!(body_label(&fed), None);
    }
}
