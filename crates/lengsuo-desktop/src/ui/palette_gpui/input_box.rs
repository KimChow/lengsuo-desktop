//! 面板搜索框：搜索框 → 关键词的写回方向（纯逻辑）+ [`Textarea`] 元素。
//!
//! 双向绑定：
//! - **搜索框 → 关键词**：编辑器发 [`InputEvent::Change`] 时，取编辑器的完整值，
//!   与关键词做**按 `char`** 的前后缀比较（[`diff_input`]）——插入取多出的字符、
//!   删除取去掉的字符，再分别走 [`PaletteState::insert_char`] 与
//!   [`PaletteState::backspace`]。全程在 `char` 边界上，中文 / emoji 不会被切半个。
//! - **关键词 → 搜索框**：打开面板、Esc 关闭后重新打开、宿主经 `state_mut()`
//!   改状态时，由 [`PanelSync::pull_into_search_box`] 给出要写回的值。
//!
//! 订阅句柄存放在 `CommandPaletteGpui::_subscription`（drop 即退订）。
//!
//! ⚠️ 输入路径已接 `Textarea`（GPUI 的 `EntityInputHandler` 负责 IME 组合文本），
//! **Windows 中文 IME 可用性未验证**，取决于独立的 IME 修复卡。
#![cfg(feature = "ui")]

use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::palette_state::PaletteState;
use crate::ui::command_palette::{self, CommandPaletteView};

/// 搜索框占位文案。
///
/// 直接复用描述层的常量（它也出现在 `CommandPaletteView::placeholder` 里），
/// 不在这里另写一份字面量。
pub(super) const PLACEHOLDER: &str = command_palette::PLACEHOLDER;

/// 编辑器内容相对旧值的一次编辑（全部基于 `char`，不是字节）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TextEdit {
    /// 在 `at`（字符序号）处插入了 `text`。
    Insert { at: usize, text: String },
    /// 从 `start`（字符序号）起删掉了 `removed`。
    Delete { start: usize, removed: String },
    /// 不是「插入 / 删除」这一形态的变化 —— 调用方改为整段写回关键词。
    Replace,
}

/// 比较旧值与新值：只认可**一个连续区间**被插入或删除的编辑。
///
/// 其余情形（同时增删、整段替换、中间改动且长度不变）返回 [`TextEdit::Replace`]。
pub(super) fn diff_input(old: &str, new: &str) -> TextEdit {
    if old == new {
        return TextEdit::Replace;
    }
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();

    // 公共前缀长度（字符数）。
    let prefix = old_chars
        .iter()
        .zip(new_chars.iter())
        .take_while(|(a, b)| a == b)
        .count();
    // 公共后缀长度（不越过前缀）。
    let suffix = old_chars[prefix..]
        .iter()
        .rev()
        .zip(new_chars[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();

    let old_mid = &old_chars[prefix..old_chars.len() - suffix];
    let new_mid = &new_chars[prefix..new_chars.len() - suffix];
    match (old_mid.is_empty(), new_mid.is_empty()) {
        (true, false) => TextEdit::Insert {
            at: prefix,
            text: new_mid.iter().collect(),
        },
        (false, true) => TextEdit::Delete {
            start: prefix,
            removed: old_mid.iter().collect(),
        },
        _ => TextEdit::Replace,
    }
}

/// 双向同步账本：记住上一轮两边的值，用来判断**哪一侧**真的变了。
///
/// 纯数据、不依赖窗口，可直接单测。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PanelSyncState {
    /// 上一轮搜索框（编辑器）里的值。
    last_textarea: String,
    /// 上一轮面板关键词。
    last_keyword: String,
}

/// 写回探针：两边的同步去向。
pub(super) trait PanelSync {
    /// 搜索框的新值 ⇒ 写入关键词。返回关键词是否真的变了。
    fn accept_search_box(&mut self, state: &mut PaletteState, text: &str) -> bool;

    /// 关键词 ⇒ 搜索框。返回需要写进编辑器的值；一致时返回 `None`。
    ///
    /// 关键词被外部改写（例如打开面板时清空）⇒ 返回 `Some(关键词)`，
    /// 回写路径据此把编辑器的值也清空。
    fn pull_into_search_box(&mut self, state: &PaletteState, current: &str) -> Option<String>;
}

impl PanelSync for PanelSyncState {
    fn accept_search_box(&mut self, state: &mut PaletteState, text: &str) -> bool {
        if text == state.keyword() {
            self.last_textarea = text.to_owned();
            self.last_keyword = text.to_owned();
            return false;
        }

        // 差分基准：编辑器里的旧值与关键词以**严格一致**者为准。
        // （账本可能过时，例如宿主在构造后直接设过关键词。）
        let base = if self.last_textarea == self.last_keyword {
            self.last_textarea.clone()
        } else {
            state.keyword().to_owned()
        };

        if text.is_empty() {
            // 清空（含编辑器内全选删除）：一次清到底，省得逐字退格。
            state.set_keyword("");
        } else {
            match diff_input(&base, text) {
                TextEdit::Insert { text, .. } => {
                    for c in text.chars() {
                        state.insert_char(c);
                    }
                }
                TextEdit::Delete { removed, .. } => {
                    for _ in removed.chars() {
                        state.backspace();
                    }
                }
                // 非单点编辑：以编辑器为准整段写回。
                TextEdit::Replace => state.set_keyword(text),
            }
        }

        self.last_textarea = text.to_owned();
        self.last_keyword = state.keyword().to_owned();
        true
    }

    fn pull_into_search_box(&mut self, state: &PaletteState, current: &str) -> Option<String> {
        let keyword = state.keyword();
        if current == keyword {
            self.last_textarea = current.to_owned();
            self.last_keyword = keyword.to_owned();
            return None;
        }
        if self.last_keyword == keyword {
            // 关键词这一侧没变 ⇒ 变的是搜索框：交给 `accept_search_box` 处理，
            // 不在这里拿旧关键词把它盖掉（否则输入会被吞）。
            return None;
        }
        self.last_textarea = keyword.to_owned();
        self.last_keyword = keyword.to_owned();
        Some(keyword.to_owned())
    }
}

/// 顶部搜索框：「>」提示符 + 可编辑的 [`Textarea`] + 描述层给的占位提示。
pub(super) fn render_search_box(
    textarea: &Entity<TextareaState>,
    view: &CommandPaletteView,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(">"),
        )
        .child(
            div()
                .flex_1()
                .child(Textarea::new(textarea).appearance(false)),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(view.placeholder),
        )
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
    use super::PLACEHOLDER;
    use crate::ui::command_palette;

    #[test]
    fn placeholder_is_the_description_layers_own_text() {
        assert_eq!(PLACEHOLDER, command_palette::PLACEHOLDER);
    }
}
