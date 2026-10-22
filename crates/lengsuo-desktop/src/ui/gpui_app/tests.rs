//! 根视图的纯逻辑单元测试（不依赖窗口）。
//!
//! 文件级 `#![cfg(test)]`、不包 `mod tests {}`（避免 `module_inception`）；
//! 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
//! `test` 属性宏带进来并遮蔽标准库的 `#[test]`。
#![cfg(test)]

use crate::state::env_group::EnvGroupState;
use crate::state::hotkeys::HotkeyRegistry;
use crate::state::session_stream::{MessageRole, SessionMessage, SessionStreamState};
use crate::ui::input_area::{InputAreaState, InputMode, OutgoingInput};
use crate::ui::layout::{
    RightPanelLayout, RightPanelTab, WorkbenchLayout, RIGHT_PANEL_COLLAPSED_WIDTH,
    RIGHT_PANEL_DEFAULT_WIDTH, SIDEBAR_DEFAULT_WIDTH, STREAM_MIN_WIDTH,
};
use crate::ui::layout_gpui::{DividerDrag, DividerSide};
use serde_json::json;

use super::demo::{default_registry, demo_session, secondary_views};
use super::launch::{has_display_server, HOTKEY_HINT, NO_DISPLAY_MESSAGE, WINDOW_TITLE};
use super::settings_snapshot;
use super::sync::{echo_message, next_echo_seq, regions_for, should_finish_send, tab_strip_layout};

#[test]
fn demo_session_covers_four_roles_and_one_pending_approval() {
    let state = demo_session();
    let mut roles: Vec<MessageRole> = state.messages().iter().map(|m| m.role).collect();
    roles.sort_by_key(|role| format!("{role:?}"));
    roles.dedup();
    assert_eq!(roles.len(), 4, "演示数据必须覆盖四类消息");
    assert_eq!(state.pending_approvals().count(), 1);
}

/// 版式单一来源：渲染读到的宽度 = 分隔条持有的版式算出的宽度。
///
/// 拖拽改的是 [`DividerGpui`] 内部持有的那份 [`WorkbenchLayout`]；这里用与
/// 分隔条同一条拖拽路径（[`DividerDrag`]）改宽度，再经 [`regions_for`]
/// 读回 —— 根视图渲染走的就是这一条路径，中间没有任何副本。
#[test]
fn rendered_widths_come_from_the_divider_layout() {
    let mut divider_layout = WorkbenchLayout::new();
    let mut drag = DividerDrag::new();
    assert!(drag.begin(DividerSide::Left, 100.0, &divider_layout));
    assert!(drag.drag_to(150.0, &mut divider_layout));
    assert!(drag.end());

    let widths = regions_for(&divider_layout, 1280.0);
    let expected = divider_layout.regions(1280.0);
    assert_eq!(widths, expected);
    assert_eq!(widths.sidebar, SIDEBAR_DEFAULT_WIDTH + 50.0, "拖拽结果生效");
    assert_eq!(widths.right_panel, RIGHT_PANEL_DEFAULT_WIDTH);
    assert_eq!(widths.stream, 1280.0 - widths.sidebar - widths.right_panel);

    // 右侧分隔条拖拽同样从同一份版式读出。
    let mut drag = DividerDrag::new();
    assert!(drag.begin(DividerSide::Right, 900.0, &divider_layout));
    assert!(drag.drag_to(860.0, &mut divider_layout));
    let widths = regions_for(&divider_layout, 1280.0);
    assert_eq!(
        widths.right_panel,
        divider_layout.right_panel().effective_width()
    );
    assert_eq!(widths.right_panel, RIGHT_PANEL_DEFAULT_WIDTH + 40.0);
}

/// tab 切换映射：由版式里的 tab 派生的右栏布局，选中态与 tab 一致。
///
/// `tab_strip_layout` 是 `sync_layout` 用来把手写进 `RightPanelGpui` /
/// `RightPanelTabGpui` 的那个纯函数；这里断言它把选中 tab 原样带过去。
#[test]
fn tab_selection_drives_the_right_panel_layout() {
    for tab in RightPanelTab::ALL {
        let mut authority = WorkbenchLayout::new();
        authority.right_panel_mut().select_tab(tab);
        let panel = tab_strip_layout(&authority, &RightPanelLayout::new());
        assert_eq!(panel.tab(), tab, "右侧面板选中的 tab 必须与 tab 条一致");
        assert!(!panel.is_collapsed());

        let rendered = crate::ui::right_panel::render(&panel, &demo_session());
        assert_eq!(rendered.active_tab, tab);
        assert!(
            rendered
                .tabs
                .iter()
                .any(|button| button.tab == tab && button.selected),
            "tab 条上被选中的入口必须就是当前 tab"
        );
    }
}

