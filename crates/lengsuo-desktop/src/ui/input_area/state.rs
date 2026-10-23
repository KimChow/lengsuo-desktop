//! `InputAreaState` 的核心方法：草稿 / 模式 / 发送 / 停止。

use super::{InputAreaState, InputMode, OutgoingInput, SendButton, SendButtonKind, StopIntent};

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
    ///
    /// 等待确认期间（`in_flight`）形态是「停止」：文案变「停止」、可点，
    /// 点击后由接线层产出 [`StopIntent`]。
    #[must_use]
    pub fn send_button(&self) -> SendButton {
        if self.in_flight {
            SendButton {
                label: "停止",
                enabled: true,
                kind: SendButtonKind::Stop,
            }
        } else {
            SendButton {
                label: "发送",
                enabled: self.can_send(),
                kind: SendButtonKind::Send,
            }
        }
    }

    /// 产出停止意图（**不执行任何停止动作**）；返回 `None` 表示当前不在等待态。
    ///
    /// 产出后解除等待态，按钮恢复「发送」。
    pub fn take_stop(&mut self) -> Option<StopIntent> {
        if !self.in_flight {
            return None;
        }
        self.in_flight = false;
        Some(StopIntent { mode: self.mode })
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
