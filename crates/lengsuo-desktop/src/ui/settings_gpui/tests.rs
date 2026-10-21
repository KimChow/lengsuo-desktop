//! 设置窗口的单元测试。
//!
//! 文件级 `#![cfg(test)]`、不包 `mod tests {}`（避免 clippy `module_inception`）；
//! 显式导入符号而**不用** `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的
//! `test` 属性宏带进来并遮蔽标准库的 `#[test]`（速查表 §12.3）。
//!
//! 这些测试只覆盖**描述层**（`view.rs` 不门控）⇒ 不开 `ui` feature 也跑得到。
#![cfg(test)]

use std::collections::HashSet;

use crate::theme::tokens::Theme as SemanticTheme;
use crate::ui::settings_gpui::view::{
    theme_label, PanelId, SettingsView, ThemeChoice, PANELS, THEME_CHOICES,
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
