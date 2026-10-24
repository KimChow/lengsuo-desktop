//! 产物槽浮层（波次 7，卡 `ui-overlay-artifact-slot`，对齐 ui-parity-spec §9 A46）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `artifactSlot`。
//! 数据源在运行时侧：[`ArtifactSlotState::initial`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "产物槽";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空态提示（已下发但零条目）。
pub const NO_ARTIFACTS: &str = "暂无数据";
/// 产物计数头模板（占位符 `{count}`）。
pub const TITLE_COUNT: &str = "任务产生制品（{count}个）：";
/// 文件变更小节头模板（占位符 `{count}`）。
pub const FILES_TITLE: &str = "文件变更（{count}个）：";
/// 主操作：打开网页预览。
pub const OPEN_PREVIEW: &str = "打开网页预览 →";
/// 次操作：在浏览器中打开。
pub const OPEN_IN_BROWSER: &str = "在浏览器中预览";
/// 「查看所有产物」入口模板（占位符 `{count}`）。
pub const VIEW_ALL: &str = "查看所有产物 ({count})";
/// 来源面板标题模板（占位符 `{count}`）。
pub const SOURCES_TITLE: &str = "引用来源 ({count})";
/// 来源面板标签。
pub const SOURCES_LABEL: &str = "来源";
/// 来源计数模板（占位符 `{count}`）。
pub const SOURCES_COUNT_LABEL: &str = "{count}个来源";
/// 来源面板说明模板（占位符 `{count}`）。
pub const SOURCES_TOOLTIP: &str = "引用 {count} 篇资料作为参考";
/// 来源面板关闭按钮。
pub const SOURCES_CLOSE: &str = "关闭";

/// 已下发的产物槽快照（纯数据；Ⓐ 不持凭据、不判权限）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactSlotData {
    /// 文件变更条目（名称即展示行）。
    pub file_changes: Vec<String>,
    /// 网页预览地址（`None` ⇒ 不渲染预览行）。
    pub preview_url: Option<String>,
    /// 引用来源条目（名称即展示行）。
    pub sources: Vec<String>,
}

/// 浮层可枚举状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactSlotState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空集）。
    NotDelivered,
    /// 数据已下发的正常态。
    Ready(ArtifactSlotData),
}

/// 把模板里的 `{count}` 占位符替换成数值。
fn fill_count(tmpl: &str, count: usize) -> String {
    tmpl.replace("{count}", &count.to_string())
}

impl ArtifactSlotData {
    /// 产物总数 = 文件变更条数 + 网页预览（各计 1）。
    pub fn artifact_count(&self) -> usize {
        self.file_changes.len() + usize::from(self.preview_url.is_some())
    }

    /// 是否零产物且零来源（空输入 ⇒ 空态，不是空列表）。
    pub fn is_empty(&self) -> bool {
        self.artifact_count() == 0 && self.sources.is_empty()
    }

    /// 「任务产生制品（{count}个）：」。
    pub fn title_text(&self) -> String {
        fill_count(TITLE_COUNT, self.artifact_count())
    }

    /// 「文件变更（{count}个）：」。
    pub fn files_title_text(&self) -> String {
        fill_count(FILES_TITLE, self.file_changes.len())
    }

    /// 「查看所有产物 ({count})」。
    pub fn view_all_text(&self) -> String {
        fill_count(VIEW_ALL, self.artifact_count())
    }

    /// 「引用来源 ({count})」。
    pub fn sources_title_text(&self) -> String {
        fill_count(SOURCES_TITLE, self.sources.len())
    }

    /// 「{count}个来源」。
    pub fn sources_count_text(&self) -> String {
        fill_count(SOURCES_COUNT_LABEL, self.sources.len())
    }

    /// 「引用 {count} 篇资料作为参考」。
    pub fn sources_tooltip_text(&self) -> String {
        fill_count(SOURCES_TOOLTIP, self.sources.len())
    }
}

impl ArtifactSlotState {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self::NotDelivered
    }

    /// 是否处于未下发态。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self, Self::NotDelivered)
    }

    /// 内容区居中文本：未下发 / 空态；有条目时返回 `None`（改渲染条目行）。
    pub fn body_text(&self) -> Option<String> {
        match self {
            Self::NotDelivered => Some(NOT_DELIVERED.to_string()),
            Self::Ready(data) if data.is_empty() => Some(NO_ARTIFACTS.to_string()),
            Self::Ready(_) => None,
        }
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}
#[cfg(test)]
mod tests {
    use super::{
        fill_count, main_area, ArtifactSlotData, ArtifactSlotState, FILES_TITLE, NOT_DELIVERED,
        NO_ARTIFACTS, TITLE, TITLE_COUNT, VIEW_ALL,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }
    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
    #[test]
    fn count_templates_fill_placeholder() {
        assert_eq!(fill_count(TITLE_COUNT, 3), "任务产生制品（3个）：");
        assert_eq!(fill_count(FILES_TITLE, 2), "文件变更（2个）：");
        assert_eq!(fill_count(VIEW_ALL, 3), "查看所有产物 (3)");
    }
    #[test]
    fn counts_come_from_data_not_hardcoded() {
        let data = ArtifactSlotData {
            file_changes: vec!["a.rs".to_string(), "b.md".to_string()],
            preview_url: Some("https://example.invalid/preview".to_string()),
            sources: vec!["来源一".to_string(), "来源二".to_string(), "来源三".to_string()],
        };
        assert_eq!(data.artifact_count(), 3);
        assert!(!data.is_empty());
        assert_eq!(data.title_text(), "任务产生制品（3个）：");
        assert_eq!(data.files_title_text(), "文件变更（2个）：");
        assert_eq!(data.view_all_text(), "查看所有产物 (3)");
        let no_preview = ArtifactSlotData {
            preview_url: None,
            ..data
        };
        assert_eq!(no_preview.artifact_count(), 2);
    }
    #[test]
    fn empty_input_is_empty_state_not_blank_list() {
        let state = ArtifactSlotState::Ready(ArtifactSlotData::default());
        assert!(ArtifactSlotData::default().is_empty());
        assert_eq!(state.body_text().as_deref(), Some(NO_ARTIFACTS));
        assert_eq!(NO_ARTIFACTS, "暂无数据");
    }
}
