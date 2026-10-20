//! 会话流渲染：消息、工具卡片、审批、断档提示。
//!
//! 输入是 [`SessionStreamState`]（纯状态），输出是 [`Block`] 描述列表；
//! 本模块不依赖 GPUI，便于在无窗口环境下测试渲染语义。
//!
//! 已实现的渲染分支：
//! - `user`：右对齐气泡
//! - `assistant`：左对齐正文（**纯文本**，尚未接入 Markdown 引擎，见文档缺口）
//! - `tool_call`：折叠卡片，标题走 [`crate::theme::tool_titles`]
//! - `tool_result`：终端输出 / 文件列表（可点击）/ diff
//! - `files`：文件卡片网格（[`files_grid`]，供调用方直接使用）
//! - `approval`：弹窗式审批卡片（[`crate::ui::approval_card`]）
//!
//! 断档约定：`is_stale()` 时**首屏必须出现显著提示**，不得假装历史完整。

use crate::state::session_stream::{MessageRole, SessionMessage, SessionStreamState};
use crate::theme::tool_titles::tool_title;
use crate::ui::approval_card::{self, ApprovalCardView};
use crate::ui::tool_card::ToolCard;

/// 断档提示文案（界面必须原样可见）。
pub const STALE_NOTICE: &str = "事件流中断，正在恢复";

/// 文本对齐方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// `tool_result` 的展示形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolResultKind {
    /// 等宽终端输出。
    Terminal,
    /// 可点击的文件列表。
    FileList,
    /// 统一 diff。
    Diff,
}

/// 文件卡片中的一项。`clickable` 为真时界面可打开该路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    pub path: String,
    pub display: String,
    pub clickable: bool,
}

impl FileItem {
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            display: path.rsplit('/').next().unwrap_or(path).to_owned(),
            clickable: true,
        }
    }
}

/// 工具结果视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultView {
    /// 人话标题（如「读取 src/main.rs」），不含内部工具名。
    pub title: String,
    pub kind: ToolResultKind,
    /// 终端输出或 diff 原文（文件列表时为空）。
    pub text: String,
    pub files: Vec<FileItem>,
}

impl ToolResultView {
    #[must_use]
    pub fn from_message(msg: &SessionMessage) -> Self {
        let (kind, files) = classify_result(&msg.content);
        Self {
            title: tool_title(msg.tool.as_deref().unwrap_or_default(), Some("结果")),
            kind,
            text: if kind == ToolResultKind::FileList {
                String::new()
            } else {
                msg.content.clone()
            },
            files,
        }
    }
}

/// 一屏可渲染的块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// 用户消息气泡，右对齐。
    UserBubble { text: String, align: Align },
    /// 助手正文，左对齐。
    AssistantText { text: String, align: Align },
    /// 工具调用折叠卡片。
    ToolCall(ToolCard),
    /// 工具结果（终端 / 文件列表 / diff）。
    ToolResult(ToolResultView),
    /// 文件卡片网格。
    FilesGrid { files: Vec<FileItem> },
    /// 待审批卡片。
    Approval(ApprovalCardView),
    /// 断档提示。
    StaleNotice { text: &'static str },
}

/// 渲染整个会话流。
///
/// - 断档时提示块置于**最前**，保证首屏可见；
/// - 只渲染**未决策**的审批卡片（已决策项不再弹窗）；
/// - 渲染只读状态，不会替用户作出任何审批决策。
#[must_use]
pub fn render(state: &SessionStreamState) -> Vec<Block> {
    let mut blocks = Vec::with_capacity(state.messages().len() + 2);
    if state.is_stale() {
        blocks.push(Block::StaleNotice { text: STALE_NOTICE });
    }
    blocks.extend(state.messages().iter().map(render_message));
    blocks.extend(
        approval_card::pending_cards(state)
            .into_iter()
            .map(Block::Approval),
    );
    blocks
}

