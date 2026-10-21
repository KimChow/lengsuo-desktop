//! 设置窗口的单元测试。
//!
//! 文件级 `#![cfg(test)]`、不包 `mod tests {}`（避免 clippy `module_inception`）；
//! 显式导入符号而**不用** `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的
//! `test` 属性宏带进来并遮蔽标准库的 `#[test]`（速查表 §12.3）。
//!
//! 这些测试只覆盖**描述层**（`view.rs` 不门控）⇒ 不开 `ui` feature 也跑得到。
#![cfg(test)]

use std::collections::HashSet;

use crate::state::hotkeys::{Binding, Hotkey, Key, Modifiers, DEFAULT_BINDINGS};
use crate::state::org_perm::{Capability, CapabilityFlags, DenialReason};
use crate::theme::tokens::Theme as SemanticTheme;
use crate::ui::settings_gpui::view::{
    capability_label, capability_rows, command_label, format_hotkey, has_delivered_marks,
    hotkey_panel, theme_label, CapabilityStatus, HotkeyPanel, PanelId, SettingsView, ThemeChoice,
    EMPTY_BINDINGS_TEXT, PANELS, THEME_CHOICES, UNKNOWN_COMMAND_LABEL,
};

#[test]
fn panels_are_exactly_fifteen_and_ordered() {
    assert_eq!(PANELS.len(), 15, "设置窗口必须是 15 个面板");
    let ids: HashSet<PanelId> = PANELS.iter().map(|panel| panel.id).collect();
    assert_eq!(ids.len(), 15, "面板 id 互不重复");
    // 顺序即展示顺序：账号 → 模型 → 外观 …… 结尾是 MCP 授权同意。
    assert_eq!(PANELS.first().map(|panel| panel.id), Some(PanelId::Account));
    assert_eq!(
        PANELS.last().map(|panel| panel.id),
        Some(PanelId::McpConsent)
    );
    assert_eq!(
        PANELS[2].id,
        PanelId::Appearance,
        "第 3 项必须是外观（主题）"
    );
    // 标题不能为空。
    assert!(
        PANELS.iter().all(|panel| !panel.title.trim().is_empty()),
        "面板标题不得为空"
    );
}

#[test]
fn default_selection_is_first_panel() {
    let view = SettingsView::default();
    assert_eq!(view.selected, PanelId::Account);
    assert_eq!(view.current_title(), "账号");
    assert_eq!(view.panels.len(), 15, "默认视图带完整面板清单");
}

#[test]
fn theme_choices_cover_system_light_dark() {
    assert_eq!(THEME_CHOICES.len(), 3, "主题必须三档");
    assert!(THEME_CHOICES.contains(&ThemeChoice::System));
    assert!(THEME_CHOICES.contains(&ThemeChoice::Light));
    assert!(THEME_CHOICES.contains(&ThemeChoice::Dark));
    // 互不重复。
    let unique: HashSet<ThemeChoice> = THEME_CHOICES.iter().copied().collect();
    assert_eq!(unique.len(), 3);
}

#[test]
fn theme_labels_are_distinct() {
    let labels: Vec<&str> = THEME_CHOICES.iter().copied().map(theme_label).collect();
    assert!(labels.iter().all(|label| !label.trim().is_empty()));
    let unique: HashSet<&str> = labels.iter().copied().collect();
    assert_eq!(unique.len(), 3, "三个主题档位的文案必须互不相同");
}

#[test]
fn theme_choice_maps_to_the_semantic_theme_enum() {
    // 语义层（`tokens::Theme`）与描述层的取值一一对应。
    assert_eq!(ThemeChoice::System.semantic(), SemanticTheme::System);
    assert_eq!(ThemeChoice::Light.semantic(), SemanticTheme::Light);
    assert_eq!(ThemeChoice::Dark.semantic(), SemanticTheme::Dark);
    // 语义层的 `System` 按约定回退到 Light（与 `tokens::Theme::resolved()` 同口径）。
    assert_eq!(
        ThemeChoice::System.semantic().resolved(),
        SemanticTheme::Light
    );
}

