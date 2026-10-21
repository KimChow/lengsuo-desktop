//! 「点文件 → 跳预览」的**跳转意图**（纯逻辑，无 GPUI、无 IO、**不落盘**）。
//!
//! 对齐基准：`docs/ui-parity-spec.md` 的 **A4**（点文件跳预览）。
//!
//! 分层约定：
//! - **描述层**（本文件）：只描述「要打开哪个文件、从哪儿点的」，纯数据，无窗口可测。
//! - **接线层**（`ui/session_gpui/file_card.rs`）：把点击变成一次意图。
//!
//! 硬约束：
//! - ⛔ **本仓不实现预览器**（预览器套件属 P3，见 `ui-parity-spec.md` A14）：
//!   点击只**产出意图**，不读文件、不打开窗口、不落盘。
//! - ⛔ **不累积**：点新的文件 ⇒ 意图**替换**旧的（界面上只留最近一次「要跳哪儿」）。
//! - fail-closed：路径为空 / 空白 ⇒ **不产出**意图（宁可什么都不跳，也不跳到空路径）。

/// 点击来源（点了会话流里的哪一处）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileIntentSource {
    /// 会话流里的**文件卡片**（文件网格）。
    FileCard,
    /// 工具结果里的**文件列表行**。
    FileList,
}

/// 一次「点文件 → 跳预览」的跳转意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIntent {
    /// 目标文件路径。
    pub path: String,
    /// 点击来源。
    pub source: FileIntentSource,
}

impl FileIntent {
    /// 构造一次跳转意图（路径原样保留，不做路径规范化）。
    #[must_use]
    pub fn new(path: &str, source: FileIntentSource) -> Self {
        Self {
            path: path.to_owned(),
            source,
        }
    }
}

/// 跳转意图的**状态位**：只留**最近一次**意图。
///
/// ⛔ 不累积：点第二个文件 ⇒ 第一个的意图被**替换**掉（界面上只保留「现在要跳哪儿」）。
#[derive(Debug, Clone, Default)]
pub struct FileIntentState {
    current: Option<FileIntent>,
}

impl FileIntentState {
    /// 新建（默认**没有**任何意图 —— 默认态干净）。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前意图（没点过时是 `None`）。
    #[must_use]
    pub fn intent(&self) -> Option<&FileIntent> {
        self.current.as_ref()
    }

    /// 当前意图的目标路径（没有则 `None`）。
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.current.as_ref().map(|intent| intent.path.as_str())
    }

    /// 记下一次跳转意图：**替换**旧意图（⛔ 不累积）。
    ///
    /// fail-closed：路径为空 / 空白 ⇒ **不产出**意图，也不清掉旧意图
    /// （空路径不是一次有效的点击）。
    pub fn request(&mut self, path: &str, source: FileIntentSource) {
        if path.trim().is_empty() {
            return;
        }
        self.current = Some(FileIntent::new(path, source));
    }

    /// 清掉意图（宿主在预览器消费后调用；本卡不消费）。
    pub fn clear(&mut self) {
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块可能通配导入 GPUI，会把 GPUI 自己的
    // `test` 属性宏带进作用域并遮蔽标准库的 `#[test]`。
    use super::{FileIntent, FileIntentSource, FileIntentState};

    #[test]
    fn the_default_state_has_no_intent() {
        let state = FileIntentState::new();
        assert_eq!(state.intent(), None, "默认态必须干净");
        assert_eq!(state.path(), None);
    }

    #[test]
    fn a_request_produces_an_intent() {
        let mut state = FileIntentState::new();
        state.request("src/main.rs", FileIntentSource::FileCard);

        let intent = state.intent().expect("点过文件就该有跳转意图");
        assert_eq!(intent.path, "src/main.rs");
        assert_eq!(intent.source, FileIntentSource::FileCard);
        assert_eq!(state.path(), Some("src/main.rs"));
    }

    #[test]
    fn a_new_request_replaces_the_old_intent_instead_of_accumulating() {
        let mut state = FileIntentState::new();
        state.request("a.rs", FileIntentSource::FileCard);
        state.request("b.rs", FileIntentSource::FileList);

        assert_eq!(state.path(), Some("b.rs"), "意图应更新为新文件");
        assert_eq!(
            state.intent().map(|i| i.source),
            Some(FileIntentSource::FileList)
        );
        assert_eq!(
            state.intent(),
            Some(&FileIntent::new("b.rs", FileIntentSource::FileList)),
            "只留最近一次意图，不累积"
        );
    }

    #[test]
    fn a_blank_path_produces_no_intent() {
        let mut state = FileIntentState::new();
        state.request("   ", FileIntentSource::FileCard);
        assert_eq!(state.intent(), None, "空路径不得产出跳转意图");
    }

    #[test]
    fn clearing_removes_the_intent() {
        let mut state = FileIntentState::new();
        state.request("a.rs", FileIntentSource::FileCard);
        state.clear();
        assert_eq!(state.intent(), None);
    }
}
