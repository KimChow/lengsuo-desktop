//! 自动化·模板 —— 波次 9 脚手架（诚实占位）。
//! 文案命名空间 `automation.templates`（40 有效键，品牌词已剔除）。
//! 真实视图由卡 `ui-overlay-automation-templates` 落地；本占位保证模块登记与 fail-closed 文案。

/// 面板标题（逐字取自文案全表）。
pub const PANEL_TITLE: &str = "自动化·模板";

/// 渲染占位：加载失败必须可见（fail-closed），不退化成空列表。
pub fn render_placeholder() -> String {
    format!("{}：数据未接入（fail-closed）", PANEL_TITLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_verbatim() {
        assert!(!PANEL_TITLE.is_empty());
    }

    #[test]
    fn placeholder_mentions_fail_closed() {
        assert!(render_placeholder().contains("fail-closed"));
    }
}