/// 渲染单条消息（四类角色各有分支）。
#[must_use]
pub fn render_message(msg: &SessionMessage) -> Block {
    match msg.role {
        MessageRole::User => Block::UserBubble {
            text: msg.content.clone(),
            align: Align::Right,
        },
        MessageRole::Assistant => Block::AssistantText {
            text: msg.content.clone(),
            align: Align::Left,
        },
        MessageRole::ToolCall => Block::ToolCall(ToolCard::from_message(msg)),
        MessageRole::ToolResult => Block::ToolResult(ToolResultView::from_message(msg)),
    }
}

/// 文件卡片网格（`files` 消息）。
#[must_use]
pub fn files_grid(paths: &[String]) -> Block {
    Block::FilesGrid {
        files: paths.iter().map(|p| FileItem::new(p)).collect(),
    }
}

/// 判定 `tool_result` 的形态并提取文件项。
///
/// 判定顺序：diff → 文件列表 → 终端输出。宁可落到终端也不误判，
/// 因为误判会隐藏原始输出。
#[must_use]
pub fn classify_result(content: &str) -> (ToolResultKind, Vec<FileItem>) {
    if looks_like_diff(content) {
        return (ToolResultKind::Diff, Vec::new());
    }
    let files = parse_file_list(content);
    if !files.is_empty() {
        return (ToolResultKind::FileList, files);
    }
    (ToolResultKind::Terminal, Vec::new())
}

fn looks_like_diff(content: &str) -> bool {
    content.lines().any(|line| {
        line.starts_with("diff --git ") || line.starts_with("@@ ") || line.starts_with("+++ ")
    })
}

