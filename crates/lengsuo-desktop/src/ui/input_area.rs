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

use std::collections::HashSet;

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
    draft: String,
    mode: InputMode,
    /// 已提交、等待运行时确认中；此期间禁止重复发送。
    in_flight: bool,
    /// 候选文件路径（由调用方注入；本仓不读盘）。
    candidates: Vec<String>,
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

    /// 注入候选路径（去重、保持顺序）。
    ///
    /// 候选列表由**调用方**提供（宿主 Glob 结果）；本仓不读盘、不扫描文件系统。
    pub fn set_candidates(&mut self, paths: impl IntoIterator<Item = String>) {
        let mut seen: HashSet<String> = HashSet::new();
        self.candidates = paths
            .into_iter()
            .map(|p| p.trim().to_owned())
            .filter(|p| !p.is_empty() && seen.insert(p.clone()))
            .collect();
    }

    /// 当前候选路径（只读）。
    #[must_use]
    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    /// 按查询词过滤候选（大小写不敏感）；空查询返回全部。
    ///
    /// 命中规则：路径**前缀**或路径**子串**包含查询词即命中（`src/a.rs`
    /// 用 `a` 也能命中，方便只想记文件名时快选）。
    #[must_use]
    pub fn filter_candidates(&self, query: &str) -> Vec<String> {
        let q = query.trim().to_lowercase();
        self.candidates
            .iter()
            .filter(|p| q.is_empty() || p.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    /// 从草稿里解析 `@` 之后的查询词（到下一个空白为止）；没有 `@` 返回 `None`。
    ///
    /// `@` 必须在词首（草稿开头或前面是空白）才算引用标记，
    /// 否则 `someone@example.com` 这类邮箱不会被误判成文件引用。
    #[must_use]
    pub fn mention_query(&self) -> Option<String> {
        let idx = self.mention_index()?;
        Some(self.draft[idx + 1..].to_owned())
    }

    /// 草稿里 `@` 标记的字节下标（词首的 `@`）。
    ///
    /// 找不到合法的 `@` 标记时返回 `None`。解析与插入共用这一处判定，
    /// 避免「查询解析得到的位置」与「实际替换的位置」不一致。
    #[must_use]
    pub fn mention_index(&self) -> Option<usize> {
        let idx = self.draft.rfind('@')?;
        if idx > 0 {
            let prev = self.draft[..idx].chars().next_back()?;
            if !prev.is_whitespace() {
                return None;
            }
        }
        let tail = &self.draft[idx + 1..];
        if tail.contains(char::is_whitespace) {
            return None;
        }
        Some(idx)
    }

    /// 把候选路径插入草稿：替换掉 `@查询词`，得到 `@路径 `。
    pub fn insert_mention(&mut self, path: &str) {
        let Some(idx) = self.mention_index() else {
            return;
        };
        let mut next = String::with_capacity(self.draft.len() + path.len() + 1);
        next.push_str(&self.draft[..idx]);
        next.push('@');
        next.push_str(path);
        next.push(' ');
        self.draft = next;
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：保持测试只依赖公开契约。
    use crate::ui::input_area::{InputAreaState, InputMode, SendButtonKind, StopIntent};

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

    #[test]
    fn mode_is_remembered_across_sends() {
        let mut area = InputAreaState::new();
        area.set_mode(InputMode::Expert);
        area.set_draft("第一问");
        let first = area.submit().expect("应可发送");
        area.finish_send();
        area.set_draft("第二问");
        let second = area.submit().expect("应可发送");
        assert_eq!(first.mode, InputMode::Expert);
        assert_eq!(second.mode, InputMode::Expert, "模式在会话内必须保持");
        assert_eq!(area.mode(), InputMode::Expert);
    }

    // ==== A5：发送后变停止按钮 ====
    #[test]
    fn send_button_turns_into_stop_while_waiting_for_ack() {
        let mut area = InputAreaState::new();
        area.set_draft("你好");
        let send = area.send_button();
        assert_eq!(send.kind, SendButtonKind::Send);
        assert_eq!(send.label, "发送");
        assert!(send.enabled);

        area.submit().expect("应可发送");
        let stop = area.send_button();
        assert_eq!(stop.kind, SendButtonKind::Stop, "等待确认期间应为停止按钮");
        assert_eq!(stop.label, "停止");
        assert!(stop.enabled, "停止按钮始终可点");
    }

    #[test]
    fn take_stop_yields_intent_and_restores_send_button() {
        let mut area = InputAreaState::new();
        area.set_mode(InputMode::Craft);
        area.set_draft("干活");
        area.submit().expect("应可发送");

        let intent = area.take_stop().expect("等待态应产出停止意图");
        assert_eq!(
            intent,
            StopIntent {
                mode: InputMode::Craft
            }
        );
        assert!(!area.is_in_flight(), "产出停止意图后解除等待态");
        let send = area.send_button();
        assert_eq!(send.kind, SendButtonKind::Send);
        assert_eq!(send.label, "发送");
        assert!(!send.enabled, "草稿已清空，发送仍不可用");
    }

    #[test]
    fn take_stop_returns_none_when_not_waiting() {
        let mut area = InputAreaState::new();
        assert!(area.take_stop().is_none(), "非等待态不得产出停止意图");
        area.set_draft("你好");
        area.submit().expect("应可发送");
        area.take_stop().expect("第一次应产出");
        assert!(area.take_stop().is_none(), "只能产出一次");
    }

    // ==== A12：@ 文件引用 ====

    #[test]
    fn set_candidates_dedupes_and_drops_blanks() {
        let mut area = InputAreaState::new();
        area.set_candidates([
            "src/main.rs".to_owned(),
            "  src/main.rs  ".to_owned(),
            "   ".to_owned(),
            "src/lib.rs".to_owned(),
        ]);
        assert_eq!(
            area.candidates(),
            ["src/main.rs".to_owned(), "src/lib.rs".to_owned()]
        );
    }

    #[test]
    fn filter_candidates_is_case_insensitive_and_empty_query_returns_all() {
        let mut area = InputAreaState::new();
        area.set_candidates([
            "src/Main.rs".to_owned(),
            "docs/readme.md".to_owned(),
            "src/lib.rs".to_owned(),
        ]);
        assert_eq!(area.filter_candidates("").len(), 3, "空查询返回全部");
        assert_eq!(
            area.filter_candidates("SRC"),
            ["src/Main.rs".to_owned(), "src/lib.rs".to_owned()],
            "前缀匹配且大小写不敏感"
        );
        assert_eq!(
            area.filter_candidates("readme"),
            ["docs/readme.md".to_owned()]
        );
        assert!(area.filter_candidates("nope").is_empty());
    }

    #[test]
    fn mention_query_parses_word_after_at_sign() {
        let mut area = InputAreaState::new();
        area.set_draft("看下 @src/mai");
        assert_eq!(area.mention_query().as_deref(), Some("src/mai"));
        area.set_draft("看下 @src/main.rs 谢谢");
        assert_eq!(area.mention_query(), None, "@ 后含空白就不是 @ 查询");
        area.set_draft("没有 @ 符号");
        assert_eq!(area.mention_query(), None);
    }

    #[test]
    fn mention_query_ignores_at_sign_inside_a_word() {
        let mut area = InputAreaState::new();
        area.set_draft("someone@example.com");
        assert_eq!(area.mention_query(), None, "邮箱里的 @ 不是引用标记");
        area.set_draft("@src/main.rs");
        assert_eq!(area.mention_query().as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn insert_mention_replaces_query_with_path() {
        let mut area = InputAreaState::new();
        area.set_draft("看下 @src/mai");
        area.insert_mention("src/main.rs");
        assert_eq!(area.draft(), "看下 @src/main.rs ");
        assert_eq!(area.mention_query(), None, "插入后不应再是 @ 查询");
    }

    #[test]
    fn insert_mention_is_a_noop_without_at_query() {
        let mut area = InputAreaState::new();
        area.set_draft("普通输入");
        area.insert_mention("src/main.rs");
        assert_eq!(area.draft(), "普通输入", "没有 @ 查询时不得改动草稿");
    }
}