/// tab 条回写时保留自己的宽度（宽度归分隔条管），但选中 / 折叠以左侧为准。
#[test]
fn tab_strip_layout_keeps_width_but_follows_the_authority() {
    let mut authority = WorkbenchLayout::new();
    authority
        .right_panel_mut()
        .select_tab(RightPanelTab::Preview);
    let current = RightPanelLayout::new();
    let synced = tab_strip_layout(&authority, &current);
    assert_eq!(synced.tab(), RightPanelTab::Preview);
    assert_eq!(synced.width(), current.width(), "宽度不由 tab 条决定");

    authority.right_panel_mut().collapse();
    let collapsed = tab_strip_layout(&authority, &current);
    assert!(collapsed.is_collapsed(), "折叠态同样以左侧为准");
}

/// fail-closed：描述层最小值下三区宽度都不为 0（分隔条最小宽度必须保留）。
#[test]
fn minimum_widths_never_collapse_a_region() {
    let mut layout = WorkbenchLayout::new();
    layout.set_sidebar_width(0.0);
    layout.right_panel_mut().set_width(0.0);
    let widths = regions_for(&layout, 800.0);
    assert!(widths.sidebar > 0.0);
    assert!(widths.stream >= STREAM_MIN_WIDTH);
    assert!(widths.right_panel > 0.0);

    layout.right_panel_mut().collapse();
    let collapsed = regions_for(&layout, 800.0);
    assert_eq!(collapsed.right_panel, RIGHT_PANEL_COLLAPSED_WIDTH);
    assert!(collapsed.stream >= STREAM_MIN_WIDTH);

    // 窗口宽度取不到（NaN / 负值 / 0）时也不得渲染成 0 宽。
    for viewport in [f32::NAN, -1.0, 0.0] {
        let widths = regions_for(&WorkbenchLayout::new(), viewport);
        assert!(widths.sidebar > 0.0, "viewport={viewport}");
        assert!(widths.stream > 0.0, "viewport={viewport}");
        assert!(widths.right_panel > 0.0, "viewport={viewport}");
    }
}

#[test]
fn root_registry_opens_palette_on_ctrl_k() {
    let registry = default_registry();
    let hotkey = crate::state::hotkeys::Hotkey::parse("Ctrl+K").expect("内置快捷键可解析");
    assert_eq!(
        registry.command_for(hotkey),
        Some(crate::state::hotkeys::command_ids::PALETTE_OPEN)
    );
}

#[test]
fn window_text_is_non_empty() {
    assert!(!WINDOW_TITLE.is_empty());
    assert!(!HOTKEY_HINT.is_empty());
    assert!(!NO_DISPLAY_MESSAGE.is_empty());
}

#[test]
fn headless_linux_has_no_display_server() {
    // 本机（47）没有显示服务器：预检必须判定为「无」，`launch()` 才能
    // 立刻返回错误而不是进事件循环卡死。
    if cfg!(any(target_os = "linux", target_os = "freebsd"))
        && std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
    {
        assert!(!has_display_server());
    }
    // 有窗口系统的目标平台一律视为有显示服务器。
    if !cfg!(any(target_os = "linux", target_os = "freebsd")) {
        assert!(has_display_server());
    }
}

#[test]
fn secondary_views_are_constructible() {
    let (_env, _org) = secondary_views();
}

// ---- e37：宿主把状态快照送进设置窗口 ----

/// 宿主交给设置窗口的快照必须**真的带着宿主的状态**。
///
/// 这条断言锁住规格 A8 点名的缺口：「`with_data` / `with_bindings` 无调用者
/// ⇒ 清单恒为空」——现在快照是唯一把宿主状态送进设置窗口的通道。
#[test]
fn settings_snapshot_carries_the_env_state_and_the_hotkeys() {
    let mut env = EnvGroupState::new();
    env.apply_entries(vec![json!({"id": "e1", "name": "构建机 A"})]);
    let registry = HotkeyRegistry::with_defaults().expect("内置表不该冲突");

    let snap = settings_snapshot(&env, &registry);

    assert_eq!(snap.data.raw_count(), 1, "环境快照必须带着宿主的环境条目");
    assert!(!snap.bindings.is_empty(), "快捷键快照必须带着内置绑定表");
}

