//! 设置窗口的单元测试。
//!
//! 文件级 `#![cfg(test)]`、不包 `mod tests {}`（避免 clippy `module_inception`）；
//! 显式导入符号而**不用** `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的
//! `test` 属性宏带进来并遮蔽标准库的 `#[test]`（速查表 §12.3）。
//!
//! 这些测试只覆盖**描述层**（`view.rs` 不门控）⇒ 不开 `ui` feature 也跑得到。
#![cfg(test)]

use std::collections::HashSet;

use crate::state::org_perm::{Capability, CapabilityFlags, DenialReason};
use crate::theme::tokens::Theme as SemanticTheme;
use crate::ui::settings_gpui::view::{
    capability_label, capability_rows, has_delivered_marks, theme_label, CapabilityStatus, PanelId,
    SettingsView, ThemeChoice, PANELS, THEME_CHOICES,
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