#[test]
fn select_and_set_theme_report_whether_anything_changed() {
    let mut view = SettingsView::default();
    // 选到同一个面板：没有变化 ⇒ 不必重绘。
    assert!(!view.select(PanelId::Account));
    assert!(view.select(PanelId::Appearance));
    assert_eq!(view.selected, PanelId::Appearance);
    assert_eq!(view.current_title(), "外观");

    // 主题同理。
    assert!(!view.set_theme(ThemeChoice::System), "默认就是跟随系统");
    assert!(view.set_theme(ThemeChoice::Dark));
    assert_eq!(view.theme, ThemeChoice::Dark);
    assert!(!view.set_theme(ThemeChoice::Dark));
}

#[test]
fn panel_slugs_are_unique_and_stable() {
    let slugs: HashSet<&str> = PANELS.iter().map(|panel| panel.id.slug()).collect();
    assert_eq!(
        slugs.len(),
        15,
        "面板 slug 必须互不重复（GPUI 元素 id 靠它唯一）"
    );
    // slug 用小写连字符，天然适合做 id 的一部分。
    assert!(slugs.iter().all(|slug| slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')));
}

#[test]
fn capability_labels_cover_all_nine_capabilities() {
    assert_eq!(Capability::ALL.len(), 9, "能力位必须正好 9 个");
    for cap in Capability::ALL {
        let label = capability_label(cap);
        assert!(!label.trim().is_empty(), "{cap:?} 缺中文名");
        // 中文名不得直接抄服务端字段名（`can_view_members` 之类）。
        assert!(
            !label.starts_with("can_"),
            "{cap:?} 的中文名不该是服务端字段名"
        );
    }
    // 文案互不相同：9 个能力不能共用一个名字。
    let unique: HashSet<&str> = Capability::ALL
        .iter()
        .copied()
        .map(capability_label)
        .collect();
    assert_eq!(unique.len(), 9, "9 个能力位的中文名必须互不相同");
}

#[test]
fn empty_flags_mean_not_delivered_not_denied() {
    // 空标记集合 ⇒ 服务端**什么都没说** ⇒ 一律「未下发」。
    let flags = CapabilityFlags::default();
    assert!(!has_delivered_marks(&flags), "空集合不得被当成已下发");

    let rows = capability_rows(&flags);
    assert_eq!(rows.len(), 9, "9 个能力位都要列出来");
    for row in &rows {
        assert_eq!(
            row.status,
            CapabilityStatus::NotDelivered,
            "{} 必须显示「未下发」而不是「不可用」",
            row.label
        );
        assert_eq!(row.status.text(), "服务端未下发");
    }
}

#[test]
fn delivered_flags_distinguish_available_denied_and_missing() {
    let flags = CapabilityFlags::from_payload(&serde_json::json!({
        "can_view_members": true,
        "can_delete_role": false,
        "deny_reasons": {"can_delete_role": "NO_PERMISSION"}
    }));

    assert!(has_delivered_marks(&flags), "下发了标记");
    let rows = capability_rows(&flags);
    assert_eq!(rows.len(), 9);

    let status_of = |cap: Capability| {
        rows.iter()
            .find(|row| row.capability == cap)
            .map(|row| row.status)
            .expect("9 个能力位都必须列全")
    };

    assert_eq!(
        status_of(Capability::ViewMembers),
        CapabilityStatus::Available
    );
    assert_eq!(
        status_of(Capability::DeleteRole),
        CapabilityStatus::Denied(Some(DenialReason::NoPermission))
    );
    // 服务端没下发的那个 ⇒ 未下发（**不是**不可用）。
    assert_eq!(
        status_of(Capability::EditRole),
        CapabilityStatus::NotDelivered
    );

    // 文案必须能区分开：可用 / 被拒（有原因）/ 未下发。
    assert_eq!(CapabilityStatus::Available.text(), "可用");
    assert_eq!(
        CapabilityStatus::Denied(Some(DenialReason::NoPermission)).text(),
        "不可用：你没有该操作权限"
    );
    assert_eq!(
        CapabilityStatus::Denied(Some(DenialReason::NoEntitlement)).text(),
        "不可用：当前套餐不包含此功能"
    );
    assert_eq!(
        CapabilityStatus::Denied(Some(DenialReason::PolicyDenied)).text(),
        "不可用：该操作已被企业策略禁止"
    );
    assert_eq!(CapabilityStatus::NotDelivered.text(), "服务端未下发");
}

#[test]
fn denied_without_server_reason_does_not_fabricate_one() {
    // 服务端只给了 `false`，没给原因码 ⇒ 不编造原因，但仍不是「未下发」。
    let flags = CapabilityFlags::from_payload(&serde_json::json!({"can_edit_role": false}));
    let rows = capability_rows(&flags);
    let row = rows
        .iter()
        .find(|row| row.capability == Capability::EditRole)
        .expect("编辑角色必须在清单里");
    assert_eq!(row.status, CapabilityStatus::Denied(None));
    assert_eq!(row.status.text(), "不可用");
}

#[test]
fn the_ui_never_infers_permissions_from_roles_or_entitlement() {
    // 载荷自称管理员 / 有权益，但**没有** `can_*` ⇒ 一律「未下发」，不可用。
    let flags = CapabilityFlags::from_payload(&serde_json::json!({
        "roles": ["admin"],
        "is_admin": true,
        "entitled": true
    }));
    for row in capability_rows(&flags) {
        assert_eq!(row.status, CapabilityStatus::NotDelivered);
    }
}

#[test]
fn hotkey_formats_single_and_double_modifiers() {
    // 单修饰：Ctrl+K。
    assert_eq!(
        format_hotkey(&Hotkey::new(Modifiers::CTRL, Key::Character('k'))),
        "Ctrl+K"
    );
    // 双修饰：Ctrl+Shift+P。
    assert_eq!(
        format_hotkey(&Hotkey::new(Modifiers::CTRL_SHIFT, Key::Character('p'))),
        "Ctrl+Shift+P"
    );
}

#[test]
fn hotkey_without_modifiers_has_no_stray_plus() {
    // 无修饰键 ⇒ 只输出键名，**不**出现多余的 `+`。
    assert_eq!(format_hotkey(&Hotkey::plain(Key::Escape)), "Escape");
    assert_eq!(format_hotkey(&Hotkey::plain(Key::Enter)), "Enter");
    assert!(!format_hotkey(&Hotkey::plain(Key::Escape)).starts_with('+'));
}

#[test]
fn hotkey_format_handles_non_ascii_keys_without_panicking() {
    // 非 ASCII 键：不能 panic，必须给出可展示的文本。
    let text = format_hotkey(&Hotkey::plain(Key::Character('设')));
    assert!(!text.is_empty(), "非 ASCII 键也必须格式化成非空文本");
    // 组合键同理。
    let text = format_hotkey(&Hotkey::new(Modifiers::CTRL, Key::Character('设')));
    assert!(
        text.contains('设'),
        "非 ASCII 键的字符必须出现在结果里：{text}"
    );
}

#[test]
fn hotkey_format_handles_named_and_function_keys() {
    assert_eq!(format_hotkey(&Hotkey::plain(Key::Space)), "Space");
    assert_eq!(format_hotkey(&Hotkey::plain(Key::PageUp)), "PageUp");
    assert_eq!(format_hotkey(&Hotkey::plain(Key::Function(5))), "F5");
}

#[test]
fn every_default_binding_has_a_chinese_label() {
    // 「不许漏项」的可机器判定形式：内置绑定**每一条**都要有非空中文说明，
    // 且不得落到「未知命令」兜底上。
    assert!(!DEFAULT_BINDINGS.is_empty(), "内置绑定表不得为空");
    for binding in DEFAULT_BINDINGS {
        let label = command_label(binding.command_id);
        assert!(
            !label.trim().is_empty(),
            "{} 缺中文说明",
            binding.command_id
        );
        assert!(
            !label.is_ascii(),
            "{} 的说明必须是中文，不能是英文 id：{label}",
            binding.command_id
        );
        assert_ne!(
            label, UNKNOWN_COMMAND_LABEL,
            "{} 没有中文说明（落到了未知命令兜底）",
            binding.command_id
        );
    }
}

#[test]
fn command_label_falls_back_without_fabricating() {
    // 未知命令 id ⇒ 明确的兜底文案，**不**返回空串、**不**编造说明。
    assert_eq!(command_label("no.such.command"), UNKNOWN_COMMAND_LABEL);
    // 命令面板里已知的命令复用它自己的标题。
    assert_eq!(command_label("session.new"), "新建会话");
}

#[test]
fn empty_binding_table_means_no_shortcuts_not_a_blank_table() {
    // fail-closed：绑定表为空 ⇒ 明确说「没有可用的快捷键」，
    // 不得渲染成空表或「一切正常」。
    match hotkey_panel(&[]) {
        HotkeyPanel::Empty => {}
        other => panic!("空绑定表必须判定为 Empty：{other:?}"),
    }
    assert_eq!(EMPTY_BINDINGS_TEXT, "没有可用的快捷键");
}

#[test]
fn hotkey_panel_lists_every_binding_without_merging() {
    // `Ctrl+K` 与 `Ctrl+Shift+P` 指向**同一个**命令，但它们是两条绑定 ⇒
    // 都得列出来，不得去重合并。
    match hotkey_panel(DEFAULT_BINDINGS) {
        HotkeyPanel::Rows(rows) => {
            assert_eq!(rows.len(), DEFAULT_BINDINGS.len());
            let palette_rows = rows
                .iter()
                .filter(|row| row.command_id == "palette.open")
                .count();
            assert_eq!(palette_rows, 2, "同一命令的两个键都要列出");
            // 每行的按键文本都不能为空。
            assert!(rows.iter().all(|row| !row.keys.trim().is_empty()));
            // 行里的按键文本必须与绑定本身对得上。
            for (row, binding) in rows.iter().zip(DEFAULT_BINDINGS) {
                assert_eq!(row.keys, format_hotkey(&binding.hotkey));
            }
        }
        other => panic!("非空绑定表必须判定为 Rows：{other:?}"),
    }
}

#[test]
fn hotkey_panel_keeps_binding_order() {
    // 行顺序 = 绑定表顺序（注册表的添加顺序），不排序、不去重。
    let rows = match hotkey_panel(DEFAULT_BINDINGS) {
        HotkeyPanel::Rows(rows) => rows,
        other => panic!("非空绑定表必须判定为 Rows：{other:?}"),
    };
    for (row, binding) in rows.iter().zip(DEFAULT_BINDINGS) {
        assert_eq!(row.command_id, binding.command_id);
    }
}

#[test]
fn hotkey_panel_row_carries_a_chinese_label() {
    let rows = match hotkey_panel(DEFAULT_BINDINGS) {
        HotkeyPanel::Rows(rows) => rows,
        other => panic!("非空绑定表必须判定为 Rows：{other:?}"),
    };
    for row in &rows {
        assert!(
            !row.label.trim().is_empty(),
            "{} 缺中文说明",
            row.command_id
        );
        assert!(
            !row.label.is_ascii(),
            "{} 的说明必须是中文：{}",
            row.command_id,
            row.label
        );
        assert_ne!(row.label, UNKNOWN_COMMAND_LABEL);
    }
}

#[test]
fn a_single_binding_still_renders_as_a_row_not_as_empty() {
    // 「筛选无结果」与「真实为空」要分开：一条绑定 ⇒ 一行，不是空态。
    let binding = Binding {
        hotkey: Hotkey::new(Modifiers::CTRL, Key::Character('s')),
        command_id: "file.save",
    };
    match hotkey_panel(&[binding]) {
        HotkeyPanel::Rows(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].keys, "Ctrl+S");
            assert_eq!(rows[0].label, "保存文件");
        }
        other => panic!("一条绑定必须渲染成行：{other:?}"),
    }
}
