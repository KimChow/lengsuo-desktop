//! 「视图 ← 运行时」数据源契约（**消费侧**，纯逻辑层：无 GPUI、无 IO、无 async）。
//!
//! ⚠️ 本文件当前是**占位骨架**：只落盘 [`LoadState`] 与 [`Provenance`] 两个基础类型
//! 与模块登记。由 `ui-datasource-contract` 卡按 `docs/ui-runtime-contract.md` §2
//! 把 C1~C5（会话流增量 / 工具卡片 / 审批 / 停止投递 / 终端 PTY）的**消费侧类型**
//! 与 `ViewDataSource` trait 补齐。
//!
//! 纪律（`docs/ui-runtime-contract.md` §1 第 3 条）：
//! - 契约未就绪 ⇒ 界面**保持演示数据 + 如实标注**；⛔ 不许伪造服务端数据、⛔ 不许假装已接通。
//! - **fail-closed**：加载失败必须能表示成 [`LoadState::Failed`]，界面显示错误，
//!   ⛔ 不许静默退化成空列表。
//! - 本层**只描述界面怎么消费**，⛔ 不实现任何运行时语义（那是运行时/服务端线的活）。

/// 一次视图读取的三态（fail-closed：失败必须能被表达，不能静默退化成空）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState<T> {
    /// 正在加载（界面显示加载态）。
    Loading,
    /// 成功：界面可渲染这份快照。
    Ready(T),
    /// 失败：带原因（界面**必须**显示错误文案）。
    Failed(String),
}

impl<T> LoadState<T> {
    /// 是否已就绪。
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// 失败原因；未失败 ⇒ `None`。
    #[must_use]
    pub fn failure(&self) -> Option<&str> {
        match self {
            Self::Failed(reason) => Some(reason.as_str()),
            _ => None,
        }
    }
}

/// 数据来源标注：界面**如实显示**当前是演示数据还是真实运行时数据。
///
/// 契约未就绪时界面必须仍标 [`Provenance::Demo`]，⛔ 不许标成 `Runtime`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// 演示数据（占位）。
    Demo,
    /// 真实运行时/服务端数据。
    Runtime,
}

impl Provenance {
    /// 是否仍是演示数据。
    #[must_use]
    pub fn is_demo(self) -> bool {
        matches!(self, Self::Demo)
    }
}

#[cfg(test)]
mod tests {
    use super::{LoadState, Provenance};

    #[test]
    fn failure_carries_its_reason() {
        let s: LoadState<u8> = LoadState::Failed("运行时未接入".to_string());
        assert_eq!(s.failure(), Some("运行时未接入"));
        assert!(!s.is_ready());
    }

    #[test]
    fn ready_has_no_failure() {
        let s: LoadState<u8> = LoadState::Ready(1);
        assert!(s.is_ready());
        assert_eq!(s.failure(), None);
    }

    #[test]
    fn demo_is_not_runtime() {
        assert!(Provenance::Demo.is_demo());
        assert!(!Provenance::Runtime.is_demo());
    }
}
