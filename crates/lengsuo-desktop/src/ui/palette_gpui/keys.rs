//! 按键转换：平台按键 → 既有的 [`Hotkey`] 形状 / 面板语义按键。
//!
//! 纯函数，不碰 GPUI 元素树之外的任何东西；测试随 `cargo test` 一起跑。
#![cfg(feature = "ui")]

use gpui_kit::Keystroke;

use crate::state::hotkeys::{Hotkey, Key as HotkeyKey, Modifiers as HotkeyModifiers};
use crate::ui::command_palette::PaletteKey;

/// 把平台按键转换成既有的 [`Hotkey`] 形状。
///
/// 转换规则与 [`Hotkey::parse`] 一致：ASCII 字母统一小写，修饰键**完全相等**才匹配。
#[must_use]
pub fn hotkey_from_keystroke(keystroke: &Keystroke) -> Option<Hotkey> {
    let modifiers = HotkeyModifiers {
        ctrl: keystroke.modifiers.control,
        shift: keystroke.modifiers.shift,
        alt: keystroke.modifiers.alt,
        meta: keystroke.modifiers.platform,
    };
    let key = match keystroke.key.as_str() {
        "escape" => HotkeyKey::Escape,
        "enter" => HotkeyKey::Enter,
        "backspace" => HotkeyKey::Backspace,
        "tab" => HotkeyKey::Tab,
        "space" => HotkeyKey::Space,
        "up" => HotkeyKey::Up,
        "down" => HotkeyKey::Down,
        "left" => HotkeyKey::Left,
        "right" => HotkeyKey::Right,
        other => {
            let mut chars = other.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            HotkeyKey::Character(if c.is_ascii_alphabetic() {
                c.to_ascii_lowercase()
            } else {
                c
            })
        }
    };
    Some(Hotkey::new(modifiers, key))
}

/// 把平台按键转换成面板语义按键。
///
/// 带 `Ctrl` / `Alt` / `Meta` 的组合键**不是**文本输入（否则 `Ctrl+S` 会把
/// `s` 打进搜索框）；命名键（方向键、回车、Esc、退格）照常映射。
#[must_use]
pub fn palette_key_from_keystroke(keystroke: &Keystroke) -> Option<PaletteKey> {
    match keystroke.key.as_str() {
        "up" => return Some(PaletteKey::Up),
        "down" => return Some(PaletteKey::Down),
        "enter" => return Some(PaletteKey::Enter),
        "escape" => return Some(PaletteKey::Escape),
        "backspace" => return Some(PaletteKey::Backspace),
        _ => {}
    }
    if keystroke.modifiers.control || keystroke.modifiers.alt || keystroke.modifiers.platform {
        return None;
    }
    // `key_char` 是「这次按键能打出的字符」，中文输入法走的就是它。
    let text = keystroke
        .key_char
        .as_deref()
        .unwrap_or(keystroke.key.as_str());
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() || c.is_control() {
        return None;
    }
    Some(PaletteKey::Char(c))
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
    use super::{hotkey_from_keystroke, palette_key_from_keystroke};
    use crate::state::hotkeys::{command_ids, Hotkey};
    use crate::ui::command_palette::PaletteKey;
    use gpui_kit::{Keystroke, Modifiers};

    /// 造一个平台按键事件。
    fn keystroke(key: &str, key_char: Option<&str>, ctrl: bool, shift: bool) -> Keystroke {
        Keystroke {
            modifiers: Modifiers {
                control: ctrl,
                shift,
                ..Modifiers::default()
            },
            key: key.to_owned(),
            key_char: key_char.map(str::to_owned),
        }
    }

    #[test]
    fn named_keys_map_to_palette_keys() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("up", None, false, false)),
            Some(PaletteKey::Up)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("down", None, false, false)),
            Some(PaletteKey::Down)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("enter", None, false, false)),
            Some(PaletteKey::Enter)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("escape", None, false, false)),
            Some(PaletteKey::Escape)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("backspace", None, false, false)),
            Some(PaletteKey::Backspace)
        );
    }

    #[test]
    fn plain_letters_become_char_keys() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("k", Some("k"), false, false)),
            Some(PaletteKey::Char('k'))
        );
    }

    #[test]
    fn ime_character_is_preferred_over_the_raw_key() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("a", Some("设"), false, false)),
            Some(PaletteKey::Char('设'))
        );
    }

    #[test]
    fn control_combinations_are_not_text_input() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("s", Some("s"), true, false)),
            None,
            "Ctrl+S 不能把 s 打进搜索框"
        );
    }

    #[test]
    fn ctrl_k_converts_to_the_registered_hotkey() {
        let hotkey = hotkey_from_keystroke(&keystroke("k", Some("k"), true, false))
            .expect("Ctrl+K 应能转换");
        assert_eq!(
            hotkey,
            Hotkey::parse("Ctrl+K").expect("内置快捷键文本必须可解析")
        );
    }

    #[test]
    fn ctrl_shift_p_converts_to_the_registered_hotkey() {
        let hotkey = hotkey_from_keystroke(&keystroke("p", Some("P"), true, true))
            .expect("Ctrl+Shift+P 应能转换");
        assert_eq!(
            hotkey,
            Hotkey::parse("Ctrl+Shift+P").expect("内置快捷键文本必须可解析")
        );
    }

    #[test]
    fn the_default_registry_opens_the_palette() {
        let registry = super::super::default_registry();
        assert_eq!(
            registry.command_for(Hotkey::parse("Ctrl+K").expect("可解析")),
            Some(command_ids::PALETTE_OPEN)
        );
    }
}
