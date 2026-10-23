//! 输入区状态：多行输入 + 模式选择 + 发送/停止按钮 + `@` 文件引用。
//!
//! 五个模式与参考客户端一致：quick / ask / plan / craft / expert。
//! 本模块只管输入与提交语义，不接运行时；发送后的投递由调用方负责。
//! 发送必须由用户显式触发，**不自动发送**，空输入不得发送。
//!
//! 对齐 A5 / A11 / A12：**A11** 模式值由 [`InputMode::ALL`] 给出（界面层做下拉）；
//! **A5** 等待确认期间按钮形态是「停止」，本层只**产出停止意图**（[`StopIntent`]）、
//! **不执行**停止；**A12** 候选文件列表**由调用方注入**
//! （[`InputAreaState::set_candidates`]），本仓**不读盘**、不扫描文件系统。
//!
//! 目录结构：`input_area/mod.rs` + `state.rs`（核心方法）+ `mention.rs`（`@` 提及）
//! + `chips.rs`（附件 chip）+ `tests.rs`（原 `mod tests`）。公开 API 与拆分前一字不变。

mod chips;
mod mention;
mod state;
#[cfg(test)]
mod tests;

pub use chips::{chip_label, chip_order};

/// 会话模式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputMode {
    Quick,
    /// 默认模式：问答。
    #[default]
    Ask,
    Plan,
    Craft,
    Expert,
}

impl InputMode {
    /// 界面上的模式顺序。
    pub const ALL: [Self; 5] = [
        Self::Quick,
        Self::Ask,
        Self::Plan,
        Self::Craft,
        Self::Expert,
    ];

    /// 展示文案。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Quick => "快速",
            Self::Ask => "问答",
            Self::Plan => "规划",
            Self::Craft => "执行",
            Self::Expert => "专家",
        }
    }

    /// 稳定标识（传给运行时，不用文案派发）。
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Ask => "ask",
            Self::Plan => "plan",
            Self::Craft => "craft",
            Self::Expert => "expert",
        }
    }

    /// 按 id 查找（未知值返回 `None`，不静默回落到某个模式）。
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.id() == id)
    }
}

/// 停止意图：界面层只上报「用户要求停止」，**不执行停止**。
///
/// 真正的 `agent:stop` 属运行时职责，界面层不得越界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopIntent {
    /// 停止时所处的会话模式。
    pub mode: InputMode,
}

/// 发送按钮的两种形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendButtonKind {
    /// 可发送：显示「发送」。
    Send,
    /// 等待确认中：显示「停止」。
    Stop,
}

/// 发送按钮渲染描述。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendButton {
    /// 按钮文案：「发送」/「停止」。
    pub label: &'static str,
    /// 是否可点。
    pub enabled: bool,
    /// 按钮形态（决定文案、配色与点击行为）。
    pub kind: SendButtonKind,
}

/// 一次待投递的输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingInput {
    pub text: String,
    pub mode: InputMode,
}

/// 输入区状态。
///
/// 默认值：空草稿、问答模式、不在等待态、无候选路径。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputAreaState {
    pub(super) draft: String,
    pub(super) mode: InputMode,
    /// 已提交、等待运行时确认中；此期间禁止重复发送。
    pub(super) in_flight: bool,
    /// 候选文件路径（由调用方注入；本仓不读盘）。
    pub(super) candidates: Vec<String>,
    /// 附件路径（由调用方注入；本仓不读盘、不发上传请求）。
    pub(super) attachments: Vec<String>,
}
