//! 输入区状态：多行输入 + 模式选择 + 发送按钮。
//!
//! 五个模式与参考客户端一致：quick / ask / plan / craft / expert。
//! 本模块只管输入与提交语义，不接运行时；发送后的投递由调用方负责。
//! 发送必须由用户显式触发，**不自动发送**，空输入不得发送。

/// 会话模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Quick,
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

/// 发送按钮的渲染描述。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendButton {
    pub label: &'static str,
    pub enabled: bool,
}

/// 一次待投递的输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingInput {
    pub text: String,
    pub mode: InputMode,
}

/// 输入区状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputAreaState {
    draft: String,
    mode: InputMode,
    /// 已提交、等待运行时确认中；此期间禁止重复发送。
    in_flight: bool,
}

impl Default for InputAreaState {
    fn default() -> Self {
        Self {
            draft: String::new(),
            mode: InputMode::Ask,
            in_flight: false,
        }
    }
}

impl InputAreaState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn draft(&self) -> &str {
        &self.draft
    }

    /// 设置草稿：允许换行（多行输入）。
    pub fn set_draft(&mut self, text: impl Into<String>) {
        self.draft = text.into();
    }

    /// 追加一次按键输入（含换行）。
    pub fn push_str(&mut self, text: &str) {
        self.draft.push_str(text);
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        self.draft.lines().count().max(1)
    }

    #[must_use]
    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// 切换模式。未知 id 返回 `false` 且不改变当前模式。
    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    /// 按 id 切换模式。
    pub fn set_mode_id(&mut self, id: &str) -> bool {
        match InputMode::from_id(id) {
            Some(m) => {
                self.mode = m;
                true
            }
            None => false,
        }
    }

    #[must_use]
    pub fn is_in_flight(&self) -> bool {
        self.in_flight
    }

    /// 运行时确认已受理，解除等待。
    pub fn finish_send(&mut self) {
        self.in_flight = false;
    }

    /// 只有非空草稿且没有未完成的发送时才可发送。
    #[must_use]
    pub fn can_send(&self) -> bool {
        !self.in_flight && !self.draft.trim().is_empty()
    }

    /// 发送按钮渲染描述。
    #[must_use]
    pub fn send_button(&self) -> SendButton {
        SendButton {
            label: "发送",
            enabled: self.can_send(),
        }
    }

    /// 提交输入。空输入或发送中返回 `None`（不清空、不投递）。
    pub fn submit(&mut self) -> Option<OutgoingInput> {
        if !self.can_send() {
            return None;
        }
        let text = self.draft.trim().to_owned();
        self.draft.clear();
        self.in_flight = true;
        Some(OutgoingInput {
            text,
            mode: self.mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_five_modes_have_unique_ids() {
        assert_eq!(InputMode::ALL.len(), 5);
        let mut ids: Vec<&str> = InputMode::ALL.iter().map(|m| m.id()).collect();
        assert_eq!(ids, ["quick", "ask", "plan", "craft", "expert"]);
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len());
        for m in InputMode::ALL {
            assert_eq!(InputMode::from_id(m.id()), Some(m));
        }
        assert_eq!(InputMode::from_id("turbo"), None);
    }

    #[test]
    fn empty_input_cannot_be_sent() {
        let mut area = InputAreaState::new();
        assert!(!area.can_send());
        assert!(!area.send_button().enabled);
        assert_eq!(area.submit(), None);
        area.set_draft("   \n\t ");
        assert!(!area.can_send(), "纯空白也不得发送");
    }

    #[test]
    fn multiline_draft_is_preserved_and_sent_with_mode() {
        let mut area = InputAreaState::new();
        area.set_mode(InputMode::Plan);
        area.set_draft("第一行\n第二行\n");
        assert_eq!(area.line_count(), 2);
        assert!(area.send_button().enabled);
        let out = area.submit().expect("应可发送");
        assert_eq!(out.text, "第一行\n第二行");
        assert_eq!(out.mode, InputMode::Plan);
        assert_eq!(area.draft(), "");
        assert!(area.is_in_flight());
        assert!(!area.can_send(), "等待确认期间不得重复发送");
        assert_eq!(area.submit(), None);
        area.finish_send();
        assert_eq!(area.send_button().label, "发送");
    }

    #[test]
    fn set_mode_id_rejects_unknown_without_changing_mode() {
        let mut area = InputAreaState::new();
        assert_eq!(area.mode(), InputMode::Ask);
        assert!(!area.set_mode_id("nope"));
        assert_eq!(area.mode(), InputMode::Ask, "未知模式不得静默切换");
        assert!(area.set_mode_id("expert"));
        assert_eq!(area.mode(), InputMode::Expert);
    }
}
