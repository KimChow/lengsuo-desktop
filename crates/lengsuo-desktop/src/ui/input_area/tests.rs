//! 原 `input_area.rs` 的 `mod tests`，整体迁出；测试逐条保留。

// 显式导入（不用 `use super::*`）：保持测试只依赖公开契约。
use crate::ui::input_area::{InputAreaState, InputMode, SendButtonKind, StopIntent};

#[test]
fn all_five_modes_have_unique_ids() {
    assert_eq!(InputMode::ALL.len(), 5);
    let mut ids: Vec<&str> = InputMode::ALL.iter().map(|m| m.id()).collect();
    assert_eq!(ids, ["quick", "ask", "plan", "craft", "expert"]);
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len());
    for m in InputMode::ALL {
        assert_eq!(InputMode::from_id(m.id()), Some(m));
    }
    assert_eq!(InputMode::from_id("turbo"), None);
}

#[test]
fn empty_input_cannot_be_sent() {
    let mut area = InputAreaState::new();
    assert!(!area.can_send());
    assert!(!area.send_button().enabled);
    assert_eq!(area.submit(), None);
    area.set_draft("   \n\t ");
    assert!(!area.can_send(), "纯空白也不得发送");
}

#[test]
fn multiline_draft_is_preserved_and_sent_with_mode() {
    let mut area = InputAreaState::new();
    area.set_mode(InputMode::Plan);
    area.set_draft("第一行\n第二行\n");
    assert_eq!(area.line_count(), 2);
    assert!(area.send_button().enabled);
    let out = area.submit().expect("应可发送");
    assert_eq!(out.text, "第一行\n第二行");
    assert_eq!(out.mode, InputMode::Plan);
    assert_eq!(area.draft(), "");
    assert!(area.is_in_flight());
    assert!(!area.can_send(), "等待确认期间不得重复发送");
    assert_eq!(area.submit(), None);
    area.finish_send();
    assert_eq!(area.send_button().label, "发送");
}

#[test]
fn set_mode_id_rejects_unknown_without_changing_mode() {
    let mut area = InputAreaState::new();
    assert_eq!(area.mode(), InputMode::Ask);
    assert!(!area.set_mode_id("nope"));
    assert_eq!(area.mode(), InputMode::Ask, "未知模式不得静默切换");
    assert!(area.set_mode_id("expert"));
    assert_eq!(area.mode(), InputMode::Expert);
}

#[test]
fn mode_is_remembered_across_sends() {
    let mut area = InputAreaState::new();
    area.set_mode(InputMode::Expert);
    area.set_draft("第一问");
    let first = area.submit().expect("应可发送");
    area.finish_send();
    area.set_draft("第二问");
    let second = area.submit().expect("应可发送");
    assert_eq!(first.mode, InputMode::Expert);
    assert_eq!(second.mode, InputMode::Expert, "模式在会话内必须保持");
    assert_eq!(area.mode(), InputMode::Expert);
}

// ==== A5：发送后变停止按钮 ====
#[test]
fn send_button_turns_into_stop_while_waiting_for_ack() {
    let mut area = InputAreaState::new();
    area.set_draft("你好");
    let send = area.send_button();
    assert_eq!(send.kind, SendButtonKind::Send);
    assert_eq!(send.label, "发送");
    assert!(send.enabled);

    area.submit().expect("应可发送");
    let stop = area.send_button();
    assert_eq!(stop.kind, SendButtonKind::Stop, "等待确认期间应为停止按钮");
    assert_eq!(stop.label, "停止");
    assert!(stop.enabled, "停止按钮始终可点");
}

#[test]
fn take_stop_yields_intent_and_restores_send_button() {
    let mut area = InputAreaState::new();
    area.set_mode(InputMode::Craft);
    area.set_draft("干活");
    area.submit().expect("应可发送");

    let intent = area.take_stop().expect("等待态应产出停止意图");
    assert_eq!(
        intent,
        StopIntent {
            mode: InputMode::Craft
        }
    );
    assert!(!area.is_in_flight(), "产出停止意图后解除等待态");
    let send = area.send_button();
    assert_eq!(send.kind, SendButtonKind::Send);
    assert_eq!(send.label, "发送");
    assert!(!send.enabled, "草稿已清空，发送仍不可用");
}

#[test]
fn take_stop_returns_none_when_not_waiting() {
    let mut area = InputAreaState::new();
    assert!(area.take_stop().is_none(), "非等待态不得产出停止意图");
    area.set_draft("你好");
    area.submit().expect("应可发送");
    area.take_stop().expect("第一次应产出");
    assert!(area.take_stop().is_none(), "只能产出一次");
}

// ==== A12：@ 文件引用 ====

#[test]
fn set_candidates_dedupes_and_drops_blanks() {
    let mut area = InputAreaState::new();
    area.set_candidates([
        "src/main.rs".to_owned(),
        "  src/main.rs  ".to_owned(),
        "   ".to_owned(),
        "src/lib.rs".to_owned(),
    ]);
    assert_eq!(
        area.candidates(),
        ["src/main.rs".to_owned(), "src/lib.rs".to_owned()]
    );
}

#[test]
fn filter_candidates_is_case_insensitive_and_empty_query_returns_all() {
    let mut area = InputAreaState::new();
    area.set_candidates([
        "src/Main.rs".to_owned(),
        "docs/readme.md".to_owned(),
        "src/lib.rs".to_owned(),
    ]);
    assert_eq!(area.filter_candidates("").len(), 3, "空查询返回全部");
    assert_eq!(
        area.filter_candidates("SRC"),
        ["src/Main.rs".to_owned(), "src/lib.rs".to_owned()],
        "前缀匹配且大小写不敏感"
    );
    assert_eq!(
        area.filter_candidates("readme"),
        ["docs/readme.md".to_owned()]
    );
    assert!(area.filter_candidates("nope").is_empty());
}

#[test]
fn mention_query_parses_word_after_at_sign() {
    let mut area = InputAreaState::new();
    area.set_draft("看下 @src/mai");
    assert_eq!(area.mention_query().as_deref(), Some("src/mai"));
    area.set_draft("看下 @src/main.rs 谢谢");
    assert_eq!(area.mention_query(), None, "@ 后含空白就不是 @ 查询");
    area.set_draft("没有 @ 符号");
    assert_eq!(area.mention_query(), None);
}

#[test]
fn mention_query_ignores_at_sign_inside_a_word() {
    let mut area = InputAreaState::new();
    area.set_draft("someone@example.com");
    assert_eq!(area.mention_query(), None, "邮箱里的 @ 不是引用标记");
    area.set_draft("@src/main.rs");
    assert_eq!(area.mention_query().as_deref(), Some("src/main.rs"));
}

#[test]
fn insert_mention_replaces_query_with_path() {
    let mut area = InputAreaState::new();
    area.set_draft("看下 @src/mai");
    area.insert_mention("src/main.rs");
    assert_eq!(area.draft(), "看下 @src/main.rs ");
    assert_eq!(area.mention_query(), None, "插入后不应再是 @ 查询");
}

#[test]
fn insert_mention_is_a_noop_without_at_query() {
    let mut area = InputAreaState::new();
    area.set_draft("普通输入");
    area.insert_mention("src/main.rs");
    assert_eq!(area.draft(), "普通输入", "没有 @ 查询时不得改动草稿");
}