/// 每行一个路径，且至少一行带路径特征（`/` 或扩展名），才视为文件列表。
fn parse_file_list(content: &str) -> Vec<FileItem> {
    let lines: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let all_paths = lines.iter().all(|l| {
        !l.chars().any(char::is_whitespace)
            && !l.contains("://")
            && !l.starts_with('-')
            && (l.contains('/') || l.contains('.') || l.ends_with('/'))
    });
    if !all_paths {
        return Vec::new();
    }
    lines.into_iter().map(FileItem::new).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::session_stream::PendingApproval;

    fn msg(role: MessageRole, content: &str, tool: Option<&str>) -> SessionMessage {
        SessionMessage {
            role,
            content: content.into(),
            tool: tool.map(str::to_owned),
            ts_ms: 0,
        }
    }

    fn sample_state() -> SessionStreamState {
        let mut s = SessionStreamState::new();
        assert!(s.push_message(1, msg(MessageRole::User, "你好", None)));
        assert!(s.push_message(2, msg(MessageRole::Assistant, "**好的**", None)));
        assert!(s.push_message(3, msg(MessageRole::ToolCall, "src/main.rs", Some("Read"))));
        assert!(s.push_message(
            4,
            msg(MessageRole::ToolResult, "fn main() {}", Some("Read"))
        ));
        s
    }

    #[test]
    fn four_message_roles_have_distinct_branches() {
        let blocks: Vec<Block> = sample_state()
            .messages()
            .iter()
            .map(render_message)
            .collect();
        assert!(matches!(
            blocks[0],
            Block::UserBubble {
                align: Align::Right,
                ..
            }
        ));
        assert!(matches!(
            blocks[1],
            Block::AssistantText {
                align: Align::Left,
                ..
            }
        ));
        assert!(matches!(blocks[2], Block::ToolCall(_)));
        assert!(matches!(blocks[3], Block::ToolResult(_)));
    }

    #[test]
    fn tool_call_title_is_human_and_hides_internal_name() {
        let block = render_message(&msg(MessageRole::ToolCall, "src/main.rs", Some("Read")));
        match block {
            Block::ToolCall(card) => {
                assert_eq!(card.title, "读取 src/main.rs");
                assert!(card.folded);
            }
            other => panic!("期望 ToolCall，得到 {other:?}"),
        }

        let block = render_message(&msg(MessageRole::ToolCall, "x", Some("InternalToolXyz")));
        let Block::ToolCall(card) = block else {
            panic!("期望 ToolCall");
        };
        assert!(!card.title.contains("InternalToolXyz"));
    }

    #[test]
    fn tool_result_classifies_terminal_file_list_and_diff() {
        let terminal = ToolResultView::from_message(&msg(
            MessageRole::ToolResult,
            "$ cargo test\nrunning 3 tests",
            Some("Bash"),
        ));
        assert_eq!(terminal.kind, ToolResultKind::Terminal);
        assert_eq!(terminal.text, "$ cargo test\nrunning 3 tests");

        let list = ToolResultView::from_message(&msg(
            MessageRole::ToolResult,
            "src/main.rs\nsrc/lib.rs\nREADME.md",
            Some("Glob"),
        ));
        assert_eq!(list.kind, ToolResultKind::FileList);
        assert_eq!(list.files.len(), 3);
        assert!(list.files.iter().all(|f| f.clickable));
        assert_eq!(list.files[0].display, "main.rs");

        let diff = ToolResultView::from_message(&msg(
            MessageRole::ToolResult,
            "diff --git a/x b/x\n@@ -1 +1 @@\n-a\n+b",
            Some("Edit"),
        ));
        assert_eq!(diff.kind, ToolResultKind::Diff);
        assert!(diff.text.contains("@@"));
    }

    #[test]
    fn tool_result_title_does_not_leak_internal_name() {
        let view = ToolResultView::from_message(&msg(MessageRole::ToolResult, "x", Some("Hush")));
        assert!(!view.title.contains("Hush"));
    }

    #[test]
    fn stale_state_renders_prominent_notice_first() {
        let mut s = sample_state();
        s.mark_gap();
        let blocks = render(&s);
        assert_eq!(
            blocks.first(),
            Some(&Block::StaleNotice { text: STALE_NOTICE }),
            "断档提示必须在首屏最前"
        );
        assert!(STALE_NOTICE.contains("中断"));
    }

    #[test]
    fn healthy_stream_has_no_stale_notice() {
        let blocks = render(&sample_state());
        assert!(!blocks
            .iter()
            .any(|b| matches!(b, Block::StaleNotice { .. })));
    }

    #[test]
    fn pending_approvals_render_and_render_does_not_resolve_them() {
        let mut s = sample_state();
        s.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "Bash".into(),
            summary: "rm -rf build".into(),
            resolved: None,
        });
        let blocks = render(&s);
        let approvals: Vec<&Block> = blocks
            .iter()
            .filter(|b| matches!(b, Block::Approval(_)))
            .collect();
        assert_eq!(approvals.len(), 1);
        let Block::Approval(card) = approvals[0] else {
            unreachable!()
        };
        assert!(card.pending);
        assert!(card.buttons.iter().all(|b| b.enabled));
        assert_eq!(
            s.pending_approvals().count(),
            1,
            "渲染是只读的，不得默认放行审批"
        );
    }

    #[test]
    fn decided_approval_disappears_and_cannot_be_acted_again() {
        let mut s = sample_state();
        s.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "Bash".into(),
            summary: "rm -rf build".into(),
            resolved: None,
        });
        assert!(approval_card::decide(
            &mut s,
            "a1",
            approval_card::ApprovalAction::Deny
        ));
        assert!(!render(&s).iter().any(|b| matches!(b, Block::Approval(_))));
        assert!(!approval_card::decide(
            &mut s,
            "a1",
            approval_card::ApprovalAction::Allow
        ));
    }

    #[test]
    fn files_grid_makes_each_file_clickable() {
        let block = files_grid(&["src/main.rs".into(), "docs/a.md".into()]);
        match block {
            Block::FilesGrid { files } => {
                assert_eq!(files.len(), 2);
                assert!(files.iter().all(|f| f.clickable));
                assert_eq!(files[1].display, "a.md");
            }
            other => panic!("期望 FilesGrid，得到 {other:?}"),
        }
    }

    #[test]
    fn plain_terminal_output_is_not_misclassified_as_files() {
        let (kind, files) = classify_result("Compiling lengsuo-desktop v0.1.0\nFinished dev");
        assert_eq!(kind, ToolResultKind::Terminal);
        assert!(files.is_empty());
    }
}
