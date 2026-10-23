//! 附件 chip 的纯函数与状态（描述层）：文件名截取、插入序去重、增删。
//!
//! 对齐 A12 附件族（13 册 §3.2「粘贴图片显示缩略 chip」）：附件路径
//! **由调用方注入**（粘贴等运行时链路上报），本仓不读盘、不发请求；
//! 本层只提供 chip 视图所需的状态与派生。

use super::InputAreaState;

/// chip 标签：取路径末段文件名。
///
/// 三分支：带目录取末段；无 `/` 原样返回；空串返回空串。
#[must_use]
pub fn chip_label(path: &str) -> String {
    match path.rfind('/') {
        Some(idx) => path[idx + 1..].to_owned(),
        None => path.to_owned(),
    }
}

/// chip 顺序：保持插入序，重复路径只保留首次出现。
#[must_use]
pub fn chip_order(paths: &[String]) -> Vec<String> {
    let mut ordered = Vec::with_capacity(paths.len());
    for path in paths {
        if !ordered.contains(path) {
            ordered.push(path.clone());
        }
    }
    ordered
}

impl InputAreaState {
    /// 附件路径列表（只读）。
    #[must_use]
    pub fn attachments(&self) -> &[String] {
        &self.attachments
    }

    /// 设置附件列表：按 [`chip_order`] 语义去重保序。
    pub fn set_attachments(&mut self, paths: impl IntoIterator<Item = String>) {
        self.attachments = chip_order(&paths.into_iter().collect::<Vec<_>>());
    }

    /// 移除一个附件（chip 移除记号的唯一入口）；返回是否真的移除了。
    pub fn remove_attachment(&mut self, path: &str) -> bool {
        match self.attachments.iter().position(|p| p == path) {
            Some(idx) => {
                self.attachments.remove(idx);
                true
            }
            None => false,
        }
    }
}
