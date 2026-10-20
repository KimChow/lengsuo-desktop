//! GPUI 渲染层（阶段二，`feature = "ui"`）。
//!
//! 阶段一（本批）：本模块**只有占位**，不存在任何 GPUI 依赖。
//! 阶段二：`ui = ["dep:gpui"]` 打开后在此实现视图。
//!
//! 渲染层约束：
//! - 数据只来自 [`crate::state`] 与 [`crate::theme`]，**不实现业务语义**。
//! - **不持有凭据**；不直连运行时。
//! - 中文输入依赖 Windows IME 修复（见任务卡 `d-ime-*`），修复前不得声称输入可用。

/// 阶段二入口占位。
///
/// 现在只用于确认「不开 `ui` feature 也能整体编译与测试」这一双门禁约定。
#[must_use]
pub fn renderer_pending() -> &'static str {
    "gpui renderer not yet wired (phase 2)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_two_entry_is_explicitly_pending() {
        assert!(renderer_pending().contains("not yet wired"));
    }
}
