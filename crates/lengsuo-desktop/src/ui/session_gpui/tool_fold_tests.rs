//! 工具卡片纯逻辑的单元测试（从 [`super::tool_fold`] 拆出，控制单文件行数）。
//!
//! 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
//! `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。

// 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
// `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。
use super::tool_fold::{
    category_of, describe, fold_summary, short_title_arg, tool_call_details, tool_icon,
    visible_text, MAX_SUMMARY_CHARS, MAX_TITLE_ARG_CHARS,
};
use crate::state::session_stream::{MessageRole, SessionMessage};
use crate::ui::tool_card::ToolCard;

fn call(tool: &str, content: &str) -> SessionMessage {
    SessionMessage {
        role: MessageRole::ToolCall,
        content: content.into(),
        tool: Some(tool.into()),
        ts_ms: 0,
    }
}

fn user(content: &str) -> SessionMessage {
    SessionMessage {
        role: MessageRole::User,
        content: content.into(),
        tool: None,
        ts_ms: 0,
    }
}

/// 五类真实触发（读取 / 创建 / 修改 / 执行命令 / 搜索）的样例参数。
///
/// ⚠️ 参数里**不得**出现工具英文名本身（否则反例测试会误判）；
/// 命令正文可以显示（这是参数，不是工具名）。
const FIVE: [(&str, &str, &str); 5] = [
    ("Read", "src/ui/session_gpui/mod.rs", "读取"),
    ("Write", "docs/session-render-status.md", "创建"),
    ("Edit", "crates/lengsuo-desktop/src/ui/tool_card.rs", "修改"),
    ("Bash", "cargo test --workspace", "执行命令"),
    ("Grep", "fn render_tool_call", "搜索"),
];

#[test]
fn r13_1_no_english_tool_name_is_visible_for_the_five_kinds() {
    for (tool, args, human) in FIVE {
        let msg = call(tool, args);
        let detail = tool_call_details(std::slice::from_ref(&msg))
            .into_iter()
            .next()
            .expect("工具调用消息应产出接线细节");
        let card = ToolCard::from_message(&msg);
        let view = describe(&card, Some(&detail), true);
        let text = visible_text(&view).to_lowercase();

        assert!(
            !text.contains(&tool.to_lowercase()),
            "界面不得出现工具英文名 {tool}，实际：{text}"
        );
        assert!(
            view.title.starts_with(human),
            "标题应是人话标题（以 {human} 开头），实际：{}",
            view.title
        );
        assert!(
            !view.title.contains(&tool.to_lowercase()),
            "标题不得含工具英文名 {tool}，实际：{}",
            view.title
        );
        assert!(
            !view.icon.is_empty(),
            "每张卡片都要有图标（文字 / unicode）"
        );
    }
}

#[test]
fn long_arguments_are_capped_with_an_ellipsis() {
    let long = "x".repeat(200);
    let summary = fold_summary(&long);
    assert!(
        summary.chars().count() <= MAX_SUMMARY_CHARS,
        "摘要必须 ≤ {MAX_SUMMARY_CHARS} 字，实际 {}",
        summary.chars().count()
    );
    assert!(summary.ends_with('…'), "超长摘要必须带省略号");
}

#[test]
fn short_arguments_are_kept_verbatim() {
    assert_eq!(fold_summary("src/main.rs"), "src/main.rs");
    assert_eq!(fold_summary("  cargo test  "), "cargo test");
}

#[test]
fn the_title_argument_is_capped_with_an_ellipsis() {
    let long = "a".repeat(200);
    let short = short_title_arg(&long);
    assert!(
        short.chars().count() <= MAX_TITLE_ARG_CHARS,
        "标题参数 ≤ {MAX_TITLE_ARG_CHARS} 字，实际 {}",
        short.chars().count()
    );
    assert!(short.ends_with('…'));
    assert_eq!(short_title_arg("src/main.rs"), "src/main.rs");
}

