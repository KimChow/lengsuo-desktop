//! 宿主交给设置窗口的只读状态快照。
//!
//! 从 `mod.rs` 拆出（e50）：内容与行为逐字不变，只是换文件。
use crate::state::env_group::EnvGroupState;
use crate::state::hotkeys::{Binding, HotkeyRegistry};

/// 宿主交给设置窗口的**状态快照**（纯数据，可无窗口测）。
///
/// ⛔ **只读快照**：设置窗口拿到的是宿主当下的克隆，不是活引用 ⇒ 宿主后续变化
/// **不会**反映到**已打开**的设置窗口。
#[derive(Debug, Clone)]
pub(in crate::ui) struct SettingsSnapshot {
    /// 浏览器环境分组状态（→「数据管理」面板）。
    pub data: EnvGroupState,
    /// 快捷键绑定（→「个性化」面板）。
    pub bindings: Vec<Binding>,
}

/// 从宿主的两份状态取快照。
///
/// ⛔ **只读克隆**：设置窗口不与宿主共享状态 ⇒ 宿主后续变化**不会**反映到已开窗口。
pub(in crate::ui) fn settings_snapshot(
    env_state: &EnvGroupState,
    registry: &HotkeyRegistry,
) -> SettingsSnapshot {
    SettingsSnapshot {
        data: env_state.clone(),
        bindings: registry.bindings().to_vec(),
    }
}