/// 快照是**只读克隆**：宿主后续变化**不会**反映到已取好的快照
/// ⇒ 已打开的设置窗口不会被宿主改写。
#[test]
fn settings_snapshot_is_a_read_only_copy() {
    let mut env = EnvGroupState::new();
    env.apply_entries(vec![json!({"id": "e1", "name": "构建机 A"})]);
    let registry = HotkeyRegistry::with_defaults().expect("内置表不该冲突");

    let snap = settings_snapshot(&env, &registry);

    // 宿主侧后来又拉了一轮：条目从 1 条变成 2 条。
    env.apply_entries(vec![
        json!({"id": "e2", "name": "构建机 B"}),
        json!({"id": "e3", "name": "构建机 C"}),
    ]);

    assert_eq!(env.raw_count(), 2, "宿主侧确实变了");
    assert_eq!(snap.data.raw_count(), 1, "快照必须停在宿主取它那一刻的样子");
}

/// 一条已发出的输入（描述层 `submit` 的产物）。
fn outgoing(text: &str) -> OutgoingInput {
    OutgoingInput {
        text: text.to_owned(),
        mode: InputMode::Ask,
    }
}

/// 直接构造一条非用户消息（模拟「回应抵达」）。
fn reply(content: &str) -> SessionMessage {
    SessionMessage {
        role: MessageRole::Assistant,
        content: content.to_owned(),
        tool: None,
        ts_ms: 0,
    }
}

// ---- e14：发送 → 本地回显 ----

#[test]
fn echo_is_a_user_message_with_the_original_text() {
    let out = outgoing("  帮我看一下构建日志  ");
    let echo = echo_message(5, &out);
    assert_eq!(echo.role, MessageRole::User);
    assert_eq!(echo.content, "  帮我看一下构建日志  ", "原文逐字保留");
    assert_eq!(echo.tool, None, "用户消息不关联工具");
}

#[test]
fn two_consecutive_echoes_get_strictly_increasing_seq() {
    let mut state = demo_session();
    let before = state.last_seq();

    let first = echo_message(next_echo_seq(&state), &outgoing("第一条"));
    let seq1 = next_echo_seq(&state);
    assert!(state.push_message(seq1, first), "回显必须被会话流接受");

    let seq2 = next_echo_seq(&state);
    let second = echo_message(seq2, &outgoing("第二条"));
    assert!(state.push_message(seq2, second), "第二条回显也必须被接受");

    assert_eq!(state.last_seq(), before + 2, "连续两次回显 last_seq 加 2");
    assert!(seq2 > seq1, "seq 严格递增");
    assert!(!state.is_stale(), "回显不制造断档");
}

#[test]
fn echo_alone_does_not_finish_the_waiting_state() {
    let mut state = demo_session();
    let before = state.last_seq();
    let out = outgoing("本地回显");
    let seq = next_echo_seq(&state);
    assert!(state.push_message(seq, echo_message(seq, &out)));

    // 尚未发出：没有待确认序号 ⇒ 不解除。
    assert!(!should_finish_send(None, &state));
    // 已发出，但最新一条就是回显（User）⇒ 仍在等待运行时确认。
    assert!(!should_finish_send(Some(seq), &state));
    assert_eq!(state.last_seq(), before + 1);

    // 空会话同理：没有回应就不解除。
    assert!(!should_finish_send(Some(1), &SessionStreamState::new()));
}

#[test]
fn a_non_user_reply_finishes_the_waiting_state() {
    let mut state = demo_session();
    let seq = next_echo_seq(&state);
    assert!(state.push_message(seq, echo_message(seq, &outgoing("本地回显"))));
    assert!(!should_finish_send(Some(seq), &state), "回显本身不算回应");

    let reply_seq = next_echo_seq(&state);
    assert!(state.push_message(reply_seq, reply("收到，正在处理。")));
    assert!(
        should_finish_send(Some(seq), &state),
        "出现非 User 回应后应解除等待态"
    );
}

/// 空白输入不产生回显：复用描述层 `submit`（即 `can_send`）语义，不重新实现。
#[test]
fn blank_input_never_produces_an_echo() {
    let mut input = InputAreaState::new();
    assert!(input.submit().is_none(), "空草稿不产出");

    input.set_draft("  \n\t ");
    assert!(!input.can_send());
    assert!(input.submit().is_none(), "纯空白不产出");
    assert_eq!(input.draft(), "  \n\t ", "未发送则草稿保持原样");
    assert!(!input.is_in_flight(), "未发送不进入等待态");

    // 非空草稿：产出一次，且期间保持等待态（回显不清 in_flight）。
    input.set_draft("你好");
    let out = input.submit().expect("非空草稿应产出");
    assert_eq!(out.text, "你好");
    assert!(input.is_in_flight(), "回显后仍等待运行时确认");
}