#[test]
fn the_folded_state_is_the_default() {
    let msg = call("Read", "src/main.rs");
    let card = ToolCard::from_message(&msg);
    assert!(card.folded, "工具调用卡片默认折叠");

    let ui = super::SessionUiState::default();
    let detail = tool_call_details(std::slice::from_ref(&msg))
        .into_iter()
        .next()
        .expect("工具调用消息应产出接线细节");
    assert!(ui.is_tool_folded(&detail.id), "界面态里没有记录时默认折叠");
}

#[test]
fn the_fold_state_is_remembered_by_message_id() {
    let messages = vec![call("Read", "a.rs"), call("Write", "b.rs")];
    let details = tool_call_details(&messages);
    assert_eq!(details.len(), 2);
    assert_ne!(details[0].id, details[1].id, "同屏 id 必须唯一");

    let mut ui = super::SessionUiState::default();
    ui.toggle_tool_fold(&details[0].id);
    assert!(!ui.is_tool_folded(&details[0].id), "展开后应为展开态");
    assert!(ui.is_tool_folded(&details[1].id), "另一张卡片不受影响");
    // 「重绘」= 再读一次同一份界面态：展开态不丢。
    assert!(!ui.is_tool_folded(&details[0].id));
}

#[test]
fn expanding_one_card_does_not_expand_others() {
    let messages = vec![call("Read", "a.rs"), call("Bash", "cargo test")];
    let details = tool_call_details(&messages);
    let mut ui = super::SessionUiState::default();
    ui.toggle_tool_fold(&details[1].id);
    assert!(!ui.is_tool_folded(&details[1].id));
    assert!(ui.is_tool_folded(&details[0].id), "反例：不该连带展开");
}

#[test]
fn the_expanded_state_shows_the_full_arguments() {
    let args = "y".repeat(300);
    let msg = call("Read", &args);
    let detail = tool_call_details(std::slice::from_ref(&msg))
        .into_iter()
        .next()
        .expect("工具调用消息应产出接线细节");
    let card = ToolCard::from_message(&msg);

    let folded = describe(&card, Some(&detail), true);
    assert!(
        folded.summary.chars().count() <= MAX_SUMMARY_CHARS,
        "折叠态只显示摘要"
    );
    assert!(folded.summary.ends_with('…'));

    let expanded = describe(&card, Some(&detail), false);
    assert_eq!(expanded.args, args, "展开态显示完整参数，不截断");
    assert_eq!(expanded.args.chars().count(), 300);
}

#[test]
fn the_message_id_is_stable_across_redraws() {
    let messages = vec![call("Read", "src/main.rs")];
    let first = tool_call_details(&messages)[0].id.clone();
    let second = tool_call_details(&messages)[0].id.clone();
    assert_eq!(first, second, "同一条消息每次渲染必须拿到同一个 id");
}

#[test]
fn the_message_id_does_not_change_when_earlier_messages_arrive() {
    let before = vec![call("Read", "src/main.rs")];
    let after = vec![user("帮我看看"), call("Read", "src/main.rs")];
    assert_eq!(
        tool_call_details(&before)[0].id,
        tool_call_details(&after)[0].id,
        "前面插入消息不得让已有卡片的 id 变掉（否则折叠态会丢）"
    );
}

#[test]
fn identical_tool_calls_still_get_distinct_ids() {
    let messages = vec![call("Read", "src/main.rs"), call("Read", "src/main.rs")];
    let details = tool_call_details(&messages);
    assert_eq!(details.len(), 2);
    assert_ne!(details[0].id, details[1].id, "同屏 id 必须唯一");
}

#[test]
fn every_tool_category_has_an_icon() {
    for tool in ["Read", "Write", "Edit", "Bash", "Grep"] {
        assert!(!tool_icon(Some(tool)).is_empty());
    }
    assert!(!tool_icon(None).is_empty(), "未知工具也要有兜底图标");
}

#[test]
fn the_category_word_is_human_never_english() {
    for (tool, _args, human) in FIVE {
        assert_eq!(category_of(Some(tool)), human);
        assert!(
            !category_of(Some(tool)).contains(tool),
            "类别词不得含工具英文名 {tool}"
        );
    }
    assert_eq!(category_of(None), "执行操作", "未知工具兜底为人话");
}
